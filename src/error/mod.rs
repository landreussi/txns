use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("could not open transactions file")]
    FileError(#[from] std::io::Error),
    #[error("could not parse CSV rows to transaction")]
    CsvError(#[from] csv::Error),
    #[error(transparent)]
    BusinessError(#[from] crate::domain::error::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
