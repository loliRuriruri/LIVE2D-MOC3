//! Integer-level XOR obfuscation (`CaffBinaryPrimitives` behaviour).
//!
//! Two independent MIT implementations (Stretchy Studio `caff_packer.py`,
//! moc2cmo `src/caff/writer.rs`) agree on this scheme: multi-byte integers
//! are XORed with the sign-extended key, `i64` uses a key mask that
//! duplicates the 32-bit key in both halves (or sign-extends for negative
//! keys), and byte strings XOR every byte with the key's low byte.

/// The mask used for `i64` XOR (matches both pinned implementations).
pub fn int64_mask(key: i32) -> u64 {
    if key < 0 {
        (u64::from(u32::MAX) << 32) | u64::from(key as u32)
    } else {
        let key = u64::from(key as u32);
        (key << 32) | key
    }
}

/// Obfuscate or de-obfuscate one byte.
pub fn xor_byte(value: u8, key: i32) -> u8 {
    value ^ key as u8
}

/// Encode `u32` with the key.
pub fn xor_u32(value: u32, key: i32) -> u32 {
    value ^ key as u32
}

/// Encode `u64` with the key mask.
pub fn xor_u64(value: u64, key: i32) -> u64 {
    value ^ int64_mask(key)
}

/// Big-endian `i32` bytes with the key applied.
pub fn xor_i32_bytes(value: i32, key: i32) -> [u8; 4] {
    xor_u32(value as u32, key).to_be_bytes()
}

/// Big-endian `i64` bytes with the key mask applied.
pub fn xor_i64_bytes(value: i64, key: i32) -> [u8; 8] {
    xor_u64(value as u64, key).to_be_bytes()
}

/// Append a byte string XORed with the key.
pub fn xor_bytes_into(out: &mut Vec<u8>, bytes: &[u8], key: i32) {
    if key == 0 {
        out.extend_from_slice(bytes);
    } else {
        out.extend(bytes.iter().map(|byte| byte ^ key as u8));
    }
}
