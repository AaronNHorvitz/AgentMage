//! Bounded image inspection, decoded-pixel redaction, visual comparison, and generation admission.

use std::collections::BTreeSet;

use agentmage_kernel_contracts::{CONTRACT_SCHEMA_VERSION, WorkspacePath};
use serde::{Deserialize, Serialize};

use crate::word_generation::valid_identifier;
use crate::word_ooxml::word_sha256;

const MAX_DIMENSION: u32 = 8_192;
const MAX_PIXEL_BYTES: usize = 64 * 1_024 * 1_024;
const BMP_HEADER_BYTES: usize = 54;
const PARTS_PER_MILLION: u64 = 1_000_000;

/// Closed image-container identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageFormat {
    /// Uncompressed 32-bit Windows bitmap, admitted for full decoding and regeneration.
    BmpRgba32,
    /// Portable Network Graphics, admitted for bounded metadata inspection only.
    Png,
}

/// Closed decoded color-space identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageColorSpace {
    /// Eight-bit red, green, blue, and alpha channels.
    SrgbRgba8,
    /// Eight-bit red, green, and blue channels declared by a PNG header.
    SrgbRgb8,
    /// Eight-bit grayscale declared by a PNG header.
    Gray8,
    /// A syntactically valid PNG color declaration that this adapter does not decode.
    UnsupportedPng,
}

/// Content-minimized source provenance supplied by the caller.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImageProvenance {
    /// Stable source identity.
    pub source_id: String,
    /// SHA-256 of the exact source bytes.
    pub source_sha256: String,
    /// Optional owning presentation slide number.
    pub slide_number: Option<u32>,
    /// Optional owning presentation object identity.
    pub object_id: Option<u32>,
}

/// Bounded image metadata and capability disposition.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImageInspection {
    /// Kernel contract schema version.
    pub schema_version: u16,
    /// Exact source provenance.
    pub provenance: ImageProvenance,
    /// Recognized image format.
    pub format: ImageFormat,
    /// Pixel width.
    pub width: u32,
    /// Pixel height.
    pub height: u32,
    /// Declared or decoded color space.
    pub color_space: ImageColorSpace,
    /// Exact source byte size.
    pub source_bytes: u64,
    /// Bytes outside the decoded pixel payload or required container structure.
    pub ancillary_metadata_bytes: u64,
    /// True only when exact RGBA pixels were decoded and hash-checked.
    pub decoded_pixels_available: bool,
    /// True only when the source can enter model context without a further decoder.
    pub safe_for_model_context: bool,
    /// Unsupported-feature reason codes in canonical order.
    pub unsupported_features: Vec<String>,
    /// False because inspection receives caller-supplied bytes.
    pub filesystem_effect_performed: bool,
    /// False because inspection never resolves remote content.
    pub network_access_performed: bool,
    /// False because inspection launches no viewer or embedded content.
    pub execution_performed: bool,
}

/// Exact row-major decoded RGBA8 image.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecodedRgbaImage {
    /// Pixel width.
    pub width: u32,
    /// Pixel height.
    pub height: u32,
    /// Exact row-major RGBA8 bytes.
    pub rgba: Vec<u8>,
    /// SHA-256 of `rgba`.
    pub rgba_sha256: String,
}

/// Half-open sensitive or edit rectangle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImageRegion {
    /// Inclusive left coordinate.
    pub x: u32,
    /// Inclusive top coordinate.
    pub y: u32,
    /// Positive width.
    pub width: u32,
    /// Positive height.
    pub height: u32,
}

/// A bounded, content-addressed local view proposal.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImageViewProposal {
    /// Stable proposal identity.
    pub proposal_id: String,
    /// Approved vision-capable profile identity.
    pub profile_id: String,
    /// Exact source digest.
    pub source_sha256: String,
    /// Exact decoded-pixel digest.
    pub rgba_sha256: String,
    /// True only when the profile is explicitly vision-capable.
    pub vision_capable: bool,
    /// True only when every declared sensitive region was redacted first.
    pub sensitive_regions_cleared: bool,
    /// Always true: the caller must separately authorize context insertion.
    pub proposal_only: bool,
    /// False because this function displays and persists nothing.
    pub filesystem_effect_performed: bool,
    /// False because no provider is contacted.
    pub network_access_performed: bool,
    /// False because no native viewer is launched.
    pub execution_performed: bool,
}

