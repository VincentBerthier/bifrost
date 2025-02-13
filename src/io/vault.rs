// File: src/io/vault.rs
// Project: Bifrost
// Creation date: Sunday 09 February 2025
// Author: Vincent Berthier <vincent.berthier@posteo.org>
// -----
// Last modified: Thursday 13 February 2025 @ 09:56:27
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

use std::{path::PathBuf, sync::OnceLock};

use tokio::fs::remove_file;
use tracing::{debug, instrument, trace};

use crate::{account::Wallet, crypto::Pubkey};

use super::{
    index::Index,
    location::AccountDiskLocation,
    support::create_folder,
    trash::{AccountFile, Trash},
    Result,
};

pub static VAULT_PATH: OnceLock<PathBuf> = OnceLock::new();

#[mutants::skip]
#[expect(clippy::unwrap_used)]
pub fn set_vault_path<P>(path: P)
where
    P: Into<PathBuf>,
{
    VAULT_PATH.set(path.into()).unwrap();
}

#[expect(clippy::expect_used)]
pub fn get_vault_path() -> &'static PathBuf {
    VAULT_PATH.get().expect("vault path is not set")
}

/// Storage for all accounts on the blockchain.
pub struct Vault {
    /// The index of known accounts.
    index: Index,
    /// The list of out-of-date accounts stored on the disk.
    trash: Trash,
}

impl Vault {
    /// Load or creates the vault.
    ///
    /// # Errors
    /// Only if the vault could not be initialized,
    /// which would only happen because of a file system error
    /// such as a permission issue.
    #[instrument]
    pub async fn load_or_create() -> Result<Self> {
        debug!("initializing vault");
        Self::init_vault().await?;
        Ok(Self {
            index: Index::load_or_create().await,
            trash: Trash::load_or_create().await,
        })
    }

    /// Initializes the vault.
    ///
    /// This mostly just creates the folder architecture if it's needed.
    ///
    /// # Errors
    /// Can only happen in case of file system errors.
    #[mutants::skip]
    #[instrument]
    pub async fn init_vault() -> Result<()> {
        debug!("initializing vault");
        let path = get_vault_path();
        if path.exists() {
            return Ok(());
        }
        for folder in ["accounts", "transactions", "blocks"] {
            create_folder(path.join(folder)).await?;
        }

        Ok(())
    }

    /// Creates or loads an account from the disk.
    ///
    /// # Parameters
    /// * `key` - The public key of the account to load/create,
    ///
    /// # Errors
    /// If the index failed to load an existing account.
    #[instrument(skip(self))]
    pub async fn get(&self, key: &Pubkey) -> Result<Wallet> {
        debug!("getting account");
        Ok((self.index.load(key).await?).unwrap_or_default())
    }

    // TODO: will need to handle saving the same account multiple times for the same slot
    // it could work as it is, it’s just inneficient
    /// Saves an account on the disk.
    ///
    /// # Parameters
    /// * `key` - The public key of the account to save,
    /// * `acconut` - The account to save,
    /// * `slot` - The current slot.
    ///
    /// # Errors
    /// Only if there was a problem saving the account on the disk.
    #[instrument(skip(self, account))]
    pub async fn save_account(&mut self, key: Pubkey, account: &Wallet, slot: u64) -> Result<()> {
        debug!("saving account");
        if let Some(&old_loc) = self.index.find(&key) {
            trace!(
                ?old_loc,
                "account was already known, placing its old location into the trash"
            );
            self.trash.insert(old_loc)?;
        }

        let loc = AccountDiskLocation::new_from_write(account, slot).await?;
        self.index.set_account(key, loc);

        Ok(())
    }

    /// Saves the vault on the disk (index and trash).
    ///
    /// # Errors
    /// Only if there was a problem saving the vault on the disk.
    #[instrument(skip(self))]
    pub async fn save(&self) -> Result<()> {
        debug!("saving vault");
        self.index.save().await?;
        self.trash.save().await
    }

