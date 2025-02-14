// File: src/validator/processor.rs
// Project: Bifrost
// Creation date: Saturday 08 February 2025
// Author: Vincent Berthier <vincent.berthier@posteo.org>
// -----
// Last modified: Friday 14 February 2025 @ 15:44:46
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

use std::sync::{Arc, LazyLock};

use async_channel::{unbounded, Receiver, Sender};
use tokio::{
    select,
    sync::{
        mpsc::{channel, Receiver as TReceiver, Sender as TSender},
        oneshot::Receiver as OReceiver,
        RwLock,
    },
};
use tracing::{debug, info, instrument, trace, warn};

use super::{Error, Result};
use crate::{
    account::{AccountMeta, TransactionAccount, Wallet},
    crypto::Pubkey,
    io::Vault,
    program::dispatcher::dispatch,
    transaction::{CompiledInstruction, Transaction},
};

static TRANSACTION_QUEUE: LazyLock<TransactionQueue> = LazyLock::new(TransactionQueue::new);

const TRANSACTION_FEE: u64 = 5_000;
const CURRENT_SLOT: u64 = 1;

struct TransactionQueue {
    sender: Arc<Sender<(Transaction, TSender<Status>)>>,
    receiver: Arc<Receiver<(Transaction, TSender<Status>)>>,
}

impl TransactionQueue {
    fn new() -> Self {
        let (tx, rx) = unbounded();
        Self {
            sender: Arc::new(tx),
            receiver: Arc::new(rx),
        }
    }