/// Full-regeneration pixel redaction receipt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImageRedactionReceipt {
    /// Kernel contract schema version.
    pub schema_version: u16,
    /// Stable redaction identity.
    pub redaction_id: String,
    /// Exact source pixel digest.
    pub source_rgba_sha256: String,
    /// Exact output pixel digest.
    pub output_rgba_sha256: String,
    /// Exact regenerated BMP digest.
    pub output_bmp_sha256: String,
    /// Canonically ordered redacted regions.
    pub regions: Vec<ImageRegion>,
    /// True only after every decoded pixel in every region is verified opaque black.
    pub decoded_pixel_scan_passed: bool,
    /// True only when regeneration retained no ancillary source bytes.
    pub metadata_removed: bool,
    /// True because native visual review remains separately required.
    pub human_visual_review_required: bool,
    /// False because the output is an in-memory proposal.
    pub filesystem_effect_performed: bool,
    /// False because redaction uses no network.
    pub network_access_performed: bool,
    /// False because redaction launches no renderer.
    pub execution_performed: bool,
}

/// Regenerated redacted image proposal.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RedactedImage {
    /// Decoded redacted pixels.
    pub image: DecodedRgbaImage,
    /// Deterministic 32-bit BMP bytes.
    pub bmp: Vec<u8>,
    /// Verifiable redaction receipt.
    pub receipt: ImageRedactionReceipt,
}

/// Exact persistence proposal for a previously verified redacted image.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImageExportProposal {
    /// Stable export identity.
    pub export_id: String,
    /// Canonical workspace-relative proposed output path.
    pub output_path: WorkspacePath,
    /// Exact redaction receipt identity.
    pub redaction_id: String,
    /// Exact exported BMP bytes.
    pub bmp: Vec<u8>,
    /// SHA-256 of `bmp`.
    pub bmp_sha256: String,
    /// True only when the decoded-pixel scan passed.
    pub decoded_pixel_scan_passed: bool,
    /// True only when source ancillary metadata was removed.
    pub metadata_removed: bool,
    /// Always true; persistence requires a separate write approval.
    pub proposal_only: bool,
    /// False because this function writes nothing.
    pub filesystem_effect_performed: bool,
    /// False because export uses no network.
    pub network_access_performed: bool,
    /// False because export launches no viewer or content.
    pub execution_performed: bool,
}

/// Artifact class associated with a visual comparison.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VisualArtifactKind {
    /// Document page render.
    Document,
    /// Presentation slide render.
    Slide,
    /// Standalone raster image.
    Image,
    /// User-interface screenshot.
    UserInterface,
}

/// Exact deterministic before/after comparison.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImageVisualComparison {
    /// Kernel contract schema version.
    pub schema_version: u16,
    /// Stable comparison identity.
    pub comparison_id: String,
    /// Compared artifact class.
    pub artifact_kind: VisualArtifactKind,
    /// Exact before pixel digest.
    pub before_rgba_sha256: String,
    /// Exact after pixel digest.
    pub after_rgba_sha256: String,
    /// Number of pixels with any changed channel.
    pub changed_pixels: u64,
    /// Changed-pixel ratio in integer parts per million.
    pub changed_pixel_ratio_ppm: u32,
    /// Maximum absolute channel delta.
    pub maximum_channel_delta: u8,
    /// Tight changed rectangle, absent for identical images.
    pub difference_bounds: Option<ImageRegion>,
    /// True only when dimensions and declared thresholds pass.
    pub machine_checks_passed: bool,
    /// Always true until separately retained native visual/accessibility review exists.
    pub human_visual_review_required: bool,
    /// False because comparison is pure.
    pub filesystem_effect_performed: bool,
    /// False because comparison uses no network.
    pub network_access_performed: bool,
    /// False because comparison launches no content.
    pub execution_performed: bool,
}

/// Closed generation route.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageGenerationRoute {
    /// Approved local image profile.
    Local,
    /// Explicitly approved provider-backed image profile.
    Provider,
}

/// Exact disclosure preview for an optional image generation/edit operation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImageGenerationPreview {
    /// Stable operation identity.
    pub operation_id: String,
    /// Exact approved profile identity.
    pub profile_id: String,
    /// Selected route.
    pub route: ImageGenerationRoute,
    /// Whether strict-local policy is active.
    pub strict_local: bool,
    /// Hash of the complete prompt or edit instruction; raw text is not retained.
    pub instruction_sha256: String,
    /// Optional exact source image digest for editing.
    pub source_image_sha256: Option<String>,
    /// Content-minimized disclosure codes in canonical order.
    pub disclosures: Vec<String>,
    /// Hash of the entire preview excluding this field.
    pub preview_sha256: String,
    /// Always true; generation is not performed by this contract.
    pub approval_required: bool,
    /// Always true; this function only proposes a route.
    pub proposal_only: bool,
}

/// Content-minimized completion receipt bound to an exact approved preview.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImageGenerationReceipt {
    /// Stable operation identity.
    pub operation_id: String,
    /// Exact approved preview digest.
    pub approved_preview_sha256: String,
    /// Exact produced image digest.
    pub output_image_sha256: String,
    /// Selected route.
    pub route: ImageGenerationRoute,
    /// True only when provider use was explicitly disclosed and approved.
    pub provider_use_approved: bool,
    /// False: this boundary validates a supplied result and performs no provider call.
    pub network_access_performed: bool,
    /// False: persistence remains separately authorized.
    pub filesystem_effect_performed: bool,
}

