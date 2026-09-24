//! Bounds-checked byte reader with endian awareness.
//!
//! All access goes through `slice()`, which validates `offset + len` against
//! the file length before anything is decoded. There is no unchecked
//! indexing anywhere in this module.

use serde::Serialize;

use crate::error::{ErrorKind, Moc3Error, Moc3Result};
use crate::version::ByteOrder;

/// A 64-byte MOC3 identifier field.
///
/// The field is a fixed 64-byte, NUL-padded, UTF-8 string on disk. The raw
/// bytes are preserved: invalid sequences are reported (and kept as hex)
/// rather than silently replaced.
#[derive(Debug, Clone, Serialize)]
pub struct IdField {
    /// Decoded text (lossy when `valid_utf8` is false).
    pub text: String,
    /// Whether the bytes before the terminator form valid UTF-8.
    pub valid_utf8: bool,
    /// Whether a NUL terminator was present inside the 64 bytes.
    pub terminated: bool,
    /// Whether non-NUL bytes follow the first terminator.
    pub trailing_bytes_after_terminator: bool,
    /// Hex dump of the raw 64 bytes, only present when not valid UTF-8.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_hex: Option<String>,
}

/// Fixed-size identifier length in bytes.
pub const ID_LEN: u64 = 64;

/// A read-only view over the whole file with a byte order.
#[derive(Debug, Clone, Copy)]
pub struct ByteView<'a> {
    data: &'a [u8],
    order: ByteOrder,
}

impl<'a> ByteView<'a> {
    /// Wrap a byte slice.
    pub fn new(data: &'a [u8], order: ByteOrder) -> Self {
        Self { data, order }
    }

    /// Total file length in bytes.
    pub fn file_len(&self) -> u64 {
        self.data.len() as u64
    }

    /// Byte order used by this view.
    pub fn order(&self) -> ByteOrder {
        self.order
    }

