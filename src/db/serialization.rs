use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
};

use bincode::{DefaultOptions, Options};
use serde::{Deserialize, Serialize};

fn bincode_options() -> impl Options {
    DefaultOptions::new().with_limit(2147483648)
}

pub fn read<T: for<'de> Deserialize<'de>>(source: impl Read) -> anyhow::Result<T> {
    Ok(bincode_options().deserialize_from(source)?)
}

pub fn read_from_file<T: for<'de> Deserialize<'de>>(path: impl AsRef<Path>) -> anyhow::Result<T> {
    read(File::open(path)?)
}

pub fn write_to_file<T: Serialize>(obj: &T, path: impl AsRef<Path>) -> anyhow::Result<()> {
    write(obj, File::create(path)?)
}

pub fn write<T: Serialize>(obj: &T, sink: impl Write) -> anyhow::Result<()> {
    Ok(bincode_options().serialize_into(sink, obj)?)
}
