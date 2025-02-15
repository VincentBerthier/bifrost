// File: src/crypto/error.rs
// Project: Bifrost
// Creation date: Friday 07 February 2025
// Author: Vincent Berthier <vincent.berthier@posteo.org>
// -----
// Last modified: Sunday 16 February 2025 @ 00:41:21
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
use ed25519_dalek::SignatureError;

/// Errors of the cryptography module.
#[derive(Debug, Display, From)]
#[display("during a cryptographic operation: {_variant}")]
pub enum Error {
    /// Impossible to generate an off curve public key with the given seeds.
    NoOffcurveKeyForSeeds,
    /// Could not obtain the lock on the random engine used to generate private keys.
    RandomEnginePoisonedLock,
    /// Tried to used too many seeds to derive a public key.
    TooManySeeds,
    /// When byte array doesn't have the right size for a block hash
    #[display("the given hash is not compatible with a block hash")]
    WrongHashLength,
    /// Could not decode a string as `base58`
    #[from]
    Bs58Decoding(bs58::decode::Error),
    /// Failed to verify a signature
    #[from]
    Signature(SignatureError),
}

impl core::error::Error for Error {}
