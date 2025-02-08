// File: src/crypto/signature.rs
// Project: Bifrost
// Creation date: Friday 07 February 2025
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

use std::fmt;

use borsh::{BorshDeserialize, BorshSerialize};
use ed25519_dalek::{VerifyingKey, SIGNATURE_LENGTH};
use tracing::{debug, instrument};

use super::{Pubkey, Result};

/// The signature of a transaction.
#[derive(Copy, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct Signature {
    data: [u8; SIGNATURE_LENGTH],
}

impl Signature {
    /// Verify that the signature matches a public key and message.
    ///
    /// # Parameters
    /// * `pubkey` - the public key who supposedly signed the message,
    /// * `message` - the message that was signed.
    ///
    /// # Errors
    /// If the signature does *not* match.
    ///
    /// # Example
    /// ```rust
    /// # use bifrost::crypto::{Keypair, Error};
    /// let key = Keypair::generate();
    /// let message = b"some message";
    /// let signature = key.sign(message);
    /// assert!(signature.verify(&key.pubkey(), message).is_ok());
    ///
    ///
    /// # Ok::<(), Error>(())
    /// ```
    #[instrument(skip_all, fields(?self, ?pubkey))]
    pub fn verify<B>(&self, pubkey: &Pubkey, message: B) -> Result<()>
    where
        B: AsRef<[u8]>,
    {
        debug!("verifying signature");
        let key: VerifyingKey = pubkey.into();
        let signature = ed25519_dalek::Signature::from_bytes(&self.data);
        Ok(key.verify_strict(message.as_ref(), &signature)?)
    }
}

impl From<ed25519_dalek::Signature> for Signature {
    fn from(value: ed25519_dalek::Signature) -> Self {
        Self {
            data: value.to_bytes(),
        }
    }
}

#[mutants::skip]
impl fmt::Debug for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let encoded = bs58::encode(&self.data).into_string();
        write!(f, "{encoded}",)
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {

    use std::assert_matches::assert_matches;

    use test_log::test;

    use crate::crypto::Keypair;

    type Error = Box<dyn core::error::Error>;
    type TestResult = core::result::Result<(), Error>;

    #[test]
    fn check_signature() -> TestResult {
        // Given
        let message = b"some super important data for sure";
        let key1 = Keypair::generate();
        let pubkey1 = key1.pubkey();
        let key2 = Keypair::generate();
        let pubkey2 = key2.pubkey();

        // When
        let signature = key1.sign(message);

        // Then
        signature.verify(&pubkey1, message)?;
        assert_matches!(
            signature.verify(&pubkey2, message),
            Err(super::super::Error::Signature(_))
        );

        Ok(())
    }
}
