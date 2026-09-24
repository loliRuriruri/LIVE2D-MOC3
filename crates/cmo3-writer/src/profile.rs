//! CMO3 version profile (work order sections 19-20).
//!
//! One target profile only: **Cubism Editor 5.x compatible**, built from the
//! two pinned MIT sources that agree on the document skeleton
//! (Stretchy Studio `cmo3_generate.py`, commit `5fd958def9ed`; moc2cmo
//! `src/decompiler/xml/constants.rs`, commit `2527e24e93`).
//!
//! Everything version-shaped lives here: processing instructions, file
//! format version, editor edition, target model version. Other editor
//! versions are explicitly unsupported/unvalidated (work order section 72).

/// A `<?version Name:N?>` processing instruction pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VersionPi {
    /// Simple class name.
    pub class: &'static str,
    /// Version number.
    pub version: u32,
}

/// Import processing instructions are emitted in this fixed order.
pub const IMPORT_CLASSES: &[&str] = &[
    "com.live2d.cubism.doc.model.ACForm",
    "com.live2d.cubism.doc.model.ACParameterControllableSource",
    "com.live2d.cubism.doc.model.CModelInfo",
    "com.live2d.cubism.doc.model.CModelSource",
    "com.live2d.cubism.doc.model.affecter.ACAffecterForm",
    "com.live2d.cubism.doc.model.affecter.ACAffecterSource",
    "com.live2d.cubism.doc.model.affecter.CAffecterSourceSet",
    "com.live2d.cubism.doc.model.affecter.glue.CGlueForm",
    "com.live2d.cubism.doc.model.affecter.glue.CGlueSource",
    "com.live2d.cubism.doc.model.deformer.ACDeformerForm",
    "com.live2d.cubism.doc.model.deformer.ACDeformerSource",
    "com.live2d.cubism.doc.model.deformer.CDeformerSourceSet",
    "com.live2d.cubism.doc.model.deformer.rotation.CRotationDeformerForm",
    "com.live2d.cubism.doc.model.deformer.rotation.CRotationDeformerSource",
    "com.live2d.cubism.doc.model.deformer.warp.CWarpDeformerForm",
    "com.live2d.cubism.doc.model.deformer.warp.CWarpDeformerSource",
    "com.live2d.cubism.doc.model.drawable.ACDrawableForm",
    "com.live2d.cubism.doc.model.drawable.ACDrawableSource",
    "com.live2d.cubism.doc.model.drawable.CDrawableSourceSet",
    "com.live2d.cubism.doc.model.drawable.ColorComposition",
    "com.live2d.cubism.doc.model.drawable.TextureState",
    "com.live2d.cubism.doc.model.drawable.artMesh.CArtMeshForm",
    "com.live2d.cubism.doc.model.drawable.artMesh.CArtMeshSource",
    "com.live2d.cubism.doc.model.extension.ACExtension",
    "com.live2d.cubism.doc.model.extension.editableMesh.CEditableMeshExtension",
    "com.live2d.cubism.doc.model.extension.meshGenerator.CMeshGeneratorExtension",
    "com.live2d.cubism.doc.model.extension.meshGenerator.MeshGenerateSetting",
    "com.live2d.cubism.doc.model.extension.textureInput.ACTextureInput",
    "com.live2d.cubism.doc.model.extension.textureInput.CTextureInputExtension",
    "com.live2d.cubism.doc.model.extension.textureInput.CTextureInput_ModelImage",
    "com.live2d.cubism.doc.model.extension.textureInput.CTextureInput_TextureAtlasRegion",
    "com.live2d.cubism.doc.model.extension.textureInput.inputFilter.CLayerInputData",
    "com.live2d.cubism.doc.model.extension.textureInput.inputFilter.CLayerSelectorMap",
    "com.live2d.cubism.doc.model.extension.textureInput.inputFilter.ModelImageFilterEnv",
    "com.live2d.cubism.doc.model.extension.textureInput.inputFilter.ModelImageFilterSet",
    "com.live2d.cubism.doc.model.id.CAffecterId",
    "com.live2d.cubism.doc.model.id.CDeformerId",
    "com.live2d.cubism.doc.model.id.CDrawableId",
    "com.live2d.cubism.doc.model.id.CParameterId",
    "com.live2d.cubism.doc.model.id.CPartId",
    "com.live2d.cubism.doc.model.interpolator.InterpolationType",
    "com.live2d.cubism.doc.model.interpolator.KeyOnParameter",
    "com.live2d.cubism.doc.model.interpolator.KeyformBindingSource",
    "com.live2d.cubism.doc.model.interpolator.KeyformGridAccessKey",
    "com.live2d.cubism.doc.model.interpolator.KeyformGridSource",
    "com.live2d.cubism.doc.model.interpolator.KeyformOnGrid",
    "com.live2d.cubism.doc.model.interpolator.extendedInterpolation.ExtendedInterpolationType",
    "com.live2d.cubism.doc.model.morphTarget.KeyFormMorphTargetSet",
    "com.live2d.cubism.doc.model.morphTarget.MorphTargetBlendWeightConstraintSet",
    "com.live2d.cubism.doc.model.options.edition.EditorEdition",
    "com.live2d.cubism.doc.model.param.CParameterSource",
    "com.live2d.cubism.doc.model.param.CParameterSource$Type",
    "com.live2d.cubism.doc.model.param.CParameterSourceSet",
    "com.live2d.cubism.doc.model.param.group.CParameterGroup",
    "com.live2d.cubism.doc.model.param.group.CParameterGroupSet",
    "com.live2d.cubism.doc.model.parts.CPartForm",
    "com.live2d.cubism.doc.model.parts.CPartSource",
    "com.live2d.cubism.doc.model.parts.CPartSourceSet",
    "com.live2d.cubism.doc.model.texture.CTextureManager",
    "com.live2d.cubism.doc.model.texture.textureAtlas.CTextureAtlas",
    "com.live2d.cubism.doc.model.texture.textureAtlas.CTextureAtlas$ModelImageEntry",
    "com.live2d.cubism.doc.model.texture.LayeredImageWrapper",
    "com.live2d.cubism.doc.model.texture.TextureImageGroup",
    "com.live2d.cubism.doc.model.texture.modelImage.CModelImage",
    "com.live2d.cubism.doc.model.texture.modelImage.CModelImageGroup",
    "com.live2d.cubism.doc.resources.ACImageLayer",
    "com.live2d.cubism.doc.resources.ACLayerEntry",
    "com.live2d.cubism.doc.resources.ACLayerGroup",
    "com.live2d.cubism.doc.resources.CLayer",
    "com.live2d.cubism.doc.resources.CLayerGroup",
    "com.live2d.cubism.doc.resources.CLayerIdentifier",
    "com.live2d.cubism.doc.resources.CLayeredImage",
    "com.live2d.cubism.doc.resources.LayerSet",
    "com.live2d.doc.CoordType",
    "com.live2d.graphics.CImageCanvas",
    "com.live2d.graphics.CImageResource",
    "com.live2d.graphics.cachedImage.CCachedImage",
    "com.live2d.graphics.cachedImage.CCachedImageManager",
    "com.live2d.graphics.cachedImage.CachedImageType",
    "com.live2d.graphics.filter.AValueConnector",
    "com.live2d.graphics.filter.FilterEnv",
    "com.live2d.graphics.filter.FilterEnv$EnvValueSet",
    "com.live2d.graphics.filter.FilterSet",
    "com.live2d.graphics.filter.FilterSet$EnvConnection",
    "com.live2d.graphics.filter.FilterValue",
    "com.live2d.graphics.filter.concreteConnector.EnvValueConnector",
    "com.live2d.graphics.filter.concreteConnector.FilterOutputValueConnector",
    "com.live2d.graphics.filter.filterInstance.FilterInstance",
    "com.live2d.graphics.filter.id.FilterInstanceId",
    "com.live2d.graphics.filter.id.FilterValueId",
    "com.live2d.graphics.psd.blend.ACBlend",
    "com.live2d.graphics.psd.blend.CBlend_Normal",
    "com.live2d.graphics3d.editableMesh.GEditableMesh2",
    "com.live2d.graphics3d.component.GTransform2",
    "com.live2d.graphics3d.texture.Anisotropy",
    "com.live2d.graphics3d.texture.GTexture",
    "com.live2d.graphics3d.texture.GTexture$FilterMode",
    "com.live2d.graphics3d.texture.GTexture2D",
    "com.live2d.graphics3d.texture.MagFilter",
    "com.live2d.graphics3d.texture.MinFilter",
    "com.live2d.graphics3d.texture.WrapMode",
    "com.live2d.graphics3d.type.GVector2",
    "com.live2d.type.CAffine",
    "com.live2d.type.CAffecterGuid",
    "com.live2d.type.CColor",
    "com.live2d.type.CDeformerGuid",
    "com.live2d.type.CDrawableGuid",
    "com.live2d.type.CExtensionGuid",
    "com.live2d.type.CFloatColor",
    "com.live2d.type.CFormGuid",
    "com.live2d.type.CLayerGuid",
    "com.live2d.type.CLayeredImageGuid",
    "com.live2d.type.CModelGuid",
    "com.live2d.type.CModelImageGuid",
    "com.live2d.type.CParameterGroupGuid",
    "com.live2d.type.CParameterGuid",
    "com.live2d.type.CPartGuid",
    "com.live2d.type.CPoint",
    "com.live2d.type.CRect",
    "com.live2d.type.CSize",
    "com.live2d.type.CTextureAtlasGuid",
    "com.live2d.type.GEditableMeshGuid",
    "com.live2d.type.GTextureGuid",
    "com.live2d.type.StaticFilterDefGuid",
];

