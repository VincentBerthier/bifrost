// File: src/transaction/instruction.rs
// Project: Bifrost
// Creation date: Saturday 08 February 2025
// Author: Vincent Berthier <vincent.berthier@posteo.org>
// -----
// Last modified: Sunday 09 February 2025 @ 16:15:10
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

use crate::{account::AccountMeta, crypto::Pubkey};

/// An instruction compiled and ready to be executed on the blockchain.
#[derive(Clone, Debug, Default, BorshSerialize, BorshDeserialize)]
pub struct CompiledInstruction {
    /// The id in the message's accounts of the program executing this instruction.
    pub program_account_id: u8,
    /// The payload for the instruction.
    pub data: Vec<u8>,
    /// The id in the message's accounts of the accounts referenced by the instruction.
    pub accounts: Vec<u8>,
}

/// An instruction to execute on the blockchain.
#[derive(Clone, Debug, BorshSerialize)]
pub struct Instruction {
    /// Public key of the program to run.
    program_id: Pubkey,
    /// List of accounts expected by the instruction.
    accounts: Vec<AccountMeta>,
    /// Binary encoded payload for the transaction.
    data: Vec<u8>,
}

#[expect(clippy::missing_const_for_fn, reason = "false positive")]
impl Instruction {
    /// Create a new instruction.
    ///
    /// # Parameters
    /// * `program_id` - the public key of the program,
    /// * `accounts` - list of accounts expected by the instruction,
    /// * `payload` - the payload of the transaction.
    ///
    /// # Example
    /// ```rust
    /// # use bifrost::{
    ///     Error,
    ///     account::{AccountMeta, Writable},
    ///     crypto::{Keypair, Pubkey},
    ///     transaction::Instruction
    /// };
    /// # const PROGRAM: Pubkey = Pubkey::from_bytes(&[2; 32]);
    /// let keypair = Keypair::generate();
    /// let instr = Instruction::new(
    ///     PROGRAM,
    ///     vec![AccountMeta::signing(keypair.pubkey(), Writable::Yes)?],
    ///     &Vec::<u8>::new()
    /// );
    /// // write me later
    /// # Ok::<(), Error>(())
    /// ```
    #[expect(clippy::unwrap_used)]
    pub fn new<A, D>(program_id: Pubkey, accounts: A, payload: &D) -> Self
    where
        A: Into<Vec<AccountMeta>>,
        D: BorshSerialize,
    {
        let data = borsh::to_vec(payload).unwrap();
        Self {
            program_id,
            accounts: accounts.into(),
            data,
        }
    }

    /// Get the instruction's payload
    #[mutants::skip]
    #[must_use]
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Get the executing program's public key.
    #[must_use]
    pub fn program(&self) -> &Pubkey {
        &self.program_id
    }

    /// Get the list of accounts expected by the instruction.
    #[must_use]
    pub fn accounts(&self) -> &[AccountMeta] {
        &self.accounts
    }
}

impl CompiledInstruction {
    /// Creates a new compiled instruction.
    ///
    /// # Parameters
    /// * `program_account_id` - The id of the program in the [`Message`]'s accounts.
    /// * `data` - The instruction's payload,
    /// * `accounts` - The id of the accounts referenced by the instruction in the [`Message`]'s accounts.
    #[must_use]
    pub const fn new(program_account_id: u8, data: Vec<u8>, accounts: Vec<u8>) -> Self {
        Self {
            program_account_id,
            data,
            accounts,
        }
    }
}
