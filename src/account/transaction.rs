use std::{cell::RefCell, rc::Rc};

use crate::crypto::Pubkey;

use super::{AccountMeta, Error, Result, Wallet};

/// Stores all data regarding an account needed by an instruction
/// to allow it to access or modify its data.
pub struct TransactionAccount<'a> {
    /// The public key of the account
    pub key: Pubkey,
    readonly: bool,
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
    pub fn new(meta: &AccountMeta, account: &'a mut Wallet) -> Self {
        Self {
            key: *meta.key(),
            readonly: !meta.is_writable(),
            prisms: Rc::new(RefCell::new(&mut account.prisms)),
        }
    }

    fn set_prisms(&self, amount: u64) -> Result<()> {
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
    pub fn add_prisms(&self, amount: u64) -> Result<()> {
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
    pub fn sub_prisms(&self, amount: u64) -> Result<()> {
        let res = self
            .prisms
            .borrow()
            .checked_sub(amount)
            .ok_or(Error::ArithmeticOverflow)?;
        self.set_prisms(res)
    }
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
