use tracing::{debug, instrument};

use crate::{account::TransactionAccount, crypto::Pubkey};

use super::{
    system::{self, SYSTEM_PROGRAM},
    Error, Result,
};

/// Dispatches an instruction to the program handling it.
///
/// # Parameters
/// * `instruction` - The instruction to execute,
/// * `accounts` - The accounts referenced by the instruction.
///
/// # Errors
/// If the program is unknown or failed to run.
#[instrument(skip_all)]
pub fn dispatch(program: &Pubkey, accounts: &[TransactionAccount], payload: &[u8]) -> Result<()> {
    debug!(
        %program,
        "received new instruction to handle"
    );
    match *program {
        SYSTEM_PROGRAM => system::execute_instruction(accounts, payload),
        key => Err(Error::UnknownProgram { key }),
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {

    use std::assert_matches::assert_matches;

    use test_log::test;

    use crate::account::{AccountMeta, TransactionAccount, Wallet, Writable};
    use crate::crypto::Keypair;
    use crate::program::system;
    use crate::transaction::Instruction;

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

        let instruction = system::instruction::transfer(key1, key2, AMOUNT)?;

        // When
        dispatch(&SYSTEM_PROGRAM, &accounts_vec, instruction.data())?;

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

        let instruction = Instruction::new(program, [meta1, meta2], &Vec::<u8>::new());

        // When
        let res = dispatch(&program, &accounts_vec, instruction.data());

        // Then
        assert_matches!(res, Err(err) if matches!(err, Error::UnknownProgram { key } if key == program));

        Ok(())
    }
}
