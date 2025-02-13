// File: src/account/meta.rs
// Project: Bifrost
// Creation date: Saturday 08 February 2025
// Author: Vincent Berthier <vincent.berthier@posteo.org>
// -----
// Last modified: Thursday 13 February 2025 @ 09:59:26
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

use borsh::{BorshDeserialize, BorshSerialize};
use tracing::{debug, instrument, warn};

use crate::crypto::Pubkey;

use super::{
    error::ErrorType,
    types::{AccountType, Writable},
    Error, Result,
};

/// The metadata of accounts an instruction will refer to.
#[derive(Clone, Copy, Debug, BorshSerialize, BorshDeserialize)]
pub struct AccountMeta {
    /// The public key of the account.
    key: Pubkey,
    /// The type of account (important when there's a need to create it)
    kind: AccountType,
    /// Whether the account is read-only or writable.
    writable: Writable,
}

impl AccountMeta {
    /// Create metadata for a signing account.
    ///
    /// # Parameters
    /// * `key` - The public key of the account,
    /// * `writable` - Whether the account is read-only or writable.
    ///
    /// # Returns
    /// Metadata for a signing account
    ///
    /// # Errors
    /// If the key is not on the curve.
    ///
    /// # Example
    /// ```rust
    /// # use bifrost::Error;
    /// # use bifrost::crypto::Keypair;
    /// # use bifrost::account::{Writable, AccountMeta};
    /// let key = Keypair::generate().pubkey();
    /// let meta = AccountMeta::signing(key, Writable::Yes)?;
    /// assert!(meta.is_signing());
    ///
    /// # Ok::<(), Error>(())
    /// ```
    #[instrument]
    pub fn signing(key: Pubkey, writable: Writable) -> Result<Self> {
        debug!("creating new signing wallet meta account");
        Self::check_on_curve(&key)?;
        Ok(Self {
            key,
            kind: AccountType::Signing,
            writable,
        })
    }

    /// Create metadata for wallet, *i.e.* a user's identity.
    ///
    /// # Parameters
    /// * `key` - The public key of the account,
    /// * `writable` - Whether the account is read-only or writable.
    ///
    /// # Returns
    /// Metadata for a wallet account
    ///
    /// # Errors
    /// If the key is not on the curve.
    ///
    /// # Example
    /// ```rust
    /// # use bifrost::Error;
    /// # use bifrost::crypto::Keypair;
    /// # use bifrost::account::{Writable, AccountMeta};
    /// let key = Keypair::generate().pubkey();
    /// let meta = AccountMeta::wallet(key, Writable::Yes)?;
    /// assert!(!meta.is_signing());
    ///
    /// # Ok::<(), Error>(())
    /// ```
    #[instrument]
    pub fn wallet(key: Pubkey, writable: Writable) -> Result<Self> {
        debug!("creating new wallet meta account");
        Self::check_on_curve(&key)?;
        Ok(Self {
            key,
            kind: AccountType::Wallet,
            writable,
        })
    }

    #[instrument]
    fn check_on_curve(key: &Pubkey) -> Result<()> {
        debug!("checking if the key is on the ed25519 curve");
        if !key.is_oncurve() {
            return Err(super::Error::MetaAccountCreation {
                key: *key,
                kind: ErrorType::WalletNotOnCurve,
            });
        }
        Ok(())
    }

    /// Create metadata for a program.
    ///
    /// # Parameters
    /// * `key` - The public key of the account,
    ///
    /// # Returns
    /// Metadata for a program account
    ///
    /// # Errors
    /// If the `key` was on the curve.
    ///
    /// # Example
    /// ```rust
    /// # use bifrost::Error;
    /// # use bifrost::crypto::{Keypair, Seeds};
    /// # use bifrost::account::{Writable, AccountMeta};
    /// let seeds = Seeds::new(&[&b"key1"])?;
    /// let offcurve = seeds.generate_offcurve()?.0;
    /// let meta = AccountMeta::program(offcurve)?;
    /// assert!(!meta.is_signing());
    ///
    /// # Ok::<(), Error>(())
    /// ```
    #[instrument]
    pub fn program(key: Pubkey) -> Result<Self> {
        debug!("creating new program meta account");
        if key.is_oncurve() {
            return Err(super::Error::MetaAccountCreation {
                key,
                kind: ErrorType::NonWalletOnCurve,
            });
        }
        Ok(Self {
            key,
            kind: AccountType::Program,
            writable: Writable::No,
        })
    }

