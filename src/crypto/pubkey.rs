// File: src/crypto/pubkey.rs
// Project: Bifrost
// Creation date: Friday 07 February 2025
// Author: Vincent Berthier <vincent.berthier@posteo.org>
// -----
// Last modified: Friday 07 February 2025 @ 18:01:46
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
    fmt::{Debug, Display, Formatter},
    str::FromStr,
};

use curve25519_dalek::edwards::CompressedEdwardsY;
use ed25519_dalek::{VerifyingKey, PUBLIC_KEY_LENGTH};
use tracing::{debug, instrument};

use super::error::Error;

/// A public key
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Pubkey {
    /// Byte representation of the public key.
    key: [u8; PUBLIC_KEY_LENGTH],
}

impl Pubkey {
    /// Creates a public key from an array of bytes.
    ///
    /// # Parameters
    /// * `bytes` - Byte array of length 32 representing the public key.
    ///
    /// # Returns
    /// The newly created public key.
    ///
    /// # Example
    /// ```rust
    /// # use bifrost::crypto::Pubkey;
    /// let array = [0_u8; 32];
    /// let pubkey = Pubkey::from_bytes(&array);
    /// ```
    #[must_use]
    pub const fn from_bytes(bytes: &[u8; PUBLIC_KEY_LENGTH]) -> Self {
        Self { key: *bytes }
    }

    /// Check if the public key is on or off the `ed25519` curve
    ///
    /// # Returns
    /// `true` if the public key is on the `ed25519` curve, false otherwise.
    ///
    /// # Example
    /// ```rust
    /// # use bifrost::crypto::{Keypair, Error};
    /// let key = Keypair::generate()?.pubkey();
    /// assert!(key.is_oncurve());
    ///
    /// # Ok::<(), Error>(())
    /// ```
    #[instrument(skip_all, fields(%self))]
    pub fn is_oncurve(&self) -> bool {
        debug!("checking if key is on curve");
        matches!(CompressedEdwardsY::from_slice(&self.key), Ok(key) if key.decompress().is_some())
    }
}

impl From<VerifyingKey> for Pubkey {
    fn from(value: VerifyingKey) -> Self {
        Self {
            key: value.to_bytes(),
        }
    }
}

#[expect(clippy::unwrap_used, clippy::fallible_impl_from)]
impl From<&Pubkey> for VerifyingKey {
    fn from(value: &Pubkey) -> Self {
        Self::from_bytes(&value.key).unwrap()
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

#[mutants::skip]
impl Debug for Pubkey {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let encoded = bs58::encode(&self.key).into_string();
        write!(f, "{encoded}")
    }
}

#[mutants::skip]
impl Display for Pubkey {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let encoded = bs58::encode(&self.key).into_string();
        write!(f, "{encoded}")
    }
}

#[mutants::skip]
impl AsRef<[u8]> for Pubkey {
    fn as_ref(&self) -> &[u8] {
        &self.key
    }
}
