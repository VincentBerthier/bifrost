// File: src/io/support.rs
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

use std::{any::type_name, fmt::Debug, path::PathBuf, sync::LazyLock};

use borsh::{BorshDeserialize, BorshSerialize};
use memmap2::MmapOptions;
use tokio::{
    fs::{self, File, OpenOptions},
    io::AsyncWriteExt,
    sync::Semaphore,
};
use tracing::{debug, instrument, trace};

use crate::io::Error;

use super::Result;

// We don’t want writes (specifically happends) to happen at the same time otherwise we risk getting some garbled mess
// Not the most optimal solution (it’d need to be per file maybe), but good enough for our purposes
static SEMAPHORE: LazyLock<Semaphore> = LazyLock::new(|| Semaphore::new(1));

#[instrument]
pub async fn read_from_file<P, T>(path: P) -> Result<T>
where
    P: Into<PathBuf> + Debug,
    T: BorshDeserialize,
{
    debug!("reading data from file");
    let data = fs::read(path.into()).await?;
    trace!(kind = type_name::<T>(), "casting data");
    let res: T = borsh::from_slice(&data)?;
    Ok(res)
}

#[instrument]
pub async fn read_from_file_map<P, T>(path: P, offset: u64, size: u64) -> Result<T>
where
    P: Into<PathBuf> + Debug,
    T: BorshDeserialize,
{
    debug!("reading data from file memmap");
    let file = File::open(path.into()).await?;
    let file_len = file.metadata().await?.len();
    if offset + size > file_len {
        return Err(Error::OutOfBounds {
            from: offset,
            to: offset + size,
            size: file_len,
        });
    }
    trace!("file is open, reading memory block");

    // SAFETY:
    // underlying function is unsafe, but this is fine.
    #[expect(clippy::cast_possible_truncation)]
    let mmap = unsafe {
        MmapOptions::new()
            .offset(offset)
            .len(size as usize)
            .map(&file)?
    };

    let res: T = borsh::from_slice(&mmap)?;
    Ok(res)
}

#[expect(clippy::unwrap_used)]
#[instrument(skip(data))]
pub async fn write_to_file<P, B>(path: P, data: &B) -> Result<()>
where
    P: Into<PathBuf> + Debug,
    B: BorshSerialize + Send + Sync,
{
    debug!(kind = type_name::<B>(), "writing data to file");
    let data = borsh::to_vec(data).unwrap();
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path.into())
        .await?;
    file.write_all(&data).await?;
    file.flush().await?;
    Ok(())
}

#[expect(clippy::unwrap_used)]
#[instrument(skip(data))]
pub async fn append_to_file<P, B>(path: P, data: &B) -> Result<(u64, u64)>
where
    P: Into<PathBuf> + Debug,
    B: BorshSerialize + Send + Sync,
{
    debug!(kind = type_name::<B>(), "appending data to file");
    let data = borsh::to_vec(data).unwrap();
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path.into())
        .await?;
    let _guard = SEMAPHORE.acquire().await?;
    let offset = file.metadata().await?.len();
    file.write_all(&data).await?;
    file.flush().await?;
    Ok((data.len() as u64, offset))
}

#[instrument]
pub async fn create_folder<P>(path: P) -> Result<()>
where
    P: Into<PathBuf> + Debug,
{
    debug!("creating folder");
    let path = path.into();
    if !path.exists() {
        fs::create_dir_all(path).await?;
    }

    Ok(())
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {

    use std::assert_matches::assert_matches;
    use std::path::Path;

    use test_log::test;
    use tokio::fs::remove_file;

    use crate::account::Wallet;

    use super::super::Error;
    use super::*;
    type TestResult = core::result::Result<(), Box<dyn core::error::Error>>;

    #[test(tokio::test)]
    async fn write_doesnt_create_folder() {
        // Given
        let path = Path::new("/nowhere/at/all.txt");

        // When
        let res = write_to_file(path, &Vec::<u8>::new()).await;

        // Then
        assert_matches!(res, Err(Error::FileSystem(err)) if matches!(err.kind(), std::io::ErrorKind::NotFound));
    }

    #[test(tokio::test)]
    async fn folder_creation_fails_when_no_permission() {
        // Given
        let path = Path::new("/root/bifrost/io-support-1");

        // When
        let res = create_folder(path).await;

        // Then
        assert_matches!(res, Err(Error::FileSystem(err)) if matches!(err.kind(), std::io::ErrorKind::PermissionDenied));
    }

    #[test(tokio::test)]
    async fn simple_read() -> TestResult {
        // Given
        let root_path = Path::new("/tmp/bifrost/io-support-1").join("accounts");
        if !root_path.exists() {
            create_folder(&root_path).await?;
        }
        let path = root_path.join("0.1");
        if path.exists() {
            remove_file(&path).await?;
        }
        let wallet = Wallet { prisms: 989_237 };
        let (write_size, _offset) = append_to_file(&path, &wallet).await?;
        let _ = append_to_file(&path, &wallet).await?;

        // When
        let _: Wallet = read_from_file_map(path, write_size, write_size).await?;

        Ok(())
    }

    #[test(tokio::test)]
    async fn cannot_read_out_of_bounds() -> TestResult {
        // Given
        let root_path = Path::new("/tmp/bifrost/io-support-2").join("accounts");
        if !root_path.exists() {
            create_folder(&root_path).await?;
        }
        let path = root_path.join("0.1");
        if path.exists() {
            remove_file(&path).await?;
        }
        let wallet = Wallet { prisms: 989_237 };
        let (write_size, _offset) = append_to_file(&path, &wallet).await?;

        // When
        let reloaded: Result<Wallet> = read_from_file_map(path, write_size, write_size).await;

        // Then
        assert_matches!(reloaded, Err(Error::OutOfBounds { from, to, size }) if from == 8 && to == 16 && size == 8);

        Ok(())
    }
}
