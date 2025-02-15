// File: src/validator/block.rs
// Project: Bifrost
// Creation date: Sunday 16 February 2025
// Author: Vincent Berthier <vincent.berthier@posteo.org>
// -----
// Last modified: Sunday 16 February 2025 @ 01:20:00
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

use std::fmt::Debug;

use sha2::{Digest as _, Sha512};
use tracing::{debug, instrument};

use crate::crypto::Signature;

use super::blockhash::BlockHash;

pub const GENESIS_BLOCK: &str =
    "4n1FyWzYPeGUndCLBAaWVMKZ5gCv1EJvgKwTrLSpnz8uJQ7E3zdhTXaFg4UaiLP9aPK5dmccZK2qKfZjYgc16kzd";

#[derive(Clone, Debug)]
pub struct Block {
    pub hash: BlockHash,
    pub parent: BlockHash,
    pub slot: u64,
    pub transactions: Vec<Signature>,
}

impl Block {
    #[expect(clippy::unwrap_used)]
    pub fn genesis() -> Self {
        Self {
            hash: BlockHash::default(),
            parent: GENESIS_BLOCK.parse().unwrap(),
            slot: 1,
            transactions: Vec::new(),
        }
    }

    fn add_transaction(&mut self, sig: Signature) {
        self.transactions.push(sig);
    }

    #[instrument(skip_all, fields(slot = self.slot))]
    fn finalize(&mut self) -> Self {
        debug!("finalizing block");

        let hash = self.get_hash();
        self.hash = hash;
        let res = self.clone();
        self.slot += 1;
        self.transactions.clear();
        self.parent = hash;

        res
    }

    #[expect(clippy::little_endian_bytes, clippy::unwrap_used)]
    #[instrument(skip_all, fields(slot = self.slot, parent = ?self.parent, sigs = self.transactions.len()))]
    pub fn get_hash(&self) -> BlockHash {
        debug!("getting block hash");
        let mut hasher = Sha512::new();
        hasher.update(self.parent);
        hasher.update(self.slot.to_le_bytes());
        self.transactions.iter().for_each(|sig| hasher.update(sig));

        BlockHash::from_bytes(&hasher.finalize()).unwrap()
    }
}

impl PartialEq for Block {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {

    use test_log::test;

    use super::*;
    type TestResult = core::result::Result<(), Box<dyn core::error::Error>>;

    #[expect(clippy::unwrap_used)]
    fn hand_generate() -> Vec<Block> {
        let mut res = Vec::new();
        let mut block = Block {
            hash: BlockHash::default(),
            parent: GENESIS_BLOCK.parse().unwrap(),
            slot: 0,
            transactions: Vec::new(),
        };

        for slot in 1..=10 {
            block.slot = slot;
            block.hash = block.get_hash();
            res.push(block.clone());
            block.parent = block.hash;
        }

        res
    }

    #[test]
    fn chain_empty_blocks() {
        // Given
        let expected = hand_generate();
        let mut blocks = Vec::<Block>::new();
        let mut block = Block::genesis();

        // When
        for _slot in 1_u8..=10 {
            blocks.push(block.finalize());
        }

        // Then
        blocks
            .iter()
            .zip(expected.iter())
            .for_each(|(b, e)| assert_eq!(b, e));
    }

    #[test]
    fn add_signature_changes_hash() -> TestResult {
        // Given
        const SIG: &str = "C8i3iCwbBEj18akAHUGFE8AxrbRCmHV4T12CnWBnV3z9AAKSxVR2RJMgUFYXqUPfaHKJnHqsftgwNFJ81G9voNf";
        let sig: Signature = SIG.parse()?;
        let mut block1 = Block::genesis();
        let mut block2 = Block::genesis();

        // When
        let finalized_block1 = block1.finalize();
        block2.add_transaction(sig);
        let finalized_block2 = block2.finalize();

        // Then
        assert_ne!(finalized_block1, finalized_block2);

        Ok(())
    }
}