/// Stable fail-closed image-workflow error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageWorkflowError {
    /// An identity, digest, source, profile, region, or threshold is invalid.
    InvalidInput,
    /// The image type or exact feature is not admitted.
    UnsupportedFormat,
    /// A dimension or decoded-byte ceiling was exceeded.
    ResourceLimit,
    /// Sensitive pixels would cross a model/export boundary without verified redaction.
    SensitiveContent,
    /// Strict-local policy or disclosure approval forbids the selected route.
    RouteDenied,
    /// A source or approval binding is stale.
    BindingMismatch,
}

impl ImageWorkflowError {
    /// Stable content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "image.input.invalid",
            Self::UnsupportedFormat => "image.format.unsupported",
            Self::ResourceLimit => "image.resource.limit",
            Self::SensitiveContent => "image.sensitive.redaction-required",
            Self::RouteDenied => "image.generation.route-denied",
            Self::BindingMismatch => "image.binding.mismatch",
        }
    }
}

impl std::fmt::Display for ImageWorkflowError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for ImageWorkflowError {}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn pixel_len(width: u32, height: u32) -> Result<usize, ImageWorkflowError> {
    if width == 0 || height == 0 || width > MAX_DIMENSION || height > MAX_DIMENSION {
        return Err(ImageWorkflowError::ResourceLimit);
    }
    let length = usize::try_from(width)
        .ok()
        .and_then(|value| {
            usize::try_from(height)
                .ok()
                .and_then(|height| value.checked_mul(height))
        })
        .and_then(|value| value.checked_mul(4))
        .ok_or(ImageWorkflowError::ResourceLimit)?;
    if length > MAX_PIXEL_BYTES {
        return Err(ImageWorkflowError::ResourceLimit);
    }
    Ok(length)
}

fn validate_image(image: &DecodedRgbaImage) -> Result<(), ImageWorkflowError> {
    if image.rgba.len() != pixel_len(image.width, image.height)?
        || !valid_sha256(&image.rgba_sha256)
        || word_sha256(&image.rgba) != image.rgba_sha256
    {
        return Err(ImageWorkflowError::InvalidInput);
    }
    Ok(())
}

fn validate_provenance(
    provenance: &ImageProvenance,
    source: &[u8],
) -> Result<(), ImageWorkflowError> {
    if !valid_identifier(&provenance.source_id)
        || !valid_sha256(&provenance.source_sha256)
        || word_sha256(source) != provenance.source_sha256
        || provenance.slide_number.is_some_and(|value| value == 0)
        || provenance.object_id.is_some_and(|value| value == 0)
    {
        return Err(ImageWorkflowError::BindingMismatch);
    }
    Ok(())
}

fn le_u32(bytes: &[u8], offset: usize) -> Result<u32, ImageWorkflowError> {
    let value = bytes
        .get(offset..offset + 4)
        .ok_or(ImageWorkflowError::InvalidInput)?;
    Ok(u32::from_le_bytes([value[0], value[1], value[2], value[3]]))
}

fn be_u32(bytes: &[u8], offset: usize) -> Result<u32, ImageWorkflowError> {
    let value = bytes
        .get(offset..offset + 4)
        .ok_or(ImageWorkflowError::InvalidInput)?;
    Ok(u32::from_be_bytes([value[0], value[1], value[2], value[3]]))
}

/// Decodes the admitted uncompressed 32-bit BMP subset into canonical top-down RGBA8.
pub fn decode_bmp_rgba(source: &[u8]) -> Result<DecodedRgbaImage, ImageWorkflowError> {
    if source.len() < BMP_HEADER_BYTES || source.get(..2) != Some(b"BM") || le_u32(source, 14)? < 40
    {
        return Err(ImageWorkflowError::UnsupportedFormat);
    }
    let offset =
        usize::try_from(le_u32(source, 10)?).map_err(|_| ImageWorkflowError::ResourceLimit)?;
    let width = le_u32(source, 18)?;
    let raw_height = i32::from_le_bytes(
        source[22..26]
            .try_into()
            .map_err(|_| ImageWorkflowError::InvalidInput)?,
    );
    let height = raw_height.unsigned_abs();
    let planes = u16::from_le_bytes(
        source[26..28]
            .try_into()
            .map_err(|_| ImageWorkflowError::InvalidInput)?,
    );
    let bits = u16::from_le_bytes(
        source[28..30]
            .try_into()
            .map_err(|_| ImageWorkflowError::InvalidInput)?,
    );
    if width == 0 || raw_height == 0 || planes != 1 || bits != 32 || le_u32(source, 30)? != 0 {
        return Err(ImageWorkflowError::UnsupportedFormat);
    }
    let expected = pixel_len(width, height)?;
    let pixels = source
        .get(
            offset
                ..offset
                    .checked_add(expected)
                    .ok_or(ImageWorkflowError::ResourceLimit)?,
        )
        .ok_or(ImageWorkflowError::InvalidInput)?;
    let mut rgba = vec![0_u8; expected];
    let row_bytes = usize::try_from(width).map_err(|_| ImageWorkflowError::ResourceLimit)? * 4;
    for output_y in 0..usize::try_from(height).map_err(|_| ImageWorkflowError::ResourceLimit)? {
        let source_y = if raw_height > 0 {
            usize::try_from(height).unwrap_or(0) - 1 - output_y
        } else {
            output_y
        };
        for x in 0..usize::try_from(width).map_err(|_| ImageWorkflowError::ResourceLimit)? {
            let input = source_y * row_bytes + x * 4;
            let output = output_y * row_bytes + x * 4;
            rgba[output] = pixels[input + 2];
            rgba[output + 1] = pixels[input + 1];
            rgba[output + 2] = pixels[input];
            rgba[output + 3] = pixels[input + 3];
        }
    }
    Ok(DecodedRgbaImage {
        width,
        height,
        rgba_sha256: word_sha256(&rgba),
        rgba,
    })
}

