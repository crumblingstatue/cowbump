use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
};

/// Dummy version value for future proofing.
/// The space created by this can be used in the future
/// for versioning the data formats and defining migrations.
const DUMMY_VERSION: u8 = 0;

use rmp_serde::{encode::write_named, from_read};
use serde::{Deserialize, Serialize};

pub fn read<T: for<'de> Deserialize<'de>>(mut source: impl Read) -> anyhow::Result<T> {
    let mut ver = [0];
    source.read_exact(&mut ver)?;
    Ok(from_read(zstd::Decoder::new(source)?)?)
}

pub fn read_from_file<T: for<'de> Deserialize<'de>>(path: impl AsRef<Path>) -> anyhow::Result<T> {
    read(File::open(path)?)
}

pub fn write_to_file<T: Serialize>(obj: &T, path: impl AsRef<Path>) -> anyhow::Result<()> {
    write(obj, File::create(path)?)
}

pub fn write<T: Serialize>(obj: &T, mut sink: impl Write) -> anyhow::Result<()> {
    sink.write_all(&[DUMMY_VERSION])?;
    let mut zstd = zstd::Encoder::new(sink, 0)?;
    write_named(&mut zstd, obj)?;
    zstd.finish()?;
    Ok(())
}
