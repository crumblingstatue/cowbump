use std::io::{Read, Write};

use bincode::{DefaultOptions, Options};

use super::local::LocalDb;

fn bincode_options() -> impl Options {
    DefaultOptions::new().with_limit(2147483648)
}

pub fn read_local(source: impl Read) -> anyhow::Result<LocalDb> {
    Ok(bincode_options().deserialize_from(source)?)
}

pub fn write_local(db: &LocalDb, sink: impl Write) -> anyhow::Result<()> {
    Ok(bincode_options().serialize_into(sink, db)?)
}
