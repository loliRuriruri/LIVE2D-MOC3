//! Texture asset input model (work order sections 42-46).
//!
//! Pixels are supplied separately from the semantic model; this crate never
//! fabricates images (no checkerboard placeholders, no resizing).

/// One texture page supplied by the caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextureAsset {
    /// Source page index (the number art meshes reference).
    pub page: u32,
    /// PNG bytes (verbatim; never re-encoded).
    pub bytes: Vec<u8>,
    /// Source path (metadata only; never used as an archive entry name).
    pub source_path: Option<String>,
    /// Image width when known.
    pub width: Option<u32>,
    /// Image height when known.
    pub height: Option<u32>,
}

/// All texture assets passed to the writer.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextureAssets {
    /// Pages in ascending page order.
    pub assets: Vec<TextureAsset>,
}

impl TextureAssets {
    /// Empty asset set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append an asset.
    pub fn push(&mut self, asset: TextureAsset) {
        self.assets.push(asset);
        self.assets.sort_by_key(|asset| asset.page);
    }

    /// Asset for a page.
    pub fn for_page(&self, page: u32) -> Option<&TextureAsset> {
        self.assets.iter().find(|asset| asset.page == page)
    }

    /// FNV-1a hash of one asset's bytes (dedup identity).
    pub fn hash_of(asset: &TextureAsset) -> String {
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        for byte in &asset.bytes {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        format!("{hash:016x}")
    }

    /// Deterministic archive entry name for a page.
    pub fn archive_name(page: u32) -> String {
        format!("imageFileBuf_{page}.png")
    }
}
