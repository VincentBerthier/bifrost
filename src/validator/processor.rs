// File: src/validator/processor.rs
// Project: Bifrost
// Creation date: Saturday 08 February 2025
// Author: Vincent Berthier <vincent.berthier@posteo.org>
// -----
// Last modified: Thursday 13 February 2025 @ 09:49:56
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
use crate::{
    account::{AccountMeta, TransactionAccount, Wallet},
    crypto::{Pubkey, Signature},
    io::Vault,
    program::dispatcher::dispatch,
    transaction::{CompiledInstruction, Transaction},
};

static TRANSACTION_QUEUE: LazyLock<Mutex<VecDeque<Transaction>>> =
    LazyLock::new(|| Mutex::new(VecDeque::new()));
static TRANSACTION_RECEIVED: LazyLock<Arc<Notify>> = LazyLock::new(|| Arc::new(Notify::new()));
static TRANSACTIONS_STATUS: LazyLock<Arc<Mutex<HashMap<Signature, Status>>>> =
    LazyLock::new(|| Arc::new(Mutex::new(HashMap::new())));

static VAULT: OnceLock<Arc<Mutex<Vault>>> = OnceLock::new();

const CURRENT_SLOT: u64 = 1;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
enum Status {
    Failed,
    #[default]
    Pending,
    Running,
    Succeeded,
}

#[instrument]
async fn update_trx_status(sig: Signature, status: Status) {
    debug!("setting transaction status");
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
    let vault = Vault::load_or_create().await.unwrap();
    let _ = VAULT.get_or_init(|| Arc::new(Mutex::new(vault)));
    loop {
        trace!("waiting for notification");
        TRANSACTION_RECEIVED.notified().await;
        let Some(trx) = TRANSACTION_QUEUE.lock().await.pop_front() else {
            warn!("got notified of transaction presence but didn’t find one…");
            continue;
        };
        let sig = *trx.signature().unwrap();
        match execute_transaction(trx).await {
            Ok(()) => update_trx_status(sig, Status::Succeeded).await,
            Err(err) => {
                warn!("transaction {sig:?} failed to run: {err}");
                update_trx_status(sig, Status::Failed).await;
            }
        }
    }
}

#[expect(clippy::unwrap_used)]
#[instrument(skip_all, fields(sig = ?trx.signature().unwrap()))]
async fn execute_transaction(trx: Transaction) -> Result<()> {
    debug!("executing transaction");
    let metas = trx.message().accounts();
    let mut accounts = get_transaction_accounts(metas).await?;
    let mut trx_accounts = accounts.iter_mut().collect::<Vec<_>>();
    {
        trace!("preparing accounts");
        let trx_accounts2 = trx_accounts
            .iter_mut()
            .enumerate()
            .map(|(i, account)| TransactionAccount::new(&metas[i], account))
            .collect::<Vec<_>>();

        trace!("looping through instructions");
        for instruction in &trx.message().instructions {
            let program = metas[instruction.program_account_id as usize].key();
            execute_instruction(program, instruction, &trx_accounts2)?;
        }
    }

    save_accounts(metas, accounts).await?;

    Ok(())
}

#[instrument(skip_all)]
fn execute_instruction(
    program: &Pubkey,
    instruction: &CompiledInstruction,
    accounts: &[TransactionAccount],
) -> Result<()> {
    debug!("executing instruction");
    let mut instr_accounts = Vec::new();
    for i in &instruction.accounts {
        instr_accounts.push(accounts[*i as usize].clone());
    }

    dispatch(program, &instr_accounts, &instruction.data)?;

    Ok(())
}

#[instrument(skip_all)]
#[expect(clippy::significant_drop_tightening)]
async fn get_transaction_accounts(metas: &[AccountMeta]) -> Result<Vec<Wallet>> {
    debug!("getting the instruction’s account from the disk, creating them if necessary");
    let vault = VAULT.get().ok_or(Error::VaultLock)?.lock().await;
    let mut res = Vec::new();
    for meta in metas {
        let account = vault.get(meta.key()).await?;
        res.push(account);
    }

    Ok(res)
}

