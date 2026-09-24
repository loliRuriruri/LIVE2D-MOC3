//! Minimal MODEL_IMAGE-mode CMO3 document serialization (AGENT.5.1).
//!
//! The orchestrator allocates every pool id up front and passes explicit ids
//! into the builders (`builders.rs`). Structure follows the pinned evidence
//! in `docs/CMO3_IMAGE_PIPELINE_EVIDENCE.md`; writer-required constants are
//! recorded in `docs/CMO3_WRITER_DEFAULTS.md`.
//!
//! **Precondition:** the project must pass [`crate::validate::validate_typed`]
//! (and the result must be rescanned by [`crate::validate::scan_xml`]) before
//! the bytes are used. [`crate::write_minimal_cmo3`] enforces this; calling
//! `serialize` directly on an unvalidated project can produce dangling refs
//! (for example a missing layered image or an empty part list).

mod builders;

use crate::ids::{GuidAllocator, GuidMode, ObjectPool};
use crate::model::{ChildRef, Cmo3Project, ParentRef};
use crate::profile::{fixed_guids, EDITOR_5_PROFILE, IMPORT_CLASSES, ROOT_PART_ID};
use crate::xml::{bool_leaf, float_leaf, int_leaf, string_leaf, XmlElement};

use builders::{
    art_mesh_source, child_guid, editable_mesh_extension, empty_carray, empty_hash_map,
    env_connection, env_connector_entry, filter_output_connector, gtexture2d, image_resource,
    layer_entry_super, layered_image_wrapper, mesh_generator_extension, model_image,
    model_image_env, null_leaf, part_source, reference, reference_anon, texture_input_extension,
    texture_input_model_image, MeshIds, SharedIds,
};

/// Serialized document plus allocation facts.
#[derive(Debug, Clone)]
pub struct SerializedCmo3 {
    /// XML text with declaration and processing instructions.
    pub xml: String,
    /// Object pool trace `(xs.id, element, semantic)`.
    pub pool_trace: Vec<(usize, String, Option<String>)>,
    /// GUID atoms allocated.
    pub guid_count: usize,
}

/// Identity options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IdentityOptions {
    /// Deterministic GUIDs (default; required for reproducible output).
    pub deterministic_guids: bool,
    /// CAFF obfuscation key.
    pub archive_key: i32,
}

impl Default for IdentityOptions {
    fn default() -> Self {
        Self {
            deterministic_guids: true,
            archive_key: crate::caff::DEFAULT_KEY,
        }
    }
}

/// Fixed `FilterValueId` string ids in both sources' order.
pub const FILTER_VALUE_ID_NAMES: [&str; 8] = [
    "ilf_outputLayerData",
    "mi_input_layerInputData",
    "ilf_inputLayerData",
    "mi_currentImageGuid",
    "ilf_currentImageGuid",
    "mi_output_image",
    "mi_output_transform",
    "ilf_inputLayer",
];

const INLINE_OUTPUT_IMAGE_RES: &str = "ilf_outputImageRes";
const INLINE_OUTPUT_TRANSFORM: &str = "ilf_outputTransform";

/// Filter value display names (index-aligned; inline ids at 6 and 8).
pub const FILTER_VALUE_NAMES: [&str; 9] = [
    "Select Layer",
    "Import Layer",
    "Import Layer selection",
    "Current GUID",
    "GUID of Selected Source Image",
    "Output image",
    "Output Image (Resource Format)",
    "LayerToCanvas transform",
    "LayerToCanvas transform",
];

/// `FilterValue` -> shared `FilterValueId` index (`None` = inline id).
/// Pinned pairing: value 7 uses `mi_output_transform`; values 6 and 8 carry
/// the inline `ilf_outputImageRes`/`ilf_outputTransform` ids.
const FILTER_VALUE_ID_INDEX: [Option<usize>; 9] = [
    Some(0), // ilf_outputLayerData
    Some(1), // mi_input_layerInputData
    Some(2), // ilf_inputLayerData
    Some(3), // mi_currentImageGuid
    Some(4), // ilf_currentImageGuid
    Some(5), // mi_output_image
    None,    // ilf_outputImageRes (inline)
    Some(6), // mi_output_transform
    None,    // ilf_outputTransform (inline)
];

