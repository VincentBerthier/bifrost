// File: src/account/transaction.rs
// Project: Bifrost
// Creation date: Thursday 13 February 2025
// Author: Vincent Berthier <vincent.berthier@posteo.org>
// -----
// Last modified: Thursday 13 February 2025 @ 09:45:33
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

use std::{cell::RefCell, rc::Rc};

use tracing::{debug, instrument};

use crate::crypto::Pubkey;

use super::{AccountMeta, Error, Result, Wallet};

/// Stores all data regarding an account needed by an instruction
/// to allow it to access or modify its data.
#[derive(Clone)]
pub struct TransactionAccount<'a> {
    /// The public key of the account
    pub key: Pubkey,
    /// Is the account writable or read only.
    pub readonly: bool,
    /// Is the account signing the transaction or not.
    pub is_signer: bool,
    prisms: Rc<RefCell<&'a mut u64>>,
}

impl<'a> TransactionAccount<'a> {
    /// Creates a new `TransactionAccount` to use by a transaction / instruction.
    ///
    /// # Parameters
    /// * `meta` - The metadata related to the account,
    /// * `accounts` - The actual account data.
    ///
    /// # Example
    /// ```rust
    /// # use bifrost::{account::{AccountMeta, Wallet, Writable, TransactionAccount}, crypto::Keypair, Error};
    /// let mut wallet = Wallet { prisms: 1_000 };
    /// let key = Keypair::generate().pubkey();
    /// let meta = AccountMeta::wallet(key, Writable::Yes)?;
    /// let info = TransactionAccount::new(&meta, &mut wallet);
    ///
    /// # Ok::<(), Error>(())
    /// ```
    #[instrument(skip_all)]
    pub fn new(meta: &AccountMeta, account: &'a mut Wallet) -> Self {
        debug!("creating new TransactionAccount for {}", meta.key());
        Self {
            key: *meta.key(),
            readonly: !meta.is_writable(),
            is_signer: meta.is_signing(),
            prisms: Rc::new(RefCell::new(&mut account.prisms)),
        }
    }

    #[instrument(skip(self))]
    fn set_prisms(&self, amount: u64) -> Result<()> {
        debug!(
            "setting prisms to {amount} (from {})",
            *self.prisms.borrow()
        );
        if self.readonly {
            return Err(Error::ModificationOfReadOnlyAccount { key: self.key });
        }
        **self.prisms.borrow_mut() = amount;

        Ok(())
    }

    /// Adds a given amount of prisms to the account.
    ///
    /// # Parameters
    /// * `amount` - the amount to add to the account,
    ///
    /// # Errors
    /// If there is an arithmetic overflow or if the account
    /// is read only.
    #[instrument(skip(self))]
    pub fn add_prisms(&self, amount: u64) -> Result<()> {
        debug!(current = *self.prisms.borrow(), "adding {amount} prisms");
        let res = self
            .prisms
            .borrow()
            .checked_add(amount)
            .ok_or(Error::ArithmeticOverflow)?;

        self.set_prisms(res)
    }

    /// Subtracts a given amount of prisms to the account.
    ///
    /// # Parameters
    /// * `amount` - the amount to subtract to the account,
    ///
    /// # Errors
    /// If there is an arithmetic overflow or if the account
    /// is read only.
    #[instrument(skip(self))]
    pub fn sub_prisms(&self, amount: u64) -> Result<()> {
        debug!(
            current = *self.prisms.borrow(),
            "subtracting {amount} prisms"
        );
        let res = self
            .prisms
            .borrow()
            .checked_sub(amount)
            .ok_or(Error::ArithmeticOverflow)?;
        self.set_prisms(res)
    }
}

/// Accesses the next account in the list.
///
/// # Parameters
/// * `iter` - the account iterator
///
/// # Errors
/// If called when all accounts have been accessed already.
pub fn next_account<'a, 'b, I>(iter: &mut I) -> Result<I::Item>
where
    I: Iterator<Item = &'a TransactionAccount<'b>>,
{
    iter.next().ok_or(Error::MissingAccounts)
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {

    use std::assert_matches::assert_matches;

    use test_log::test;

    use crate::account::{Wallet, Writable};
    use crate::crypto::Keypair;

    // use super::super::Error;
    use super::*;
    type TestResult = core::result::Result<(), Box<dyn core::error::Error>>;

    #[test]
    fn modify_account_through_info() -> Result<()> {
        // Given
        const AMOUNT: u64 = 983_983;
        let mut wallet = Wallet { prisms: AMOUNT };
        let key = Keypair::generate().pubkey();
        let meta = AccountMeta::wallet(key, Writable::Yes)?;
        let info = TransactionAccount::new(&meta, &mut wallet);

        // When
        info.add_prisms(1_000)?;

        // Then
        assert_eq!(wallet.prisms, AMOUNT + 1_000);

        Ok(())
    }

    #[test]
    fn sub_prisms() -> TestResult {
        // Given
        const AMOUNT: u64 = 983_983;
        let mut wallet = Wallet { prisms: AMOUNT };
        let key = Keypair::generate().pubkey();
        let meta = AccountMeta::wallet(key, Writable::Yes)?;
        let info = TransactionAccount::new(&meta, &mut wallet);

        // When
        info.sub_prisms(1_000)?;

        // Then
        assert_eq!(wallet.prisms, AMOUNT - 1_000);

        Ok(())
    }

    #[test]
    fn prevent_arithmetic_overflow() -> TestResult {
        // Given
        const AMOUNT: u64 = u64::MAX - 100;
        let mut wallet1 = Wallet { prisms: AMOUNT };
        let key1 = Keypair::generate().pubkey();
        let meta1 = AccountMeta::wallet(key1, Writable::Yes)?;
        let info1 = TransactionAccount::new(&meta1, &mut wallet1);
        let mut wallet2 = Wallet { prisms: 100 };
        let key2 = Keypair::generate().pubkey();
        let meta2 = AccountMeta::wallet(key2, Writable::Yes)?;
        let info2 = TransactionAccount::new(&meta2, &mut wallet2);

        // When
        let res1 = info1.add_prisms(1_000);
        let res2 = info2.sub_prisms(1_000);

        // Then
        assert_matches!(res1, Err(err) if matches!(err, Error::ArithmeticOverflow));
        assert_matches!(res2, Err(err) if matches!(err, Error::ArithmeticOverflow));

        Ok(())
    }

    #[test]
    fn cannot_modify_read_only_account() -> TestResult {
        // Given
        const AMOUNT: u64 = 983_983;
        let mut wallet = Wallet { prisms: AMOUNT };
        let key = Keypair::generate().pubkey();
        let meta = AccountMeta::wallet(key, Writable::No)?;
        let info = TransactionAccount::new(&meta, &mut wallet);

        // When
        let res1 = info.add_prisms(1_000);
        let res2 = info.sub_prisms(1_000);

        // Then
        assert_matches!(res1, Err(err) if matches!(err, Error::ModificationOfReadOnlyAccount{ .. }));
        assert_matches!(res2, Err(err) if matches!(err, Error::ModificationOfReadOnlyAccount{ .. }));

        Ok(())
    }
}