/// The pinned target profile: Cubism Editor 5.x compatible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cmo3VersionProfile {
    /// Human-readable profile id.
    pub id: &'static str,
    /// `<root fileFormatVersion>`.
    pub file_format_version: u32,
    /// `EditorEdition.edition`.
    pub editor_edition: i32,
    /// `targetVersionNo`.
    pub target_version_no: i32,
    /// `latestVersionOfLastModelerNo`.
    pub latest_modeler_no: i32,
    /// Version processing instructions.
    pub version_pis: &'static [VersionPi],
}

/// The single supported profile.
pub const EDITOR_5_PROFILE: Cmo3VersionProfile = Cmo3VersionProfile {
    id: "cubism-editor-5x-compatible",
    file_format_version: 402_030_000,
    editor_edition: 15,
    target_version_no: 3000,
    latest_modeler_no: 5_000_000,
    // Union of both pinned writers (moc2cmo adds the rotation form PI that
    // Stretchy's generator does not emit).
    version_pis: &[
        VersionPi {
            class: "CArtMeshSource",
            version: 4,
        },
        VersionPi {
            class: "CRotationDeformerForm",
            version: 1,
        },
        VersionPi {
            class: "KeyformGridSource",
            version: 1,
        },
        VersionPi {
            class: "CParameterGroup",
            version: 4,
        },
        VersionPi {
            class: "SerializeFormatVersion",
            version: 2,
        },
        VersionPi {
            class: "CModelSource",
            version: 4,
        },
        VersionPi {
            class: "CFloatColor",
            version: 1,
        },
        VersionPi {
            class: "CLabelColor",
            version: 0,
        },
        VersionPi {
            class: "CModelImage",
            version: 3,
        },
    ],
};

