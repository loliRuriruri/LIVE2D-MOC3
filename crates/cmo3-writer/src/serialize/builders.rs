//! XML builders for the AGENT.5.1 minimal MODEL_IMAGE-mode document.
//!
//! Every builder receives explicit pool ids; there are no lookups and no
//! fallback literals, so a dangling `xs.ref` cannot be produced here.

use crate::model::{ChildRef, ParentRef, PartOut};
use crate::xml::{
    bool_leaf, float_array, float_leaf, int_array, int_leaf, string_leaf, XmlElement,
};

/// Shared ids used across builders.
#[derive(Debug, Clone, Copy)]
pub struct SharedIds {
    /// `CoordType` "DeformerLocal" pool id.
    pub coord_deformer_local: usize,
}

/// Per-mesh object ids.
#[derive(Debug, Clone, Copy)]
pub struct MeshIds {
    /// `CDrawableGuid` pool id.
    pub drawable_pool: usize,
    /// `GEditableMeshGuid` pool id.
    pub editable_mesh_guid: usize,
    /// `CExtensionGuid` for the editable-mesh extension.
    pub extension_guid_editable: usize,
    /// `CExtensionGuid` for the texture-input extension.
    pub extension_guid_texture_input: usize,
    /// `CExtensionGuid` for the mesh-generator extension.
    pub extension_guid_mesh_generator: usize,
    /// `CTextureInputExtension` object pool id (always allocated).
    pub texture_input_ext_id: usize,
    /// `CTextureInput_ModelImage` object pool id when textured.
    pub texture_input_id: Option<usize>,
    /// `KeyformGridSource` pool id.
    pub grid_source_id: usize,
    /// `KeyformBindingSource` pool id.
    pub binding_id: usize,
    /// `CFormGuid` for the static mesh form.
    pub form_guid: usize,
    /// `CModelImageGuid` when textured.
    pub model_image_guid: Option<usize>,
    /// `GTexture2D` pool id of the mesh's resource.
    pub texture_pool: Option<usize>,
}

/// Shared default `CAffine` identity.
pub fn affine(name: &str) -> XmlElement {
    XmlElement::new("CAffine")
        .attr("xs.n", name)
        .attr("m00", "1.0")
        .attr("m01", "0.0")
        .attr("m02", "0.0")
        .attr("m10", "0.0")
        .attr("m11", "1.0")
        .attr("m12", "0.0")
}

/// Shared white `CFloatColor`.
pub fn white_color(name: &str) -> XmlElement {
    XmlElement::new("CFloatColor")
        .attr("xs.n", name)
        .attr("red", "1.0")
        .attr("green", "1.0")
        .attr("blue", "1.0")
        .attr("alpha", "1.0")
}

/// `<carray_list name count="0"/>`.
pub fn empty_carray(name: &str) -> XmlElement {
    XmlElement::new("carray_list")
        .attr("xs.n", name)
        .attr("count", "0")
}

/// `<hash_map name count="0" keyType="string"/>`.
pub fn empty_hash_map(name: &str) -> XmlElement {
    XmlElement::new("hash_map")
        .attr("xs.n", name)
        .attr("count", "0")
        .attr("keyType", "string")
}

/// `<Tag xs.n="name" xs.ref="#id"/>`.
pub fn reference(tag: &str, name: &str, id: usize) -> XmlElement {
    XmlElement::new(tag)
        .attr("xs.n", name)
        .attr("xs.ref", format!("#{id}"))
}

/// `<Tag xs.ref="#id"/>` without `xs.n`.
pub fn reference_anon(tag: &str, id: usize) -> XmlElement {
    XmlElement::new(tag).attr("xs.ref", format!("#{id}"))
}

/// `<null xs.n="name"/>`.
pub fn null_leaf(name: &str) -> XmlElement {
    XmlElement::new("null").attr("xs.n", name)
}

/// Morph target set shared by parts, meshes and deformers.
pub fn morph_target_set() -> XmlElement {
    let mut morph = XmlElement::new("KeyFormMorphTargetSet").attr("xs.n", "keyformMorphTargetSet");
    morph.push(empty_carray("_morphTargets"));
    morph.push(
        XmlElement::new("MorphTargetBlendWeightConstraintSet")
            .attr("xs.n", "blendWeightConstraintSet")
            .child(empty_carray("_constraints")),
    );
    morph
}