/// Deterministically encodes canonical top-down RGBA8 as uncompressed 32-bit BMP.
pub fn encode_bmp_rgba(image: &DecodedRgbaImage) -> Result<Vec<u8>, ImageWorkflowError> {
    validate_image(image)?;
    let total = BMP_HEADER_BYTES
        .checked_add(image.rgba.len())
        .ok_or(ImageWorkflowError::ResourceLimit)?;
    let total_u32 = u32::try_from(total).map_err(|_| ImageWorkflowError::ResourceLimit)?;
    let mut output = vec![0_u8; total];
    output[..2].copy_from_slice(b"BM");
    output[2..6].copy_from_slice(&total_u32.to_le_bytes());
    output[10..14].copy_from_slice(&(BMP_HEADER_BYTES as u32).to_le_bytes());
    output[14..18].copy_from_slice(&40_u32.to_le_bytes());
    output[18..22].copy_from_slice(&image.width.to_le_bytes());
    output[22..26].copy_from_slice(&image.height.to_le_bytes());
    output[26..28].copy_from_slice(&1_u16.to_le_bytes());
    output[28..30].copy_from_slice(&32_u16.to_le_bytes());
    output[34..38].copy_from_slice(
        &u32::try_from(image.rgba.len())
            .map_err(|_| ImageWorkflowError::ResourceLimit)?
            .to_le_bytes(),
    );
    let row_bytes =
        usize::try_from(image.width).map_err(|_| ImageWorkflowError::ResourceLimit)? * 4;
    for source_y in
        0..usize::try_from(image.height).map_err(|_| ImageWorkflowError::ResourceLimit)?
    {
        let output_y = usize::try_from(image.height).unwrap_or(0) - 1 - source_y;
        for x in 0..usize::try_from(image.width).map_err(|_| ImageWorkflowError::ResourceLimit)? {
            let input = source_y * row_bytes + x * 4;
            let target = BMP_HEADER_BYTES + output_y * row_bytes + x * 4;
            output[target] = image.rgba[input + 2];
            output[target + 1] = image.rgba[input + 1];
            output[target + 2] = image.rgba[input];
            output[target + 3] = image.rgba[input + 3];
        }
    }
    Ok(output)
}

/// Inspects admitted image metadata without file, network, viewer, or content-execution effects.
pub fn inspect_image(
    source: &[u8],
    provenance: ImageProvenance,
) -> Result<ImageInspection, ImageWorkflowError> {
    validate_provenance(&provenance, source)?;
    let source_bytes =
        u64::try_from(source.len()).map_err(|_| ImageWorkflowError::ResourceLimit)?;
    if source.get(..2) == Some(b"BM") {
        let image = decode_bmp_rgba(source)?;
        let canonical_size = BMP_HEADER_BYTES + image.rgba.len();
        return Ok(ImageInspection {
            schema_version: CONTRACT_SCHEMA_VERSION,
            provenance,
            format: ImageFormat::BmpRgba32,
            width: image.width,
            height: image.height,
            color_space: ImageColorSpace::SrgbRgba8,
            source_bytes,
            ancillary_metadata_bytes: u64::try_from(source.len().saturating_sub(canonical_size))
                .unwrap_or(u64::MAX),
            decoded_pixels_available: true,
            safe_for_model_context: true,
            unsupported_features: Vec::new(),
            filesystem_effect_performed: false,
            network_access_performed: false,
            execution_performed: false,
        });
    }
    const PNG: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
    if source.get(..8) == Some(PNG)
        && source.get(12..16) == Some(b"IHDR")
        && be_u32(source, 8)? == 13
    {
        let width = be_u32(source, 16)?;
        let height = be_u32(source, 20)?;
        pixel_len(width, height)?;
        if source.get(24).copied() != Some(8) {
            return Err(ImageWorkflowError::UnsupportedFormat);
        }
        let color_space = match source.get(25).copied() {
            Some(0) => ImageColorSpace::Gray8,
            Some(2) => ImageColorSpace::SrgbRgb8,
            Some(6) => ImageColorSpace::SrgbRgba8,
            _ => ImageColorSpace::UnsupportedPng,
        };
        return Ok(ImageInspection {
            schema_version: CONTRACT_SCHEMA_VERSION,
            provenance,
            format: ImageFormat::Png,
            width,
            height,
            color_space,
            source_bytes,
            ancillary_metadata_bytes: source_bytes.saturating_sub(33),
            decoded_pixels_available: false,
            safe_for_model_context: false,
            unsupported_features: vec!["png.pixel-decoder.not-admitted".to_owned()],
            filesystem_effect_performed: false,
            network_access_performed: false,
            execution_performed: false,
        });
    }
    Err(ImageWorkflowError::UnsupportedFormat)
}

