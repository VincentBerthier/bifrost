// File: src/program/testing_dummy.rs
// Project: Bifrost
// Creation date: Friday 14 February 2025
// Author: Vincent Berthier <vincent.berthier@posteo.org>
// -----
// Last modified: Friday 14 February 2025 @ 16:41:49
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

use super::Result;

/// The System's program id (`BifrostTestingSystemProgram11111111111111111`)
pub const TESTING_PROGRAM: Pubkey = Pubkey::from_bytes(&[
    159, 65, 158, 196, 5, 88, 89, 176, 224, 101, 212, 80, 151, 14, 225, 182, 96, 196, 131, 59, 87,
    252, 174, 1, 124, 135, 56, 32, 33, 180, 0, 0,
]);

#[derive(Debug, BorshSerialize, BorshDeserialize)]
enum SystemInstruction {
    BurnPrisms(u64),
}

/// Executes a testing program's instruction.
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
        SystemInstruction::BurnPrisms(amount) => burn_prisms(accounts, amount),
    }
}

#[instrument(skip(accounts))]
fn burn_prisms(accounts: &[TransactionAccount], amount: u64) -> Result<()> {
    debug!("transferring prisms");
    let mut accounts_iter = accounts.iter();
    let payer = next_account(&mut accounts_iter)?;
    let _receiver = next_account(&mut accounts_iter)?;
    payer.sub_prisms(amount)?;
    // we would expect to have the add prisms here, but it’s not.
    Ok(())
}

/// Get the instructions for the system program.
pub mod instruction {
    use crate::{
        account::{AccountMeta, Writable},
        crypto::Pubkey,
        transaction::Instruction,
    };

    use super::{Result, SystemInstruction, TESTING_PROGRAM};

    /// Prisms transfer instruction.
    ///
    /// # Parameters
    /// * `from` - The account the prisms are taken from,
    /// * `to` - The account receiving the prisms,
    /// * `amount` - The amount of prisms to receive.
    ///
    /// # Errors
    /// If either account is not on the `ed25519` curve.
    pub fn burn_prisms(from: Pubkey, to: Pubkey, amount: u64) -> Result<Instruction> {
        let accounts = vec![
            AccountMeta::signing(from, Writable::Yes)?,
            AccountMeta::wallet(to, Writable::Yes)?,
        ];
        Ok(Instruction::new(
            TESTING_PROGRAM,
            accounts,
            &SystemInstruction::BurnPrisms(amount),
        ))
    }
}
