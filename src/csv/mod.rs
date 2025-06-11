use std::io::{Read, Write};

use csv::{Reader, Result, Writer};

use crate::domain::{account::Account, transaction::Transaction};

/// Parse [`Transaction`]s from a reader.
///
/// This function assumes the content is a valid CSV, otherwise it will throw an
/// error.
pub fn read(reader: impl Read) -> Result<Vec<Transaction>> {
    Reader::from_reader(reader).into_deserialize().collect()
}

pub fn write(accounts: Vec<Account>, writer: impl Write) -> Result<()> {
    let mut writer = Writer::from_writer(writer);

    for account in accounts {
        writer.serialize(account)?;
    }

    writer.flush()?;
    Ok(())
}