    async fn send(&self, transaction: Transaction, status_tx: TSender<Status>) {
        #[expect(
            clippy::unwrap_used,
            reason = "can only fail if the validator is terminated"
        )]
        self.sender.send((transaction, status_tx)).await.unwrap();
    }

    fn get_receiver(&self) -> Arc<Receiver<(Transaction, TSender<Status>)>> {
        Arc::clone(&self.receiver)
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
enum Status {
    Failed,
    #[default]
    Pending,
    Running,
    Succeeded,
}

#[instrument(skip_all)]
async fn register_transaction(trx: Transaction) -> Result<TReceiver<Status>> {
    debug!("registering new transaction");
    if !trx.is_valid() {
        warn!("cannot add an invalid transaction (signature issue)");
        return Err(Error::InvalidTransactionSignatures);
    }

    trace!("adding transaction");
    let (tx, rx) = channel(5);
    #[expect(clippy::unwrap_used, reason = "channel was just created, can’t fail")]
    tx.send(Status::Pending).await.unwrap();
    TRANSACTION_QUEUE.send(trx, tx).await;

    Ok(rx)
}

#[mutants::skip]
#[instrument(skip_all)]
async fn processor(vault: Arc<RwLock<Vault>>, stop_control: OReceiver<()>) {
    let mut stop_control = stop_control;
    let queue = TRANSACTION_QUEUE.get_receiver();
    loop {
        trace!("waiting for notification");
        select! {
            Ok(()) = &mut stop_control => {
                info!("stop control called, ending processor thread");
                break;
            }
            Ok((trx, tx_status)) = queue.recv() => {
                trace!("transaction received");
                execute_transaction(&vault, trx, tx_status).await;
            }
            else => {
                warn!("something weird happened here…");
            }
        }
    }
    debug!("processor thread exited");
}

#[expect(clippy::unwrap_used, reason = "the receivers cannot have been dropped")]
async fn execute_transaction(vault: &RwLock<Vault>, trx: Transaction, tx_status: TSender<Status>) {
    let sig = *trx.signature().unwrap();
    match execute_transaction_inner(vault, trx).await {
        Ok(()) => tx_status.send(Status::Succeeded).await.unwrap(),
        Err(err) => {
            warn!("transaction {sig:?} failed to run: {err}");
            tx_status.send(Status::Failed).await.unwrap();
        }
    }
}

#[expect(clippy::unwrap_used)]
#[instrument(skip_all, fields(sig = ?trx.signature().unwrap()))]
async fn execute_transaction_inner(vault: &RwLock<Vault>, trx: Transaction) -> Result<()> {
    debug!("executing transaction");
    let metas = trx.message().accounts();
    let payer = trx.message().get_payer().unwrap();
    let mut accounts = get_transaction_accounts(vault, metas).await?;
    let mut mut_accounts = accounts.iter_mut().collect::<Vec<_>>();

    let payer_id = metas.iter().position(|meta| *meta.key() == payer).unwrap();
    mut_accounts[payer_id].prisms -= TRANSACTION_FEE;
    let total_prisms = mut_accounts
        .iter()
        .fold(0, |acc, account| acc + account.prisms);

    {
        trace!("preparing accounts");
        let trx_accounts = mut_accounts
            .iter_mut()
            .enumerate()
            .map(|(i, account)| TransactionAccount::new(&metas[i], account))
            .collect::<Vec<_>>();

        trace!("looping through instructions");
        for instruction in &trx.message().instructions {
            let program = metas[instruction.program_account_id as usize].key();
            execute_instruction(program, instruction, &trx_accounts)?;
        }
    }
    let new_total_prisms = accounts.iter().fold(0, |acc, account| acc + account.prisms);
    if total_prisms != new_total_prisms {
        warn!("there was a change in the total of prisms: ignoring transaction");
        return Err(Error::PrismTotalChanged);
    }

    save_accounts(vault, metas, accounts).await?;

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
async fn get_transaction_accounts(
    vault: &RwLock<Vault>,
    metas: &[AccountMeta],
) -> Result<Vec<Wallet>> {
    debug!("getting the instruction’s account from the disk, creating them if necessary");
    let vault = vault.read().await;
    let mut res = Vec::new();
    for meta in metas {
        let account = vault.get(meta.key()).await?;
        res.push(account);
    }

    Ok(res)
}

#[instrument(skip_all)]
#[expect(clippy::significant_drop_tightening)]
async fn save_accounts(
    vault: &RwLock<Vault>,
    metas: &[AccountMeta],
    accounts: Vec<Wallet>,
) -> Result<()> {
    debug!("saving accounts on the disk");
    let mut vault = vault.write().await;
    for (meta, account) in metas.iter().zip(accounts.iter()) {
        if !meta.is_writable() {
            continue;
        }
        vault
            .save_account(*meta.key(), account, CURRENT_SLOT)
            .await?;
    }

    Ok(())
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    #![expect(clippy::shadow_unrelated)]

    use std::assert_matches::assert_matches;
    use std::fs::remove_dir_all;
    use std::path::PathBuf;

    use ed25519_dalek::PUBLIC_KEY_LENGTH;
    use test_log::test;
    use tokio::sync::oneshot::{channel, Sender as OSender};
    use tokio::task::JoinHandle;
    use tracing::info;

    use crate::account::{AccountMeta, Wallet, Writable};
    use crate::crypto::{Keypair, Pubkey};
    use crate::io::set_vault_path;
    use crate::program::{system, testing_dummy};
    use crate::transaction::{Instruction, Transaction};

    use super::super::Error;
    use super::*;
    type TestResult = core::result::Result<(), Box<dyn core::error::Error>>;
    type Result<T> = core::result::Result<T, Box<dyn core::error::Error>>;

    pub const PROGRAM: Pubkey = Pubkey::from_bytes(&[2; PUBLIC_KEY_LENGTH]);

    async fn reset_vault<P>(path: P) -> Result<Vault>
    where
        P: Into<PathBuf>,
    {
        let path = path.into();
        set_vault_path(&path);
        if path.exists() {
            remove_dir_all(path)?;
        }
        let vault = Vault::load_or_create().await?;

        Ok(vault)
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

    fn launch_transaction_processor(vault: Arc<RwLock<Vault>>) -> (OSender<()>, JoinHandle<()>) {
        let (tx, rx) = channel();
        let handle = tokio::spawn(async { processor(vault, rx).await });
        (tx, handle)
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
    async fn run_system_transfer_transaction() -> TestResult {
        // Given
        const VAULT: &str = "/tmp/bifrost/validator-3";
        const AMOUNT: u64 = 1_000_000;

        let mut vault = reset_vault(VAULT).await?;

        let key1 = Keypair::generate();
        let key2 = Keypair::generate().pubkey();
        let wallet1_before = Wallet { prisms: AMOUNT };

        vault
            .save_account(key1.pubkey(), &wallet1_before, 0)
            .await?;
        vault.save().await?;

        let vault = Arc::new(RwLock::new(vault));

        let (stop_control, handle) = launch_transaction_processor(Arc::clone(&vault));
        let mut trx = Transaction::new(0);
        let instruction = system::instruction::transfer(key1.pubkey(), key2, 500_000)?;
        trx.add(&[instruction])?;
        trx.sign(&key1)?;

        // When
        let mut status = Status::Pending;
        let mut rx = register_transaction(trx).await?;
        while let Some(new_status) = rx.recv().await {
            info!("received new transaction status: {new_status:?}");
            status = new_status;
        }
        #[expect(clippy::unwrap_used)]
        stop_control.send(()).unwrap();
        handle.await?;
        vault.write().await.save().await?;

        // Then
        let vault = Vault::load_or_create().await?;
        let wallet1_after = vault.get(&key1.pubkey()).await?;
        let wallet2_after = vault.get(&key2).await?;
        assert_eq!(status, Status::Succeeded);
        assert_eq!(wallet1_after.prisms, AMOUNT - 500_000 - TRANSACTION_FEE);
        assert_eq!(wallet2_after.prisms, 500_000);

        Ok(())
    }

    #[test(tokio::test)]
    async fn fail_system_transfer_transaction() -> TestResult {
        // Given
        const VAULT: &str = "/tmp/bifrost/validator-4";
        const AMOUNT: u64 = 500_000;

        let mut vault = reset_vault(VAULT).await?;

        let key1 = Keypair::generate();
        let key2 = Keypair::generate().pubkey();
        let wallet1_before = Wallet { prisms: AMOUNT };

        vault
            .save_account(key1.pubkey(), &wallet1_before, 0)
            .await?;
        vault.save().await?;

        let vault = Arc::new(RwLock::new(vault));
        let (stop_control, handle) = launch_transaction_processor(vault);
        let mut trx = Transaction::new(0);
        let instruction = system::instruction::transfer(key1.pubkey(), key2, 500_000)?;
        trx.add(&[instruction])?;
        trx.sign(&key1)?;

        // When
        let mut status = Status::Pending;
        let mut rx = register_transaction(trx).await?;
        while let Some(new_status) = rx.recv().await {
            info!("received new transaction status: {new_status:?}");
            status = new_status;
        }
        #[expect(clippy::unwrap_used)]
        stop_control.send(()).unwrap();
        handle.await?;

        // Then
        assert_eq!(status, Status::Failed);

        Ok(())
    }

    #[test(tokio::test)]
    async fn prisms_total_changed() -> TestResult {
        // Given
        const VAULT: &str = "/tmp/bifrost/validator-5";
        const AMOUNT: u64 = 1_000_000;

        let mut vault = reset_vault(VAULT).await?;

        let key1 = Keypair::generate();
        let key2 = Keypair::generate().pubkey();
        let wallet1_before = Wallet { prisms: AMOUNT };

        vault
            .save_account(key1.pubkey(), &wallet1_before, 0)
            .await?;
        vault.save().await?;
        let vault = Arc::new(RwLock::new(vault));

        let (stop_control, handle) = launch_transaction_processor(vault);
        let mut trx = Transaction::new(0);
        let instruction = testing_dummy::instruction::burn_prisms(key1.pubkey(), key2, 500_000)?;
        trx.add(&[instruction])?;
        trx.sign(&key1)?;

        // When
        let mut status = Status::Pending;
        let mut rx = register_transaction(trx).await?;
        while let Some(new_status) = rx.recv().await {
            info!("received new transaction status: {new_status:?}");
            status = new_status;
        }
        #[expect(clippy::unwrap_used)]
        stop_control.send(()).unwrap();
        handle.await?;

        // Then
        assert_eq!(status, Status::Failed);

        Ok(())
    }
}
