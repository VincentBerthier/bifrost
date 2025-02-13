// File: src/io/index.rs
// Project: Bifrost
// Creation date: Sunday 09 February 2025
// Author: Vincent Berthier <vincent.berthier@posteo.org>
// -----
// Last modified: Tuesday 11 February 2025 @ 11:31:28
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

use crate::{account::Wallet, crypto::Pubkey, io::support::write_to_file};

use super::{
    location::AccountDiskLocation, support::read_from_file, vault::get_vault_path, Error, Result,
};

#[derive(BorshSerialize, BorshDeserialize)]
pub struct Index {
    accounts: HashMap<Pubkey, AccountDiskLocation>,
}

impl Index {
    #[instrument]
    pub async fn load_or_create() -> Self {
        debug!("initializing index");
        if let Ok(index) = Self::load_from_disk().await {
            trace!("index could be reloaded from the disk");
            return index;
        }

        warn!("index could not be reloaded from the disk: starting from scratch");
        Self {
            accounts: HashMap::new(),
        }
    }

    #[instrument]
    async fn load_from_disk() -> Result<Self> {
        let index_path = Self::get_path();
        if !index_path.exists() {
            return Err(Error::IndexFileNotFound);
        }
        read_from_file(index_path).await
    }

    #[instrument(skip(self))]
    pub async fn load(&self, key: &Pubkey) -> Result<Option<Wallet>> {
        let Some(loc) = self.find(key) else {
            trace!("account was not found in the index");
            return Ok(None);
        };

        trace!("account was found, reading it from the disk");
        Some(loc.read().await).transpose()
    }

    pub fn find(&self, key: &Pubkey) -> Option<&AccountDiskLocation> {
        self.accounts.get(key)
    }

    #[instrument(skip_all, fields(%key))]
    pub fn set_account(&mut self, key: Pubkey, loc: AccountDiskLocation) {
        debug!("adding account to the index");
        self.accounts.insert(key, loc);
    }

    #[instrument(skip(self))]
    pub fn accounts_on_file(&self, slot: u64, id: u8) -> Vec<Pubkey> {
        self.accounts
            .iter()
            .filter(|(_key, loc)| loc.slot == slot && loc.id == id)
            .map(|(key, _loc)| key)
            .copied()
            .collect()
    }

    #[instrument(skip_all)]
    pub async fn save(&self) -> Result<()> {
        debug!("saving index to file");
        write_to_file(Self::get_path(), self).await
    }