#[instrument(skip_all)]
#[expect(clippy::significant_drop_tightening)]
async fn save_accounts(metas: &[AccountMeta], accounts: Vec<Wallet>) -> Result<()> {
    debug!("saving accounts on the disk");
    let mut vault = VAULT.get().ok_or(Error::VaultLock)?.lock().await;
    for (meta, account) in metas.iter().zip(accounts.iter()) {
        vault
            .save_account(*meta.key(), account, CURRENT_SLOT)
            .await?;
    }

    Ok(())
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    #![expect(clippy::unwrap_used)]

    use std::assert_matches::assert_matches;
    use std::fs::remove_dir_all;
    use std::path::PathBuf;

    use ed25519_dalek::PUBLIC_KEY_LENGTH;
    use test_log::test;
    use tokio::time::{sleep, Duration};

    use crate::account::{AccountMeta, Wallet, Writable};
    use crate::crypto::{Keypair, Pubkey};
    use crate::io::set_vault_path;
    use crate::program::system;
    use crate::transaction::{Instruction, Transaction};

    use super::super::Error;
    use super::*;
    type TestResult = core::result::Result<(), Box<dyn core::error::Error>>;
    type Result<T> = core::result::Result<T, Box<dyn core::error::Error>>;

    pub const PROGRAM: Pubkey = Pubkey::from_bytes(&[2; PUBLIC_KEY_LENGTH]);

    fn reset_vault<P>(path: P) -> Result<()>
    where
        P: Into<PathBuf>,
    {
        let path = path.into();
        set_vault_path(&path);
        if path.exists() {
            remove_dir_all(path)?;
        }

        Ok(())
    }

    fn create_unsigned_transaction() -> Result<Transaction> {
        let keypair = Keypair::generate();
        let mut trx = Transaction::new(0);
        let instruction = Instruction::new(
            PROGRAM,
            vec![
                AccountMeta::signing(keypair.pubkey(), Writable::Yes)?,
                AccountMeta::wallet(keypair.pubkey(), Writable::No)?,
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
                AccountMeta::signing(keypair.pubkey(), Writable::Yes)?,
                AccountMeta::wallet(keypair.pubkey(), Writable::No)?,
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
        const VAULT: &str = "/tmp/bifrost/validator-1";
        reset_vault(VAULT)?;
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
    async fn run_system_transfer_transaction() -> TestResult {
        // Given
        const VAULT: &str = "/tmp/bifrost/validator-3";
        const AMOUNT: u64 = 1_000;

        reset_vault(VAULT)?;
        let mut vault_init = Vault::load_or_create().await?;

        let key1 = Keypair::generate();
        let key2 = Keypair::generate().pubkey();
        let wallet1_before = Wallet { prisms: AMOUNT };

        vault_init
            .save_account(key1.pubkey(), &wallet1_before, 0)
            .await?;
        vault_init.save().await?;
        drop(vault_init);

        launch_transaction_processor();
        let mut trx = Transaction::new(0);
        let instruction = system::instruction::transfer(key1.pubkey(), key2, AMOUNT)?;
        trx.add(&[instruction])?;
        trx.sign(&key1)?;
        let sig = *trx.signature().unwrap();

        // When
        register_transaction(trx).await?;
        sleep(Duration::from_millis(5)).await;

        // Then
        super::VAULT
            .get()
            .ok_or(Error::VaultLock)?
            .lock()
            .await
            .save()
            .await?;
        let vault = Vault::load_or_create().await?;
        let wallet1_after = vault.get(&key1.pubkey()).await?;
        let wallet2_after = vault.get(&key2).await?;
        assert_matches!(TRANSACTIONS_STATUS.lock().await.get(&sig), Some(&status) if status == Status::Succeeded);
        assert_eq!(wallet1_after.prisms, 0);
        assert_eq!(wallet2_after.prisms, AMOUNT);

        Ok(())
    }
}
