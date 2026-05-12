//! Filesystem helpers that preserve path context in `SpliceError::Io`.
//!
//! `From<std::io::Error> for SpliceError` produces a `SpliceError::Io` with
//! `path = "<unknown>"`, which loses information for the user. These helpers
//! wrap the common `std::fs` operations so the originating path is captured.

use crate::error::SpliceError;
use std::path::Path;

pub(crate) fn read<P: AsRef<Path>>(path: P) -> Result<Vec<u8>, SpliceError> {
    std::fs::read(path.as_ref()).map_err(|source| SpliceError::Io {
        path: path.as_ref().to_path_buf(),
        source,
    })
}

pub(crate) fn read_to_string<P: AsRef<Path>>(path: P) -> Result<String, SpliceError> {
    std::fs::read_to_string(path.as_ref()).map_err(|source| SpliceError::Io {
        path: path.as_ref().to_path_buf(),
        source,
    })
}

pub(crate) fn write<P: AsRef<Path>, C: AsRef<[u8]>>(
    path: P,
    contents: C,
) -> Result<(), SpliceError> {
    std::fs::write(path.as_ref(), contents.as_ref()).map_err(|source| SpliceError::Io {
        path: path.as_ref().to_path_buf(),
        source,
    })
}