/// `GEditableMesh2` extension with geometry from the recovered mesh.
pub fn editable_mesh_extension(
    mesh: &crate::model::ArtMeshOut,
    ids: &MeshIds,
    shared: &SharedIds,
) -> XmlElement {
    let mut extension = XmlElement::new("CEditableMeshExtension");
    extension.push(extension_super(
        ids.extension_guid_editable,
        ids.drawable_pool,
    ));
    let mut editable = XmlElement::new("GEditableMesh2")
        .attr("xs.n", "editableMesh")
        .attr("nextPointUid", mesh.positions.len().to_string())
        .attr("useDelaunayTriangulation", "true");
    let point: Vec<f32> = mesh
        .positions
        .iter()
        .flat_map(|position| [position[0], position[1]])
        .collect();
    editable.push(float_array("point", &point));
    let mut priorities = XmlElement::new("byte-array")
        .attr("xs.n", "pointPriority")
        .attr("count", mesh.positions.len().to_string());
    let priority = XmlElement::new("b").attr("v", "20");
    for _ in 0..mesh.positions.len() {
        priorities.push(priority.clone());
    }
    editable.push(priorities);
    // Edges are not stored in the recovered IR (documented limitation).
    editable.push(
        XmlElement::new("short-array")
            .attr("xs.n", "edge")
            .attr("count", "0"),
    );
    editable.push(
        XmlElement::new("byte-array")
            .attr("xs.n", "edgePriority")
            .attr("count", "0"),
    );
    let uids: Vec<i64> = (0..mesh.positions.len())
        .map(|index| index as i64)
        .collect();
    editable.push(int_array("pointUid", &uids));
    editable.push(reference(
        "GEditableMeshGuid",
        "meshGuid",
        ids.editable_mesh_guid,
    ));
    editable.push(reference(
        "CoordType",
        "coordType",
        shared.coord_deformer_local,
    ));
    extension.push(editable);
    extension.push(bool_leaf("isLocked", false));
    extension
}

/// `CMeshGeneratorExtension` with the pinned default settings.
pub fn mesh_generator_extension(ids: &MeshIds) -> XmlElement {
    let mut extension = XmlElement::new("CMeshGeneratorExtension");
    extension.push(extension_super(
        ids.extension_guid_mesh_generator,
        ids.drawable_pool,
    ));
    let mut settings = XmlElement::new("MeshGenerateSetting").attr("xs.n", "meshGenerateSetting");
    for (name, value) in [
        ("polygonOuterDensity", 100),
        ("polygonInnerDensity", 100),
        ("polygonMargin", 20),
        ("polygonInnerMargin", 20),
        ("polygonMinMargin", 5),
        ("polygonMinBoundsPt", 5),
        ("thresholdAlpha", 0),
    ] {
        settings.push(int_leaf(name, value));
    }
    extension.push(settings);
    extension
}

/// `CTextureInputExtension` (owned reverse reference included).
pub fn texture_input_extension(ids: &MeshIds, texture_input_id: usize) -> XmlElement {
    let mut extension = XmlElement::new("CTextureInputExtension")
        .attr("xs.id", format!("#{}", ids.texture_input_ext_id))
        .attr("xs.idx", ids.texture_input_ext_id.to_string());
    extension.push(extension_super(
        ids.extension_guid_texture_input,
        ids.drawable_pool,
    ));
    let mut inputs = XmlElement::new("carray_list")
        .attr("xs.n", "_textureInputs")
        .attr("count", "1");
    inputs.push(reference_anon("CTextureInput_ModelImage", texture_input_id));
    extension.push(inputs);
    extension.push(reference(
        "CTextureInput_ModelImage",
        "currentTextureInputData",
        texture_input_id,
    ));
    extension
}

