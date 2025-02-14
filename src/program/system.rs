// File: src/program/system.rs
// Project: Bifrost
// Creation date: Wednesday 12 February 2025
// Author: Vincent Berthier <vincent.berthier@posteo.org>
// -----
// Last modified: Thursday 13 February 2025 @ 09:48:37
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
use tracing::{debug, instrument};

use crate::{
    account::{next_account, TransactionAccount},
    crypto::Pubkey,
};

use super::{Error, Result};

/// The System's program id (`BifrostSystemProgram111111111111111111111111`)
pub const SYSTEM_PROGRAM: Pubkey = Pubkey::from_bytes(&[
    2, 190, 236, 171, 26, 147, 23, 185, 158, 168, 176, 152, 117, 167, 48, 232, 60, 78, 120, 154,
    96, 248, 193, 153, 0, 203, 246, 209, 37, 0, 0, 0,
]);

#[derive(Debug, BorshSerialize, BorshDeserialize)]
enum SystemInstruction {
    Transfer(u64),
}

/// Executes a system program's instruction.
///
/// # Parameters
/// * `accounts` - The accounts needed by the instruction,
/// * `payload` - The data payload for the instruction.
///
/// # Errors
/// if the instruction fails to complete (missing accounts, arithmetic overflows, *etc.*).
#[instrument(skip_all)]
pub fn execute_instruction(accounts: &[TransactionAccount], payload: &[u8]) -> Result<()> {
    debug!("received system insruction");
    match borsh::from_slice(payload)? {
        SystemInstruction::Transfer(amount) => transfer(accounts, amount),
    }
}

#[instrument(skip(accounts))]
fn transfer(accounts: &[TransactionAccount], amount: u64) -> Result<()> {
    debug!("transferring prisms");
    let mut accounts_iter = accounts.iter();
    let payer = next_account(&mut accounts_iter)?;
    let receiver = next_account(&mut accounts_iter)?;
    if !payer.is_signer {
        return Err(Error::Custom(format!(
            "{} must be a signing account",
            payer.key
        )));
    }
    debug!("from {} to {}", payer.key, receiver.key);
    payer.sub_prisms(amount)?;
    receiver.add_prisms(amount)?;
    Ok(())
}

/// Get the instructions for the system program.
pub mod instruction {
    use crate::{
        account::{AccountMeta, Writable},
        crypto::Pubkey,
        transaction::Instruction,
    };

    use super::{Result, SystemInstruction, SYSTEM_PROGRAM};

    /// Prisms transfer instruction.
    ///
    /// # Parameters
    /// * `from` - The account the prisms are taken from,
    /// * `to` - The account receiving the prisms,
    /// * `amount` - The amount of prisms to receive.
    ///
    /// # Errors
    /// If either account is not on the `ed25519` curve.
    pub fn transfer(from: Pubkey, to: Pubkey, amount: u64) -> Result<Instruction> {
        let accounts = vec![
            AccountMeta::signing(from, Writable::Yes)?,
            AccountMeta::wallet(to, Writable::Yes)?,
        ];
        Ok(Instruction::new(
            SYSTEM_PROGRAM,
            accounts,
            &SystemInstruction::Transfer(amount),
        ))
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {

    use std::assert_matches::assert_matches;

    use test_log::test;

    use crate::account::{AccountMeta, TransactionAccount, Wallet, Writable};
    use crate::crypto::Keypair;

    use super::super::Error;
    use super::*;
    type TestResult = core::result::Result<(), Box<dyn core::error::Error>>;

    #[expect(clippy::unwrap_used)]
    #[test]
    fn execute_transfer_instruction() -> TestResult {
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

        let payload = borsh::to_vec(&SystemInstruction::Transfer(100)).unwrap();

        // When
        execute_instruction(&accounts_vec, &payload)?;

        // Then
        assert_eq!(wallet1.prisms, AMOUNT - 100);
        assert_eq!(wallet2.prisms, 100);

        Ok(())
    }

    #[test]
    fn execute_transfer_fails_with_one_account() -> TestResult {
        // Given
        const AMOUNT: u64 = 1_000;
        let key1 = Keypair::generate().pubkey();
        let meta1 = AccountMeta::signing(key1, Writable::Yes)?;
        let mut wallet1 = Wallet { prisms: AMOUNT };

        let accounts_vec = vec![TransactionAccount::new(&meta1, &mut wallet1)];

        #[expect(clippy::unwrap_used)]
        let payload = borsh::to_vec(&SystemInstruction::Transfer(100)).unwrap();

        // When
        let res = execute_instruction(&accounts_vec, &payload);

        // Then
        assert_matches!(res, Err(error) if matches!(error, Error::Account(_)));

        Ok(())
    }

    #[test]
    fn no_signer_fails_instruction() -> TestResult {
        // Given
        const AMOUNT: u64 = 1_000;
        let key1 = Keypair::generate().pubkey();
        let key2 = Keypair::generate().pubkey();
        let meta1 = AccountMeta::wallet(key1, Writable::Yes)?;
        let meta2 = AccountMeta::wallet(key2, Writable::Yes)?;
        let mut wallet1 = Wallet { prisms: AMOUNT };
        let mut wallet2 = Wallet { prisms: 0 };

        let accounts_vec = vec![
            TransactionAccount::new(&meta1, &mut wallet1),
            TransactionAccount::new(&meta2, &mut wallet2),
        ];

        #[expect(clippy::unwrap_used)]
        let payload = borsh::to_vec(&SystemInstruction::Transfer(100)).unwrap();

        // When
        let res = execute_instruction(&accounts_vec, &payload);

        // Then
        assert_matches!(res, Err(error) if matches!(error, Error::Custom { .. }));

        Ok(())
    }
}
