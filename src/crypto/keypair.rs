// File: src/crypto/keypair.rs
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

use std::sync::{Mutex, OnceLock};

use ed25519_dalek::{SigningKey, KEYPAIR_LENGTH};
use rand::SeedableRng as _;
use rand_chacha::ChaCha20Rng;

use super::{pubkey::Pubkey, Error, Result};

static RNG: OnceLock<Mutex<ChaCha20Rng>> = OnceLock::new();

struct Keypair {
    key: [u8; KEYPAIR_LENGTH],
}

impl Keypair {
    fn generate() -> Result<Self> {
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

    pub fn pubkey(&self) -> Pubkey {
        #[expect(clippy::unwrap_used, reason = "array is guaranteed to be right here")]
        let keypair = SigningKey::from_keypair_bytes(&self.key).unwrap();
        keypair.verifying_key().into()
    }
}

#[cfg(test)]
fn init_rand_engine() -> Mutex<ChaCha20Rng> {
    let seed = 0_u64;
    let rng = ChaCha20Rng::seed_from_u64(seed);

    Mutex::new(rng)
}

#[cfg_attr(coverage_nightly, coverage(off))]
#[cfg(not(test))]
fn init_rand_engine() -> Mutex<ChaCha20Rng> {
    let rng = ChaCha20Rng::from_entropy();

    Mutex::new(rng)
}

#[cfg(test)]
mod tests {
    use test_log::test;

    use super::*;
    type Error = Box<dyn core::error::Error>;
    type Result<T> = core::result::Result<T, Error>;

    #[test]
    fn generate_keypair() -> Result<()> {
        let _ = Keypair::generate()?;

        Ok(())
    }

    #[test]
    fn get_pubkey() -> Result<()> {
        // When
        let keypair = Keypair::generate()?;
        let pubkey = keypair.pubkey();

        // Then
        assert_eq!(
            pubkey,
            "H1LS9EF2cPrmmM828buVJSvvbztLc9buJPHMpqTmgEpa".parse()?
        );

        Ok(())
    }
}