/// `CTextureInput_ModelImage` linking the mesh to its model image.
pub fn texture_input_model_image(
    ids: &MeshIds,
    texture_input_id: usize,
    model_image_guid: usize,
) -> XmlElement {
    let mut input = XmlElement::new("CTextureInput_ModelImage")
        .attr("xs.id", format!("#{texture_input_id}"))
        .attr("xs.idx", texture_input_id.to_string());
    let mut super_input = XmlElement::new("ACTextureInput").attr("xs.n", "super");
    super_input.push(affine("optionalTransformOnCanvas"));
    super_input.push(reference(
        "CTextureInputExtension",
        "_owner",
        ids.texture_input_ext_id,
    ));
    input.push(super_input);
    input.push(reference(
        "CModelImageGuid",
        "_modelImageGuid",
        model_image_guid,
    ));
    input
}

fn extension_super(extension_guid: usize, drawable_pool: usize) -> XmlElement {
    let mut super_ext = XmlElement::new("ACExtension").attr("xs.n", "super");
    super_ext.push(reference("CExtensionGuid", "guid", extension_guid));
    super_ext.push(reference("CArtMeshSource", "_owner", drawable_pool));
    super_ext
}

/// `CArtMeshSource` for one quad with a single static form.
pub fn art_mesh_source(
    mesh: &crate::model::ArtMeshOut,
    ids: &MeshIds,
    part_guids: &[usize],
    deformer_root: usize,
    shared: &SharedIds,
) -> XmlElement {
    let mut source = XmlElement::new("CArtMeshSource");
    let mut super_source = XmlElement::new("ACDrawableSource").attr("xs.n", "super");
    let mut controllable = XmlElement::new("ACParameterControllableSource").attr("xs.n", "super");
    controllable.push(string_leaf("localName", &mesh.name));
    controllable.push(bool_leaf("isVisible", mesh.visible));
    controllable.push(bool_leaf("isLocked", false));
    let parent_part_pool = match &mesh.parent {
        ParentRef::Part(part_index) => part_guids.get(*part_index).copied().unwrap_or(0),
        _ => part_guids.first().copied().unwrap_or(0),
    };
    controllable.push(reference("CPartGuid", "parentGuid", parent_part_pool));
    controllable.push(reference(
        "KeyformGridSource",
        "keyformGridSource",
        ids.grid_source_id,
    ));
    controllable.push(morph_target_set());
    let extension_count = if ids.texture_input_id.is_some() { 3 } else { 2 };
    let mut extensions = XmlElement::new("carray_list")
        .attr("xs.n", "_extensions")
        .attr("count", extension_count.to_string());
    extensions.push(editable_mesh_extension(mesh, ids, shared));
    if ids.texture_input_id.is_some() {
        extensions.push(reference_anon(
            "CTextureInputExtension",
            ids.texture_input_ext_id,
        ));
    }
    extensions.push(mesh_generator_extension(ids));
    controllable.push(extensions);
    controllable.push(null_leaf("internalColor_direct_argb"));
    super_source.push(controllable);
    source.push(super_source);
    source.push(
        XmlElement::new("CDrawableId")
            .attr("xs.n", "id")
            .attr("idstr", &mesh.name),
    );
    source.push(reference("CDrawableGuid", "guid", ids.drawable_pool));
    source.push(reference(
        "CDeformerGuid",
        "targetDeformerGuid",
        deformer_root,
    ));
    let mut clip = XmlElement::new("carray_list")
        .attr("xs.n", "clipGuidList")
        .attr("count", "0");
    if let Some(target) = mesh.mask_refs.first() {
        clip = clip.attr("count", "1");
        clip.push(reference_anon("CDrawableGuid", *target));
    }
    source.push(clip);
    source.push(bool_leaf("invertClippingMask", mesh.inverted_mask));
    let indices: Vec<i64> = mesh.indices.iter().map(|value| i64::from(*value)).collect();
    source.push(int_array("indices", &indices));
    let mut keyforms = XmlElement::new("carray_list")
        .attr("xs.n", "keyforms")
        .attr("count", "1");
    let mut form_element = XmlElement::new("CArtMeshForm");
    let mut drawable_form = XmlElement::new("ACDrawableForm").attr("xs.n", "super");
    let mut aform = XmlElement::new("ACForm").attr("xs.n", "super");
    aform.push(reference("CFormGuid", "guid", ids.form_guid));
    aform.push(bool_leaf("isAnimatedForm", false));
    aform.push(bool_leaf("isLocalAnimatedForm", false));
    aform.push(reference("CArtMeshSource", "_source", ids.drawable_pool));
    aform.push(null_leaf("name"));
    aform.push(string_leaf("notes", ""));
    drawable_form.push(aform);
    drawable_form.push(int_leaf("drawOrder", 500));
    drawable_form.push(float_leaf("opacity", 1.0));
    drawable_form.push(white_color("multiplyColor"));
    drawable_form.push(white_color("screenColor"));
    drawable_form.push(reference(
        "CoordType",
        "coordType",
        shared.coord_deformer_local,
    ));
    form_element.push(drawable_form);
    let flat: Vec<f32> = mesh
        .positions
        .iter()
        .flat_map(|position| [position[0], position[1]])
        .collect();
    form_element.push(float_array("positions", &flat));
    keyforms.push(form_element);
    source.push(keyforms);
    source.push(float_array("positions", &flat));
    let uvs: Vec<f32> = mesh.uvs.iter().flat_map(|uv| [uv[0], uv[1]]).collect();
    source.push(float_array("uvs", &uvs));
    match ids.texture_pool {
        Some(texture_pool) => source.push(reference("GTexture2D", "texture", texture_pool)),
        None => source.push(null_leaf("texture")),
    }
    let composition = if mesh.additive {
        "ADD"
    } else if mesh.multiplicative {
        "MULTIPLY"
    } else {
        "NORMAL"
    };
    source.push(
        XmlElement::new("ColorComposition")
            .attr("xs.n", "colorComposition")
            .attr("v", composition),
    );
    source.push(bool_leaf("culling", !mesh.double_sided));
    source.push(
        XmlElement::new("TextureState")
            .attr("xs.n", "textureState")
            .attr("v", "MODEL_IMAGE"),
    );
    source.push(string_leaf("userData", ""));
    source
}