    /// Merge the metadata of two different accounts.
    ///
    /// If one account is writable, the merge will be.
    /// If one account is a signer, the merge will be too.
    ///
    /// # Parameters
    /// * `other` - the account to merge with,
    ///
    /// # Returns
    /// The merged account metadata.
    ///
    /// # Errors
    /// If the two accounts are not compatible (for example a Mint and a Purse).
    ///
    /// # Example
    /// ```rust
    /// # use bifrost::{Error, crypto::Keypair, account::{AccountMeta, Writable}};
    /// let key = Keypair::generate().pubkey();
    /// let mut meta1 = AccountMeta::wallet(key, Writable::No)?;
    /// let meta2 = AccountMeta::wallet(key, Writable::Yes)?;
    /// meta1.merge(&meta2);
    /// assert!(meta1.is_writable());
    /// # Ok::<(), Error>(())
    /// ```
    #[instrument]
    pub fn merge(&mut self, other: &Self) -> Result<()> {
        debug!("merging meta accounts");
        if !self.kind.is_compatible(other.kind) {
            warn!("attempted to merge non-compatible accounts");
            return Err(Error::MergeIncompatibleAccountTypes(self.kind, other.kind));
        }

        if other.is_writable() {
            self.writable = Writable::Yes;
        }

        if other.is_signing() {
            self.kind = AccountType::Signing;
        }

        Ok(())
    }

    /// Checks whether the account is a signing one or not.
    #[must_use]
    pub const fn is_signing(&self) -> bool {
        matches!(self.kind, AccountType::Signing)
    }

    /// Checks whether the account is read-only or writable
    #[must_use]
    pub const fn is_writable(&self) -> bool {
        matches!(self.writable, Writable::Yes)
    }

    /// Get the account's public key
    #[must_use]
    pub const fn key(&self) -> &Pubkey {
        &self.key
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {

    use std::assert_matches::assert_matches;

    use test_log::test;

    use crate::crypto::{Keypair, Seeds};

    use super::super::Error;
    use super::*;
    type TestResult = core::result::Result<(), Box<dyn core::error::Error>>;

    #[test]
    fn only_allow_wallets_on_the_curve() -> TestResult {
        // Given
        let seeds = Seeds::new(&[&b"key1"])?;
        let offcurve = seeds.generate_offcurve()?.0;
        let oncurve = Keypair::generate().pubkey();

        // When
        let _res = AccountMeta::program(offcurve)?;
        let res = AccountMeta::program(oncurve);

        // Then
        assert_matches!(
            res,
            Err(Error::MetaAccountCreation { kind, .. }) if matches!(kind, ErrorType::NonWalletOnCurve),
        );
        Ok(())
    }

    #[test]
    fn wallets_must_be_on_curve() -> TestResult {
        // Given
        let seeds = Seeds::new(&[&b"key1"])?;
        let offcurve = seeds.generate_offcurve()?.0;
        let oncurve = Keypair::generate().pubkey();

        // When
        let res1 = AccountMeta::wallet(oncurve, Writable::No)?;
        let res2 = AccountMeta::wallet(offcurve, Writable::No);

        // Then
        assert!(!res1.is_writable());
        assert_matches!(
            res2,
            Err(Error::MetaAccountCreation { kind, .. }) if matches!(kind, ErrorType::WalletNotOnCurve),
        );
        Ok(())
    }

    #[test]
    fn accounts_must_be_compatible() -> TestResult {
        // Given
        let seeds = Seeds::new(&[&b"key1"])?;
        let offcurve = seeds.generate_offcurve()?.0;
        let oncurve = Keypair::generate().pubkey();
        let mut program = AccountMeta::program(offcurve)?;
        let wallet = AccountMeta::wallet(oncurve, Writable::No)?;

        // When
        let res = program.merge(&wallet);

        // Then
        assert_matches!(res, Err(Error::MergeIncompatibleAccountTypes(_, _)));
        Ok(())
    }
}
