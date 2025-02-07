// File: src/crypto/seeds.rs
// Project: Bifrost
// Creation date: Friday 07 February 2025
// Author: Vincent Berthier <vincent.berthier@posteo.org>
// -----
// Last modified: Friday 07 February 2025 @ 16:53:30
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

use std::fmt::{self, Debug};

use sha2::{Digest, Sha256};
use tracing::{debug, instrument, trace, warn};

use super::{pubkey::Pubkey, Error, Result};

const MAX_SEEDS: usize = 32;

/// The seeds to use to derive an off-curve public key.
pub struct Seeds {
    /// Number of seeds.
    n: usize,
    /// `Hasher` generating the public key.
    hasher: Sha256,
}

impl Seeds {
    /// Create a new `Seeds` object from one or more individual seeds.
    ///
    /// # Parameters
    /// * `seeds` - The seeds (an array of `u8` slices),
    ///
    /// # Returns
    /// The Seeds object.
    ///
    /// # Errors
    /// If too many seeds where added.
    ///
    /// # Example
    /// ```rust
    /// # use bifrost::crypto::{Seeds, Pubkey, Error};
    /// let seeds = Seeds::new(&[b"seed 1", b"seed 2"])?;
    ///
    /// # Ok::<(), Error>(())
    /// ```
    #[instrument(skip_all)]
    pub fn new<S>(seeds: &[S]) -> Result<Self>
    where
        S: AsRef<[u8]>,
    {
        debug!("creating new Seed");
        if seeds.len() > MAX_SEEDS {
            warn!("tried to set too many seeds");
            return Err(Error::TooManySeeds);
        }
        let mut hasher = Sha256::new();
        seeds.iter().for_each(|seed| hasher.update(seed));
        Ok(Self {
            n: seeds.len(),
            hasher,
        })
    }
    /// Add new seeds
    ///
    /// # Parameters
    /// * `seeds` - The seeds (an array of `u8` slices),
    ///
    /// # Errors
    /// If too many seeds where added.
    ///
    /// # Example
    /// ```rust
    /// # use bifrost::crypto::{Seeds, Keypair, Pubkey, Error};
    /// let mut seeds = Seeds::new(&[b"seed 1", b"seed 2"])?;
    /// let key1 = Keypair::generate()?.pubkey();
    /// let key2 = Keypair::generate()?.pubkey();
    /// seeds.add(&[&key1, &key2]);
    ///
    /// # Ok::<(), Error>(())
    /// ```
    pub fn add<S>(&mut self, seeds: &[S]) -> Result<()>
    where
        S: AsRef<[u8]>,
    {
        let n = seeds.len();
        if n + self.n > MAX_SEEDS {
            warn!("tried to set too many seeds");
            return Err(Error::TooManySeeds);
        }
        self.n += n;
        seeds.iter().for_each(|seed| self.hasher.update(seed));

        Ok(())
    }

    /// Generate the public key corresponding to the given seeds.
    ///
    /// In some cases, the given seeds as they are would generate
    /// a public key on the `ed25519` curve. To prevent this
    /// (as much as possible anyway), a 'bump' (an `u8`) is introduced
    /// to try to 'push' the key away from the curve.
    ///
    /// While it succeeds 'most' of the time, it's not a guarantee. Since
    /// the process is deterministic however, a small change to the seed
    /// should solve the problem.
    ///
    /// # Returns
    /// A tuple `(Pubkey, u8)` with the generated public key and the bump
    /// that made sure it was off-curve.
    ///
    /// # Errors
    /// If no off-curve key could be generated.
    ///
    /// # Example
    /// ```rust
    /// # use bifrost::crypto::{Seeds, Pubkey, Error};
    /// let seeds = Seeds::new(&[b"seed 1", b"seed 2"])?;
    /// let (key, bump) = seeds.generate_offcurve()?;
    /// assert!(!key.is_oncurve());
    ///
    /// # Ok::<(), Error>(())
    /// ```
    #[instrument(skip_all)]
    pub fn generate_offcurve(&self) -> Result<(Pubkey, u8)> {
        debug!("generation off-curve public key");
        for bump in 0..255 {
            trace!("trying with bump {bump}");
            let mut hasher = self.hasher.clone();
            hasher.update([bump]);
            let hash = hasher.finalize();
            let pubkey = Pubkey::from_bytes(&hash.as_slice().try_into()?);
            if !pubkey.is_oncurve() {
                trace!("resulting key '{pubkey}' is off-curve, returning");
                return Ok((pubkey, bump));
            }
            trace!("the key is on-curve, trying next bump if possible");
        }
        warn!("could not generate an off-curve public key with the given seeds!");
        Err(Error::NoOffcurveKeyForSeeds)
    }
}

impl Debug for Seeds {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Seeds {{ n: {}}}", self.n)
    }
}

#[cfg(test)]
mod tests {

    use std::assert_matches::assert_matches;

    use test_log::test;

    use crate::crypto::Keypair;

    use super::*;
    type Error = Box<dyn core::error::Error>;
    type TestResult = core::result::Result<(), Error>;

    #[test]
    fn generate_offcurve_pubkey() -> TestResult {
        // Given
        let str_seeds = [b"seed 1".as_slice(), b"seed 2".as_slice()];
        let pubkey = Keypair::generate()?.pubkey();

        // When
        let mut seeds = Seeds::new(&str_seeds)?;
        seeds.add(&[pubkey])?;
        let generated = seeds.generate_offcurve()?.0;

        // Then
        assert!(!generated.is_oncurve());

        Ok(())
    }

    #[test]
    fn prevent_too_many_seeds() -> TestResult {
        // Given
        let mut normal_seeds = Seeds::new(&[[0; 4]; MAX_SEEDS])?;

        // When
        let init_seeds = Seeds::new(&[[0; 4]; MAX_SEEDS + 1]);
        let add_seeds = normal_seeds.add(&[[0_u8; 2]]);

        // Then
        assert_matches!(init_seeds, Err(super::super::Error::TooManySeeds));
        assert_matches!(add_seeds, Err(super::super::Error::TooManySeeds));

        Ok(())
    }
}
