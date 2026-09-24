//! MOC3 format version identification and version-gated layout facts.
//!
//! Version byte mapping (verified against the official Cubism Core version
//! enum and the PurismCore clean-room reimplementation):
//!
//! | byte | Cubism Editor | offset slots | count ints |
//! |------|---------------|--------------|------------|
//! | 1    | 3.0.00-3.2.07 | 160          | 32         |
//! | 2    | 3.3.00-3.3.03 | 160          | 32         |
//! | 3    | 4.0.00-4.1.05 | 160          | 32         |
//! | 4    | 4.2.00-4.2.04 | 160          | 32         |
//! | 5    | 5.0.00-5.2.03 | 160          | 64         |
//! | 6    | 5.3.00+       | 480          | 64         |

use serde::Serialize;

/// Byte order of the file payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ByteOrder {
    /// Little endian (endian flag byte `0`; all observed exports).
    Little,
    /// Big endian (endian flag byte `1`; experimental support).
    Big,
}

impl ByteOrder {
    /// Human-readable name.
    pub fn label(self) -> &'static str {
        match self {
            ByteOrder::Little => "little-endian",
            ByteOrder::Big => "big-endian",
        }
    }
}

/// MOC3 format version wrapper around the raw version byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct MocVersion(u8);

impl MocVersion {
    /// Highest version byte this parser understands.
    pub const MAX_SUPPORTED: u8 = 6;

    /// Parse and validate a version byte.
    pub fn from_byte(byte: u8) -> Option<Self> {
        if (1..=Self::MAX_SUPPORTED).contains(&byte) {
            Some(Self(byte))
        } else {
            None
        }
    }

    /// Raw version byte as stored in the file header.
    pub fn byte(self) -> u8 {
        self.0
    }

    /// Cubism Editor version range this format version corresponds to.
    pub fn cubism_range(self) -> &'static str {
        match self.0 {
            1 => "3.0.00-3.2.07",
            2 => "3.3.00-3.3.03",
            3 => "4.0.00-4.1.05",
            4 => "4.2.00-4.2.04",
            5 => "5.0.00-5.2.03",
            _ => "5.3.00+",
        }
    }

    /// Short human-readable label, e.g. `2 (3.3.00-3.3.03)`.
    pub fn label(self) -> String {
        format!("{} ({})", self.0, self.cubism_range())
    }

    /// Number of `u32` slots in the section offset table.
    pub fn offset_slots(self) -> usize {
        if self.0 >= 6 {
            480
        } else {
            160
        }
    }

    /// Number of `i32` values in the count info section.
    pub fn count_info_ints(self) -> usize {
        if self.0 >= 5 {
            64
        } else {
            32
        }
    }

    /// `warp.quad_transform` keyform flag exists (3.3+, byte >= 2).
    pub fn has_quad_transform(self) -> bool {
        self.0 >= 2
    }

    /// Blend shape / color keyform tables exist (4.2+, byte >= 4).
    pub fn has_v42_sections(self) -> bool {
        self.0 >= 4
    }

    /// 5.0 blend shape targets and keyform colors exist (byte >= 5).
    pub fn has_v50_sections(self) -> bool {
        self.0 >= 5
    }

    /// 5.3 offscreen rendering sections exist (byte >= 6).
    pub fn has_v53_sections(self) -> bool {
        self.0 >= 6
    }

    /// Number of section slots actually defined for this version.
    pub fn used_slots(self) -> usize {
        crate::table::SLOT_COUNT_BY_VERSION
            .get((self.0 - 1) as usize)
            .copied()
            .unwrap_or(crate::table::SLOT_DEFS.len())
    }
}
