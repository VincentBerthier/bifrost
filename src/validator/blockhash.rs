// File: src/validator/blockhash.rs
// Project: Bifrost
// Creation date: Sunday 16 February 2025
// Author: Vincent Berthier <vincent.berthier@posteo.org>
// -----
// Last modified: Sunday 16 February 2025 @ 01:13:25
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

use std::{fmt::Debug, str::FromStr};

use super::{Error, Result};

/// The type of a block hash.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BlockHash([u8; 64]);

impl BlockHash {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let bytes = bytes
            .to_vec()
            .try_into()
            .map_err(|_err| Error::WrongHashLength)?;
        Ok(Self(bytes))
    }
}

impl Default for BlockHash {
    fn default() -> Self {
        Self([0; 64])
    }
}

impl Debug for BlockHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string = bs58::encode(self).into_string();
        write!(f, "{string}")
    }
}

impl FromStr for BlockHash {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let bytes = bs58::decode(s).into_vec()?;
        let hash = bytes.try_into().map_err(|_err| Error::WrongHashLength)?;
        Ok(Self(hash))
    }
}

impl AsRef<[u8]> for BlockHash {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {

    use std::assert_matches::assert_matches;

    use test_log::test;

    use crate::validator::block::GENESIS_BLOCK;

    use super::super::{Error, Result};
    use super::*;
    type TestResult = core::result::Result<(), Box<dyn core::error::Error>>;

    #[test]
    fn parse_block_hash() -> TestResult {
        // Given
        const INVALID_HASH: &str =
            "LBAaWVMKZ5gCv1EJvgKwTrLSpnz8uJQ7E3zdhTXaFg4UaiLP9aPK5dmccZK2qKfZjYgc16kzd";
        const INVALID_BYTES: [u8; 32] = [0; 32];

        // When
        let _: BlockHash = GENESIS_BLOCK.parse()?;
        let invalid1: Result<BlockHash> = INVALID_HASH.parse();
        let invalid2 = BlockHash::from_bytes(&INVALID_BYTES);

        // Then
        assert_matches!(invalid1, Err(Error::WrongHashLength));
        assert_matches!(invalid2, Err(Error::WrongHashLength));

        Ok(())
    }
}