fn validate_regions(
    image: &DecodedRgbaImage,
    regions: &[ImageRegion],
) -> Result<(), ImageWorkflowError> {
    if regions.is_empty()
        || regions.len() > 256
        || regions.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return Err(ImageWorkflowError::InvalidInput);
    }
    for region in regions {
        if region.width == 0
            || region.height == 0
            || region
                .x
                .checked_add(region.width)
                .is_none_or(|right| right > image.width)
            || region
                .y
                .checked_add(region.height)
                .is_none_or(|bottom| bottom > image.height)
        {
            return Err(ImageWorkflowError::InvalidInput);
        }
    }
    Ok(())
}

/// Produces a local view proposal only for an approved vision profile and cleared sensitive regions.
pub fn prepare_image_view(
    proposal_id: &str,
    profile_id: &str,
    vision_capable: bool,
    source_sha256: &str,
    image: &DecodedRgbaImage,
    sensitive_regions: &[ImageRegion],
    verified_redaction_receipt: Option<&ImageRedactionReceipt>,
) -> Result<ImageViewProposal, ImageWorkflowError> {
    validate_image(image)?;
    if !valid_identifier(proposal_id)
        || !valid_identifier(profile_id)
        || !valid_sha256(source_sha256)
        || !vision_capable
    {
        return Err(ImageWorkflowError::InvalidInput);
    }
    let cleared = if sensitive_regions.is_empty() {
        true
    } else {
        validate_regions(image, sensitive_regions)?;
        verified_redaction_receipt.is_some_and(|receipt| {
            receipt.output_rgba_sha256 == image.rgba_sha256
                && receipt.regions == sensitive_regions
                && receipt.decoded_pixel_scan_passed
                && receipt.metadata_removed
        })
    };
    if !cleared {
        return Err(ImageWorkflowError::SensitiveContent);
    }
    Ok(ImageViewProposal {
        proposal_id: proposal_id.to_owned(),
        profile_id: profile_id.to_owned(),
        source_sha256: source_sha256.to_owned(),
        rgba_sha256: image.rgba_sha256.clone(),
        vision_capable,
        sensitive_regions_cleared: true,
        proposal_only: true,
        filesystem_effect_performed: false,
        network_access_performed: false,
        execution_performed: false,
    })
}

/// Regenerates a BMP after replacing exact decoded regions with opaque black pixels.
pub fn redact_image(
    redaction_id: &str,
    source: &DecodedRgbaImage,
    regions: Vec<ImageRegion>,
) -> Result<RedactedImage, ImageWorkflowError> {
    validate_image(source)?;
    if !valid_identifier(redaction_id) {
        return Err(ImageWorkflowError::InvalidInput);
    }
    validate_regions(source, &regions)?;
    let mut rgba = source.rgba.clone();
    let width = usize::try_from(source.width).map_err(|_| ImageWorkflowError::ResourceLimit)?;
    for region in &regions {
        for y in region.y..region.y + region.height {
            for x in region.x..region.x + region.width {
                let index =
                    (usize::try_from(y).unwrap_or(0) * width + usize::try_from(x).unwrap_or(0)) * 4;
                rgba[index..index + 4].copy_from_slice(&[0, 0, 0, 255]);
            }
        }
    }
    let image = DecodedRgbaImage {
        width: source.width,
        height: source.height,
        rgba_sha256: word_sha256(&rgba),
        rgba,
    };
    let bmp = encode_bmp_rgba(&image)?;
    let reopened = decode_bmp_rgba(&bmp)?;
    if reopened != image {
        return Err(ImageWorkflowError::BindingMismatch);
    }
    let receipt = ImageRedactionReceipt {
        schema_version: CONTRACT_SCHEMA_VERSION,
        redaction_id: redaction_id.to_owned(),
        source_rgba_sha256: source.rgba_sha256.clone(),
        output_rgba_sha256: image.rgba_sha256.clone(),
        output_bmp_sha256: word_sha256(&bmp),
        regions,
        decoded_pixel_scan_passed: true,
        metadata_removed: bmp.len() == BMP_HEADER_BYTES + image.rgba.len(),
        human_visual_review_required: true,
        filesystem_effect_performed: false,
        network_access_performed: false,
        execution_performed: false,
    };
    Ok(RedactedImage {
        image,
        bmp,
        receipt,
    })
}

