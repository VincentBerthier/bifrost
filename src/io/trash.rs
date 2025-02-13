// File: src/io/trash.rs
// Project: Bifrost
// Creation date: Monday 10 February 2025
// Author: Vincent Berthier <vincent.berthier@posteo.org>
// -----
// Last modified: Thursday 13 February 2025 @ 09:51:51
// Modified by: Vincent Berthier
// -----
// Copyright (c) 2025 <Vincent Berthier>
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the 'Software'), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED 'AS IS', WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use std::{collections::HashMap, path::PathBuf};

use borsh::{BorshDeserialize, BorshSerialize};
use tracing::{debug, instrument, trace, warn};

use crate::io::support::write_to_file;

use super::{
    location::AccountDiskLocation, support::read_from_file, vault::get_vault_path, Error, Result,
    MAX_ACCOUNT_FILE_SIZE,
};

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Hash,
    PartialEq,
    Eq,
    BorshDeserialize,
    BorshSerialize,
    PartialOrd,
    Ord,
)]
pub struct AccountFile {
    pub slot: u64,
    pub id: u8,
}

impl AccountFile {
    const fn from_loc(loc: AccountDiskLocation) -> Self {
        Self {
            slot: loc.slot,
            id: loc.id,
        }
    }
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Hash,
    PartialEq,
    Eq,
    BorshDeserialize,
    BorshSerialize,
    PartialOrd,
    Ord,
)]
struct Loc {
    offset: u64,
    size: u64,
}

impl Loc {
    const fn from_loc(loc: AccountDiskLocation) -> Self {
        Self {
            offset: loc.offset,
            size: loc.size,
        }
    }
}

#[derive(Default, BorshDeserialize, BorshSerialize, Debug, Clone, PartialEq, Eq)]
pub struct Trash {
    trash: HashMap<AccountFile, Vec<Loc>>,
}

impl Trash {
    #[instrument]
    pub async fn load_or_create() -> Self {
        debug!("initializing trash");
        if let Ok(trash) = Self::load_from_disk().await {
            trace!("trash could be reloaded from the disk");
            return trash;
        }

        warn!("trash could not be reloaded from the disk: starting from scratch");
        Self {
            trash: HashMap::new(),
        }
    }

    #[instrument]
    async fn load_from_disk() -> Result<Self> {
        let trash_path = Self::get_path();
        if !trash_path.exists() {
            return Err(Error::TrashFileNotFound);
        }
        read_from_file(trash_path).await
    }

    #[instrument(skip(self))]
    pub fn insert(&mut self, loc: AccountDiskLocation) -> Result<()> {
        debug!("adding location to the trash");
        let entry = self.trash.entry(AccountFile::from_loc(loc)).or_default();
        let file_loc = Loc::from_loc(loc);

        if entry.contains(&file_loc) {
            return Err(Error::DuplicateLocationInTrash { loc });
        }

        entry.push(file_loc);
        Ok(())
    }

    #[instrument(skip(self))]
    pub fn remove(&mut self, file: &AccountFile) {
        debug!("removing location from the trash");
        self.trash.remove(file);
    }

    #[instrument(skip_all)]
    pub async fn save(&self) -> Result<()> {
        debug!("saving trash to file");
        write_to_file(Self::get_path(), self).await
    }

    #[expect(clippy::integer_division)]
    #[instrument(skip_all)]
    pub async fn get_files_to_clean(&self) -> Vec<AccountFile> {
        self.trash
            .iter()
            .map(|(file, vloc)| (file, vloc.iter().fold(0_u64, |acc, loc| acc + loc.size)))
            .filter(|(_file, size)| *size >= MAX_ACCOUNT_FILE_SIZE / 2)
            .map(|(file, _s)| file)
            .copied()
            .collect()
    }

    pub fn len(&self) -> usize {
        self.trash.len()
    }

    fn get_path() -> PathBuf {
        get_vault_path().join("trash")
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {

    use std::assert_matches::assert_matches;
    use std::fs::remove_dir_all;
    use std::path::PathBuf;

    use test_log::test;

    use crate::account::Wallet;
    use crate::crypto::Keypair;
    use crate::io::vault::{set_vault_path, Vault};

    use super::super::Error;
    use super::*;
    type TestResult = core::result::Result<(), Box<dyn core::error::Error>>;

    fn reset_vault<P>(path: P) -> Result<()>
    where
        P: Into<PathBuf>,
    {
        let path = path.into();
        set_vault_path(&path);
        if path.exists() {
            remove_dir_all(path)?;
        }

        Ok(())
    }

    fn get_loc(slot: u64, id: u8, offset: u64, size: u64) -> AccountDiskLocation {
        AccountDiskLocation {
            slot,
            id,
            offset,
            size,
        }
    }

    #[test]
    fn cannot_put_same_loc_twice() -> TestResult {
        // Given
        let mut trash = Trash::default();
        let loc = get_loc(0, 0, 0, 0);
        trash.insert(loc)?;

        // When
        let res = trash.insert(loc);

        // Then
        assert_matches!(res, Err(Error::DuplicateLocationInTrash { .. }));

        Ok(())
    }

    #[expect(clippy::default_numeric_fallback)]
    #[test(tokio::test)]
    async fn find_files_to_clean() -> TestResult {
        // Given
        const VAULT: &str = "/tmp/bifrost/trash-1";
        reset_vault(VAULT)?;
        let mut vault = Vault::load_or_create().await?;
        let key = Keypair::generate().pubkey();

        for slot in 0..4 {
            for i in 0..100 {
                if i % 2 == 0 {
                    vault
                        .save_account(key, &Wallet { prisms: 983_373 }, slot)
                        .await?;
                } else {
                    vault
                        .save_account(Keypair::generate().pubkey(), &Wallet { prisms: 99 }, slot)
                        .await?;
                }
            }
        }
        vault.save().await?;

        // When
        let trash = Trash::load_or_create().await;
        let files_to_clean = trash.get_files_to_clean().await;

        // Then
        assert_eq!(files_to_clean.len(), 12, "{files_to_clean:?}");

        Ok(())
    }
}
