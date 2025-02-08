// File: src/io/support.rs
// Project: Bifrost
// Creation date: Sunday 09 February 2025
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
    any::type_name,
    fmt::Debug,
    fs::{self, OpenOptions},
    io::Write as _,
    path::PathBuf,
};

use borsh::{BorshDeserialize, BorshSerialize};
use tracing::{debug, instrument, trace};

use super::Result;

#[instrument]
pub fn read_from_file<P, T>(path: P) -> Result<T>
where
    P: Into<PathBuf> + Debug,
    T: BorshDeserialize,
{
    debug!("reading data from file");
    let data = fs::read(path.into())?;
    trace!(kind = type_name::<T>(), "casting data");
    let res: T = borsh::from_slice(&data)?;
    Ok(res)
}

#[expect(clippy::unwrap_used)]
#[instrument(skip(data))]
pub fn write_to_file<P, B>(path: P, data: &B) -> Result<()>
where
    P: Into<PathBuf> + Debug,
    B: BorshSerialize,
{
    debug!(kind = type_name::<B>(), "writing data to file");
    let data = borsh::to_vec(data).unwrap();
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path.into())?;
    Ok(file.write_all(&data)?)
}

#[instrument]
pub fn create_folder<P>(path: P) -> Result<()>
where
    P: Into<PathBuf> + Debug,
{
    debug!("creating folder");
    let path = path.into();
    if !path.exists() {
        fs::create_dir_all(path)?;
    }

    Ok(())
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {

    use std::assert_matches::assert_matches;
    use std::path::Path;

    use test_log::test;

    use super::super::Error;
    use super::*;
    type TestResult = core::result::Result<(), Box<dyn core::error::Error>>;

    #[test]
    fn write_doesnt_create_folder() {
        // Given
        let path = Path::new("/nowhere/at/all.txt");

        // When
        let res = write_to_file(path, &Vec::<u8>::new());

        // Then
        assert_matches!(res, Err(Error::FileSystem(err)) if matches!(err.kind(), std::io::ErrorKind::NotFound));
    }

    #[test]
    fn folder_creation_fails_when_no_permission() {
        // Given
        let path = Path::new("/root/bifrost/io-support");

        // When
        let res = create_folder(path);

        // Then
        assert_matches!(res, Err(Error::FileSystem(err)) if matches!(err.kind(), std::io::ErrorKind::PermissionDenied));
    }
}
