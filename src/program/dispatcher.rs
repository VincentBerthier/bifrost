use tracing::{debug, instrument};

use crate::{account::Accounts, transaction::Instruction};

use super::{
    system::{self, SYSTEM_PROGRAM},
    Error, Result,
};

#[instrument(skip_all)]
fn dispatch<'a>(instruction: &Instruction, accounts: &'a Accounts<'a>) -> Result<()> {
    debug!(
        program = %instruction.program(),
        "received new instruction to handle"
    );
    match *instruction.program() {
        SYSTEM_PROGRAM => system::execute_instruction(accounts, instruction.data()),
        key => Err(Error::UnknownProgram { key }),
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {

    use std::assert_matches::assert_matches;

    use test_log::test;

    use crate::account::{AccountMeta, Accounts, TransactionAccount, Wallet, Writable};
    use crate::crypto::Keypair;
    use crate::program::system;

    // use super::super::Error;
    use super::*;
    type TestResult = core::result::Result<(), Box<dyn core::error::Error>>;

    #[test]
    fn send_instruction_to_system_program() -> TestResult {
        // Given
        const AMOUNT: u64 = 1_000;
        let key1 = Keypair::generate().pubkey();
        let key2 = Keypair::generate().pubkey();
        let meta1 = AccountMeta::signing(key1, Writable::Yes)?;
        let meta2 = AccountMeta::wallet(key2, Writable::Yes)?;
        let mut wallet1 = Wallet { prisms: AMOUNT };
        let mut wallet2 = Wallet { prisms: 0 };

        let accounts_vec = vec![
            TransactionAccount::new(&meta1, &mut wallet1),
            TransactionAccount::new(&meta2, &mut wallet2),
        ];
        let accounts = Accounts::new(accounts_vec.as_slice());

        let instruction = system::instruction::transfer(key1, key2, AMOUNT)?;

        // When
        dispatch(&instruction, &accounts)?;

        // Then
        assert_eq!(wallet1.prisms, 0);
        assert_eq!(wallet2.prisms, AMOUNT);

        Ok(())
    }

    #[test]
    fn unknow_program() -> TestResult {
        // Given
        const AMOUNT: u64 = 1_000;
        let key1 = Keypair::generate().pubkey();
        let key2 = Keypair::generate().pubkey();
        let program = Keypair::generate().pubkey();
        let meta1 = AccountMeta::signing(key1, Writable::Yes)?;
        let meta2 = AccountMeta::wallet(key2, Writable::Yes)?;
        let mut wallet1 = Wallet { prisms: AMOUNT };
        let mut wallet2 = Wallet { prisms: 0 };

        let accounts_vec = vec![
            TransactionAccount::new(&meta1, &mut wallet1),
            TransactionAccount::new(&meta2, &mut wallet2),
        ];
        let accounts = Accounts::new(accounts_vec.as_slice());

        let instruction = Instruction::new(program, [meta1, meta2], &Vec::<u8>::new());

        // When
        let res = dispatch(&instruction, &accounts);

        // Then
        assert_matches!(res, Err(err) if matches!(err, Error::UnknownProgram { key } if key == program));

        Ok(())
    }
}
