// File: src/io/location.rs
// Project: Bifrost
// Creation date: Sunday 09 February 2025
// Author: Vincent Berthier <vincent.berthier@posteo.org>
// -----
// Last modified: Sunday 09 February 2025 @ 21:08:15
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

use std::{
    collections::{hash_map::Entry, HashMap},
    path::PathBuf,
    sync::LazyLock,
};

use borsh::{BorshDeserialize, BorshSerialize};
use tokio::sync::RwLock;
use tracing::{debug, instrument, trace};

use crate::account::Wallet;

use super::{
    support::{append_to_file, read_from_file_map},
    vault::get_vault_path,
    Result,
};

static SLOT_ID: LazyLock<RwLock<HashMap<u64, u8>>> = LazyLock::new(|| RwLock::new(HashMap::new()));

#[cfg(test)]
#[cfg_attr(test, mutants::skip)]
pub const MAX_ACCOUNT_FILE_SIZE: u64 = 250;

#[cfg(not(test))]
#[cfg_attr(not(test), mutants::skip)]
pub const MAX_ACCOUNT_FILE_SIZE: u64 = 10 * 1024 * 1024;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct AccountDiskLocation {
    pub slot: u64,
    pub id: u8,
    pub offset: u64,
    pub size: u64,
}

impl AccountDiskLocation {
    pub async fn read(&self) -> Result<Wallet> {
        let path = Self::get_path(self.slot, self.id);
        read_from_file_map(path, self.offset, self.size).await
    }

    #[instrument(skip(account))]
    pub async fn new_from_write<A>(account: &A, slot: u64) -> Result<Self>
    where
        A: BorshSerialize + Send + Sync,
    {
        debug!("writing new account");
        let id = Self::slot_id(slot).await;
        trace!(id, "got current file id");

        let path = Self::get_path(slot, id);
        let (size, offset) = append_to_file(path, &account).await?;
        trace!(size, offset, "file is written to disk");

        if size + offset >= MAX_ACCOUNT_FILE_SIZE {
            trace!(
                "current file is {} bytes, max is {MAX_ACCOUNT_FILE_SIZE}",
                size + offset
            );
            Self::next_id(slot).await;
        }

        Ok(Self {
            slot,
            id,
            offset,
            size,
        })
    }

    #[instrument]
    pub async fn next_id(slot: u64) {
        debug!(slot, "going to the next file id");
        match SLOT_ID.write().await.entry(slot) {
            Entry::Vacant(entry) => {
                entry.insert(Self::get_id_from_files(slot) + 1);
            }
            Entry::Occupied(entry) => *entry.into_mut() += 1,
        }
    }

    #[instrument]
    async fn slot_id(slot: u64) -> u8 {
        debug!("getting file id");
        if let Some(&id) = SLOT_ID.read().await.get(&slot) {
            return id;
        }
        let id = Self::get_id_from_files(slot);
        SLOT_ID.write().await.insert(slot, id);
        id
    }

    #[expect(clippy::unwrap_used)]
    #[instrument]
    fn get_id_from_files(slot: u64) -> u8 {
        debug!("retrieving the slot id from the files");
        let path = get_vault_path().join("accounts");
        let filter = format!("{slot}.");
        std::fs::read_dir(path)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .filter(|name| name.starts_with(&filter))
            .map(|name| name.split('.').next_back().unwrap().parse().unwrap())
            .max()
            .unwrap_or_default()
    }

    fn get_path(slot: u64, id: u8) -> PathBuf {
        get_vault_path()
            .join("accounts")
            .join(format!("{slot}.{id}"))
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {

    use test_log::test;

    use crate::io::support::write_to_file;
    use crate::io::vault::{set_vault_path, Vault};

    use super::*;
    type TestResult = core::result::Result<(), Box<dyn core::error::Error>>;

    #[test(tokio::test)]
    async fn slot_id_increase() -> TestResult {
        const VAULT: &str = "/tmp/bifrost/location-1";
        set_vault_path(VAULT);
        Vault::init_vault().await?;
        // When
        AccountDiskLocation::next_id(0).await;

        // Then
        assert_eq!(AccountDiskLocation::slot_id(0).await, 1);
        assert_eq!(MAX_ACCOUNT_FILE_SIZE, 250);

        Ok(())
    }

    #[expect(clippy::default_numeric_fallback)]
    #[test(tokio::test)]
    async fn slot_from_file() -> TestResult {
        // Given
        const VAULT: &str = "/tmp/bifrost/location-2";
        set_vault_path(VAULT);
        Vault::init_vault().await?;
        write_to_file(get_vault_path().join("accounts").join("0.0"), &[1, 2, 3]).await?;
        write_to_file(get_vault_path().join("accounts").join("0.1"), &[1, 2, 3]).await?;
        write_to_file(get_vault_path().join("accounts").join("0.2"), &[1, 2, 3]).await?;
        write_to_file(get_vault_path().join("accounts").join("0.4"), &[1, 2, 3]).await?;

        // When
        let id = AccountDiskLocation::get_id_from_files(0);

        // Then
        assert_eq!(id, 4);

        Ok(())
    }
}
