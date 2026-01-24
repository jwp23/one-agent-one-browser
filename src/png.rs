use crate::image::RgbImage;
use std::io::{Write, BufWriter};

const PNG_SIGNATURE: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];
const COLOR_TYPE_TRUECOLOR: u8 = 2;
const BIT_DEPTH_8: u8 = 8;
const FILTER_NONE: u8 = 0;
const COMPRESSION_METHOD_DEFLATE: u8 = 0;
const FILTER_METHOD_ADAPTIVE: u8 = 0;
const INTERLACE_NONE: u8 = 0;

pub fn write_rgb_png(path: &std::path::Path, image: &RgbImage) -> Result<(), String> {
    let file = std::fs::File::create(path)
        .map_err(|err| format!("Failed to create {}: {err}", path.display()))?;
    let mut writer = BufWriter::new(file);

    writer
        .write_all(&PNG_SIGNATURE)
        .map_err(|err| format!("Failed to write PNG signature: {err}"))?;

    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&image.width.to_be_bytes());
    ihdr.extend_from_slice(&image.height.to_be_bytes());
    ihdr.push(BIT_DEPTH_8);
    ihdr.push(COLOR_TYPE_TRUECOLOR);
    ihdr.push(COMPRESSION_METHOD_DEFLATE);
    ihdr.push(FILTER_METHOD_ADAPTIVE);
    ihdr.push(INTERLACE_NONE);
    write_chunk(&mut writer, *b"IHDR", &ihdr)?;

    let scanlines = build_scanlines(image)?;
    let compressed = zlib_compress_stored(&scanlines)?;
    write_chunk(&mut writer, *b"IDAT", &compressed)?;
    write_chunk(&mut writer, *b"IEND", &[])?;

    writer
        .flush()
        .map_err(|err| format!("Failed to flush {}: {err}", path.display()))?;

    Ok(())
}

fn build_scanlines(image: &RgbImage) -> Result<Vec<u8>, String> {
    let row_stride = image.row_stride_bytes();
    let total_len = image
        .height
        .checked_mul(row_stride as u32 + 1)
        .ok_or_else(|| "Scanline buffer size overflow".to_owned())? as usize;

    let mut out = Vec::with_capacity(total_len);
    for row in 0..image.height as usize {
        out.push(FILTER_NONE);
        let start = row
            .checked_mul(row_stride)
            .ok_or_else(|| "Scanline offset overflow".to_owned())?;
        let end = start
            .checked_add(row_stride)
            .ok_or_else(|| "Scanline offset overflow".to_owned())?;
        out.extend_from_slice(
            image
                .data
                .get(start..end)
                .ok_or_else(|| "Scanline slice out of bounds".to_owned())?,
        );
    }
    Ok(out)
}

fn zlib_compress_stored(uncompressed: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    out.push(0x78);
    out.push(0x01);

    let mut adler = Adler32::new();
    adler.update(uncompressed);

    let mut offset = 0usize;
    while offset < uncompressed.len() {
        let remaining = uncompressed.len() - offset;
        let block_len = remaining.min(u16::MAX as usize);
        let is_final = offset + block_len == uncompressed.len();

        out.push(if is_final { 0x01 } else { 0x00 });

        let len_u16: u16 = block_len
            .try_into()
            .map_err(|_| "DEFLATE block length out of range".to_owned())?;
        let nlen_u16: u16 = !len_u16;
        out.extend_from_slice(&len_u16.to_le_bytes());
        out.extend_from_slice(&nlen_u16.to_le_bytes());

        out.extend_from_slice(
            uncompressed
                .get(offset..offset + block_len)
                .ok_or_else(|| "DEFLATE slice out of bounds".to_owned())?,
        );
        offset += block_len;
    }

    out.extend_from_slice(&adler.finish().to_be_bytes());
    Ok(out)
}

fn write_chunk(
    writer: &mut impl Write,
    chunk_type: [u8; 4],
    data: &[u8],
) -> Result<(), String> {
    let len_u32: u32 = data
        .len()
        .try_into()
        .map_err(|_| "PNG chunk too large".to_owned())?;

    writer
        .write_all(&len_u32.to_be_bytes())
        .map_err(|err| format!("Failed to write PNG chunk length: {err}"))?;
    writer
        .write_all(&chunk_type)
        .map_err(|err| format!("Failed to write PNG chunk type: {err}"))?;
    writer
        .write_all(data)
        .map_err(|err| format!("Failed to write PNG chunk data: {err}"))?;

    let mut crc_input = Vec::with_capacity(chunk_type.len() + data.len());
    crc_input.extend_from_slice(&chunk_type);
    crc_input.extend_from_slice(data);
    let crc = crc32(&crc_input);

    writer
        .write_all(&crc.to_be_bytes())
        .map_err(|err| format!("Failed to write PNG chunk CRC: {err}"))?;

    Ok(())
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &b in bytes {
        crc ^= b as u32;
        for _ in 0..8 {
            if crc & 1 == 1 {
                crc = (crc >> 1) ^ 0xEDB8_8320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

struct Adler32 {
    s1: u32,
    s2: u32,
}

impl Adler32 {
    fn new() -> Self {
        Self { s1: 1, s2: 0 }
    }

    fn update(&mut self, bytes: &[u8]) {
        const MOD: u32 = 65521;
        for &b in bytes {
            self.s1 = (self.s1 + b as u32) % MOD;
            self.s2 = (self.s2 + self.s1) % MOD;
        }
    }

    fn finish(&self) -> u32 {
        (self.s2 << 16) | self.s1
    }
}