    /// Checked sub-slice access.
    pub fn slice(&self, offset: u64, len: u64, context: &'static str) -> Moc3Result<&'a [u8]> {
        let end = offset.checked_add(len).ok_or_else(|| {
            Moc3Error::in_context(
                ErrorKind::InvalidValue {
                    field: context,
                    index: 0,
                    value: -1,
                    reason: "offset + length overflows u64",
                },
                offset,
                context,
            )
        })?;
        if end > self.file_len() {
            return Err(Moc3Error::in_context(
                ErrorKind::UnexpectedEof {
                    offset,
                    needed: len,
                    available: self.file_len().saturating_sub(offset),
                },
                offset,
                context,
            ));
        }
        let start = usize::try_from(offset).map_err(|_| {
            Moc3Error::in_context(
                ErrorKind::InvalidValue {
                    field: context,
                    index: 0,
                    value: i64::MAX,
                    reason: "offset does not fit usize",
                },
                offset,
                context,
            )
        })?;
        let end = usize::try_from(end).map_err(|_| {
            Moc3Error::in_context(
                ErrorKind::InvalidValue {
                    field: context,
                    index: 0,
                    value: i64::MAX,
                    reason: "range end does not fit usize",
                },
                offset,
                context,
            )
        })?;
        self.data.get(start..end).ok_or_else(|| {
            Moc3Error::in_context(
                ErrorKind::UnexpectedEof {
                    offset,
                    needed: len,
                    available: self.file_len().saturating_sub(offset),
                },
                offset,
                context,
            )
        })
    }

    /// Read a single byte.
    pub fn u8_at(&self, offset: u64, context: &'static str) -> Moc3Result<u8> {
        let bytes = self.slice(offset, 1, context)?;
        bytes.first().copied().ok_or_else(|| {
            Moc3Error::in_context(
                ErrorKind::UnexpectedEof {
                    offset,
                    needed: 1,
                    available: 0,
                },
                offset,
                context,
            )
        })
    }

    /// Read a little/big-endian `u16`.
    pub fn u16_at(&self, offset: u64, context: &'static str) -> Moc3Result<u16> {
        let raw = self.fixed::<2>(offset, context)?;
        Ok(match self.order {
            ByteOrder::Little => u16::from_le_bytes(raw),
            ByteOrder::Big => u16::from_be_bytes(raw),
        })
    }

    /// Read a little/big-endian `u32`.
    pub fn u32_at(&self, offset: u64, context: &'static str) -> Moc3Result<u32> {
        let raw = self.fixed::<4>(offset, context)?;
        Ok(match self.order {
            ByteOrder::Little => u32::from_le_bytes(raw),
            ByteOrder::Big => u32::from_be_bytes(raw),
        })
    }

    /// Read a little/big-endian `i32`.
    pub fn i32_at(&self, offset: u64, context: &'static str) -> Moc3Result<i32> {
        Ok(self.u32_at(offset, context)? as i32)
    }

    /// Read a little/big-endian `f32` (bit pattern preserving).
    pub fn f32_at(&self, offset: u64, context: &'static str) -> Moc3Result<f32> {
        Ok(f32::from_bits(self.u32_at(offset, context)?))
    }

    /// Read `[u8; N]` at `offset`.
    pub fn fixed<const N: usize>(&self, offset: u64, context: &'static str) -> Moc3Result<[u8; N]> {
        let bytes = self.slice(offset, N as u64, context)?;
        <[u8; N]>::try_from(bytes).map_err(|_| {
            Moc3Error::in_context(
                ErrorKind::Internal {
                    what: "fixed-size slice conversion failed",
                },
                offset,
                context,
            )
        })
    }

    /// Read `count` `u32` values into a fresh `Vec`.
    pub fn u32_vec(&self, offset: u64, count: u64, context: &'static str) -> Moc3Result<Vec<u32>> {
        self.read_vec(offset, count, 4, context, |view, at| {
            view.u32_at(at, context)
        })
    }

    /// Read `count` `i32` values into a fresh `Vec`.
    pub fn i32_vec(&self, offset: u64, count: u64, context: &'static str) -> Moc3Result<Vec<i32>> {
        self.read_vec(offset, count, 4, context, |view, at| {
            view.i32_at(at, context)
        })
    }

    /// Read `count` `f32` values into a fresh `Vec`.
    pub fn f32_vec(&self, offset: u64, count: u64, context: &'static str) -> Moc3Result<Vec<f32>> {
        self.read_vec(offset, count, 4, context, |view, at| {
            view.f32_at(at, context)
        })
    }

    /// Read `count` `u16` values into a fresh `Vec`.
    pub fn u16_vec(&self, offset: u64, count: u64, context: &'static str) -> Moc3Result<Vec<u16>> {
        self.read_vec(offset, count, 2, context, |view, at| {
            view.u16_at(at, context)
        })
    }

    /// Read `count` bytes into a fresh `Vec`.
    pub fn u8_vec(&self, offset: u64, count: u64, context: &'static str) -> Moc3Result<Vec<u8>> {
        Ok(self.slice(offset, count, context)?.to_vec())
    }

    /// Read a typed array; `stride` is the on-disk element size in bytes and
    /// must match the element type (`u16` arrays advance by two bytes, not
    /// four).
    fn read_vec<T, F>(
        &self,
        offset: u64,
        count: u64,
        stride: u64,
        context: &'static str,
        mut read: F,
    ) -> Moc3Result<Vec<T>>
    where
        F: FnMut(&Self, u64) -> Moc3Result<T>,
        T: Default,
    {
        let mut out: Vec<T> = Vec::new();
        out.try_reserve_exact(count as usize).map_err(|_| {
            Moc3Error::in_context(
                ErrorKind::AllocationFailed {
                    field: context,
                    count,
                },
                offset,
                context,
            )
        })?;
        for i in 0..count {
            let step = i.checked_mul(stride).ok_or_else(|| {
                Moc3Error::in_context(
                    ErrorKind::Internal {
                        what: "element offset overflow",
                    },
                    offset,
                    context,
                )
            })?;
            let at = offset.checked_add(step).ok_or_else(|| {
                Moc3Error::in_context(
                    ErrorKind::Internal {
                        what: "element offset overflow",
                    },
                    offset,
                    context,
                )
            })?;
            out.push(read(self, at)?);
        }
        Ok(out)
    }

    /// Read one 64-byte identifier field.
    pub fn id_at(&self, offset: u64, context: &'static str) -> Moc3Result<IdField> {
        let raw = self.fixed::<64>(offset, context)?;
        Ok(decode_id(&raw))
    }

    /// Read `count` identifier fields sharing one contiguous block.
    pub fn ids_at(
        &self,
        offset: u64,
        count: u64,
        context: &'static str,
    ) -> Moc3Result<Vec<IdField>> {
        let mut out: Vec<IdField> = Vec::new();
        out.try_reserve_exact(count as usize).map_err(|_| {
            Moc3Error::in_context(
                ErrorKind::AllocationFailed {
                    field: context,
                    count,
                },
                offset,
                context,
            )
        })?;
        for i in 0..count {
            let at = offset
                .checked_add(i.checked_mul(ID_LEN).ok_or_else(|| {
                    Moc3Error::in_context(
                        ErrorKind::Internal {
                            what: "id element offset overflow",
                        },
                        offset,
                        context,
                    )
                })?)
                .ok_or_else(|| {
                    Moc3Error::in_context(
                        ErrorKind::Internal {
                            what: "id element offset overflow",
                        },
                        offset,
                        context,
                    )
                })?;
            out.push(self.id_at(at, context)?);
        }
        Ok(out)
    }
}

fn decode_id(raw: &[u8; 64]) -> IdField {
    let first_nul = raw.iter().position(|b| *b == 0);
    let content = match first_nul {
        Some(index) => raw.get(..index).unwrap_or(raw.as_slice()),
        None => raw.as_slice(),
    };
    let terminated = first_nul.is_some();
    let trailing_bytes_after_terminator = match first_nul {
        Some(index) => raw
            .get(index.saturating_add(1)..)
            .map(|rest| rest.iter().any(|b| *b != 0))
            .unwrap_or(false),
        None => false,
    };
    match std::str::from_utf8(content) {
        Ok(text) => IdField {
            text: text.to_string(),
            valid_utf8: true,
            terminated,
            trailing_bytes_after_terminator,
            raw_hex: None,
        },
        Err(_) => IdField {
            text: String::from_utf8_lossy(content).into_owned(),
            valid_utf8: false,
            terminated,
            trailing_bytes_after_terminator,
            raw_hex: Some(hex_dump(raw)),
        },
    }
}

fn hex_dump(bytes: &[u8; 64]) -> String {
    let mut out = String::with_capacity(128);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_both_byte_orders() {
        let bytes = [0x78u8, 0x56, 0x34, 0x12, 0xFF, 0xFF, 0xFF, 0x7F];
        let little = ByteView::new(&bytes, ByteOrder::Little);
        assert_eq!(little.u32_at(0, "test").unwrap_or(0), 0x1234_5678);
        assert_eq!(little.i32_at(4, "test").unwrap_or(0), 0x7FFF_FFFF);
        assert_eq!(little.u16_at(0, "test").unwrap_or(0), 0x5678);

        let big = ByteView::new(&bytes, ByteOrder::Big);
        assert_eq!(big.u32_at(0, "test").unwrap_or(0), 0x7856_3412);
        assert_eq!(big.u16_at(0, "test").unwrap_or(0), 0x7856);
    }

    #[test]
    fn u16_arrays_advance_two_bytes_per_element() {
        // Regression: the generic array reader used to advance four bytes per
        // element for every type, which broke `u16` pools once they were
        // actually read (AGENT.2 parse_full).
        let bytes = [0x01u8, 0x00, 0x02, 0x00, 0x03, 0x00, 0x04, 0x00];
        let view = ByteView::new(&bytes, ByteOrder::Little);
        let values = view.u16_vec(0, 4, "test").unwrap_or_default();
        assert_eq!(values, vec![1, 2, 3, 4]);
        let too_many = view.u16_vec(0, 5, "test");
        assert!(too_many.is_err());
    }

    #[test]
    fn out_of_bounds_reads_are_errors() {
        let bytes = [0u8; 4];
        let view = ByteView::new(&bytes, ByteOrder::Little);
        assert!(view.u32_at(1, "test").is_err());
        assert!(view.u32_at(4, "test").is_err());
        assert!(view.slice(3, 2, "test").is_err());
        assert!(view.u16_at(3, "test").is_err());
    }

    #[test]
    fn float_bit_patterns_round_trip() {
        let value = -30.0f32;
        let bytes = value.to_le_bytes();
        let view = ByteView::new(&bytes, ByteOrder::Little);
        assert_eq!(view.f32_at(0, "test").unwrap_or(f32::NAN), value);
    }

    #[test]
    fn identifier_decoding_reports_issues() {
        let mut raw = [0u8; 64];
        raw[..4].copy_from_slice(b"Test");
        let field = decode_id(&raw);
        assert_eq!(field.text, "Test");
        assert!(field.valid_utf8);
        assert!(field.terminated);
        assert!(!field.trailing_bytes_after_terminator);

        let mut trailing = [0u8; 64];
        trailing[..1].copy_from_slice(b"A");
        trailing[2] = b'B';
        let field = decode_id(&trailing);
        assert!(field.trailing_bytes_after_terminator);

        let invalid = [0xFFu8; 64];
        let field = decode_id(&invalid);
        assert!(!field.valid_utf8);
        assert!(field.raw_hex.is_some());
    }
}