/// Fixed well-known UUIDs shared by both pinned sources.
pub mod fixed_guids {
    /// Root deformer GUID (Editor compares by UUID equality).
    pub const DEFORMER_ROOT: &str = "71fae776-e218-4aee-873e-78e8ac0cb48a";
    /// Root parameter group GUID.
    pub const PARAMETER_GROUP_ROOT: &str = "e9fe6eff-953b-4ce2-be7c-4a7c3913686b";
    /// Layer-selector filter definition GUID (image pipeline, deferred).
    pub const FILTER_DEF_LAYER_SELECTOR: &str = "5e9fe1ea-0ec3-4d68-a5fa-018fc7abe301";
    /// Layer-filter mapping definition GUID (image pipeline, deferred).
    pub const FILTER_DEF_LAYER_FILTER: &str = "4083cd1f-40ba-4eda-8400-379019d55ed8";
}

/// Root part id string evidence is conflicting (`PartRoot` Stretchy generator
/// vs `__RootPart__` doc/moc2cmo); we emit the moc2cmo/doc value.
pub const ROOT_PART_ID: &str = "__RootPart__";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_values_match_both_pinned_sources() {
        let profile = EDITOR_5_PROFILE;
        assert_eq!(profile.file_format_version, 402_030_000);
        assert_eq!(profile.editor_edition, 15);
        assert_eq!(profile.target_version_no, 3000);
        assert_eq!(profile.latest_modeler_no, 5_000_000);
        assert_eq!(profile.version_pis.len(), 9);
        assert!(profile
            .version_pis
            .iter()
            .any(|pi| pi.class == "CModelSource" && pi.version == 4));
    }

    #[test]
    fn import_class_list_is_unique_and_counted() {
        let mut sorted = IMPORT_CLASSES.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), IMPORT_CLASSES.len(), "duplicate import");
        // Count pinned by docs/CMO3_VERSION_PROFILE.md.
        assert_eq!(IMPORT_CLASSES.len(), 124);
    }
}