/// Produces an exact export proposal only from the currently verified redacted pixels and bytes.
pub fn prepare_redacted_image_export(
    export_id: &str,
    output_path: WorkspacePath,
    redacted: &RedactedImage,
) -> Result<ImageExportProposal, ImageWorkflowError> {
    if !valid_identifier(export_id)
        || !output_path
            .components()
            .last()
            .is_some_and(|name| name.as_str().ends_with(".bmp") && name.as_str().len() > 4)
        || redacted.receipt.output_rgba_sha256 != redacted.image.rgba_sha256
        || redacted.receipt.output_bmp_sha256 != word_sha256(&redacted.bmp)
        || !redacted.receipt.decoded_pixel_scan_passed
        || !redacted.receipt.metadata_removed
        || decode_bmp_rgba(&redacted.bmp)? != redacted.image
    {
        return Err(ImageWorkflowError::BindingMismatch);
    }
    Ok(ImageExportProposal {
        export_id: export_id.to_owned(),
        output_path,
        redaction_id: redacted.receipt.redaction_id.clone(),
        bmp: redacted.bmp.clone(),
        bmp_sha256: redacted.receipt.output_bmp_sha256.clone(),
        decoded_pixel_scan_passed: true,
        metadata_removed: true,
        proposal_only: true,
        filesystem_effect_performed: false,
        network_access_performed: false,
        execution_performed: false,
    })
}

/// Compares exact decoded pixels for documents, slides, images, or user interfaces.
pub fn compare_images(
    comparison_id: &str,
    artifact_kind: VisualArtifactKind,
    before: &DecodedRgbaImage,
    after: &DecodedRgbaImage,
    max_changed_pixel_ratio_ppm: u32,
    max_channel_delta: u8,
) -> Result<ImageVisualComparison, ImageWorkflowError> {
    if !valid_identifier(comparison_id) || max_changed_pixel_ratio_ppm > PARTS_PER_MILLION as u32 {
        return Err(ImageWorkflowError::InvalidInput);
    }
    validate_image(before)?;
    validate_image(after)?;
    if before.width != after.width || before.height != after.height {
        return Err(ImageWorkflowError::BindingMismatch);
    }
    let mut changed = 0_u64;
    let mut maximum = 0_u8;
    let mut coordinates = Vec::new();
    for (index, (left, right)) in before
        .rgba
        .chunks_exact(4)
        .zip(after.rgba.chunks_exact(4))
        .enumerate()
    {
        let delta = (0..4)
            .map(|channel| left[channel].abs_diff(right[channel]))
            .max()
            .unwrap_or(0);
        maximum = maximum.max(delta);
        if delta > 0 {
            changed += 1;
            coordinates.push((
                u32::try_from(index % before.width as usize).unwrap_or(u32::MAX),
                u32::try_from(index / before.width as usize).unwrap_or(u32::MAX),
            ));
        }
    }
    let ratio = u32::try_from(
        (u128::from(changed) * u128::from(PARTS_PER_MILLION))
            / u128::from(u64::from(before.width) * u64::from(before.height)),
    )
    .unwrap_or(PARTS_PER_MILLION as u32);
    let difference_bounds = if coordinates.is_empty() {
        None
    } else {
        let min_x = coordinates.iter().map(|item| item.0).min().unwrap_or(0);
        let min_y = coordinates.iter().map(|item| item.1).min().unwrap_or(0);
        let max_x = coordinates.iter().map(|item| item.0).max().unwrap_or(min_x);
        let max_y = coordinates.iter().map(|item| item.1).max().unwrap_or(min_y);
        Some(ImageRegion {
            x: min_x,
            y: min_y,
            width: max_x - min_x + 1,
            height: max_y - min_y + 1,
        })
    };
    Ok(ImageVisualComparison {
        schema_version: CONTRACT_SCHEMA_VERSION,
        comparison_id: comparison_id.to_owned(),
        artifact_kind,
        before_rgba_sha256: before.rgba_sha256.clone(),
        after_rgba_sha256: after.rgba_sha256.clone(),
        changed_pixels: changed,
        changed_pixel_ratio_ppm: ratio,
        maximum_channel_delta: maximum,
        difference_bounds,
        machine_checks_passed: ratio <= max_changed_pixel_ratio_ppm && maximum <= max_channel_delta,
        human_visual_review_required: true,
        filesystem_effect_performed: false,
        network_access_performed: false,
        execution_performed: false,
    })
}