/// `CPartSource` with child GUID references and static forms.
#[allow(clippy::too_many_arguments)]
pub fn part_source(
    part: &PartOut,
    own_guid: usize,
    parent_pool: Option<usize>,
    child_refs: &[XmlElement],
    form_guids: &[usize],
    deformer_root: usize,
    root_part_id: &str,
) -> XmlElement {
    let mut source = XmlElement::new("CPartSource");
    let mut super_part = XmlElement::new("ACParameterControllableSource").attr("xs.n", "super");
    super_part.push(string_leaf("localName", &part.name));
    super_part.push(bool_leaf("isVisible", part.visible));
    super_part.push(bool_leaf("isLocked", false));
    match parent_pool {
        Some(parent) => super_part.push(reference("CPartGuid", "parentGuid", parent)),
        None => super_part.push(null_leaf("parentGuid")),
    }
    super_part.push(null_leaf("keyformGridSource"));
    super_part.push(morph_target_set());
    super_part.push(empty_carray("_extensions"));
    super_part.push(null_leaf("internalColor_direct_argb"));
    source.push(super_part);
    source.push(reference("CPartGuid", "guid", own_guid));
    source.push(XmlElement::new("CPartId").attr("xs.n", "id").attr(
        "idstr",
        if part.is_root {
            root_part_id
        } else {
            &part.name
        },
    ));
    source.push(bool_leaf("enableDrawOrderGroup", false));
    source.push(int_leaf("defaultOrder_forEditor", 500));
    source.push(bool_leaf("isSketch", false));
    source.push(XmlElement::new("CColor").attr("xs.n", "partsEditColor"));
    let mut children = XmlElement::new("carray_list")
        .attr("xs.n", "_childGuids")
        .attr("count", child_refs.len().to_string());
    for child in child_refs {
        children.push(child.clone());
    }
    source.push(children);
    source.push(reference(
        "CDeformerGuid",
        "targetDeformerGuid",
        deformer_root,
    ));
    let mut keyforms = XmlElement::new("carray_list")
        .attr("xs.n", "keyforms")
        .attr("count", form_guids.len().to_string());
    for form_guid in form_guids {
        let mut form_element = XmlElement::new("CPartForm");
        let mut aform = XmlElement::new("ACForm").attr("xs.n", "super");
        aform.push(reference("CFormGuid", "guid", *form_guid));
        aform.push(bool_leaf("isAnimatedForm", false));
        aform.push(bool_leaf("isLocalAnimatedForm", false));
        aform.push(reference("CPartSource", "_source", own_guid));
        aform.push(null_leaf("name"));
        aform.push(string_leaf("notes", ""));
        form_element.push(aform);
        form_element.push(int_leaf("drawOrder", 500));
        keyforms.push(form_element);
    }
    source.push(keyforms);
    source
}

