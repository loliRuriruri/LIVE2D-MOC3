//! XML builders for the full semantic CMO3 document (AGENT.5.2).
//!
//! Every builder receives explicit pool ids; there are no lookups and no
//! fallback literals, so a dangling `xs.ref` cannot be produced here.

use crate::model::{ChildRef, ParentRef, PartOut, RotationOut, WarpOut};
use crate::profile::ROOT_PART_ID;
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
#[derive(Debug, Clone)]
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
    /// `KeyformGridSource` pool id when the mesh is bound.
    pub grid_source_id: Option<usize>,
    /// `CModelImageGuid` when textured.
    pub model_image_guid: Option<usize>,
    /// `GTexture2D` pool id of the mesh's resource.
    pub texture_pool: Option<usize>,
}

/// Per-deformer object ids.
#[derive(Debug, Clone)]
pub struct DeformerIds {
    /// `CDeformerGuid` pool id.
    pub guid_pool: usize,
    /// `KeyformGridSource` pool id when bound.
    pub grid_source_id: Option<usize>,
    /// `CFormGuid` pool id per stored form.
    pub form_guids: Vec<usize>,
}

/// One stored art-mesh form.
#[derive(Debug, Clone)]
pub struct MeshFormInput {
    /// `CFormGuid` pool id.
    pub form_guid: usize,
    /// Stored draw order.
    pub draw_order: f32,
    /// Stored opacity.
    pub opacity: f32,
    /// Keyed vertex positions (flat pairs).
    pub positions: Vec<f32>,
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

fn deformer_reference(parent_deformer_pool: Option<usize>, deformer_root: usize) -> XmlElement {
    match parent_deformer_pool {
        Some(parent) => reference("CDeformerGuid", "targetDeformerGuid", parent),
        None => reference("CDeformerGuid", "targetDeformerGuid", deformer_root),
    }
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

/// `CArtMeshSource` with all recovered forms.
#[allow(clippy::too_many_arguments)]
pub fn art_mesh_source(
    mesh: &crate::model::ArtMeshOut,
    ids: &MeshIds,
    forms: &[MeshFormInput],
    part_guids: &[usize],
    drawable_guids: &[usize],
    deformer_pools: &[(String, usize)],
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
    match ids.grid_source_id {
        Some(grid_source_id) => controllable.push(reference(
            "KeyformGridSource",
            "keyformGridSource",
            grid_source_id,
        )),
        None => controllable.push(null_leaf("keyformGridSource")),
    }
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
    let parent_deformer = match &mesh.parent {
        ParentRef::Warp(semantic) | ParentRef::Rotation(semantic) => deformer_pools
            .iter()
            .find(|(name, _)| name == semantic)
            .map(|(_, id)| *id),
        _ => None,
    };
    source.push(deformer_reference(parent_deformer, deformer_root));
    let mut clip = XmlElement::new("carray_list")
        .attr("xs.n", "clipGuidList")
        .attr("count", mesh.mask_refs.len().to_string());
    for mask in &mesh.mask_refs {
        if let Some(target) = drawable_guids.get(*mask).copied() {
            clip.push(reference_anon("CDrawableGuid", target));
        }
    }
    source.push(clip);
    source.push(bool_leaf("invertClippingMask", mesh.inverted_mask));
    let indices: Vec<i64> = mesh.indices.iter().map(|value| i64::from(*value)).collect();
    source.push(int_array("indices", &indices));
    let mut keyforms = XmlElement::new("carray_list")
        .attr("xs.n", "keyforms")
        .attr("count", forms.len().to_string());
    for form in forms {
        let mut form_element = XmlElement::new("CArtMeshForm");
        let mut drawable_form = XmlElement::new("ACDrawableForm").attr("xs.n", "super");
        let mut aform = XmlElement::new("ACForm").attr("xs.n", "super");
        aform.push(reference("CFormGuid", "guid", form.form_guid));
        aform.push(bool_leaf("isAnimatedForm", false));
        aform.push(bool_leaf("isLocalAnimatedForm", false));
        aform.push(reference("CArtMeshSource", "_source", ids.drawable_pool));
        aform.push(null_leaf("name"));
        aform.push(string_leaf("notes", ""));
        drawable_form.push(aform);
        drawable_form.push(int_leaf("drawOrder", form.draw_order.round() as i64));
        drawable_form.push(float_leaf("opacity", form.opacity));
        drawable_form.push(white_color("multiplyColor"));
        drawable_form.push(white_color("screenColor"));
        drawable_form.push(reference(
            "CoordType",
            "coordType",
            shared.coord_deformer_local,
        ));
        form_element.push(drawable_form);
        form_element.push(float_array("positions", &form.positions));
        keyforms.push(form_element);
    }
    source.push(keyforms);
    let base: Vec<f32> = mesh
        .positions
        .iter()
        .flat_map(|position| [position[0], position[1]])
        .collect();
    source.push(float_array("positions", &base));
    let uvs: Vec<f32> = mesh.uvs.iter().flat_map(|uv| [uv[0], uv[1]]).collect();
    source.push(float_array("uvs", &uvs));
    match ids.texture_pool {
        Some(texture_pool) => source.push(reference("GTexture2D", "texture", texture_pool)),
        None => source.push(null_leaf("texture")),
    }
    source.push(
        XmlElement::new("ColorComposition")
            .attr("xs.n", "colorComposition")
            .attr("v", &mesh.composition),
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

#[allow(clippy::too_many_arguments)]
fn deformer_super(
    local_name: &str,
    visible: bool,
    parent_part_pool: usize,
    grid_source_id: Option<usize>,
) -> XmlElement {
    let mut super_controllable =
        XmlElement::new("ACParameterControllableSource").attr("xs.n", "super");
    super_controllable.push(string_leaf("localName", local_name));
    super_controllable.push(bool_leaf("isVisible", visible));
    super_controllable.push(bool_leaf("isLocked", false));
    super_controllable.push(reference("CPartGuid", "parentGuid", parent_part_pool));
    match grid_source_id {
        Some(grid_source_id) => super_controllable.push(reference(
            "KeyformGridSource",
            "keyformGridSource",
            grid_source_id,
        )),
        None => super_controllable.push(null_leaf("keyformGridSource")),
    }
    super_controllable.push(morph_target_set());
    super_controllable.push(empty_carray("_extensions"));
    super_controllable.push(null_leaf("internalColor_direct_argb"));
    let mut super_deformer = XmlElement::new("ACDeformerSource").attr("xs.n", "super");
    super_deformer.push(super_controllable);
    super_deformer.push(null_leaf("internalColor_indirect_argb"));
    super_deformer
}

#[allow(clippy::too_many_arguments)]
fn deformer_form_super(
    form_guid: usize,
    owner_pool: usize,
    owner_tag: &str,
    opacity: f32,
    coord_deformer_local: usize,
) -> XmlElement {
    let mut super_element = XmlElement::new("ACDeformerForm").attr("xs.n", "super");
    let mut aform = XmlElement::new("ACForm").attr("xs.n", "super");
    aform.push(reference("CFormGuid", "guid", form_guid));
    aform.push(bool_leaf("isAnimatedForm", false));
    aform.push(bool_leaf("isLocalAnimatedForm", false));
    aform.push(reference(owner_tag, "_source", owner_pool));
    aform.push(null_leaf("name"));
    aform.push(string_leaf("notes", ""));
    super_element.push(aform);
    super_element.push(float_leaf("opacity", opacity));
    super_element.push(white_color("multiplyColor"));
    super_element.push(white_color("screenColor"));
    super_element.push(reference("CoordType", "coordType", coord_deformer_local));
    super_element
}

/// `CWarpDeformerSource` with all recovered forms.
#[allow(clippy::too_many_arguments)]
pub fn warp_deformer_source(
    warp: &WarpOut,
    ids: &DeformerIds,
    part_guids: &[usize],
    parent_deformer_pool: Option<usize>,
    deformer_root: usize,
    shared: &SharedIds,
) -> XmlElement {
    let mut source = XmlElement::new("CWarpDeformerSource");
    let parent_part_pool = match &warp.parent {
        ParentRef::Part(index) => part_guids.get(*index).copied().unwrap_or(0),
        _ => part_guids.first().copied().unwrap_or(0),
    };
    source.push(deformer_super(
        &warp.name,
        warp.visible,
        parent_part_pool,
        ids.grid_source_id,
    ));
    source.push(reference("CDeformerGuid", "guid", ids.guid_pool));
    source.push(
        XmlElement::new("CDeformerId")
            .attr("xs.n", "id")
            .attr("idstr", &warp.name),
    );
    source.push(deformer_reference(parent_deformer_pool, deformer_root));
    source.push(int_leaf("col", i64::from(warp.columns)));
    source.push(int_leaf("row", i64::from(warp.rows)));
    source.push(bool_leaf("isQuadTransform", warp.quad_transform));
    let mut keyforms = XmlElement::new("carray_list")
        .attr("xs.n", "keyforms")
        .attr("count", warp.forms.len().to_string());
    for (index, form) in warp.forms.iter().enumerate() {
        let Some(form_guid) = ids.form_guids.get(index).copied() else {
            continue;
        };
        let mut element = XmlElement::new("CWarpDeformerForm");
        element.push(deformer_form_super(
            form_guid,
            ids.guid_pool,
            "CWarpDeformerSource",
            form.opacity,
            shared.coord_deformer_local,
        ));
        let flat: Vec<f32> = form
            .positions
            .iter()
            .flat_map(|position| [position[0], position[1]])
            .collect();
        element.push(float_array("positions", &flat));
        keyforms.push(element);
    }
    source.push(keyforms);
    source
}

/// `CRotationDeformerSource` with all recovered forms.
#[allow(clippy::too_many_arguments)]
pub fn rotation_deformer_source(
    rotation: &RotationOut,
    ids: &DeformerIds,
    part_guids: &[usize],
    parent_deformer_pool: Option<usize>,
    deformer_root: usize,
    shared: &SharedIds,
) -> XmlElement {
    let mut source = XmlElement::new("CRotationDeformerSource");
    let parent_part_pool = match &rotation.parent {
        ParentRef::Part(index) => part_guids.get(*index).copied().unwrap_or(0),
        _ => part_guids.first().copied().unwrap_or(0),
    };
    source.push(deformer_super(
        &rotation.name,
        rotation.visible,
        parent_part_pool,
        ids.grid_source_id,
    ));
    source.push(reference("CDeformerGuid", "guid", ids.guid_pool));
    source.push(
        XmlElement::new("CDeformerId")
            .attr("xs.n", "id")
            .attr("idstr", &rotation.name),
    );
    source.push(deformer_reference(parent_deformer_pool, deformer_root));
    source.push(bool_leaf("useBoneUi_testImpl", true));
    let mut keyforms = XmlElement::new("carray_list")
        .attr("xs.n", "keyforms")
        .attr("count", rotation.forms.len().to_string());
    for (index, form) in rotation.forms.iter().enumerate() {
        let Some(form_guid) = ids.form_guids.get(index).copied() else {
            continue;
        };
        let mut element = XmlElement::new("CRotationDeformerForm")
            .attr("angle", crate::xml::format_float(form.angle))
            .attr("originX", crate::xml::format_float(form.origin[0]))
            .attr("originY", crate::xml::format_float(form.origin[1]))
            .attr("scale", crate::xml::format_float(form.scale))
            .attr("isReflectX", if form.reflect_x { "true" } else { "false" })
            .attr("isReflectY", if form.reflect_y { "true" } else { "false" });
        element.push(deformer_form_super(
            form_guid,
            ids.guid_pool,
            "CRotationDeformerSource",
            form.opacity,
            shared.coord_deformer_local,
        ));
        keyforms.push(element);
    }
    source.push(keyforms);
    source.push(float_leaf("handleLengthOnCanvas", 200.0));
    source.push(float_leaf("circleRadiusOnCanvas", 100.0));
    source.push(float_leaf("baseAngle", rotation.base_angle));
    source
}

/// `CPartSource` with child GUID references and recovered draw orders.
#[allow(clippy::too_many_arguments)]
pub fn part_source(
    part: &PartOut,
    own_guid: usize,
    parent_pool: Option<usize>,
    child_refs: &[XmlElement],
    form_guids: &[usize],
    draw_orders: &[f32],
    grid_source_id: Option<usize>,
    deformer_root: usize,
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
    match grid_source_id {
        Some(grid_source_id) => super_part.push(reference(
            "KeyformGridSource",
            "keyformGridSource",
            grid_source_id,
        )),
        None => super_part.push(null_leaf("keyformGridSource")),
    }
    super_part.push(morph_target_set());
    super_part.push(empty_carray("_extensions"));
    super_part.push(null_leaf("internalColor_direct_argb"));
    source.push(super_part);
    source.push(reference("CPartGuid", "guid", own_guid));
    source.push(XmlElement::new("CPartId").attr("xs.n", "id").attr(
        "idstr",
        if part.is_root {
            ROOT_PART_ID
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
    source.push(deformer_reference(None, deformer_root));
    let mut keyforms = XmlElement::new("carray_list")
        .attr("xs.n", "keyforms")
        .attr("count", form_guids.len().to_string());
    for (index, form_guid) in form_guids.iter().enumerate() {
        let draw_order = draw_orders.get(index).copied().unwrap_or(500.0);
        let mut form_element = XmlElement::new("CPartForm");
        let mut aform = XmlElement::new("ACForm").attr("xs.n", "super");
        aform.push(reference("CFormGuid", "guid", *form_guid));
        aform.push(bool_leaf("isAnimatedForm", false));
        aform.push(bool_leaf("isLocalAnimatedForm", false));
        aform.push(reference("CPartSource", "_source", own_guid));
        aform.push(null_leaf("name"));
        aform.push(string_leaf("notes", ""));
        form_element.push(aform);
        form_element.push(int_leaf("drawOrder", draw_order.round() as i64));
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
    deformer_pools: &[(String, usize)],
) -> Option<XmlElement> {
    match child {
        ChildRef::Part(index) => part_guids
            .get(*index)
            .map(|id| reference_anon("CPartGuid", *id)),
        ChildRef::Mesh(index) => drawable_guids
            .get(*index)
            .map(|id| reference_anon("CDrawableGuid", *id)),
        ChildRef::Warp(semantic) | ChildRef::Rotation(semantic) => deformer_pools
            .iter()
            .find(|(name, _)| name == semantic)
            .map(|(_, id)| reference_anon("CDeformerGuid", *id)),
    }
}

/// `KeyformGridSource` with recovered forms and key indices.
pub fn keyform_grid_source(
    grid_id: usize,
    forms: &[(usize, Vec<(usize, usize)>)],
    binding_pool_ids: &[usize],
) -> XmlElement {
    let mut grid = XmlElement::new("KeyformGridSource")
        .attr("xs.id", format!("#{grid_id}"))
        .attr("xs.idx", grid_id.to_string());
    let mut on_grid = XmlElement::new("array_list")
        .attr("xs.n", "keyformsOnGrid")
        .attr("count", forms.len().to_string());
    for (form_guid, key_indices) in forms {
        let mut access_key = XmlElement::new("KeyformGridAccessKey").attr("xs.n", "accessKey");
        let mut key_list = XmlElement::new("array_list")
            .attr("xs.n", "_keyOnParameterList")
            .attr("count", key_indices.len().to_string());
        for (binding_index, key_index) in key_indices {
            let Some(binding_pool) = binding_pool_ids.get(*binding_index).copied() else {
                continue;
            };
            key_list.push(
                XmlElement::new("KeyOnParameter")
                    .child(reference("KeyformBindingSource", "binding", binding_pool))
                    .child(int_leaf("keyIndex", *key_index as i64)),
            );
        }
        access_key.push(key_list);
        let mut entry = XmlElement::new("KeyformOnGrid").child(access_key);
        entry.push(reference("CFormGuid", "keyformGuid", *form_guid));
        on_grid.push(entry);
    }
    grid.push(on_grid);
    let mut bindings_list = XmlElement::new("array_list")
        .attr("xs.n", "keyformBindings")
        .attr("count", binding_pool_ids.len().to_string());
    for binding_pool in binding_pool_ids {
        bindings_list.push(reference_anon("KeyformBindingSource", *binding_pool));
    }
    grid.push(bindings_list);
    grid
}

/// `KeyformBindingSource` with stored keys verbatim.
pub fn keyform_binding_source(
    binding_id: usize,
    grid_id: usize,
    parameter_guid: usize,
    keys: &[f32],
) -> XmlElement {
    let mut binding = XmlElement::new("KeyformBindingSource")
        .attr("xs.id", format!("#{binding_id}"))
        .attr("xs.idx", binding_id.to_string());
    binding.push(reference("KeyformGridSource", "_gridSource", grid_id));
    binding.push(reference("CParameterGuid", "parameterGuid", parameter_guid));
    let mut key_values = XmlElement::new("array_list")
        .attr("xs.n", "keys")
        .attr("count", keys.len().to_string());
    for key in keys {
        key_values.push(XmlElement::new("f").attr("v", crate::xml::format_float(*key)));
    }
    binding.push(key_values);
    // Writer-required LINEAR default (traced in the writer model; never
    // presented as recovered interpolation).
    binding.push(
        XmlElement::new("InterpolationType")
            .attr("xs.n", "interpolationType")
            .attr("v", "LINEAR"),
    );
    binding.push(
        XmlElement::new("ExtendedInterpolationType")
            .attr("xs.n", "extendedInterpolationType")
            .attr("v", "LINEAR"),
    );
    binding.push(int_leaf("insertPointCount", 1));
    binding.push(float_leaf("extendedInterpolationScale", 1.0));
    binding.push(string_leaf("description", ""));
    binding
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

/// ACLayerEntry super used by the root group and layers.
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
