// File: src/io/location.rs
// Project: Bifrost
// Creation date: Sunday 09 February 2025
// Author: Vincent Berthier <vincent.berthier@posteo.org>
// -----
// Last modified: Saturday 15 February 2025 @ 18:41:11
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

use std::path::{Path, PathBuf};

use borsh::{BorshDeserialize, BorshSerialize};
use tracing::{debug, instrument, warn};

use crate::{account::Wallet, io::MAX_ACCOUNT_FILE_SIZE};

use super::{
    support::{append_to_file, read_from_file_map},
    vault::get_vault_path,
    Result,
};

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct AccountDiskLocation {
    pub slot: u64,
    pub id: u8,
    pub offset: u64,
    pub size: u64,
}

impl AccountDiskLocation {
    pub async fn read(&self) -> Result<Wallet> {
        let path = get_account_path(self.slot, self.id);
        read_from_file_map(path, self.offset, self.size).await
    }
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

#[derive(Default)]
pub struct SlotWriter {
    slot: u64,
    id: u8,
    offset: u64,
    buffer: Vec<u8>,
    dropped: bool,
}

impl SlotWriter {
    #[instrument]
    pub fn new(slot: u64) -> Self {
        debug!("creating new slot writer");
        let id = get_id_from_files(slot);
        let offset = Path::new(&get_account_path(slot, id))
            .metadata()
            .map_or(0, |metadata| metadata.len());
        #[expect(clippy::cast_possible_truncation)]
        let buffer = Vec::with_capacity(MAX_ACCOUNT_FILE_SIZE as usize * 2);

        Self {
            slot,
            id,
            offset,
            buffer,
            dropped: false,
        }
    }

    pub const fn slot(&self) -> u64 {
        self.slot
    }

    #[expect(clippy::unwrap_used)]
    #[instrument(skip_all)]
    pub async fn append<A>(&mut self, account: A) -> Result<AccountDiskLocation>
    where
        A: BorshSerialize + Send + Sync,
    {
        let data = borsh::to_vec(&account).unwrap();
        let size = data.len() as u64;

        let res = self.get_account_loc(size);

        self.buffer.extend_from_slice(&data);
        self.offset += size;
        if self.offset >= MAX_ACCOUNT_FILE_SIZE {
            self.next_id().await?;
        }
        Ok(res)
    }

    async fn next_id(&mut self) -> Result<()> {
        self.flush().await?;
        self.id += 1;
        self.offset = 0;

        Ok(())
    }

    #[expect(clippy::cast_possible_truncation)]
    #[instrument(skip_all)]
    pub async fn flush(&mut self) -> Result<()> {
        debug!(slot = self.slot, id = self.id, "flushing account file");
        let mut data = Vec::with_capacity(MAX_ACCOUNT_FILE_SIZE as usize * 2);
        std::mem::swap(&mut data, &mut self.buffer);
        let slot = self.slot;
        let id = self.id;
        // tokio::spawn(async move {
        let path = get_account_path(slot, id);
        match append_to_file(path, &data).await {
            Ok(()) => (),
            Err(err) => warn!("could not write account data to file: {err}"),
        }
        // });

        Ok(())
    }

    const fn get_account_loc(&self, size: u64) -> AccountDiskLocation {
        AccountDiskLocation {
            slot: self.slot,
            id: self.id,
            offset: self.offset,
            size,
        }
    }
}

impl Drop for SlotWriter {
    #[instrument(skip(self))]
    fn drop(&mut self) {
        if !self.dropped {
            debug!(slot = self.slot, "dropping SlotWriter");
            let mut this = std::mem::take(self);
            this.dropped = true;
            tokio::spawn(async move { this.flush().await });
        }
    }
}

pub fn get_account_path(slot: u64, id: u8) -> PathBuf {
    get_vault_path()
        .join("accounts")
        .join(format!("{slot}.{id}"))
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {

    use std::fs::remove_dir_all;
    use std::path::Path;

    use test_log::test;

    use crate::io::support::write_to_file;
    use crate::io::vault::{set_vault_path, Vault};

    use super::*;
    type TestResult = core::result::Result<(), Box<dyn core::error::Error>>;

    #[expect(clippy::default_numeric_fallback)]
    #[test(tokio::test)]
    async fn slot_from_file() -> TestResult {
        // Given
        const VAULT: &str = "/tmp/bifrost/location-2";
        if Path::new(VAULT).exists() {
            remove_dir_all(Path::new(VAULT))?;
        }
        set_vault_path(VAULT);
        Vault::init_vault().await?;
        write_to_file(get_vault_path().join("accounts").join("0.0"), &[1, 2, 3]).await?;
        write_to_file(get_vault_path().join("accounts").join("0.1"), &[1, 2, 3]).await?;
        write_to_file(get_vault_path().join("accounts").join("0.2"), &[1, 2, 3]).await?;
        write_to_file(get_vault_path().join("accounts").join("0.4"), &[1, 2, 3]).await?;

        // When
        let id = get_id_from_files(0);

        // Then
        assert_eq!(id, 4);

        Ok(())
    }
}
