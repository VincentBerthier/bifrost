// File: src/transaction/message.rs
// Project: Bifrost
// Creation date: Saturday 08 February 2025
// Author: Vincent Berthier <vincent.berthier@posteo.org>
// -----
// Last modified: Thursday 13 February 2025 @ 10:01:24
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

#![expect(clippy::cast_possible_truncation)]

use borsh::{BorshDeserialize, BorshSerialize};
use tracing::{debug, instrument, trace};

use crate::{account::AccountMeta, crypto::Pubkey};

use super::{
    instruction::{CompiledInstruction, Instruction},
    Result,
};

#[non_exhaustive]
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct Message {
    /// Slot at which the transaction was created
    slot: u64,
    /// The instruction of a transaction.
    pub instructions: Vec<CompiledInstruction>,
    /// List of accounts referenced by the transaction's instructions.
    pub accounts: Vec<AccountMeta>,
}

impl Message {
    pub const fn new(slot: u64) -> Self {
        Self {
            slot,
            instructions: Vec::new(),
            accounts: Vec::new(),
        }
    }

    #[instrument(skip(self))]
    pub fn get_payer(&self) -> Option<Pubkey> {
        debug!("getting transaction payer account");
        self.accounts
            .iter()
            .find(|acc| acc.is_signing())
            .map(|acc| *acc.key())
    }

    #[instrument(skip_all)]
    pub fn add_instruction(&mut self, instruction: &Instruction) -> Result<()> {
        debug!("adding instruction to the message");
        let compiled = self.compile_instruction(instruction)?;
        self.instructions.push(compiled);

        Ok(())
    }

    #[instrument(skip_all)]
    fn compile_instruction(&mut self, instruction: &Instruction) -> Result<CompiledInstruction> {
        debug!("compile instruction");
        let mut compiled_accounts = Vec::new();
        for account in instruction.accounts() {
            let idx = self.find_or_add_account(account)?;
            compiled_accounts.push(idx);
        }
        let program_account_id =
            self.find_or_add_account(&AccountMeta::program(*instruction.program())?)?;

        Ok(CompiledInstruction::new(
            program_account_id,
            instruction.data().to_vec(),
            compiled_accounts,
        ))
    }

    #[instrument(skip_all)]
    fn find_or_add_account(&mut self, account: &AccountMeta) -> Result<u8> {
        if let Some(idx) = self.find_account(account.key()) {
            trace!("account was found in position {idx} of the transaction accounts");
            self.accounts[idx as usize].merge(account)?;
            return Ok(idx);
        }

        trace!("account wasn’t found in the transaction accounts");
        let idx = self.accounts.len() as u8;
        self.accounts.push(*account);
        Ok(idx)
    }

    #[instrument(skip_all, fields(?account))]
    fn find_account(&mut self, account: &Pubkey) -> Option<u8> {
        debug!("looking for account in transaction accounts");
        self.accounts
            .iter()
            .position(|acc| acc.key() == account)
            .map(|idx| idx as u8)
    }

    #[expect(clippy::unwrap_used)]
    pub fn to_vec(&self) -> Vec<u8> {
        borsh::to_vec(&self).unwrap()
    }

    pub fn is_valid(&self) -> bool {
        !self.instructions.is_empty() && !self.accounts.is_empty()
    }

    #[expect(clippy::missing_const_for_fn, reason = "false positive")]
    pub fn accounts(&self) -> &[AccountMeta] {
        &self.accounts
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {

    use test_log::test;

    use crate::account::Writable;
    use crate::crypto::Keypair;

    use super::*;
    type TestResult = core::result::Result<(), Box<dyn core::error::Error>>;

    #[test]
    fn empty_message_is_not_valid() -> TestResult {
        // Given
        let empty_message = Message::new(0);
        let mut with_accounts = Message::new(0);
        with_accounts.accounts.push(AccountMeta::signing(
            Keypair::generate().pubkey(),
            Writable::Yes,
        )?);
        let mut with_instruction = Message::new(0);
        with_instruction
            .instructions
            .push(CompiledInstruction::new(0, vec![0, 1, 2], vec![0, 4]));
        // Then
        assert!(!empty_message.is_valid());
        assert!(!with_accounts.is_valid());
        assert!(!with_instruction.is_valid());
        Ok(())
    }
}
