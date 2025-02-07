// File: src/transaction/instruction.rs
// Project: Bifrost
// Creation date: Saturday 08 February 2025
// Author: Vincent Berthier <vincent.berthier@posteo.org>
// -----
// Last modified: Saturday 08 February 2025 @ 20:00:03
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

use crate::{account::InstructionAccountMeta, crypto::Pubkey};

#[derive(Clone, Debug, Default, BorshSerialize, BorshDeserialize)]
pub struct CompiledInstruction {
    program_account_id: u8,
    data: Vec<u8>,
    accounts: Vec<u8>,
}

#[derive(Clone, Debug, BorshSerialize)]
pub struct Instruction {
    program_id: Pubkey,
    accounts: Vec<InstructionAccountMeta>,
    data: Vec<u8>,
}

#[expect(clippy::missing_const_for_fn, reason = "false positive")]
impl Instruction {
    #[expect(clippy::unwrap_used)]
    pub fn new<A, D>(program_id: Pubkey, accounts: A, payload: &D) -> Self
    where
        A: Into<Vec<InstructionAccountMeta>>,
        D: BorshSerialize,
    {
        let data = borsh::to_vec(payload).unwrap();
        Self {
            program_id,
            accounts: accounts.into(),
            data,
        }
    }

    #[mutants::skip]
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn program(&self) -> &Pubkey {
        &self.program_id
    }

    pub fn accounts(&self) -> &[InstructionAccountMeta] {
        &self.accounts
    }
}

impl CompiledInstruction {
    pub const fn new(program_account_id: u8, data: Vec<u8>, accounts: Vec<u8>) -> Self {
        Self {
            program_account_id,
            data,
            accounts,
        }
    }
}
