// File: src/crypto/keypair.rs
// Project: Bifrost
// Creation date: Friday 07 February 2025
// Author: Vincent Berthier <vincent.berthier@posteo.org>
// -----
// Last modified: Friday 07 February 2025 @ 17:30:50
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

use std::sync::{Mutex, OnceLock};

use ed25519_dalek::{ed25519::signature::Signer, SigningKey, KEYPAIR_LENGTH};
use rand::SeedableRng as _;
use rand_chacha::ChaCha20Rng;
use tracing::{debug, info, instrument};

use super::{pubkey::Pubkey, Error, Result, Signature};

static RNG: OnceLock<Mutex<ChaCha20Rng>> = OnceLock::new();

/// A private key
pub struct Keypair {
    /// Byte representation of the private key.
    key: [u8; KEYPAIR_LENGTH],
}

impl Keypair {
    /// Randomly generates a private key.
    ///
    /// # Returns
    /// A private key
    ///
    /// # Errors
    /// If the lock on the random engine could not be obtained.
    ///
    /// # Example
    /// ```rust
    /// # use bifrost::crypto::{Keypair, Error};
    /// let key = Keypair::generate()?;
    /// # Ok::<(), Error>(())
    /// ```
    #[instrument]
    pub fn generate() -> Result<Self> {
        debug!("generating new keypair");
        let key = {
            let mut rng = RNG
                .get_or_init(init_rand_engine)
                .lock()
                .map_err(|_err| Error::RandomEnginePoisonedLock)?;
            SigningKey::generate(&mut *rng)
        };
        Ok(Self {
            key: key.to_keypair_bytes(),
        })
    }

    /// Get the public key associated with the private key.
    ///
    /// # Returns
    /// The public key of the private key.
    ///
    /// # Example
    /// ```rust
    /// # use bifrost::crypto::{Keypair, Error};
    /// let private_key = Keypair::generate()?;
    /// let public_key = private_key.pubkey();
    ///
    /// # Ok::<(), Error>(())
    /// ```
    #[instrument(skip_all)]
    #[must_use]
    pub fn pubkey(&self) -> Pubkey {
        debug!("getting pubkey");
        #[expect(clippy::unwrap_used, reason = "array is guaranteed to be right here")]
        let keypair = SigningKey::from_keypair_bytes(&self.key).unwrap();
        keypair.verifying_key().into()
    }

    /// Sign a message.
    ///
    /// # Parameters
    /// * `message` - The message to sign,
    ///
    /// # Returns
    /// The signature of the message
    ///
    /// # Example
    /// ```rust
    /// # use bifrost::crypto::{Keypair, Error};
    /// let key = Keypair::generate()?;
    /// let message = b"some message";
    /// let signature = key.sign(message);
    ///
    /// # Ok::<(), Error>(())
    /// ```
    #[instrument(skip_all, fields(key = ?self.pubkey()))]
    pub fn sign<B>(&self, message: B) -> Signature
    where
        B: AsRef<[u8]>,
    {
        debug!("signing message");
        #[expect(clippy::unwrap_used)]
        let key = SigningKey::from_keypair_bytes(&self.key).unwrap();
        key.sign(message.as_ref()).into()
    }
}

#[cfg(test)]
fn init_rand_engine() -> Mutex<ChaCha20Rng> {
    info!("Initialized keypair random generator in TEST MODE");
    let seed = 0_u64;
    let rng = ChaCha20Rng::seed_from_u64(seed);

    Mutex::new(rng)
}

#[cfg_attr(coverage_nightly, coverage(off))]
#[cfg(not(test))]
fn init_rand_engine() -> Mutex<ChaCha20Rng> {
    info!("Initialized keypair random generator");
    let rng = ChaCha20Rng::from_entropy();

    Mutex::new(rng)
}

#[cfg(test)]
mod tests {
    use test_log::test;

    use super::*;
    type Error = Box<dyn core::error::Error>;
    type TestResult = core::result::Result<(), Error>;

    #[test]
    fn generate_keypair() -> TestResult {
        let _ = Keypair::generate()?;

        Ok(())
    }

    #[test]
    fn get_pubkey() -> TestResult {
        // When
        let keypair = Keypair::generate()?;
        let pubkey = keypair.pubkey();

        // Then
        assert_eq!(
            pubkey,
            "H1LS9EF2cPrmmM828buVJSvvbztLc9buJPHMpqTmgEpa".parse()?
        );
        assert!(pubkey.is_oncurve());

        Ok(())
    }
}
