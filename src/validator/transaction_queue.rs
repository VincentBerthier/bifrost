// File: src/validator/transaction_queue.rs
// Project: Bifrost
// Creation date: Saturday 15 February 2025
// Author: Vincent Berthier <vincent.berthier@posteo.org>
// -----
// Last modified: Saturday 15 February 2025 @ 23:29:55
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
use tokio::sync::mpsc::Sender as TSender;

use crate::transaction::Transaction;

pub static TRANSACTION_QUEUE: LazyLock<TransactionQueue> = LazyLock::new(TransactionQueue::new);

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum Status {
    Failed,
    #[default]
    Pending,
    Running,
    Succeeded,
}

pub struct TransactionQueue {
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

    pub async fn send(&self, transaction: Transaction, status_tx: TSender<Status>) {
        #[expect(
            clippy::unwrap_used,
            reason = "can only fail if the validator is terminated"
        )]
        self.sender.send((transaction, status_tx)).await.unwrap();
    }

    pub fn get_receiver(&self) -> Arc<Receiver<(Transaction, TSender<Status>)>> {
        Arc::clone(&self.receiver)
    }
}
