use std::io::{Read, Write};

use bincode::{DefaultOptions, Options};

use super::Db;

fn bincode_options() -> impl Options {
    DefaultOptions::new().with_limit(2147483648)
}

pub fn read(source: impl Read) -> anyhow::Result<Db> {
    Ok(bincode_options().deserialize_from(source)?)
}

pub fn write(db: &Db, sink: impl Write) -> anyhow::Result<()> {
    Ok(bincode_options().serialize_into(sink, db)?)
}
