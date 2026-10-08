//! Example-only PNG serialization. Pixels already come from EX; this module
//! neither renders nor performs colour conversion. Stored DEFLATE blocks avoid
//! a compression dependency. This is not an exported EX API.
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

pub(super) fn write_rgba(
    path: &Path,
    width: u32,
    height: u32,
    pixels: &[[u8; 4]],
) -> io::Result<()> {
    if width == 0 || height == 0 || pixels.len() as u64 != u64::from(width) * u64::from(height) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "PNG dimensions must match pixels",
        ));
    }
    let mut raw = Vec::new();
    for row in pixels.chunks_exact(width as usize) {
        raw.push(0); // PNG filter: none.
        for pixel in row {
            raw.extend_from_slice(pixel);
        }
    }
    let mut zlib = vec![0x78, 0x01]; // DEFLATE, 32 KiB window, no compression.
    let block_count = raw.len().div_ceil(65535);
    for (index, block) in raw.chunks(65535).enumerate() {
        zlib.push(u8::from(index + 1 == block_count)); // BFINAL and stored BTYPE.
        let length = block.len() as u16;
        zlib.extend_from_slice(&length.to_le_bytes());
        zlib.extend_from_slice(&(!length).to_le_bytes());
        zlib.extend_from_slice(block);
    }
    let (mut a, mut b) = (1_u32, 0_u32);
    for byte in raw {
        a = (a + u32::from(byte)) % 65521;
        b = (b + a) % 65521;
    }
    zlib.extend_from_slice(&((b << 16) | a).to_be_bytes());
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = File::create(path)?;
    file.write_all(b"\x89PNG\r\n\x1a\n")?;
    let mut header = Vec::new();
    header.extend_from_slice(&width.to_be_bytes());
    header.extend_from_slice(&height.to_be_bytes());
    header.extend_from_slice(&[8, 6, 0, 0, 0]); // RGBA8, DEFLATE, no interlace.
    chunk(&mut file, b"IHDR", &header)?;
    chunk(&mut file, b"IDAT", &zlib)?;
    chunk(&mut file, b"IEND", &[])?;
    file.flush()
}

fn chunk(file: &mut File, kind: &[u8; 4], data: &[u8]) -> io::Result<()> {
    let length = u32::try_from(data.len())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "PNG chunk too large"))?;
    file.write_all(&length.to_be_bytes())?;
    file.write_all(kind)?;
    file.write_all(data)?;
    let mut crc = !0_u32;
    for &byte in kind.iter().chain(data) {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb88320 & (0_u32.wrapping_sub(crc & 1)));
        }
    }
    file.write_all(&(!crc).to_be_bytes())
}
