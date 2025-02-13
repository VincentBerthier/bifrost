// File: src/program/error.rs
// Project: Bifrost
// Creation date: Wednesday 12 February 2025
// Author: Vincent Berthier <vincent.berthier@posteo.org>
// -----
// Last modified: Wednesday 12 February 2025 @ 22:35:14
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

/// Errors of the programs module.
#[derive(Debug, Display, From)]
#[display("while executing a program: {_variant}")]
pub enum Error {
    /// There were not enough accounts for the instruction
    #[display("there were not enough accounts for the instruction")]
    MissingAccounts,
    /// The instruction's payload is invalid
    #[display("payload is invalid for the program: {_0}")]
    #[from]
    InvalidPayload(std::io::Error),
    /// An error happened while trying to access or modify an account.
    #[display("error while operating on an account: {_0}")]
    #[from]
    Account(crate::account::Error),
    /// Custom error form programs.
    #[display("custom program error: {_0}")]
    Custom(String),
}

impl core::error::Error for Error {}
