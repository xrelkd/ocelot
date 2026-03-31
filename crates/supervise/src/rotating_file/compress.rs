use std::{
    io,
    io::{Read, Write},
};

use flate2::write::GzEncoder;
use lzzzz::lz4f;

pub fn compress_gzip<R: Read, W: Write>(source: &mut R, destination: &mut W) -> io::Result<()> {
    let mut encoder = GzEncoder::new(destination, flate2::Compression::default());
    let _ = io::copy(source, &mut encoder)?;
    let _ = encoder.finish()?;
    Ok(())
}

pub fn compress_lz4<R: Read, W: Write>(source: &mut R, destination: &mut W) -> io::Result<()> {
    let mut compressor = lz4f::WriteCompressor::new(destination, lz4f::Preferences::default())?;
    let _ = io::copy(source, &mut compressor)?;
    compressor.flush()?;
    Ok(())
}