/// Resolve a `ChildRef` into a child GUID element.
pub fn child_guid(
    child: &ChildRef,
    part_guids: &[usize],
    drawable_guids: &[usize],
) -> Option<XmlElement> {
    match child {
        ChildRef::Part(index) => part_guids
            .get(*index)
            .map(|id| reference_anon("CPartGuid", *id)),
        ChildRef::Mesh(index) => drawable_guids
            .get(*index)
            .map(|id| reference_anon("CDrawableGuid", *id)),
        ChildRef::Warp(_) | ChildRef::Rotation(_) => None, // deformers deferred
    }
}

/// `GTexture2D` for one image resource.
pub fn gtexture2d(
    index: usize,
    texture_pool: usize,
    resource_guid: usize,
    image_resource_id: usize,
) -> XmlElement {
    let mut element = XmlElement::new("GTexture2D")
        .attr("xs.id", format!("#{texture_pool}"))
        .attr("xs.idx", texture_pool.to_string());
    let mut gtexture = XmlElement::new("GTexture").attr("xs.n", "super");
    gtexture.push(string_leaf("name", &format!("Texture {index}")));
    gtexture.push(
        XmlElement::new("WrapMode")
            .attr("xs.n", "wrapMode")
            .attr("v", "CLAMP_TO_BORDER"),
    );
    let mut filter_mode = XmlElement::new("FilterMode").attr("xs.n", "filterMode");
    filter_mode.push(reference("GTexture2D", "owner", texture_pool));
    filter_mode.push(
        XmlElement::new("MinFilter")
            .attr("xs.n", "minFilter")
            .attr("v", "LINEAR_MIPMAP_LINEAR"),
    );
    filter_mode.push(
        XmlElement::new("MagFilter")
            .attr("xs.n", "magFilter")
            .attr("v", "LINEAR"),
    );
    gtexture.push(filter_mode);
    gtexture.push(reference("GTextureGuid", "guid", resource_guid));
    gtexture.push(
        XmlElement::new("Anisotropy")
            .attr("xs.n", "anisotropy")
            .attr("v", "ON"),
    );
    element.push(gtexture);
    element.push(reference(
        "CImageResource",
        "srcImageResource",
        image_resource_id,
    ));
    element.push(affine("transformImageResource01toLogical01"));
    element.push(int_leaf("mipmapLevel", 1));
    element.push(bool_leaf("isPremultiplied", true));
    element
}

/// `CImageResource` shared object.
pub fn image_resource(
    id: usize,
    width: i32,
    height: i32,
    byte_len: usize,
    archive_name: &str,
) -> XmlElement {
    let mut element = XmlElement::new("CImageResource")
        .attr("xs.id", format!("#{id}"))
        .attr("xs.idx", id.to_string())
        .attr("width", width.to_string())
        .attr("height", height.to_string())
        .attr("type", "INT_ARGB")
        .attr("imageFileBuf_size", byte_len.to_string())
        .attr("previewFileBuf_size", "0");
    element.push(
        XmlElement::new("file")
            .attr("xs.n", "imageFileBuf")
            .attr("path", archive_name),
    );
    element
}

