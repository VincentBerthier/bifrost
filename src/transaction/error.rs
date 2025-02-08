// File: src/transaction/error.rs
// Project: Bifrost
// Creation date: Saturday 08 February 2025
// Author: Vincent Berthier <vincent.berthier@posteo.org>
// -----
// Last modified: Sunday 09 February 2025 @ 16:42:31
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

/// Errors of the transaction module.
#[derive(Debug, Display, From)]
#[display("while handling a transaction: {_variant}")]
pub enum Error {
    /// The transaction is not signed at all.
    #[display("the transaction has no signer")]
    NoSignersOnTransaction,
    /// The transaction has the wrong number of signatures
    #[display("wrong number of signatures: expected '{expected}', but got '{actual}'")]
    WrongNumberOfSignatures {
        /// Expected number of signatures.
        expected: usize,
        /// Actual number of signatures.
        actual: usize,
    },
    /// At least one signature doesn't match a signer (or vice-versa)
    #[display("mismatch between signers and signatures")]
    SignaturesMismatch,
    /// There was an attempt to sign from an account that's not a signer.
    #[display("'{key}' is not a signing account on this transaction")]
    UnexpectedSigner {
        /// The public key of the account attempting to sign.
        key: Pubkey,
    },
    /// An error that occurred in the accounts module.
    #[display("account error: {_0}")]
    #[from]
    Account(crate::account::Error),
}

impl core::error::Error for Error {}