/// Serialize the project into `main.xml`.
pub fn serialize(project: &Cmo3Project, options: &IdentityOptions) -> SerializedCmo3 {
    let mode = if options.deterministic_guids {
        GuidMode::Deterministic
    } else {
        GuidMode::Random
    };
    let mut guids = GuidAllocator::new(mode, "live2d-recovery/cmo3");
    let mut pool = ObjectPool::new();
    let mut shared = XmlElement::new("shared");

    // ---- fixed GUID atoms -------------------------------------------------
    let model_guid = alloc_guid(&mut pool, &mut guids, "CModelGuid", "model", &mut shared);
    let root_group_guid = alloc_fixed_guid(
        &mut pool,
        "CParameterGroupGuid",
        fixed_guids::PARAMETER_GROUP_ROOT,
        "Parameter root",
        &mut shared,
    );
    let deformer_root_guid = alloc_fixed_guid(
        &mut pool,
        "CDeformerGuid",
        fixed_guids::DEFORMER_ROOT,
        "deformer root",
        &mut shared,
    );
    let selector_def = alloc_fixed_guid(
        &mut pool,
        "StaticFilterDefGuid",
        fixed_guids::FILTER_DEF_LAYER_SELECTOR,
        "CLayerSelector",
        &mut shared,
    );
    let layer_filter_def = alloc_fixed_guid(
        &mut pool,
        "StaticFilterDefGuid",
        fixed_guids::FILTER_DEF_LAYER_FILTER,
        "CLayerFilter",
        &mut shared,
    );
    let parameter_guid = alloc_guid(
        &mut pool,
        &mut guids,
        "CParameterGuid",
        "parameter:minimal",
        &mut shared,
    );
    let blend_id = pool.allocate("CBlend_Normal", None);
    shared.push(
        XmlElement::new("CBlend_Normal")
            .attr("xs.id", format!("#{blend_id}"))
            .attr("xs.idx", blend_id.to_string())
            .child(
                XmlElement::new("ACBlend")
                    .attr("xs.n", "super")
                    .child(string_leaf("displayName", "Normal")),
            ),
    );
    let coord_deformer_local = pool.allocate("CoordType", None);
    shared.push(
        XmlElement::new("CoordType")
            .attr("xs.id", format!("#{coord_deformer_local}"))
            .attr("xs.idx", coord_deformer_local.to_string())
            .child(string_leaf("coordName", "DeformerLocal")),
    );
    let coord_canvas = pool.allocate("CoordType", None);
    shared.push(
        XmlElement::new("CoordType")
            .attr("xs.id", format!("#{coord_canvas}"))
            .attr("xs.idx", coord_canvas.to_string())
            .child(string_leaf("coordName", "Canvas")),
    );
    let _ = coord_canvas;
    let shared_ids = SharedIds {
        coord_deformer_local,
    };

    // ---- filter value ids and values --------------------------------------
    let mut filter_value_ids: Vec<usize> = Vec::new();
    for name in FILTER_VALUE_ID_NAMES {
        let id = pool.allocate("FilterValueId", Some(name));
        shared.push(
            XmlElement::new("FilterValueId")
                .attr("xs.id", format!("#{id}"))
                .attr("xs.idx", id.to_string())
                .attr("idstr", name),
        );
        filter_value_ids.push(id);
    }
    let id_of = |name: &str| -> usize {
        FILTER_VALUE_ID_NAMES
            .iter()
            .position(|candidate| *candidate == name)
            .and_then(|index| filter_value_ids.get(index).copied())
            .unwrap_or(0)
    };
    let mut filter_values: Vec<usize> = Vec::new();
    for (index, name) in FILTER_VALUE_NAMES.iter().enumerate() {
        let id = pool.allocate("FilterValue", Some(name));
        let mut element = XmlElement::new("FilterValue")
            .attr("xs.id", format!("#{id}"))
            .attr("xs.idx", id.to_string());
        element.push(string_leaf("name", name));
        match FILTER_VALUE_ID_INDEX[index] {
            Some(id_index) => element.push(reference(
                "FilterValueId",
                "id",
                id_of(FILTER_VALUE_ID_NAMES[id_index]),
            )),
            None => {
                let inline = if index == 6 {
                    INLINE_OUTPUT_IMAGE_RES
                } else {
                    INLINE_OUTPUT_TRANSFORM
                };
                element.push(
                    XmlElement::new("FilterValueId")
                        .attr("xs.n", "id")
                        .attr("idstr", inline),
                );
            }
        }
        element.push(null_leaf("defaultValueInitializer"));
        filter_values.push(id);
        shared.push(element);
    }
    let value_of = |index: usize| -> usize { filter_values.get(index).copied().unwrap_or(0) };

    // ---- per-object guids and ids -----------------------------------------
    let part_guids: Vec<usize> = project
        .parts
        .iter()
        .map(|part| {
            alloc_guid(
                &mut pool,
                &mut guids,
                "CPartGuid",
                &part.semantic,
                &mut shared,
            )
        })
        .collect();
    let drawable_guids: Vec<usize> = project
        .meshes
        .iter()
        .map(|mesh| {
            alloc_guid(
                &mut pool,
                &mut guids,
                "CDrawableGuid",
                &mesh.semantic,
                &mut shared,
            )
        })
        .collect();
    let mut mesh_ids: Vec<MeshIds> = Vec::with_capacity(project.meshes.len());
    for (index, mesh) in project.meshes.iter().enumerate() {
        let editable_mesh_guid = alloc_guid(
            &mut pool,
            &mut guids,
            "GEditableMeshGuid",
            &format!("editable:{}", mesh.semantic),
            &mut shared,
        );
        let extension_guid_editable = alloc_guid(
            &mut pool,
            &mut guids,
            "CExtensionGuid",
            &format!("ext.editable:{}", mesh.semantic),
            &mut shared,
        );
        let extension_guid_texture_input = alloc_guid(
            &mut pool,
            &mut guids,
            "CExtensionGuid",
            &format!("ext.texture_input:{}", mesh.semantic),
            &mut shared,
        );
        let extension_guid_mesh_generator = alloc_guid(
            &mut pool,
            &mut guids,
            "CExtensionGuid",
            &format!("ext.mesh_generator:{}", mesh.semantic),
            &mut shared,
        );
        let texture_input_ext_id = pool.allocate(
            "CTextureInputExtension",
            Some(&format!("texture_input_ext:{}", mesh.semantic)),
        );
        let texture_input_id = mesh.model_image.map(|_| {
            pool.allocate(
                "CTextureInput_ModelImage",
                Some(&format!("texture_input:{}", mesh.semantic)),
            )
        });
        let grid_source_id = pool.allocate("KeyformGridSource", Some(&mesh.semantic));
        let binding_id = pool.allocate("KeyformBindingSource", Some(&mesh.semantic));
        let form_guid = alloc_guid(
            &mut pool,
            &mut guids,
            "CFormGuid",
            &format!("{}#static", mesh.semantic),
            &mut shared,
        );
        let _ = index;
        mesh_ids.push(MeshIds {
            drawable_pool: drawable_guids[index],
            editable_mesh_guid,
            extension_guid_editable,
            extension_guid_texture_input,
            extension_guid_mesh_generator,
            texture_input_ext_id,
            texture_input_id,
            grid_source_id,
            binding_id,
            form_guid,
            model_image_guid: None,
            texture_pool: None,
        });
    }
    let root_form_guid = alloc_guid(&mut pool, &mut guids, "CFormGuid", "form:root", &mut shared);
    let part_form_guids: Vec<Vec<usize>> = project
        .parts
        .iter()
        .map(|part| {
            part.draw_orders
                .iter()
                .enumerate()
                .map(|(index, _)| {
                    alloc_guid(
                        &mut pool,
                        &mut guids,
                        "CFormGuid",
                        &format!("{}#form{}", part.semantic, index),
                        &mut shared,
                    )
                })
                .collect()
        })
        .collect();
    let resource_guids: Vec<usize> = project
        .image_resources
        .iter()
        .map(|resource| {
            alloc_guid(
                &mut pool,
                &mut guids,
                "GTextureGuid",
                &resource.semantic,
                &mut shared,
            )
        })
        .collect();
    let layered_guid = alloc_guid(
        &mut pool,
        &mut guids,
        "CLayeredImageGuid",
        "layered_image:000000",
        &mut shared,
    );
    let model_image_guids: Vec<usize> = project
        .model_images
        .iter()
        .map(|model_image| {
            alloc_guid(
                &mut pool,
                &mut guids,
                "CModelImageGuid",
                &model_image.semantic,
                &mut shared,
            )
        })
        .collect();
    let image_resource_ids: Vec<usize> = project
        .image_resources
        .iter()
        .map(|resource| {
            let id = pool.allocate("CImageResource", Some(&resource.semantic));
            shared.push(image_resource(
                id,
                resource.width,
                resource.height,
                resource.byte_len,
                &resource.archive_name,
            ));
            id
        })
        .collect();
    let texture_ids: Vec<usize> = project
        .image_resources
        .iter()
        .map(|resource| pool.allocate("GTexture2D", Some(&resource.semantic)))
        .collect();
    for (index, _resource) in project.image_resources.iter().enumerate() {
        shared.push(gtexture2d(
            index,
            texture_ids[index],
            resource_guids[index],
            image_resource_ids[index],
        ));
    }

    // Attach per-mesh texture/model-image ids now that resources exist.
    for (index, mesh) in project.meshes.iter().enumerate() {
        if let Some(model_image_index) = mesh.model_image {
            if let Some(ids) = mesh_ids.get_mut(index) {
                ids.model_image_guid = model_image_guids.get(model_image_index).copied();
            }
        }
        if let Some(resource_index) = mesh.texture {
            if let Some(ids) = mesh_ids.get_mut(index) {
                ids.texture_pool = texture_ids.get(resource_index).copied();
            }
        }
    }

    let group_id = pool.allocate("CModelImageGroup", None);
    let layered_image_id = if project.layered_images.is_empty() {
        None
    } else {
        Some(pool.allocate("CLayeredImage", Some("layered_image:000000")))
    };
    let layer_pool_ids: Vec<usize> = project
        .layered_images
        .first()
        .map(|layered| {
            (0..layered.layers.len())
                .map(|index| pool.allocate("CLayer", Some(&format!("layer:{index:06}"))))
                .collect()
        })
        .unwrap_or_default();
    let root_group_id = if layered_image_id.is_some() {
        Some(pool.allocate("CLayerGroup", Some("layer-group:root")))
    } else {
        None
    };

    // ---- per-mesh filter objects -----------------------------------------
    let mesh_count = project.meshes.len();
    let mut filter_set_ids: Vec<usize> = Vec::with_capacity(mesh_count);
    let mut selector_instance_ids: Vec<usize> = Vec::with_capacity(mesh_count);
    let mut layer_instance_ids: Vec<usize> = Vec::with_capacity(mesh_count);
    let mut instance_id_objects: Vec<[usize; 2]> = Vec::with_capacity(mesh_count);
    let mut output_connector_ids: Vec<usize> = Vec::with_capacity(mesh_count);
    for (index, mesh) in project.meshes.iter().enumerate() {
        filter_set_ids.push(pool.allocate("ModelImageFilterSet", Some(&mesh.semantic)));
        selector_instance_ids.push(pool.allocate(
            "FilterInstance",
            Some(&format!("selector:{}", mesh.semantic)),
        ));
        layer_instance_ids.push(pool.allocate(
            "FilterInstance",
            Some(&format!("layer_filter:{}", mesh.semantic)),
        ));
        instance_id_objects.push([
            pool.allocate("FilterInstanceId", Some(&format!("filter0_{index}"))),
            pool.allocate("FilterInstanceId", Some(&format!("filter1_{index}"))),
        ]);
        output_connector_ids.push(pool.allocate(
            "FilterOutputValueConnector",
            Some(&format!("output:{}", mesh.semantic)),
        ));
    }

    // ---- shared: per-mesh extensions and texture inputs ------------------
    for (index, mesh) in project.meshes.iter().enumerate() {
        let ids = mesh_ids[index];
        shared.push(editable_mesh_extension(mesh, &ids, &shared_ids));
        if let Some(texture_input_id) = ids.texture_input_id {
            shared.push(texture_input_extension(&ids, texture_input_id));
            if let Some(model_image_guid) = ids.model_image_guid {
                shared.push(texture_input_model_image(
                    &ids,
                    texture_input_id,
                    model_image_guid,
                ));
            }
        }
        shared.push(mesh_generator_extension(&ids));
    }

    // ---- shared: filter instances, ids, connectors, sets ------------------
    for index in 0..mesh_count {
        let filter_set = filter_set_ids[index];
        let selector = selector_instance_ids[index];
        let layer_filter = layer_instance_ids[index];
        let [selector_id_object, layer_id_object] = instance_id_objects[index];
        let connector = output_connector_ids[index];

        let mut selector_instance = XmlElement::new("FilterInstance")
            .attr("xs.id", format!("#{selector}"))
            .attr("xs.idx", selector.to_string())
            .attr("filterName", "CLayerSelector");
        selector_instance.push(reference(
            "StaticFilterDefGuid",
            "filterDefGuid",
            selector_def,
        ));
        selector_instance.push(null_leaf("filterDef"));
        selector_instance.push(reference(
            "FilterInstanceId",
            "filterId",
            selector_id_object,
        ));
        let mut inputs = XmlElement::new("hash_map")
            .attr("xs.n", "inputConnectors")
            .attr("count", "2");
        inputs.push(env_connector_entry(
            id_of("ilf_inputLayerData"),
            id_of("mi_input_layerInputData"),
        ));
        inputs.push(env_connector_entry(
            id_of("ilf_currentImageGuid"),
            id_of("mi_currentImageGuid"),
        ));
        selector_instance.push(inputs);
        let mut outputs = XmlElement::new("hash_map")
            .attr("xs.n", "outputConnectors")
            .attr("count", "1");
        outputs.push(
            XmlElement::new("entry")
                .child(reference(
                    "FilterValueId",
                    "key",
                    id_of("ilf_outputLayerData"),
                ))
                .child(reference("FilterOutputValueConnector", "value", connector)),
        );
        selector_instance.push(outputs);
        selector_instance.push(reference(
            "ModelImageFilterSet",
            "ownerFilterSet",
            filter_set,
        ));
        shared.push(selector_instance);

        let mut layer_instance = XmlElement::new("FilterInstance")
            .attr("xs.id", format!("#{layer_filter}"))
            .attr("xs.idx", layer_filter.to_string())
            .attr("filterName", "CLayerFilter");
        layer_instance.push(reference(
            "StaticFilterDefGuid",
            "filterDefGuid",
            layer_filter_def,
        ));
        layer_instance.push(null_leaf("filterDef"));
        layer_instance.push(reference("FilterInstanceId", "filterId", layer_id_object));
        let mut layer_inputs = XmlElement::new("hash_map")
            .attr("xs.n", "inputConnectors")
            .attr("count", "1");
        layer_inputs.push(
            XmlElement::new("entry")
                .child(reference("FilterValueId", "key", id_of("ilf_inputLayer")))
                .child(reference("FilterOutputValueConnector", "value", connector)),
        );
        layer_instance.push(layer_inputs);
        layer_instance.push(empty_hash_map("outputConnectors"));
        layer_instance.push(reference(
            "ModelImageFilterSet",
            "ownerFilterSet",
            filter_set,
        ));
        shared.push(layer_instance);

        for (object, idstr) in [
            (selector_id_object, format!("filter0_{index}")),
            (layer_id_object, format!("filter1_{index}")),
        ] {
            shared.push(
                XmlElement::new("FilterInstanceId")
                    .attr("xs.id", format!("#{object}"))
                    .attr("xs.idx", object.to_string())
                    .attr("idstr", idstr),
            );
        }

        shared.push(filter_output_connector(
            connector,
            selector,
            id_of("ilf_outputLayerData"),
            value_of(0),
        ));

        let mut set = XmlElement::new("ModelImageFilterSet")
            .attr("xs.id", format!("#{filter_set}"))
            .attr("xs.idx", filter_set.to_string());
        let mut super_set = XmlElement::new("FilterSet").attr("xs.n", "super");
        let mut filter_map = XmlElement::new("linked_map")
            .attr("xs.n", "filterMap")
            .attr("count", "2");
        for (id_object, instance) in [
            (selector_id_object, selector),
            (layer_id_object, layer_filter),
        ] {
            filter_map.push(
                XmlElement::new("entry")
                    .child(reference("FilterInstanceId", "key", id_object))
                    .child(reference("FilterInstance", "value", instance)),
            );
        }
        super_set.push(filter_map);
        let mut external_inputs = XmlElement::new("linked_map")
            .attr("xs.n", "_externalInputs")
            .attr("count", "2");
        external_inputs.push(env_connection(
            id_of("mi_input_layerInputData"),
            selector,
            value_of(1),
            value_of(2),
        ));
        external_inputs.push(env_connection(
            id_of("mi_currentImageGuid"),
            selector,
            value_of(3),
            value_of(4),
        ));
        super_set.push(external_inputs);
        let mut external_outputs = XmlElement::new("linked_map")
            .attr("xs.n", "_externalOutputs")
            .attr("count", "2");
        external_outputs.push(env_connection(
            id_of("mi_output_image"),
            layer_filter,
            value_of(5),
            value_of(6),
        ));
        external_outputs.push(env_connection(
            id_of("mi_output_transform"),
            layer_filter,
            value_of(7),
            value_of(8),
        ));
        super_set.push(external_outputs);
        set.push(super_set);
        shared.push(set);
    }

    // ---- shared: layered image chain --------------------------------------
    if let (Some(layered_image_id), Some(root_group_id), Some(layered)) = (
        layered_image_id,
        root_group_id,
        project.layered_images.first(),
    ) {
        let root_layer_uuid = guids.allocate("CLayerGuid", "layer-group-root");
        let layer_uuids: Vec<String> = (0..layered.layers.len())
            .map(|index| guids.allocate("CLayerGuid", &format!("layer-{index}")))
            .collect();

        let mut root_group = XmlElement::new("CLayerGroup")
            .attr("xs.id", format!("#{root_group_id}"))
            .attr("xs.idx", root_group_id.to_string());
        let mut root_super = XmlElement::new("ACLayerGroup").attr("xs.n", "super");
        root_super.push(layer_entry_super(
            "root",
            "root",
            &root_layer_uuid,
            blend_id,
            layered_image_id,
            None,
        ));
        let mut children = XmlElement::new("carray_list")
            .attr("xs.n", "_children")
            .attr("count", layer_pool_ids.len().to_string());
        for layer_id in &layer_pool_ids {
            children.push(reference_anon("CLayer", *layer_id));
        }
        root_super.push(children);
        root_group.push(root_super);
        // Both pinned sources place `layerIdentifier` directly under
        // `CLayerGroup` (outside the `ACLayerGroup` super element).
        root_group.push(null_leaf("layerIdentifier"));
        shared.push(root_group);

        for (index, layer) in layered.layers.iter().enumerate() {
            let layer_id = layer_pool_ids[index];
            let mut element = XmlElement::new("CLayer")
                .attr("xs.id", format!("#{layer_id}"))
                .attr("xs.idx", layer_id.to_string());
            let mut super_layer = XmlElement::new("ACImageLayer").attr("xs.n", "super");
            super_layer.push(layer_entry_super(
                &layer.name,
                &format!("layer-{index}"),
                &layer_uuids[index],
                blend_id,
                layered_image_id,
                Some(root_group_id),
            ));
            element.push(super_layer);
            element.push(reference(
                "CImageResource",
                "imageResource",
                image_resource_ids[layer.resource],
            ));
            element.push(
                XmlElement::new("CRect")
                    .attr("xs.n", "boundsOnImageDoc")
                    .child(int_leaf("x", 0))
                    .child(int_leaf("y", 0))
                    .child(int_leaf("width", i64::from(layer.width)))
                    .child(int_leaf("height", i64::from(layer.height))),
            );
            element.push(
                XmlElement::new("CLayerIdentifier")
                    .attr("xs.n", "layerIdentifier")
                    .child(string_leaf("layerName", &layer.name))
                    .child(string_leaf("layerId", &layer.layer_id))
                    .child(int_leaf(
                        "layerIdValue_testImpl",
                        i64::from(layer.layer_id_value),
                    )),
            );
            element.push(null_leaf("icon16"));
            element.push(null_leaf("icon64"));
            element.push(
                XmlElement::new("linked_map")
                    .attr("xs.n", "layerInfo")
                    .attr("count", "0")
                    .attr("keyType", "string"),
            );
            element.push(empty_hash_map("_optionOfIOption"));
            shared.push(element);
        }

        let mut element = XmlElement::new("CLayeredImage")
            .attr("xs.id", format!("#{layered_image_id}"))
            .attr("xs.idx", layered_image_id.to_string());
        element.push(string_leaf("name", &layered.psd_name));
        element.push(string_leaf("memo", ""));
        element.push(int_leaf("width", i64::from(layered.width)));
        element.push(int_leaf("height", i64::from(layered.height)));
        element.push(
            XmlElement::new("file")
                .attr("xs.n", "psdFile")
                .attr("v", &layered.psd_name),
        );
        element.push(string_leaf("description", ""));
        element.push(reference("CLayeredImageGuid", "guid", layered_guid));
        element.push(null_leaf("psdBytes"));
        element.push(
            XmlElement::new("l")
                .attr("xs.n", "psdFileLastModified")
                .attr("v", "0"),
        );
        element.push(reference("CLayerGroup", "_rootLayer", root_group_id));
        let mut layer_set = XmlElement::new("LayerSet").attr("xs.n", "layerSet");
        layer_set.push(reference(
            "CLayeredImage",
            "_layeredImage",
            layered_image_id,
        ));
        let mut entries = XmlElement::new("carray_list")
            .attr("xs.n", "_layerEntryList")
            .attr("count", (layer_pool_ids.len() + 1).to_string());
        entries.push(reference_anon("CLayerGroup", root_group_id));
        for layer_id in &layer_pool_ids {
            entries.push(reference_anon("CLayer", *layer_id));
        }
        layer_set.push(entries);
        element.push(layer_set);
        element.push(null_leaf("icon16"));
        element.push(null_leaf("icon64"));
        shared.push(element);

        let mut group = XmlElement::new("CModelImageGroup")
            .attr("xs.id", format!("#{group_id}"))
            .attr("xs.idx", group_id.to_string());
        group.push(string_leaf("memo", ""));
        group.push(string_leaf("groupName", "RecoveredLayeredImage"));
        let mut linked = XmlElement::new("carray_list")
            .attr("xs.n", "_linkedRawImageGuids")
            .attr("count", "1");
        linked.push(reference_anon("CLayeredImageGuid", layered_guid));
        group.push(linked);
        let mut images = XmlElement::new("carray_list").attr("xs.n", "_modelImages");
        for (model_index, model_image_out) in project.model_images.iter().enumerate() {
            let layer_id = layer_pool_ids
                .get(model_image_out.layer)
                .copied()
                .unwrap_or_else(|| layer_pool_ids.first().copied().unwrap_or(0));
            let resource_id = image_resource_ids
                .get(model_image_out.resource)
                .copied()
                .unwrap_or(0);
            let (width, height) = project
                .image_resources
                .get(model_image_out.resource)
                .map(|resource| (resource.width, resource.height))
                .unwrap_or((layered.width, layered.height));
            let env = model_image_env(
                layered_guid,
                layer_id,
                id_of("mi_currentImageGuid"),
                id_of("mi_input_layerInputData"),
            );
            // `validate_typed` guarantees one model image per textured mesh,
            // so the lookups below always resolve.
            let Some(model_image_guid) = model_image_guids.get(model_index).copied() else {
                continue;
            };
            let Some(filter_set_id) = filter_set_ids.get(model_index).copied() else {
                continue;
            };
            images.push(model_image(
                model_image_guid,
                &model_image_out.name,
                filter_set_id,
                env,
                resource_id,
                group_id,
                layered_guid,
                width,
                height,
            ));
        }
        // Count derives from the entries actually emitted (no desync).
        images
            .attributes
            .push(("count".to_string(), images.children.len().to_string()));
        group.push(images);
        shared.push(group);
    }

    // ---- shared: keyform grid and binding (one static form per mesh) ------
    for ids in &mesh_ids {
        let mut grid = XmlElement::new("KeyformGridSource")
            .attr("xs.id", format!("#{}", ids.grid_source_id))
            .attr("xs.idx", ids.grid_source_id.to_string());
        let mut on_grid = XmlElement::new("array_list")
            .attr("xs.n", "keyformsOnGrid")
            .attr("count", "1");
        let mut access_key = XmlElement::new("KeyformGridAccessKey").attr("xs.n", "accessKey");
        let mut key_list = XmlElement::new("array_list")
            .attr("xs.n", "_keyOnParameterList")
            .attr("count", "1");
        key_list.push(
            XmlElement::new("KeyOnParameter")
                .child(reference("KeyformBindingSource", "binding", ids.binding_id))
                .child(int_leaf("keyIndex", 0)),
        );
        access_key.push(key_list);
        let mut entry = XmlElement::new("KeyformOnGrid").child(access_key);
        entry.push(reference("CFormGuid", "keyformGuid", ids.form_guid));
        on_grid.push(entry);
        grid.push(on_grid);
        let mut bindings_list = XmlElement::new("array_list")
            .attr("xs.n", "keyformBindings")
            .attr("count", "1");
        bindings_list.push(reference_anon("KeyformBindingSource", ids.binding_id));
        grid.push(bindings_list);
        shared.push(grid);

        let mut binding = XmlElement::new("KeyformBindingSource")
            .attr("xs.id", format!("#{}", ids.binding_id))
            .attr("xs.idx", ids.binding_id.to_string());
        binding.push(reference(
            "KeyformGridSource",
            "_gridSource",
            ids.grid_source_id,
        ));
        binding.push(reference("CParameterGuid", "parameterGuid", parameter_guid));
        let mut keys = XmlElement::new("array_list")
            .attr("xs.n", "keys")
            .attr("count", "1");
        keys.push(XmlElement::new("f").attr("v", "0.0"));
        binding.push(keys);
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
        shared.push(binding);
    }

    // ---- main tree --------------------------------------------------------
    let mut main = XmlElement::new("main");
    let mut model = XmlElement::new("CModelSource").attr("isDefaultKeyformLocked", "false");
    model.push(reference("CModelGuid", "guid", model_guid));
    model.push(string_leaf("name", &project.model_name));
    model.push(
        XmlElement::new("EditorEdition")
            .attr("xs.n", "editorEdition")
            .child(int_leaf(
                "edition",
                i64::from(EDITOR_5_PROFILE.editor_edition),
            )),
    );
    model.push(
        XmlElement::new("CImageCanvas")
            .attr("xs.n", "canvas")
            .child(int_leaf("pixelWidth", i64::from(project.canvas.width)))
            .child(int_leaf("pixelHeight", i64::from(project.canvas.height)))
            .child(XmlElement::new("CColor").attr("xs.n", "background")),
    );
    let mut parameter_set =
        XmlElement::new("CParameterSourceSet").attr("xs.n", "parameterSourceSet");
    let mut sources = XmlElement::new("carray_list")
        .attr("xs.n", "_sources")
        .attr("count", "1");
    let mut source = XmlElement::new("CParameterSource");
    source.push(int_leaf("decimalPlaces", 3));
    source.push(reference("CParameterGuid", "guid", parameter_guid));
    source.push(float_leaf("snapEpsilon", 0.001));
    source.push(float_leaf("minValue", 0.0));
    source.push(float_leaf("maxValue", 1.0));
    source.push(float_leaf("defaultValue", 0.0));
    source.push(bool_leaf("isRepeat", false));
    source.push(
        XmlElement::new("CParameterId")
            .attr("xs.n", "id")
            .attr("idstr", "Param_Minimal"),
    );
    source.push(
        XmlElement::new("Type")
            .attr("xs.n", "paramType")
            .attr("v", "NORMAL"),
    );
    source.push(string_leaf("name", "Param_Minimal"));
    source.push(string_leaf("description", ""));
    source.push(bool_leaf("combined", false));
    source.push(reference(
        "CParameterGroupGuid",
        "parentGroupGuid",
        root_group_guid,
    ));
    sources.push(source);
    parameter_set.push(sources);
    model.push(parameter_set);

    let mut texture_manager = XmlElement::new("CTextureManager").attr("xs.n", "textureManager");
    texture_manager.push(
        XmlElement::new("TextureImageGroup")
            .attr("xs.n", "textureList")
            .child(empty_carray("children")),
    );
    let raw_images = match layered_image_id {
        Some(id) => XmlElement::new("carray_list")
            .attr("xs.n", "_rawImages")
            .attr("count", "1")
            .child(layered_image_wrapper(id)),
        None => empty_carray("_rawImages"),
    };
    texture_manager.push(raw_images);
    texture_manager.push(
        XmlElement::new("carray_list")
            .attr("xs.n", "_modelImageGroups")
            .attr("count", "1")
            .child(reference_anon("CModelImageGroup", group_id)),
    );
    texture_manager.push(empty_carray("_textureAtlases"));
    texture_manager.push(bool_leaf("isTextureInputModelImageMode", true));
    texture_manager.push(int_leaf("previewReductionRatio", 1));
    texture_manager.push(empty_carray("artPathBrushUsingLayeredImageIds"));
    model.push(texture_manager);
    model.push(bool_leaf("useLegacyDrawOrder__testImpl", false));

    let mut drawable_set = XmlElement::new("CDrawableSourceSet").attr("xs.n", "drawableSourceSet");
    let mut drawable_sources = XmlElement::new("carray_list")
        .attr("xs.n", "_sources")
        .attr("count", project.meshes.len().to_string());
    for (index, mesh) in project.meshes.iter().enumerate() {
        drawable_sources.push(art_mesh_source(
            mesh,
            &mesh_ids[index],
            &part_guids,
            deformer_root_guid,
            &shared_ids,
        ));
    }
    drawable_set.push(drawable_sources);
    model.push(drawable_set);

    model.push(
        XmlElement::new("CDeformerSourceSet")
            .attr("xs.n", "deformerSourceSet")
            .child(empty_carray("_sources")),
    );
    model.push(
        XmlElement::new("CAffecterSourceSet")
            .attr("xs.n", "affecterSourceSet")
            .child(empty_carray("_sources")),
    );

    let mut part_set = XmlElement::new("CPartSourceSet").attr("xs.n", "partSourceSet");
    let mut part_sources = XmlElement::new("carray_list")
        .attr("xs.n", "_sources")
        .attr("count", project.parts.len().to_string());
    for (index, part) in project.parts.iter().enumerate() {
        let parent_pool = match &part.parent {
            ParentRef::Part(parent_index) if !part.is_root => {
                part_guids.get(*parent_index).copied()
            }
            _ => None,
        };
        let mut child_refs: Vec<XmlElement> = Vec::new();
        for child in &part.children {
            if let ChildRef::Part(child_index) = child {
                if *child_index == index {
                    continue;
                }
            }
            if let Some(element) = child_guid(child, &part_guids, &drawable_guids) {
                child_refs.push(element);
            }
        }
        let form_guids: Vec<usize> = if part.is_root {
            vec![root_form_guid]
        } else {
            part_form_guids.get(index).cloned().unwrap_or_default()
        };
        part_sources.push(part_source(
            part,
            part_guids[index],
            parent_pool,
            &child_refs,
            &form_guids,
            deformer_root_guid,
            ROOT_PART_ID,
        ));
    }
    part_set.push(part_sources);
    model.push(part_set);
    model.push(reference(
        "CPartSource",
        "rootPart",
        part_guids.first().copied().unwrap_or(0),
    ));
    model.push(
        XmlElement::new("CParameterGroupSet")
            .attr("xs.n", "parameterGroupSet")
            .child(empty_carray("_groups")),
    );
    let mut model_info = XmlElement::new("CModelInfo").attr("xs.n", "modelInfo");
    model_info.push(float_leaf("pixelsPerUnit", project.canvas.pixels_per_unit));
    model_info.push(
        XmlElement::new("CPoint")
            .attr("xs.n", "originInPixels")
            .child(int_leaf("x", i64::from(project.canvas.origin_x)))
            .child(int_leaf("y", i64::from(project.canvas.origin_y))),
    );
    model.push(model_info);
    model.push(int_leaf(
        "targetVersionNo",
        i64::from(EDITOR_5_PROFILE.target_version_no),
    ));
    model.push(int_leaf(
        "latestVersionOfLastModelerNo",
        i64::from(EDITOR_5_PROFILE.latest_modeler_no),
    ));
    main.push(model);

    // ---- document ---------------------------------------------------------
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    for pi in EDITOR_5_PROFILE.version_pis {
        out.push_str(&format!("<?version {}:{}?>\n", pi.class, pi.version));
    }
    for class in IMPORT_CLASSES {
        out.push_str(&format!("<?import {class}?>\n"));
    }
    let mut root = XmlElement::new("root").attr(
        "fileFormatVersion",
        EDITOR_5_PROFILE.file_format_version.to_string(),
    );
    root.push(shared);
    root.push(main);
    out.push_str(&root.render_body());
    let pool_trace = pool
        .trace()
        .iter()
        .map(|entry| (entry.id, entry.kind.clone(), entry.semantic.clone()))
        .collect();
    let guid_count = pool
        .trace()
        .iter()
        .filter(|entry| entry.kind.ends_with("Guid"))
        .count();
    SerializedCmo3 {
        xml: out,
        pool_trace,
        guid_count,
    }
}

