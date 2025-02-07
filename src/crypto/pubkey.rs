// File: src/crypto/pubkey.rs
// Project: Bifrost
// Creation date: Friday 07 February 2025
// Author: Vincent Berthier <vincent.berthier@posteo.org>
// -----
// Last modified: Friday 07 February 2025 @ 16:58:30
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

use std::{
    fmt::{Debug, Formatter},
    str::FromStr,
};

use ed25519_dalek::{VerifyingKey, PUBLIC_KEY_LENGTH};

use super::error::Error;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Pubkey {
    key: [u8; PUBLIC_KEY_LENGTH],
}

impl From<VerifyingKey> for Pubkey {
    fn from(value: VerifyingKey) -> Self {
        Self {
            key: value.to_bytes(),
        }
    }
}

impl FromStr for Pubkey {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let decoded = bs58::decode(s).into_vec()?;
        let bytes: [u8; PUBLIC_KEY_LENGTH] = decoded.as_slice().try_into()?;
        Ok(Self { key: bytes })
    }
}

impl Debug for Pubkey {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let encoded = bs58::encode(&self.key).into_string();
        write!(f, "{encoded}")
    }
}