fn preview_digest(preview: &ImageGenerationPreview) -> Result<String, ImageWorkflowError> {
    #[derive(Serialize)]
    struct Binding<'a> {
        operation_id: &'a str,
        profile_id: &'a str,
        route: ImageGenerationRoute,
        strict_local: bool,
        instruction_sha256: &'a str,
        source_image_sha256: &'a Option<String>,
        disclosures: &'a [String],
        approval_required: bool,
        proposal_only: bool,
    }
    serde_json::to_vec(&Binding {
        operation_id: &preview.operation_id,
        profile_id: &preview.profile_id,
        route: preview.route,
        strict_local: preview.strict_local,
        instruction_sha256: &preview.instruction_sha256,
        source_image_sha256: &preview.source_image_sha256,
        disclosures: &preview.disclosures,
        approval_required: preview.approval_required,
        proposal_only: preview.proposal_only,
    })
    .map(|bytes| word_sha256(&bytes))
    .map_err(|_| ImageWorkflowError::InvalidInput)
}

/// Creates an exact disclosure preview; provider routes fail closed in strict-local mode.
pub fn preview_image_generation(
    operation_id: &str,
    profile_id: &str,
    route: ImageGenerationRoute,
    strict_local: bool,
    instruction_sha256: &str,
    source_image_sha256: Option<String>,
    disclosures: Vec<String>,
) -> Result<ImageGenerationPreview, ImageWorkflowError> {
    if !valid_identifier(operation_id)
        || !valid_identifier(profile_id)
        || !valid_sha256(instruction_sha256)
        || source_image_sha256
            .as_deref()
            .is_some_and(|value| !valid_sha256(value))
        || disclosures.is_empty()
        || disclosures.len() > 32
        || disclosures.iter().any(|value| !valid_identifier(value))
        || disclosures.iter().collect::<BTreeSet<_>>().len() != disclosures.len()
        || disclosures.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return Err(ImageWorkflowError::InvalidInput);
    }
    if strict_local && route == ImageGenerationRoute::Provider {
        return Err(ImageWorkflowError::RouteDenied);
    }
    if route == ImageGenerationRoute::Provider
        && !disclosures
            .iter()
            .any(|value| value == "provider.external-processing")
    {
        return Err(ImageWorkflowError::RouteDenied);
    }
    let mut preview = ImageGenerationPreview {
        operation_id: operation_id.to_owned(),
        profile_id: profile_id.to_owned(),
        route,
        strict_local,
        instruction_sha256: instruction_sha256.to_owned(),
        source_image_sha256,
        disclosures,
        preview_sha256: String::new(),
        approval_required: true,
        proposal_only: true,
    };
    preview.preview_sha256 = preview_digest(&preview)?;
    Ok(preview)
}

