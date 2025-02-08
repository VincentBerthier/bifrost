// File: src/error.rs
// Project: Bifrost
// Creation date: Saturday 08 February 2025
// Author: Vincent Berthier <vincent.berthier@posteo.org>
// -----
// Last modified: Sunday 09 February 2025 @ 16:49:32
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

/// Errors of the Bifrost library.
#[derive(Debug, Display, From)]
#[display("Bifrost encountered an error {_variant}")]
pub enum Error {
    /// An error caused by the cryptography module.
    #[from]
    Crypto(crate::crypto::Error),
    /// An error caused by the accounts module.
    #[from]
    Account(crate::account::Error),
    /// An error occurred during an I/O operation.
    #[from]
    Io(crate::io::Error),
    /// An error occurring in the transactions module.
    #[from]
    Transaction(crate::transaction::Error),
    /// Error while configuring the tracing.
    #[display("while configuring the tracing: {_0}")]
    TracingConfiguration(tracing_subscriber::filter::FromEnvError),
}

impl core::error::Error for Error {}
