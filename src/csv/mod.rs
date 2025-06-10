use std::{
    collections::HashSet,
    io::{Read, Write},
};

use csv::{Reader, Result, Writer};

use crate::domain::{account::Account, transaction::Transaction};

/// Parse [`Transaction`]s from a reader.
///
/// This function assumes the content is a valid CSV, otherwise it will throw an
/// error.
/// The output type is a [`HashSet`] once we don't want to process two
/// identic transactions, probably the provider messed up with the data.
pub fn read(reader: impl Read) -> Result<HashSet<Transaction>> {
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