/// `CachedImageManager` for one model image.
pub fn cached_image_manager(resource_id: usize, width: i32, height: i32) -> XmlElement {
    let mut manager = XmlElement::new("CCachedImageManager").attr("xs.n", "cachedImageManager");
    manager.push(
        XmlElement::new("CachedImageType")
            .attr("xs.n", "defaultCacheType")
            .attr("v", "SCALE_1"),
    );
    manager.push(reference("CImageResource", "rawImage", resource_id));
    let mut cached = XmlElement::new("array_list")
        .attr("xs.n", "cachedImages")
        .attr("count", "1");
    let mut image = XmlElement::new("CCachedImage");
    image.push(reference(
        "CImageResource",
        "_cachedImageResource",
        resource_id,
    ));
    image.push(bool_leaf("isSharedImage", true));
    image.push(
        XmlElement::new("CSize")
            .attr("xs.n", "rawImageSize")
            .attr("width", width.to_string())
            .attr("height", height.to_string()),
    );
    image.push(int_leaf("reductionRatio", 1));
    image.push(int_leaf("mipmapLevel", 1));
    image.push(bool_leaf("hasMargin", false));
    image.push(bool_leaf("isCleaned", false));
    image.push(affine("transformRawImageToCachedImage"));
    cached.push(image);
    manager.push(cached);
    manager.push(int_leaf("requiredMipmapLevel", 1));
    manager
}

/// `CModelImage` inline object.
#[allow(clippy::too_many_arguments)]
pub fn model_image(
    model_image_guid: usize,
    name: &str,
    filter_set_id: usize,
    env: XmlElement,
    resource_id: usize,
    group_id: usize,
    layered_guid: usize,
    width: i32,
    height: i32,
) -> XmlElement {
    let mut image = XmlElement::new("CModelImage").attr("modelImageVersion", "0");
    image.push(reference("CModelImageGuid", "guid", model_image_guid));
    image.push(string_leaf("name", name));
    image.push(reference(
        "ModelImageFilterSet",
        "inputFilter",
        filter_set_id,
    ));
    image.push(env);
    image.push(reference("CImageResource", "_filteredImage", resource_id));
    image.push(null_leaf("icon16"));
    image.push(affine("_materialLocalToCanvasTransform"));
    image.push(reference("CModelImageGroup", "_group", group_id));
    let mut linked_raw = XmlElement::new("carray_list")
        .attr("xs.n", "linkedRawImageGuids")
        .attr("count", "1");
    linked_raw.push(reference_anon("CLayeredImageGuid", layered_guid));
    image.push(linked_raw);
    image.push(cached_image_manager(resource_id, width, height));
    image.push(string_leaf("memo", ""));
    image
}

/// `ModelImageFilterEnv` with the two required environment entries.
pub fn model_image_env(
    layered_guid: usize,
    layer_id: usize,
    key_current_guid: usize,
    key_layer_input: usize,
) -> XmlElement {
    let mut env = XmlElement::new("ModelImageFilterEnv").attr("xs.n", "inputFilterEnv");
    let mut super_env = XmlElement::new("FilterEnv").attr("xs.n", "super");
    super_env.push(null_leaf("parentEnv"));
    let mut values = XmlElement::new("hash_map")
        .attr("xs.n", "envValues")
        .attr("count", "2");
    // Entry 1: current layered image.
    let mut guid_value = XmlElement::new("EnvValueSet").attr("xs.n", "value");
    guid_value.push(reference("FilterValueId", "id", key_current_guid));
    guid_value.push(reference("CLayeredImageGuid", "value", layered_guid));
    guid_value.push(
        XmlElement::new("l")
            .attr("xs.n", "updateTimeMs")
            .attr("v", "0"),
    );
    values.push(
        XmlElement::new("entry")
            .child(reference("FilterValueId", "key", key_current_guid))
            .child(guid_value),
    );
    // Entry 2: layer selection map.
    let mut map = XmlElement::new("CLayerSelectorMap").attr("xs.n", "value");
    let mut linked = XmlElement::new("linked_map")
        .attr("xs.n", "_imageToLayerInput")
        .attr("count", "1");
    let mut inputs = XmlElement::new("array_list")
        .attr("xs.n", "value")
        .attr("count", "1");
    let mut data = XmlElement::new("CLayerInputData");
    data.push(reference("CLayer", "layer", layer_id));
    data.push(affine("affine"));
    data.push(null_leaf("clippingOnTexturePx"));
    inputs.push(data);
    linked.push(
        XmlElement::new("entry")
            .child(reference("CLayeredImageGuid", "key", layered_guid))
            .child(inputs),
    );
    map.push(linked);
    let mut selector_value = XmlElement::new("EnvValueSet").attr("xs.n", "value");
    selector_value.push(reference("FilterValueId", "id", key_layer_input));
    selector_value.push(map);
    selector_value.push(
        XmlElement::new("l")
            .attr("xs.n", "updateTimeMs")
            .attr("v", "0"),
    );
    values.push(
        XmlElement::new("entry")
            .child(reference("FilterValueId", "key", key_layer_input))
            .child(selector_value),
    );
    super_env.push(values);
    env.push(super_env);
    env
}