    /// Trims the accounts on the disk.
    ///
    /// When existing accounts are updated, their old data stays on the disk
    /// for archive purposes. The only files that are not touched (yet) are
    /// those for the latest slot.
    ///
    /// # Errors
    /// Only on I/O issues.
    ///
    /// # Parameters
    /// * `current_slot` - The current slot the blockchain is working on.
    #[instrument(skip(self))]
    pub async fn cleanup(&mut self, current_slot: u64) -> Result<()> {
        debug!("cleaning up the vault");
        let to_clean = self.trash.get_files_to_clean().await;
        for file in to_clean {
            trace!(?file, "cleaning up the file");
            let AccountFile { slot, id } = file;
            if slot == current_slot {
                trace!(?file, "file is for the current slot, skipping");
                continue;
            }
            self.relocate_accounts(slot, id).await?;
            trace!(?file, "removing file from the disk");
            remove_file(AccountDiskLocation::get_path(slot, id)).await?;
            trace!(?file, "removing file from the trash");
            self.trash.remove(&file);
        }
        Ok(())
    }

    #[instrument(skip(self))]
    async fn relocate_accounts(&mut self, slot: u64, id: u8) -> Result<()> {
        debug!("relocating accounts");
        let relocated_accounts = self.index.accounts_on_file(slot, id);
        for key in relocated_accounts {
            trace!(%key, "relocating account");
            #[expect(clippy::unwrap_used, reason = "the list was retrieved just before")]
            let account = self.index.load(&key).await?.unwrap();
            let new_loc = AccountDiskLocation::new_from_write(&account, slot).await?;
            trace!(%key, ?new_loc, "relocated to new location");
            self.index.set_account(key, new_loc);
        }
        Ok(())
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {

    use std::assert_matches::assert_matches;
    use std::fs::{read_dir, remove_dir_all};

    use test_log::test;

    use crate::account::Wallet;
    use crate::crypto::{Keypair, Pubkey};
    use crate::io::index::Index;
    use crate::io::location::AccountDiskLocation;
    use crate::io::support::read_from_file;
    use crate::io::MAX_ACCOUNT_FILE_SIZE;

    // use super::super::Error;
    use super::*;
    type TestResult = core::result::Result<(), Box<dyn core::error::Error>>;

    const AMOUNT1: u64 = 918_379_983;
    const AMOUNT2: u64 = 3;
    const AMOUNT3: u64 = 918_379_983_938;

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

    async fn setup_vault<P>(path: P) -> Result<Vec<Pubkey>>
    where
        P: Into<PathBuf>,
    {
        reset_vault(path)?;
        Vault::init_vault().await?;
        let key1 = Keypair::generate().pubkey();
        let key2 = Keypair::generate().pubkey();
        let key3 = Keypair::generate().pubkey();

        let wallet1 = Wallet { prisms: AMOUNT1 };
        let wallet2 = Wallet { prisms: AMOUNT2 };
        let wallet3 = Wallet { prisms: AMOUNT3 };

        let mut index = Index::load_or_create().await;
        let loc1 = AccountDiskLocation::new_from_write(&wallet1, 82).await?;
        let loc2 = AccountDiskLocation::new_from_write(&wallet2, 82).await?;
        let loc3 = AccountDiskLocation::new_from_write(&wallet3, 82).await?;

        index.set_account(key1, loc1);
        index.set_account(key2, loc2);
        index.set_account(key3, loc3);
        index.save().await?;

        Ok(vec![key1, key2, key3])
    }

    #[test(tokio::test)]
    async fn load_account() -> TestResult {
        // Given
        const VAULT: &str = "/tmp/bifrost/vault-1";
        let keys = setup_vault(VAULT).await?;

        // When
        let vault = Vault::load_or_create().await?;

        // Then
        let from_vault = vault.get(&keys[0]).await;
        assert_matches!(from_vault, Ok(account) if account.prisms == AMOUNT1, "{from_vault:?}");

        Ok(())
    }

    #[test(tokio::test)]
    async fn new_account() -> TestResult {
        // Given
        const VAULT: &str = "/tmp/bifrost/vault-2";
        let _keys = setup_vault(VAULT).await?;

        // When
        let vault = Vault::load_or_create().await?;

        // Then
        let from_vault = vault.get(&Keypair::generate().pubkey()).await;
        assert_matches!(from_vault, Ok(account) if account.prisms == 0, "{from_vault:?}");

        Ok(())
    }

    #[test(tokio::test)]
    async fn save_new_account() -> TestResult {
        // Given
        const VAULT: &str = "/tmp/bifrost/vault-3";
        reset_vault(VAULT)?;
        let mut vault = Vault::load_or_create().await?;
        let key = Keypair::generate().pubkey();
        let mut account = vault.get(&key).await?;

        // When
        account.prisms = 198_388;
        vault.save_account(key, &account, 0).await?;

        // Then
        let from_disk: Wallet =
            read_from_file(get_vault_path().join("accounts").join("0.0")).await?;
        assert_eq!(from_disk, account);

        Ok(())
    }

    #[test(tokio::test)]
    async fn rotate_files() -> TestResult {
        // Given
        const VAULT: &str = "/tmp/bifrost/vault-4";
        reset_vault(VAULT)?;
        let mut vault = Vault::load_or_create().await?;
        let account = Wallet {
            prisms: 938_983_237,
        };
        let data_len = borsh::to_vec(&account)?.len() as u64;
        #[expect(clippy::integer_division)]
        let account_per_file = MAX_ACCOUNT_FILE_SIZE / data_len;

        for _ in 0..=account_per_file {
            vault
                .save_account(Keypair::generate().pubkey(), &account, 0)
                .await?;
        }

        // When
        assert!(!get_vault_path().join("accounts").join("0.1").exists());
        vault
            .save_account(Keypair::generate().pubkey(), &account, 0)
            .await?;

        // Then
        assert!(get_vault_path().join("accounts").join("0.1").exists());

        Ok(())
    }

    #[test(tokio::test)]
    async fn old_account_updated_and_trashed() -> TestResult {
        // Given
        const VAULT: &str = "/tmp/bifrost/vault-5";
        reset_vault(VAULT)?;
        let mut vault = Vault::load_or_create().await?;
        let key = Keypair::generate().pubkey();
        let mut account = vault.get(&key).await?;
        account.prisms = 198_388;
        vault.save_account(key, &account, 0).await?;

        // When
        account.prisms = 397_983;
        vault.save_account(key, &account, 1).await?;
        let reloaded = vault.get(&key).await?;

        // Then
        assert_eq!(reloaded, account);

        Ok(())
    }

    #[test(tokio::test)]
    async fn updated_account_loc_trashed() -> TestResult {
        // Given
        const VAULT: &str = "/tmp/bifrost/vault-6";
        reset_vault(VAULT)?;
        let mut vault = Vault::load_or_create().await?;
        let key = Keypair::generate().pubkey();
        let mut account = vault.get(&key).await?;
        account.prisms = 198_388;
        vault.save_account(key, &account, 0).await?;

        // When
        account.prisms = 397_983;
        vault.save_account(key, &account, 1).await?;
        account.prisms = 83;
        vault.save_account(key, &account, 2).await?;

        // Then
        assert_eq!(vault.trash.len(), 2);

        Ok(())
    }

    #[expect(clippy::default_numeric_fallback)]
    #[test(tokio::test)]
    async fn cleanup_vault() -> TestResult {
        // Given
        const VAULT: &str = "/tmp/bifrost/vault-7";
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

        // When
        vault.cleanup(5).await?;

        // Then
        assert_eq!(read_dir(get_vault_path().join("accounts"))?.count(), 8);

        Ok(())
    }

    #[expect(clippy::default_numeric_fallback)]
    #[test(tokio::test)]
    async fn double_cleanup_no_effect() -> TestResult {
        // Given
        const VAULT: &str = "/tmp/bifrost/vault-8";
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

        // When
        vault.cleanup(5).await?;
        vault.cleanup(5).await?;

        // Then
        assert_eq!(read_dir(get_vault_path().join("accounts"))?.count(), 8);

        Ok(())
    }

    #[expect(clippy::default_numeric_fallback)]
    #[test(tokio::test)]
    async fn cleanup_ignore_current_slot() -> TestResult {
        // Given
        const VAULT: &str = "/tmp/bifrost/vault-9";
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

        // When
        vault.cleanup(3).await?;

        // Then
        assert_eq!(read_dir(get_vault_path().join("accounts"))?.count(), 10);

        Ok(())
    }
}
