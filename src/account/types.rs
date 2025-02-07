// File: src/account/types.rs
// Project: Bifrost
// Creation date: Saturday 08 February 2025
// Author: Vincent Berthier <vincent.berthier@posteo.org>
// -----
// Last modified: Saturday 08 February 2025 @ 16:17:11
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

use std::mem::discriminant;

use borsh::{BorshDeserialize, BorshSerialize};

/// Determines if an account is read-only or writable
#[derive(Clone, Copy, Debug, Default, BorshSerialize, BorshDeserialize)]
pub enum Writable {
    /// The account is writable
    Yes,
    /// The account is read-only.
    #[default]
    No,
}

/// The type of account.
#[derive(Clone, Copy, Debug, BorshDeserialize, BorshSerialize)]
pub enum AccountType {
    /// An account containing a program
    Program,
    /// A specialized wallet signing a transaction.
    Signing,
    /// A user's wallet (used only as identification)
    Wallet,
}

impl AccountType {
    pub const fn is_wallet(self) -> bool {
        matches!(self, Self::Wallet | Self::Signing)
    }

    pub fn is_compatible(self, other: Self) -> bool {
        (self.is_wallet() && other.is_wallet()) || (discriminant(&self) == discriminant(&other))
    }
}