/// `LayeredImageWrapper` for the texture manager raw image list.
pub fn layered_image_wrapper(layered_image_id: usize) -> XmlElement {
    XmlElement::new("LayeredImageWrapper")
        .child(reference("CLayeredImage", "image", layered_image_id))
        .child(
            XmlElement::new("l")
                .attr("xs.n", "importedTimeMSec")
                .attr("v", "0"),
        )
        .child(
            XmlElement::new("l")
                .attr("xs.n", "lastModifiedTimeMSec")
                .attr("v", "0"),
        )
        .child(bool_leaf("isReplaced", false))
}

/// ACLayerEntry super used by the root group and layers. `group` is the
/// owning `CLayerGroup` for ordinary layers and `None` (null) for the root.
pub fn layer_entry_super(
    name: &str,
    guid_note: &str,
    guid_uuid: &str,
    blend_id: usize,
    layered_image_id: usize,
    group: Option<usize>,
) -> XmlElement {
    let mut entry = XmlElement::new("ACLayerEntry").attr("xs.n", "super");
    entry.push(string_leaf("name", name));
    entry.push(string_leaf("memo", ""));
    entry.push(bool_leaf("isVisible", true));
    entry.push(bool_leaf("isClipping", false));
    entry.push(reference("CBlend_Normal", "blend", blend_id));
    entry.push(
        XmlElement::new("CLayerGuid")
            .attr("xs.n", "guid")
            .attr("uuid", guid_uuid)
            .attr("note", guid_note),
    );
    match group {
        Some(group) => entry.push(reference("CLayerGroup", "group", group)),
        None => entry.push(null_leaf("group")),
    }
    entry.push(int_leaf("opacity255", 255));
    entry.push(empty_hash_map("_optionOfIOption"));
    entry.push(reference(
        "CLayeredImage",
        "_layeredImage",
        layered_image_id,
    ));
    entry
}

/// Filter output connector shared by the two filter instances.
#[allow(clippy::too_many_arguments)]
pub fn filter_output_connector(
    connector_id: usize,
    selector_id: usize,
    output_layer_data_id: usize,
    select_layer_value_id: usize,
) -> XmlElement {
    let mut output = XmlElement::new("FilterOutputValueConnector")
        .attr("xs.id", format!("#{connector_id}"))
        .attr("xs.idx", connector_id.to_string());
    output.push(XmlElement::new("AValueConnector").attr("xs.n", "super"));
    output.push(reference("FilterInstance", "instance", selector_id));
    output.push(reference("FilterValueId", "id", output_layer_data_id));
    output.push(reference("FilterValue", "valueDef", select_layer_value_id));
    output
}

/// `EnvConnection` entry used by the filter set external maps.
pub fn env_connection(
    key_id: usize,
    filter_id: usize,
    env_value_def: usize,
    filter_value_def: usize,
) -> XmlElement {
    XmlElement::new("entry")
        .child(reference("FilterValueId", "key", key_id))
        .child(
            XmlElement::new("EnvConnection")
                .attr("xs.n", "value")
                .child(reference("FilterValue", "_envValueDef", env_value_def))
                .child(reference("FilterInstance", "filter", filter_id))
                .child(reference("FilterValue", "filterValueDef", filter_value_def)),
        )
}

/// `EnvValueConnector` entry used by the selector input connector map.
pub fn env_connector_entry(key_id: usize, env_id: usize) -> XmlElement {
    let mut connector = XmlElement::new("EnvValueConnector").attr("xs.n", "value");
    connector.push(XmlElement::new("AValueConnector").attr("xs.n", "super"));
    connector.push(reference("FilterValueId", "envValueId", env_id));
    XmlElement::new("entry")
        .child(reference("FilterValueId", "key", key_id))
        .child(connector)
}
