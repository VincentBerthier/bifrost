// File: src/io/error.rs
// Project: Bifrost
// Creation date: Sunday 09 February 2025
// Author: Vincent Berthier <vincent.berthier@posteo.org>
// -----
// Last modified: Monday 10 February 2025 @ 20:49:48
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

use derive_more::derive::{Display, From};

use super::location::AccountDiskLocation;

/// Errors of the I/O module.
#[derive(Debug, Display, From)]
#[display("during an I/O operation: {_variant}")]
pub enum Error {
    /// Tried to put the same location twice in the trash
    #[display("attempted to put {loc:?} in the trash twice")]
    DuplicateLocationInTrash {
        /// The duplicated location
        loc: AccountDiskLocation,
    },
    /// The index file wasn't found.
    #[display("the index file wasn’t found")]
    IndexFileNotFound,
    /// Attempted to read beyond file size
    #[display("attempted to read from {from} to {to} but file only has {size} bytes")]
    OutOfBounds {
        /// Starting byte offset
        from: u64,
        /// Last bytes tried to read.
        to: u64,
        /// Actual size of the file
        size: u64,
    },
    /// The trash file wasn't found.
    #[display("the trash file wasn’t found")]
    TrashFileNotFound,
    /// An operation on the file system couldn't be completed.
    #[from]
    #[display("filesystem error '{_0}'")]
    FileSystem(std::io::Error),
    /// Failed to acquire a lock on a resource.
    #[display("couldn’t acquire a resource lock: {_0}")]
    #[from]
    ResourceLock(tokio::sync::AcquireError),
}

impl core::error::Error for Error {}
