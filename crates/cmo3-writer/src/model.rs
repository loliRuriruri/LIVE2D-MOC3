//! Writer-specific CMO3 project model (work order section 7).
//!
//! This model is neither the semantic IR nor raw XML: it is the intermediate
//! representation the XML serializer consumes after all mapping decisions
//! (including writer-required defaults) have been made explicit.

/// Canvas mapping (editor space).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CanvasOut {
    /// Pixel width.
    pub width: i32,
    /// Pixel height.
    pub height: i32,
    /// Pixels per unit.
    pub pixels_per_unit: f32,
    /// Origin X in pixels.
    pub origin_x: i32,
    /// Origin Y in pixels.
    pub origin_y: i32,
}

/// One parameter.
#[derive(Debug, Clone, PartialEq)]
pub struct ParameterOut {
    /// Semantic id (`parameter:000004` or the stored name).
    pub semantic: String,
    /// Editor name (stored name or deterministic placeholder).
    pub name: String,
    /// Minimum.
    pub minimum: f32,
    /// Maximum.
    pub maximum: f32,
    /// Default.
    pub default: f32,
    /// Decimal places.
    pub decimals: i32,
    /// Repeat flag.
    pub repeat: bool,
    /// Stored key values.
    pub keys: Vec<f32>,
    /// Whether the name is a writer placeholder.
    pub synthetic_name: bool,
}

/// One keyform binding source (one parameter axis of one target grid).
#[derive(Debug, Clone, PartialEq)]
pub struct BindingOut {
    /// Semantic band id (`band:binding:000005`).
    pub semantic: String,
    /// Owning grid semantic id.
    pub grid: String,
    /// Parameter index into [`Cmo3Project::parameters`].
    pub parameter_index: usize,
    /// Stored keys (verbatim order).
    pub keys: Vec<f32>,
    /// Whether this binding's ordering was a writer-required default.
    pub ordering_default: bool,
}

/// One stored form of a grid.
#[derive(Debug, Clone, PartialEq)]
pub struct GridFormOut {
    /// Stored form index (verbatim).
    pub form_index: usize,
    /// Key index per binding (Cartesian coordinate).
    pub key_indices: Vec<usize>,
}

/// One keyform grid (one target).
#[derive(Debug, Clone, PartialEq)]
pub struct GridOut {
    /// Semantic grid id (`grid:binding:000000:art_mesh:...`).
    pub semantic: String,
    /// Target this grid belongs to.
    pub target: crate::model::TargetOut,
    /// Binding indices into [`Cmo3Project::bindings`], axis order.
    pub binding_indices: Vec<usize>,
    /// Forms in stored order.
    pub forms: Vec<GridFormOut>,
    /// Whether the recovered layout was dense.
    pub dense: bool,
}

/// A target object reference for serialization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetOut {
    /// Part by index.
    Part(usize),
    /// Warp deformer by index.
    Warp(usize),
    /// Rotation deformer by index.
    Rotation(usize),
    /// Art mesh by index.
    ArtMesh(usize),
}

/// Resolved hierarchy parent (never a mixed index space).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParentRef {
    /// Synthetic root part (index 0).
    Root,
    /// Part index.
    Part(usize),
    /// Warp deformer semantic id.
    Warp(String),
    /// Rotation deformer semantic id.
    Rotation(String),
}

/// One child reference for `_childGuids` lists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChildRef {
    /// Part index.
    Part(usize),
    /// Warp deformer semantic id.
    Warp(String),
    /// Rotation deformer semantic id.
    Rotation(String),
    /// Art mesh index.
    Mesh(usize),
}

/// One part.
#[derive(Debug, Clone, PartialEq)]
pub struct PartOut {
    /// Semantic id.
    pub semantic: String,
    /// Editor name.
    pub name: String,
    /// Hierarchy parent (root for top-level parts).
    pub parent: ParentRef,
    /// Children in deterministic order (parts, deformers, meshes).
    pub children: Vec<ChildRef>,
    /// Keyform draw orders (one per stored form).
    pub draw_orders: Vec<f32>,
    /// Grid semantic id when the part is bound.
    pub grid: Option<String>,
    /// Visible flag.
    pub visible: bool,
    /// Enabled flag.
    pub enabled: bool,
    /// Whether this is the synthetic root part.
    pub is_root: bool,
    /// Whether the name is a writer placeholder.
    pub synthetic_name: bool,
}

