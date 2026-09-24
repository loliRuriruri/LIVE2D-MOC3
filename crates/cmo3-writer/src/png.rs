//! Deterministic synthetic PNG writer (tests and fixture tooling).
//!
//! Produces a valid RGBA PNG with stored-deflate IDAT chunks; no image
//! decoding is implemented anywhere in the writer.

/// Build a canvas-sized solid-color RGBA PNG.
pub fn solid_png(width: u32, height: u32, rgba: [u8; 4]) -> Vec<u8> {
    let mut raw: Vec<u8> = Vec::with_capacity((height as usize) * (1 + width as usize * 4));
    let mut row: Vec<u8> = Vec::with_capacity(1 + width as usize * 4);
    row.push(0); // filter: none
    for _ in 0..width {
        row.extend_from_slice(&rgba);
    }
    for _ in 0..height {
        raw.extend_from_slice(&row);
    }
    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
    let mut ihdr: Vec<u8> = Vec::new();
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    out.extend_from_slice(&chunk(b"IHDR", &ihdr));
    out.extend_from_slice(&chunk(b"IDAT", &zlib_stored(&raw)));
    out.extend_from_slice(&chunk(b"IEND", &[]));
    out
}

/// PNG dimensions from the IHDR chunk, when the bytes are a PNG.
pub fn png_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 33 || bytes.get(..8) != Some(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A])
    {
        return None;
    }
    if bytes.get(8..12) != Some(&13u32.to_be_bytes()) || bytes.get(12..16) != Some(b"IHDR") {
        return None;
    }
    let width = u32::from_be_bytes(bytes.get(16..20)?.try_into().ok()?);
    let height = u32::from_be_bytes(bytes.get(20..24)?.try_into().ok()?);
    Some((width, height))
}

fn chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let mut crc_input: Vec<u8> = Vec::new();
    crc_input.extend_from_slice(kind);
    crc_input.extend_from_slice(data);
    out.extend_from_slice(&crc32(&crc_input).to_be_bytes());
    out
}

fn zlib_stored(raw: &[u8]) -> Vec<u8> {
    let mut out: Vec<u8> = vec![0x78, 0x01];
    let mut offset = 0usize;
    while offset < raw.len() {
        let take = (raw.len() - offset).min(65_535);
        let last = offset + take == raw.len();
        out.push(if last { 1 } else { 0 });
        out.extend_from_slice(&(take as u16).to_le_bytes());
        out.extend_from_slice(&(!(take as u16)).to_le_bytes());
        out.extend_from_slice(&raw[offset..offset + take]);
        offset += take;
    }
    out.extend_from_slice(&adler32(raw).to_be_bytes());
    out
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

fn adler32(bytes: &[u8]) -> u32 {
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    for byte in bytes {
        a = (a + u32::from(*byte)) % 65_521;
        b = (b + a) % 65_521;
    }
    (b << 16) | a
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solid_png_has_valid_header_and_dimensions() {
        let bytes = solid_png(4, 3, [1, 2, 3, 255]);
        assert_eq!(png_dimensions(&bytes), Some((4, 3)));
        assert!(bytes.len() > 40);
    }
}