/// Binds an externally produced image digest to the exact approved disclosure preview.
pub fn record_image_generation(
    preview: &ImageGenerationPreview,
    approved_preview_sha256: &str,
    output_image_sha256: &str,
) -> Result<ImageGenerationReceipt, ImageWorkflowError> {
    if preview.preview_sha256 != preview_digest(preview)?
        || approved_preview_sha256 != preview.preview_sha256
        || !valid_sha256(output_image_sha256)
        || (preview.strict_local && preview.route == ImageGenerationRoute::Provider)
    {
        return Err(ImageWorkflowError::BindingMismatch);
    }
    Ok(ImageGenerationReceipt {
        operation_id: preview.operation_id.clone(),
        approved_preview_sha256: approved_preview_sha256.to_owned(),
        output_image_sha256: output_image_sha256.to_owned(),
        route: preview.route,
        provider_use_approved: preview.route == ImageGenerationRoute::Provider,
        network_access_performed: false,
        filesystem_effect_performed: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use agentmage_kernel_contracts::WorkspaceId;

    fn image() -> DecodedRgbaImage {
        let rgba = vec![
            10, 20, 30, 255, 40, 50, 60, 255, 70, 80, 90, 255, 100, 110, 120, 255,
        ];
        DecodedRgbaImage {
            width: 2,
            height: 2,
            rgba_sha256: word_sha256(&rgba),
            rgba,
        }
    }

    #[test]
    fn bmp_round_trip_and_metadata_are_exact_and_effect_free() {
        let expected = image();
        let bmp = encode_bmp_rgba(&expected).expect("encode");
        assert_eq!(decode_bmp_rgba(&bmp).expect("decode"), expected);
        let provenance = ImageProvenance {
            source_id: "image-1".to_owned(),
            source_sha256: word_sha256(&bmp),
            slide_number: Some(2),
            object_id: Some(7),
        };
        let report = inspect_image(&bmp, provenance).expect("inspect");
        assert_eq!(report.format, ImageFormat::BmpRgba32);
        assert_eq!((report.width, report.height), (2, 2));
        assert!(report.safe_for_model_context);
        assert!(
            !report.filesystem_effect_performed
                && !report.network_access_performed
                && !report.execution_performed
        );
    }

    #[test]
    fn redaction_scans_decoded_pixels_and_strips_ancillary_bytes() {
        let source = image();
        let region = ImageRegion {
            x: 1,
            y: 0,
            width: 1,
            height: 2,
        };
        let redacted = redact_image("redaction-1", &source, vec![region]).expect("redact");
        assert_eq!(&redacted.image.rgba[4..8], &[0, 0, 0, 255]);
        assert_eq!(&redacted.image.rgba[12..16], &[0, 0, 0, 255]);
        assert!(redacted.receipt.decoded_pixel_scan_passed && redacted.receipt.metadata_removed);
        assert_eq!(
            decode_bmp_rgba(&redacted.bmp).expect("reopen"),
            redacted.image
        );
        assert_eq!(source.rgba[4..8], [40, 50, 60, 255]);
        let path = WorkspacePath::new(
            WorkspaceId::from_raw("workspace-image"),
            ["exports", "redacted.bmp"],
        )
        .expect("path");
        let export = prepare_redacted_image_export("export-1", path, &redacted).expect("export");
        assert!(
            export.decoded_pixel_scan_passed && export.metadata_removed && export.proposal_only
        );
        assert!(!export.filesystem_effect_performed);
    }

    #[test]
    fn sensitive_view_requires_exact_redaction_receipt() {
        let source = image();
        let region = ImageRegion {
            x: 0,
            y: 0,
            width: 1,
            height: 1,
        };
        assert_eq!(
            prepare_image_view(
                "view-1",
                "vision-local",
                true,
                &"a".repeat(64),
                &source,
                &[region],
                None
            ),
            Err(ImageWorkflowError::SensitiveContent)
        );
        let redacted = redact_image("redaction-1", &source, vec![region]).expect("redact");
        let proposal = prepare_image_view(
            "view-1",
            "vision-local",
            true,
            &"a".repeat(64),
            &redacted.image,
            &[region],
            Some(&redacted.receipt),
        )
        .expect("view");
        assert!(proposal.sensitive_regions_cleared && proposal.proposal_only);
    }

    #[test]
    fn visual_diff_covers_every_artifact_kind_and_exact_bounds() {
        let before = image();
        let mut after = before.clone();
        after.rgba[8] = 75;
        after.rgba_sha256 = word_sha256(&after.rgba);
        for kind in [
            VisualArtifactKind::Document,
            VisualArtifactKind::Slide,
            VisualArtifactKind::Image,
            VisualArtifactKind::UserInterface,
        ] {
            let report =
                compare_images("comparison-1", kind, &before, &after, 250_000, 5).expect("compare");
            assert!(report.machine_checks_passed && report.human_visual_review_required);
            assert_eq!(report.changed_pixels, 1);
            assert_eq!(
                report.difference_bounds,
                Some(ImageRegion {
                    x: 0,
                    y: 1,
                    width: 1,
                    height: 1
                })
            );
        }
    }

    #[test]
    fn generation_routes_require_exact_disclosure_and_approval() {
        assert_eq!(
            preview_image_generation(
                "generate-1",
                "provider-1",
                ImageGenerationRoute::Provider,
                true,
                &"a".repeat(64),
                None,
                vec!["provider.external-processing".to_owned()]
            ),
            Err(ImageWorkflowError::RouteDenied)
        );
        let preview = preview_image_generation(
            "generate-1",
            "provider-1",
            ImageGenerationRoute::Provider,
            false,
            &"a".repeat(64),
            None,
            vec!["provider.external-processing".to_owned()],
        )
        .expect("preview");
        assert_eq!(
            record_image_generation(&preview, &"b".repeat(64), &"c".repeat(64)),
            Err(ImageWorkflowError::BindingMismatch)
        );
        let receipt = record_image_generation(&preview, &preview.preview_sha256, &"c".repeat(64))
            .expect("receipt");
        assert!(receipt.provider_use_approved);
        assert!(!receipt.network_access_performed);
    }

    #[test]
    fn malformed_and_oversized_inputs_fail_closed() {
        assert_eq!(
            decode_bmp_rgba(b"BM"),
            Err(ImageWorkflowError::UnsupportedFormat)
        );
        let mut broken = encode_bmp_rgba(&image()).expect("encode");
        broken[28] = 24;
        assert_eq!(
            decode_bmp_rgba(&broken),
            Err(ImageWorkflowError::UnsupportedFormat)
        );
        let huge = DecodedRgbaImage {
            width: MAX_DIMENSION,
            height: MAX_DIMENSION,
            rgba: Vec::new(),
            rgba_sha256: word_sha256(&[]),
        };
        assert_eq!(
            encode_bmp_rgba(&huge),
            Err(ImageWorkflowError::ResourceLimit)
        );
    }
}
