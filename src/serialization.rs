use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
};

use rmp_serde::{encode::write_named, from_read};
use serde::{Deserialize, Serialize};

pub fn read<T: for<'de> Deserialize<'de>>(source: impl Read) -> anyhow::Result<T> {
    Ok(from_read(source)?)
}

pub fn read_from_file<T: for<'de> Deserialize<'de>>(path: impl AsRef<Path>) -> anyhow::Result<T> {
    read(File::open(path)?)
}

pub fn write_to_file<T: Serialize>(obj: &T, path: impl AsRef<Path>) -> anyhow::Result<()> {
    write(obj, File::create(path)?)
}

pub fn write<T: Serialize>(obj: &T, mut sink: impl Write) -> anyhow::Result<()> {
    Ok(write_named(&mut sink, obj)?)
}
