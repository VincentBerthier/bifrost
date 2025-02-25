// File: src/validator/error.rs
// Project: Bifrost
// Creation date: Saturday 08 February 2025
// Author: Vincent Berthier <vincent.berthier@posteo.org>
// -----
// Last modified: Sunday 16 February 2025 @ 00:10:14
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

/// Errors of the validator module.
#[derive(Debug, Display, From)]
#[display("within the validator: {_variant}")]
pub enum Error {
    /// The transaction's signatures are missing or do not match the expectation.
    #[display("the transaction’s signatures are invalid")]
    InvalidTransactionSignatures,
    /// The total amount of prisms has changed while it's not supposed to.
    #[display("prisms total has changed")]
    PrismTotalChanged,
    /// Error while sending a message to a thread
    #[display("could not send a '{kind}' message")]
    SendMessage {
        /// The kind of message that failed.
        kind: &'static str,
    },
    /// When the lock on the vault could not be obtained.
    #[display("the lock on the vault could not be obtained")]
    VaultLock,
    /// When byte array doesn't have the right size for a block hash
    #[display("the given hash is not compatible with a block hash")]
    WrongHashLength,
    /// An error occurred in the vault
    #[from]
    Io(crate::io::Error),
    /// An error occurred while running a program.
    #[from]
    Program(crate::program::Error),
    /// When a string is not a valid `bs58` encoding of a block hash
    #[from]
    HashParse(bs58::decode::Error),
}

impl core::error::Error for Error {}
