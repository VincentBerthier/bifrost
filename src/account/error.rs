// File: src/account/error.rs
// Project: Bifrost
// Creation date: Saturday 08 February 2025
// Author: Vincent Berthier <vincent.berthier@posteo.org>
// -----
// Last modified: Sunday 09 February 2025 @ 16:52:47
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

use derive_more::derive::{Display, From};

use crate::crypto::Pubkey;

use super::types::AccountType;

/// Errors of the account module.
#[derive(Debug, Display, From)]
#[display("during an account operation: {_variant}")]
pub enum Error {
    /// An operation would have caused an overflow.
    #[display("arithmetic overflow")]
    ArithmeticOverflow,
    /// Invalid key used to create account metadata
    #[display("invalid key use: {} (error: {:?})", key, kind)]
    MetaAccountCreation {
        /// The key that was used
        key: Pubkey,
        /// The type of error
        kind: ErrorType,
    },
    /// Tried to merge accounts  of different types
    #[display("tried to merge accounts of different types ({:?}, {:?})", _0, _1)]
    MergeIncompatibleAccountTypes(AccountType, AccountType),
    /// There were not enough accounts for the instruction
    #[display("there were not enough accounts for the instruction")]
    MissingAccounts,
    /// Tried to modify a read only account
    #[display("account '{key}' is read-only but there was an attempt to modify it")]
    ModificationOfReadOnlyAccount {
        /// Public key of the account
        key: Pubkey,
    },
}

#[derive(Debug)]
pub enum ErrorType {
    NonWalletOnCurve,
    WalletNotOnCurve,
}

impl core::error::Error for Error {}