fn alloc_guid(
    pool: &mut ObjectPool,
    guids: &mut GuidAllocator,
    kind: &str,
    semantic: &str,
    shared: &mut XmlElement,
) -> usize {
    let id = pool.allocate(kind, Some(semantic));
    let uuid = guids.allocate(kind, semantic);
    shared.push(
        XmlElement::new(kind)
            .attr("xs.id", format!("#{id}"))
            .attr("xs.idx", id.to_string())
            .attr("uuid", uuid)
            .attr("note", semantic),
    );
    id
}

fn alloc_fixed_guid(
    pool: &mut ObjectPool,
    kind: &str,
    uuid: &str,
    note: &str,
    shared: &mut XmlElement,
) -> usize {
    let id = pool.allocate(kind, Some(note));
    shared.push(
        XmlElement::new(kind)
            .attr("xs.id", format!("#{id}"))
            .attr("xs.idx", id.to_string())
            .attr("uuid", uuid)
            .attr("note", note),
    );
    id
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn filter_atom_lists_are_consistent() {
        assert_eq!(FILTER_VALUE_ID_NAMES.len(), 8);
        assert_eq!(FILTER_VALUE_NAMES.len(), 9);
    }

    #[test]
    fn filter_value_id_pairing_matches_pinned_sources() {
        assert_eq!(FILTER_VALUE_ID_INDEX[0], Some(0));
        assert_eq!(FILTER_VALUE_ID_INDEX[5], Some(5));
        assert_eq!(FILTER_VALUE_ID_INDEX[6], None); // inline ilf_outputImageRes
        assert_eq!(FILTER_VALUE_ID_INDEX[7], Some(6)); // mi_output_transform
        assert_eq!(FILTER_VALUE_ID_INDEX[8], None); // inline ilf_outputTransform
        assert_eq!(FILTER_VALUE_ID_NAMES[6], "mi_output_transform");
        assert_eq!(FILTER_VALUE_ID_NAMES[7], "ilf_inputLayer");
    }
}
