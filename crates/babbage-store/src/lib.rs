//! Sheet files and the sheetbook (a folder of sheets).
//!
//! One sheet is one UTF-8 text file named `<name>.bbg`: each line of the sheet is one line of the
//! file, ended by `\n`, so the files read and diff like any text. A sheetbook is a directory of them.
//! Deleting moves a file to the `.trash` folder and never removes it.

mod atomic;
mod autosave;
mod book;
pub mod export;
mod format;

pub use atomic::{Stage, write_atomic, write_atomic_until};
pub use autosave::Autosaver;
pub use book::{SheetBook, SheetId, SheetInfo, data_dir};
pub use format::{count_lines, decode, encode, title_of};

/// Everything that can go wrong in the store.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{context}: {source}")]
    Io {
        context: String,
        #[source]
        source: std::io::Error,
    },
    #[error("there is no sheet named {0:?}")]
    NoSuchSheet(String),
    #[error("a sheet named {0:?} already exists")]
    Exists(String),
    #[error("{0:?} is not a usable sheet name")]
    BadName(String),
    #[error("{0} is not valid UTF-8 text")]
    NotUtf8(String),
}

pub type Result<T> = std::result::Result<T, Error>;

pub(crate) trait IoContext<T> {
    fn ctx(self, what: impl FnOnce() -> String) -> Result<T>;
}

impl<T> IoContext<T> for std::io::Result<T> {
    fn ctx(self, what: impl FnOnce() -> String) -> Result<T> {
        self.map_err(|source| Error::Io {
            context: what(),
            source,
        })
    }
}