/// One art mesh.
#[derive(Debug, Clone, PartialEq)]
pub struct ArtMeshOut {
    /// Semantic id.
    pub semantic: String,
    /// Editor name.
    pub name: String,
    /// Hierarchy parent (part or deformer; root when top-level).
    pub parent: ParentRef,
    /// Texture page index.
    pub texture: Option<usize>,
    /// Model image index when the mesh has a texture (MODEL_IMAGE mode).
    pub model_image: Option<usize>,
    /// Base vertex positions (editor space, verbatim).
    pub positions: Vec<[f32; 2]>,
    /// UVs (verbatim).
    pub uvs: Vec<[f32; 2]>,
    /// Triangle indices.
    pub indices: Vec<u16>,
    /// Forms in stored order.
    pub forms: Vec<ArtMeshFormOut>,
    /// Visible flag.
    pub visible: bool,
    /// Additive blending bit.
    pub additive: bool,
    /// Multiplicative blending bit.
    pub multiplicative: bool,
    /// Double-sided bit.
    pub double_sided: bool,
    /// Inverted mask bit.
    pub inverted_mask: bool,
    /// Masking drawables (art mesh indices).
    pub mask_refs: Vec<usize>,
    /// Grid semantic id when bound.
    pub grid: Option<String>,
    /// Whether the name is a writer placeholder.
    pub synthetic_name: bool,
}

/// One art mesh form.
#[derive(Debug, Clone, PartialEq)]
pub struct ArtMeshFormOut {
    /// Stored draw order.
    pub draw_order: f32,
    /// Stored opacity.
    pub opacity: f32,
    /// Keyed vertex positions.
    pub positions: Vec<[f32; 2]>,
}

/// One warp deformer.
#[derive(Debug, Clone, PartialEq)]
pub struct WarpOut {
    /// Semantic id.
    pub semantic: String,
    /// Editor name.
    pub name: String,
    /// Hierarchy parent.
    pub parent: ParentRef,
    /// Children in deterministic order.
    pub children: Vec<ChildRef>,
    /// Grid columns.
    pub columns: u32,
    /// Grid rows.
    pub rows: u32,
    /// Quad transform flag.
    pub quad_transform: bool,
    /// Forms.
    pub forms: Vec<WarpFormOut>,
    /// Grid semantic id when bound.
    pub grid: Option<String>,
    /// Whether the name is a writer placeholder.
    pub synthetic_name: bool,
}

/// One warp form.
#[derive(Debug, Clone, PartialEq)]
pub struct WarpFormOut {
    /// Stored opacity.
    pub opacity: f32,
    /// Control point positions.
    pub positions: Vec<[f32; 2]>,
}

/// One rotation deformer.
#[derive(Debug, Clone, PartialEq)]
pub struct RotationOut {
    /// Semantic id.
    pub semantic: String,
    /// Editor name.
    pub name: String,
    /// Hierarchy parent.
    pub parent: ParentRef,
    /// Children in deterministic order.
    pub children: Vec<ChildRef>,
    /// Base angle.
    pub base_angle: f32,
    /// Forms.
    pub forms: Vec<RotationFormOut>,
    /// Grid semantic id when bound.
    pub grid: Option<String>,
    /// Whether the name is a writer placeholder.
    pub synthetic_name: bool,
}

/// One rotation form.
#[derive(Debug, Clone, PartialEq)]
pub struct RotationFormOut {
    /// Stored opacity.
    pub opacity: f32,
    /// Stored angle.
    pub angle: f32,
    /// Stored origin.
    pub origin: [f32; 2],
    /// Stored scale.
    pub scale: f32,
    /// Reflect X.
    pub reflect_x: bool,
    /// Reflect Y.
    pub reflect_y: bool,
}

