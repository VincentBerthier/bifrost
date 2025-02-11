// File: src/validator/processor.rs
// Project: Bifrost
// Creation date: Saturday 08 February 2025
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

use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, LazyLock, OnceLock},
};

use tokio::sync::{Mutex, Notify};
use tracing::{debug, instrument, trace, warn};

use super::{Error, Result};
use crate::{crypto::Signature, io::Vault, transaction::Transaction};

static TRANSACTION_QUEUE: LazyLock<Mutex<VecDeque<Transaction>>> =
    LazyLock::new(|| Mutex::new(VecDeque::new()));
static TRANSACTION_RECEIVED: LazyLock<Arc<Notify>> = LazyLock::new(|| Arc::new(Notify::new()));
static TRANSACTIONS_STATUS: LazyLock<Arc<Mutex<HashMap<Signature, Status>>>> =
    LazyLock::new(|| Arc::new(Mutex::new(HashMap::new())));

static VAULT: OnceLock<Arc<Vault>> = OnceLock::new();

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
enum Status {
    Failed,
    #[default]
    Pending,
    Running,
    Succeeded,
}

async fn update_trx_status(sig: Signature, status: Status) {
    TRANSACTIONS_STATUS.lock().await.insert(sig, status);
}

#[instrument(skip_all)]
async fn register_transaction(trx: Transaction) -> Result<()> {
    debug!("registering new transaction");
    if !trx.is_valid() {
        warn!("cannot add an invalid transaction (signature issue)");
        return Err(Error::InvalidTransactionSignatures);
    }

    trace!("adding transaction");
    #[expect(clippy::unwrap_used, reason = "trx is valid, so signature exists")]
    update_trx_status(trx.signature().copied().unwrap(), Status::Pending).await;
    TRANSACTION_QUEUE.lock().await.push_back(trx);
    TRANSACTION_RECEIVED.notify_one();

    Ok(())
}

#[expect(clippy::unwrap_used, reason = "trx is valid, so signature exists")]
#[instrument]
async fn processor() -> ! {
    loop {
        trace!("waiting for notification");
        TRANSACTION_RECEIVED.notified().await;
        let Some(trx) = TRANSACTION_QUEUE.lock().await.pop_front() else {
            warn!("got notified of transaction presence but didn’t find one…");
            continue;
        };
        let sig = *trx.signature().unwrap();
        update_trx_status(sig, Status::Succeeded).await;
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    #![expect(clippy::unwrap_used)]

    use std::assert_matches::assert_matches;

    use ed25519_dalek::PUBLIC_KEY_LENGTH;
    use test_log::test;
    use tokio::time::{sleep, Duration};

    use crate::account::{InstructionAccountMeta, Writable};
    use crate::crypto::{Keypair, Pubkey};
    use crate::transaction::{Instruction, Transaction};

    use super::super::Error;
    use super::*;
    type TestResult = core::result::Result<(), Box<dyn core::error::Error>>;
    type Result<T> = core::result::Result<T, Box<dyn core::error::Error>>;

    pub const PROGRAM: Pubkey = Pubkey::from_bytes(&[2; PUBLIC_KEY_LENGTH]);

    fn create_unsigned_transaction() -> Result<Transaction> {
        let keypair = Keypair::generate();
        let mut trx = Transaction::new(0);
        let instruction = Instruction::new(
            PROGRAM,
            vec![
                InstructionAccountMeta::signing(keypair.pubkey(), Writable::Yes)?,
                InstructionAccountMeta::wallet(keypair.pubkey(), Writable::No)?,
            ],
            &Vec::<u8>::new(),
        );

        trx.add(&[instruction])?;

        Ok(trx)
    }

    fn create_signed_transaction() -> Result<Transaction> {
        let keypair = Keypair::generate();
        let mut trx = Transaction::new(0);
        let instruction = Instruction::new(
            PROGRAM,
            vec![
                InstructionAccountMeta::signing(keypair.pubkey(), Writable::Yes)?,
                InstructionAccountMeta::wallet(keypair.pubkey(), Writable::No)?,
            ],
            &Vec::<u8>::new(),
        );

        trx.add(&[instruction])?;
        trx.sign(&keypair)?;

        Ok(trx)
    }

    fn launch_transaction_processor() {
        tokio::spawn(async { processor().await });
    }

    #[test(tokio::test)]
    async fn accepts_valid_transactions_only() -> TestResult {
        // Given
        let trx = create_unsigned_transaction()?;
        let trx_signed = create_signed_transaction()?;

        // When
        let res = register_transaction(trx).await;
        register_transaction(trx_signed).await?;

        // Then
        assert_matches!(res, Err(Error::InvalidTransactionSignatures));
        Ok(())
    }

    #[test(tokio::test)]
    async fn add_transaction_to_queue() -> TestResult {
        // Given
        let trx = create_signed_transaction()?;

        // When
        register_transaction(trx).await?;

        // Then
        assert_eq!(TRANSACTION_QUEUE.lock().await.len(), 1);
        Ok(())
    }

    #[test(tokio::test)]
    async fn processing_a_trx_removes_it_from_the_queue() -> TestResult {
        // Given
        let trx = create_signed_transaction()?;
        launch_transaction_processor();
        register_transaction(trx).await?;

        // When
        sleep(Duration::from_millis(2)).await;

        // Then
        assert!(TRANSACTION_QUEUE.lock().await.is_empty());
        Ok(())
    }

    #[test(tokio::test)]
    async fn processing_transaction_marks_it_suceeded() -> TestResult {
        // Given
        let trx = create_signed_transaction()?;
        let sig = *trx.signature().unwrap();
        launch_transaction_processor();
        register_transaction(trx).await?;

        // When
        sleep(Duration::from_millis(2)).await;

        // Then
        assert_matches!(TRANSACTIONS_STATUS.lock().await.get(&sig), Some(&status) if status == Status::Succeeded);

        Ok(())
    }
}