    fn get_path() -> PathBuf {
        get_vault_path().join("index")
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    #![expect(clippy::unwrap_used)]

    use std::{
        assert_matches::assert_matches,
        fs::{remove_dir_all, OpenOptions},
        io::Write,
        path::Path,
    };

    use test_log::test;

    use crate::{
        account::Wallet,
        crypto::Keypair,
        io::{
            support::append_to_file,
            vault::{set_vault_path, Vault},
            MAX_ACCOUNT_FILE_SIZE,
        },
    };

    // use super::super::Error;
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

    async fn generate_dummy_index(vault_path: &str) -> TestResult {
        reset_vault(vault_path)?;
        Vault::init_vault().await?;
        let index_path = get_vault_path().join("index");

        let key = Keypair::generate().pubkey();
        let mut accounts = HashMap::new();
        accounts.insert(key, AccountDiskLocation::default());
        let dummy = Index { accounts };
        let mut index_file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(index_path)?;
        index_file.write_all(&borsh::to_vec(&dummy).unwrap())?;

        Ok(())
    }

    #[test(tokio::test)]
    async fn init_vault_folders() -> TestResult {
        // Given
        const VAULT: &str = "/tmp/bifrost/index-1";
        reset_vault(VAULT)?;

        // When
        Vault::init_vault().await?;

        // Then
        assert!(Path::new(VAULT).join("accounts").exists());
        assert!(Path::new(VAULT).join("blocks").exists());
        assert!(Path::new(VAULT).join("transactions").exists());
        Ok(())
    }

    #[test(tokio::test)]
    async fn load_index_from_disk() -> TestResult {
        // Given
        const VAULT: &str = "/tmp/bifrost/index-2";
        generate_dummy_index(VAULT).await?;

        // When
        let index = Index::load_from_disk().await?;

        // Then
        assert_eq!(index.accounts.len(), 1);
        Ok(())
    }

    #[test(tokio::test)]
    async fn add_and_find_account() -> TestResult {
        // Given
        const SLOT: u64 = 198;
        const VAULT: &str = "/tmp/bifrost/index-3";
        reset_vault(VAULT)?;
        Vault::init_vault().await?;
        let mut index = Index::load_or_create().await;
        let loc = AccountDiskLocation {
            slot: SLOT,
            id: 0,
            offset: 0,
            size: 0,
        };
        let key = Keypair::generate().pubkey();

        // When
        index.set_account(key, loc);

        // Then
        assert_matches!(index.find(&key), Some(l) if *l == loc);
        Ok(())
    }

    #[test(tokio::test)]
    async fn save_and_reload() -> TestResult {
        // Given
        const SLOT: u64 = 201;
        const VAULT: &str = "/tmp/bifrost/index-4";
        reset_vault(VAULT)?;
        Vault::init_vault().await?;
        let mut index = Index::load_or_create().await;
        let loc = AccountDiskLocation {
            slot: SLOT,
            id: 0,
            offset: 0,
            size: 0,
        };
        let key = Keypair::generate().pubkey();
        index.set_account(key, loc);

        // When
        index.save().await?;
        let reloaded = Index::load_from_disk().await?;

        // Then
        assert_matches!(reloaded.find(&key), Some(l) if *l == loc);

        Ok(())
    }

    #[test(tokio::test)]
    async fn cannot_save_if_vault_not_init() -> TestResult {
        // Given
        const VAULT: &str = "/tmp/bifrost/index-5";
        reset_vault(VAULT)?;
        let mut index = Index::load_or_create().await;
        let loc = AccountDiskLocation::default();
        let key = Keypair::generate().pubkey();
        index.set_account(key, loc);

        // When
        let res = index.save().await;

        // Then
        assert_matches!(res, Err(Error::FileSystem(err)) if matches!(err.kind(), std::io::ErrorKind::NotFound));

        Ok(())
    }

    #[test(tokio::test)]
    async fn load_account() -> TestResult {
        // Given
        const VAULT: &str = "/tmp/bifrost/index-6";
        const SLOT: u64 = 389;
        const ID: u8 = 5;
        reset_vault(VAULT)?;
        Vault::init_vault().await?;
        let account = Wallet { prisms: 398_399 };
        let path = get_vault_path()
            .join("accounts")
            .join(format!("{SLOT}.{ID}"));
        append_to_file(&path, &account).await?;
        append_to_file(&path, &account).await?;
        append_to_file(&path, &account).await?;
        let account_data = borsh::to_vec(&account)?;
        let len = account_data.len() as u64;

        let loc = AccountDiskLocation {
            slot: SLOT,
            id: ID,
            offset: len * 2,
            size: len,
        };

        // When
        let from_file = loc.read().await?;

        // Then
        assert_eq!(from_file, account);
        Ok(())
    }

    #[expect(clippy::default_numeric_fallback, clippy::integer_division)]
    #[test(tokio::test)]
    async fn find_accounts_on_file() -> TestResult {
        // Given
        const VAULT: &str = "/tmp/bifrost/index-7";
        const SLOT: u64 = 1;
        reset_vault(VAULT)?;
        let mut vault = Vault::load_or_create().await?;
        let key = Keypair::generate().pubkey();

        for i in 0..100 {
            if i % 2 == 0 {
                vault
                    .save_account(key, &Wallet { prisms: 983_373 }, SLOT)
                    .await?;
            } else {
                vault
                    .save_account(Keypair::generate().pubkey(), &Wallet { prisms: 99 }, SLOT)
                    .await?;
            }
        }
        vault.save().await?;
        let index = Index::load_from_disk().await?;

        // When
        let accounts_on_file = index.accounts_on_file(SLOT, 0);

        // Then
        let expected =
            MAX_ACCOUNT_FILE_SIZE / borsh::to_vec(&Wallet { prisms: 0 })?.len() as u64 / 2 + 1;
        assert_eq!(accounts_on_file.len() as u64, expected);

        Ok(())
    }
}