/// One texture page asset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextureOut {
    /// Source page index.
    pub page: u32,
    /// Archive entry name (`imageFileBuf_{page}.png`).
    pub archive_name: String,
    /// Source path (metadata only; never an archive entry name).
    pub source_path: Option<String>,
    /// Width when known.
    pub width: Option<u32>,
    /// Height when known.
    pub height: Option<u32>,
    /// Byte length.
    pub byte_len: usize,
    /// FNV-1a hash of the bytes (dedup identity).
    pub hash: String,
}

/// One synthetic layered-image layer (one textured mesh).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerOut {
    /// Deterministic layer semantic id (`layer:{index}`).
    pub semantic: String,
    /// Layer name (mesh name; synthetic when the mesh name is synthetic).
    pub name: String,
    /// `layerId` string (`00-00-{index:02}-01`).
    pub layer_id: String,
    /// `layerIdValue_testImpl`.
    pub layer_id_value: i32,
    /// Canvas width.
    pub width: i32,
    /// Canvas height.
    pub height: i32,
    /// Resource index into [`Cmo3Project::image_resources`].
    pub resource: usize,
    /// Whether the layer name is a writer placeholder.
    pub synthetic_name: bool,
}

/// One synthetic layered image (always exactly one in this phase).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayeredImageOut {
    /// Semantic id (`layered_image:000000`).
    pub semantic: String,
    /// Synthetic PSD file name (never claimed as the original).
    pub psd_name: String,
    /// Canvas width.
    pub width: i32,
    /// Canvas height.
    pub height: i32,
    /// Layers in deterministic order.
    pub layers: Vec<LayerOut>,
}

/// One model image (one mesh's texture input).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelImageOut {
    /// Semantic id (`model_image:{index}`).
    pub semantic: String,
    /// Display name (mesh name).
    pub name: String,
    /// Selected layer index.
    pub layer: usize,
    /// Shared image resource index.
    pub resource: usize,
}

/// One image resource (one PNG archive entry).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageResourceOut {
    /// Semantic id (`image_resource:000000`).
    pub semantic: String,
    /// Canvas width.
    pub width: i32,
    /// Canvas height.
    pub height: i32,
    /// PNG byte length.
    pub byte_len: usize,
    /// Archive entry name.
    pub archive_name: String,
}

/// A writer-required default (never presented as recovered data).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriterDefault {
    /// Semantic entity.
    pub semantic: String,
    /// Field.
    pub field: &'static str,
    /// Why the writer had to choose.
    pub reason: &'static str,
}

/// An unsupported/omitted concept with its reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsupportedNote {
    /// Semantic entity or concept.
    pub subject: String,
    /// Reason.
    pub reason: String,
}

/// The complete writer project model.
#[derive(Debug, Clone, PartialEq)]
pub struct Cmo3Project {
    /// Profile id used.
    pub profile_id: String,
    /// Model name (synthetic when no stored name exists).
    pub model_name: String,
    /// Canvas.
    pub canvas: CanvasOut,
    /// Parameters.
    pub parameters: Vec<ParameterOut>,
    /// Bindings.
    pub bindings: Vec<BindingOut>,
    /// Grids.
    pub grids: Vec<GridOut>,
    /// Parts (index 0 is the synthetic root).
    pub parts: Vec<PartOut>,
    /// Art meshes.
    pub meshes: Vec<ArtMeshOut>,
    /// Warp deformers.
    pub warps: Vec<WarpOut>,
    /// Rotation deformers.
    pub rotations: Vec<RotationOut>,
    /// Texture pages.
    pub textures: Vec<TextureOut>,
    /// Image resources (one per archive PNG).
    pub image_resources: Vec<ImageResourceOut>,
    /// Synthetic layered images (exactly one in AGENT.5.1).
    pub layered_images: Vec<LayeredImageOut>,
    /// Model images (one per textured mesh).
    pub model_images: Vec<ModelImageOut>,
    /// Writer-required defaults.
    pub defaults: Vec<WriterDefault>,
    /// Unsupported/omitted concepts.
    pub unsupported: Vec<UnsupportedNote>,
    /// Whether best-effort mode was used.
    pub best_effort: bool,
}
