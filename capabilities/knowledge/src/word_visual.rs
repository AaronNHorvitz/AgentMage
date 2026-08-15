//! Deterministic, effect-free comparison of externally rendered Word page images.

use std::error::Error;
use std::fmt;

use agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION;
use serde::{Deserialize, Serialize};

use crate::word_generation::valid_identifier;
use crate::word_ooxml::word_sha256;

const MAX_PAGE_COUNT: usize = 4_096;
const MAX_PAGE_DIMENSION: u32 = 20_000;
const MAX_TOTAL_PIXEL_BYTES: usize = 512 * 1_024 * 1_024;
const PARTS_PER_MILLION: u64 = 1_000_000;

/// Closed platform identity for Word rendering evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WordRenderPlatform {
    /// Fedora Linux first-GA reference lane.
    Fedora,
    /// Ubuntu Linux first-GA compatibility lane.
    Ubuntu,
    /// Windows 11 x64 first-GA lane.
    Windows11X64,
    /// Apple Silicon macOS retained post-GA lane.
    MacosAppleSilicon,
}

/// Provenance class for supplied page images.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WordRenderEvidenceKind {
    /// Images were produced by the exact pinned native renderer profile.
    NativeRenderer,
    /// Images are synthetic and may test comparison semantics only.
    SyntheticFixture,
}

/// Exact renderer identity and closed visual thresholds.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WordRenderProfile {
    /// Stable profile identity.
    pub profile_id: String,
    /// Renderer implementation identity.
    pub renderer_id: String,
    /// Exact renderer version.
    pub renderer_version: String,
    /// SHA-256 of the renderer executable or admitted immutable artifact.
    pub renderer_binary_sha256: String,
    /// SHA-256 of the exact font manifest.
    pub font_manifest_sha256: String,
    /// Integer render resolution in dots per inch.
    pub dots_per_inch: u16,
    /// Maximum changed-pixel ratio in parts per million.
    pub max_changed_pixel_ratio_ppm: u32,
    /// Maximum absolute difference allowed in any RGBA channel.
    pub max_channel_delta: u8,
    /// Maximum absolute page-count difference.
    pub max_pagination_delta: u16,
    /// Maximum accepted clipping observations.
    pub max_clipping_count: u32,
    /// Maximum accepted overlap observations.
    pub max_overlap_count: u32,
    /// Maximum accepted font-fallback observations.
    pub max_font_fallback_count: u32,
    /// Maximum accepted table-regression observations.
    pub max_table_regression_count: u32,
    /// Maximum accepted image-regression observations.
    pub max_image_regression_count: u32,
}

/// One exact decoded RGBA page supplied by a renderer adapter or test fixture.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WordPageImage {
    /// One-based page number.
    pub page_number: u32,
    /// Pixel width.
    pub width: u32,
    /// Pixel height.
    pub height: u32,
    /// Exact row-major RGBA8 pixels.
    pub rgba: Vec<u8>,
    /// SHA-256 of `rgba`.
    pub rgba_sha256: String,
}

/// Exact externally produced render output consumed by the pure comparator.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WordRenderOutput {
    /// Stable output identity.
    pub render_id: String,
    /// Exact rendered package SHA-256.
    pub package_sha256: String,
    /// Platform on which rendering occurred.
    pub platform: WordRenderPlatform,
    /// Exact render-profile SHA-256.
    pub profile_sha256: String,
    /// Whether the evidence is native or synthetic.
    pub evidence_kind: WordRenderEvidenceKind,
    /// Canonically ordered decoded pages.
    pub pages: Vec<WordPageImage>,
    /// Renderer-reported clipping observations.
    pub clipping_count: u32,
    /// Renderer-reported overlapping-element observations.
    pub overlap_count: u32,
    /// Renderer-reported font substitutions.
    pub font_fallback_count: u32,
    /// Renderer-reported table layout regressions.
    pub table_regression_count: u32,
    /// Renderer-reported image regressions.
    pub image_regression_count: u32,
}

/// Tight changed-pixel rectangle for one compared page.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WordPageDifferenceBounds {
    /// Inclusive minimum X coordinate.
    pub min_x: u32,
    /// Inclusive minimum Y coordinate.
    pub min_y: u32,
    /// Inclusive maximum X coordinate.
    pub max_x: u32,
    /// Inclusive maximum Y coordinate.
    pub max_y: u32,
}

/// Exact result for one before/after page pair.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WordPageComparison {
    /// One-based page number.
    pub page_number: u32,
    /// Exact before-page RGBA SHA-256.
    pub before_rgba_sha256: String,
    /// Exact after-page RGBA SHA-256.
    pub after_rgba_sha256: String,
    /// True only when width and height match.
    pub dimensions_match: bool,
    /// Number of pixels with any changed RGBA channel.
    pub changed_pixels: u64,
    /// Changed-pixel ratio in integer parts per million.
    pub changed_pixel_ratio_ppm: u32,
    /// Maximum absolute difference observed in any RGBA channel.
    pub maximum_channel_delta: u8,
    /// Tight changed-pixel rectangle, absent when pixels are identical.
    pub difference_bounds: Option<WordPageDifferenceBounds>,
    /// True only when the page satisfies every profile threshold.
    pub passed: bool,
}

/// Complete deterministic visual-comparison report.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WordVisualComparisonReport {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable comparison identity.
    pub comparison_id: String,
    /// Exact render-profile SHA-256.
    pub profile_sha256: String,
    /// Exact before-render identity.
    pub before_render_id: String,
    /// Exact after-render identity.
    pub after_render_id: String,
    /// Platform shared by both render outputs.
    pub platform: WordRenderPlatform,
    /// Exact before-package SHA-256.
    pub before_package_sha256: String,
    /// Exact after-package SHA-256.
    pub after_package_sha256: String,
    /// Evidence class shared by both render outputs.
    pub evidence_kind: WordRenderEvidenceKind,
    /// Signed after-minus-before page-count delta.
    pub pagination_delta: i32,
    /// Canonically ordered comparisons for matching page numbers.
    pub pages: Vec<WordPageComparison>,
    /// After-render clipping observations.
    pub clipping_count: u32,
    /// After-render overlap observations.
    pub overlap_count: u32,
    /// After-render font substitutions.
    pub font_fallback_count: u32,
    /// After-render table regressions.
    pub table_regression_count: u32,
    /// After-render image regressions.
    pub image_regression_count: u32,
    /// True for synthetic evidence or any threshold failure.
    pub human_review_required: bool,
    /// True only when every machine threshold passes; synthetic evidence remains non-release.
    pub machine_checks_passed: bool,
    /// Always false; comparison does not write files.
    pub filesystem_effect_performed: bool,
    /// Always false; comparison uses no network.
    pub network_access_performed: bool,
    /// Always false; comparison launches no renderer or document content.
    pub execution_performed: bool,
}

/// Visual input or comparison failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WordVisualComparisonError {
    /// An identity, hash, threshold, or output field is invalid.
    InvalidInput,
    /// Page count, dimensions, or decoded pixel bytes exceed bounds.
    ResourceLimit,
    /// Before and after evidence use different profiles, platforms, or evidence classes.
    IncompatibleEvidence,
}

impl fmt::Display for WordVisualComparisonError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidInput => "invalid Word visual comparison input",
            Self::ResourceLimit => "Word visual comparison resource limit exceeded",
            Self::IncompatibleEvidence => "incompatible Word visual evidence",
        })
    }
}

impl Error for WordVisualComparisonError {}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_profile(profile: &WordRenderProfile) -> Result<(), WordVisualComparisonError> {
    if !valid_identifier(&profile.profile_id)
        || !valid_identifier(&profile.renderer_id)
        || profile.renderer_version.is_empty()
        || profile.renderer_version.len() > 128
        || !profile.renderer_version.is_ascii()
        || !valid_sha256(&profile.renderer_binary_sha256)
        || !valid_sha256(&profile.font_manifest_sha256)
        || !(72..=1_200).contains(&profile.dots_per_inch)
        || u64::from(profile.max_changed_pixel_ratio_ppm) > PARTS_PER_MILLION
    {
        return Err(WordVisualComparisonError::InvalidInput);
    }
    Ok(())
}

/// Returns the deterministic SHA-256 identity of a validated render profile.
pub fn word_render_profile_sha256(
    profile: &WordRenderProfile,
) -> Result<String, WordVisualComparisonError> {
    validate_profile(profile)?;
    let encoded =
        serde_json::to_vec(profile).map_err(|_| WordVisualComparisonError::InvalidInput)?;
    Ok(word_sha256(&encoded))
}

fn validate_output(
    output: &WordRenderOutput,
    expected_profile_sha256: &str,
) -> Result<(), WordVisualComparisonError> {
    if !valid_identifier(&output.render_id)
        || !valid_sha256(&output.package_sha256)
        || output.profile_sha256 != expected_profile_sha256
        || output.pages.is_empty()
        || output.pages.len() > MAX_PAGE_COUNT
    {
        return Err(WordVisualComparisonError::InvalidInput);
    }
    let mut total_bytes = 0_usize;
    for (index, page) in output.pages.iter().enumerate() {
        let expected_number =
            u32::try_from(index + 1).map_err(|_| WordVisualComparisonError::ResourceLimit)?;
        if page.page_number != expected_number
            || page.width == 0
            || page.height == 0
            || page.width > MAX_PAGE_DIMENSION
            || page.height > MAX_PAGE_DIMENSION
            || !valid_sha256(&page.rgba_sha256)
            || word_sha256(&page.rgba) != page.rgba_sha256
        {
            return Err(WordVisualComparisonError::InvalidInput);
        }
        let expected_bytes = usize::try_from(page.width)
            .ok()
            .and_then(|width| {
                usize::try_from(page.height)
                    .ok()
                    .and_then(|height| width.checked_mul(height))
            })
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or(WordVisualComparisonError::ResourceLimit)?;
        if page.rgba.len() != expected_bytes {
            return Err(WordVisualComparisonError::InvalidInput);
        }
        total_bytes = total_bytes
            .checked_add(expected_bytes)
            .ok_or(WordVisualComparisonError::ResourceLimit)?;
        if total_bytes > MAX_TOTAL_PIXEL_BYTES {
            return Err(WordVisualComparisonError::ResourceLimit);
        }
    }
    Ok(())
}

fn compare_page(
    before: &WordPageImage,
    after: &WordPageImage,
    profile: &WordRenderProfile,
) -> WordPageComparison {
    if before.width != after.width || before.height != after.height {
        return WordPageComparison {
            page_number: before.page_number,
            before_rgba_sha256: before.rgba_sha256.clone(),
            after_rgba_sha256: after.rgba_sha256.clone(),
            dimensions_match: false,
            changed_pixels: u64::from(before.width) * u64::from(before.height),
            changed_pixel_ratio_ppm: PARTS_PER_MILLION as u32,
            maximum_channel_delta: u8::MAX,
            difference_bounds: None,
            passed: false,
        };
    }
    let mut changed_pixels = 0_u64;
    let mut maximum_channel_delta = 0_u8;
    let mut bounds: Option<WordPageDifferenceBounds> = None;
    for (pixel_index, (left, right)) in before
        .rgba
        .chunks_exact(4)
        .zip(after.rgba.chunks_exact(4))
        .enumerate()
    {
        let mut pixel_changed = false;
        for channel in 0..4 {
            let delta = left[channel].abs_diff(right[channel]);
            maximum_channel_delta = maximum_channel_delta.max(delta);
            pixel_changed |= delta > 0;
        }
        if pixel_changed {
            changed_pixels += 1;
            let x = u32::try_from(pixel_index % before.width as usize).unwrap_or(u32::MAX);
            let y = u32::try_from(pixel_index / before.width as usize).unwrap_or(u32::MAX);
            match &mut bounds {
                Some(value) => {
                    value.min_x = value.min_x.min(x);
                    value.min_y = value.min_y.min(y);
                    value.max_x = value.max_x.max(x);
                    value.max_y = value.max_y.max(y);
                }
                None => {
                    bounds = Some(WordPageDifferenceBounds {
                        min_x: x,
                        min_y: y,
                        max_x: x,
                        max_y: y,
                    });
                }
            }
        }
    }
    let pixel_count = u64::from(before.width) * u64::from(before.height);
    let ratio = u32::try_from(
        (u128::from(changed_pixels) * u128::from(PARTS_PER_MILLION)) / u128::from(pixel_count),
    )
    .unwrap_or(PARTS_PER_MILLION as u32);
    WordPageComparison {
        page_number: before.page_number,
        before_rgba_sha256: before.rgba_sha256.clone(),
        after_rgba_sha256: after.rgba_sha256.clone(),
        dimensions_match: true,
        changed_pixels,
        changed_pixel_ratio_ppm: ratio,
        maximum_channel_delta,
        difference_bounds: bounds,
        passed: ratio <= profile.max_changed_pixel_ratio_ppm
            && maximum_channel_delta <= profile.max_channel_delta,
    }
}

/// Compares two externally supplied render outputs without launching a renderer.
pub fn compare_word_page_images(
    comparison_id: &str,
    profile: &WordRenderProfile,
    before: &WordRenderOutput,
    after: &WordRenderOutput,
) -> Result<WordVisualComparisonReport, WordVisualComparisonError> {
    if !valid_identifier(comparison_id) {
        return Err(WordVisualComparisonError::InvalidInput);
    }
    let profile_sha256 = word_render_profile_sha256(profile)?;
    validate_output(before, &profile_sha256)?;
    validate_output(after, &profile_sha256)?;
    if before.platform != after.platform
        || before.evidence_kind != after.evidence_kind
        || before.profile_sha256 != after.profile_sha256
    {
        return Err(WordVisualComparisonError::IncompatibleEvidence);
    }
    let pages = before
        .pages
        .iter()
        .zip(&after.pages)
        .map(|(left, right)| compare_page(left, right, profile))
        .collect::<Vec<_>>();
    let pagination_delta = i32::try_from(after.pages.len())
        .and_then(|right| i32::try_from(before.pages.len()).map(|left| right - left))
        .map_err(|_| WordVisualComparisonError::ResourceLimit)?;
    let machine_checks_passed = pagination_delta.unsigned_abs()
        <= u32::from(profile.max_pagination_delta)
        && pages.iter().all(|page| page.passed)
        && before.pages.len() == after.pages.len()
        && after.clipping_count <= profile.max_clipping_count
        && after.overlap_count <= profile.max_overlap_count
        && after.font_fallback_count <= profile.max_font_fallback_count
        && after.table_regression_count <= profile.max_table_regression_count
        && after.image_regression_count <= profile.max_image_regression_count;
    Ok(WordVisualComparisonReport {
        schema_version: CONTRACT_SCHEMA_VERSION,
        comparison_id: comparison_id.to_owned(),
        profile_sha256,
        before_render_id: before.render_id.clone(),
        after_render_id: after.render_id.clone(),
        platform: before.platform,
        before_package_sha256: before.package_sha256.clone(),
        after_package_sha256: after.package_sha256.clone(),
        evidence_kind: before.evidence_kind,
        pagination_delta,
        pages,
        clipping_count: after.clipping_count,
        overlap_count: after.overlap_count,
        font_fallback_count: after.font_fallback_count,
        table_regression_count: after.table_regression_count,
        image_regression_count: after.image_regression_count,
        human_review_required: before.evidence_kind == WordRenderEvidenceKind::SyntheticFixture
            || !machine_checks_passed,
        machine_checks_passed,
        filesystem_effect_performed: false,
        network_access_performed: false,
        execution_performed: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> WordRenderProfile {
        WordRenderProfile {
            profile_id: "word-render-v1".to_owned(),
            renderer_id: "pinned-renderer".to_owned(),
            renderer_version: "1.0.0".to_owned(),
            renderer_binary_sha256: "a".repeat(64),
            font_manifest_sha256: "b".repeat(64),
            dots_per_inch: 144,
            max_changed_pixel_ratio_ppm: 250_000,
            max_channel_delta: 16,
            max_pagination_delta: 0,
            max_clipping_count: 0,
            max_overlap_count: 0,
            max_font_fallback_count: 0,
            max_table_regression_count: 0,
            max_image_regression_count: 0,
        }
    }

    fn page(number: u32, rgba: Vec<u8>, width: u32, height: u32) -> WordPageImage {
        WordPageImage {
            page_number: number,
            width,
            height,
            rgba_sha256: word_sha256(&rgba),
            rgba,
        }
    }

    fn output(render_id: &str, pixels: Vec<u8>, profile_sha256: &str) -> WordRenderOutput {
        WordRenderOutput {
            render_id: render_id.to_owned(),
            package_sha256: "c".repeat(64),
            platform: WordRenderPlatform::Fedora,
            profile_sha256: profile_sha256.to_owned(),
            evidence_kind: WordRenderEvidenceKind::SyntheticFixture,
            pages: vec![page(1, pixels, 2, 2)],
            clipping_count: 0,
            overlap_count: 0,
            font_fallback_count: 0,
            table_regression_count: 0,
            image_regression_count: 0,
        }
    }

    #[test]
    fn identical_and_bounded_differences_are_exact_and_effect_free() {
        let profile = profile();
        let profile_hash = word_render_profile_sha256(&profile).expect("profile");
        let before = output("before", vec![0; 16], &profile_hash);
        let identical = output("identical", vec![0; 16], &profile_hash);
        let first = compare_word_page_images("comparison-1", &profile, &before, &identical)
            .expect("compare");
        assert!(first.machine_checks_passed);
        assert!(
            first.human_review_required,
            "synthetic evidence remains non-release"
        );
        assert_eq!(first.pages[0].changed_pixels, 0);
        assert!(!first.filesystem_effect_performed);
        assert!(!first.network_access_performed);
        assert!(!first.execution_performed);

        let mut changed_pixels = vec![0; 16];
        changed_pixels[12] = 10;
        let changed = output("changed", changed_pixels, &profile_hash);
        let second =
            compare_word_page_images("comparison-2", &profile, &before, &changed).expect("compare");
        assert!(second.machine_checks_passed);
        assert_eq!(second.pages[0].changed_pixels, 1);
        assert_eq!(second.pages[0].changed_pixel_ratio_ppm, 250_000);
        assert_eq!(second.pages[0].maximum_channel_delta, 10);
        assert_eq!(
            second.pages[0].difference_bounds,
            Some(WordPageDifferenceBounds {
                min_x: 1,
                min_y: 1,
                max_x: 1,
                max_y: 1,
            })
        );
    }

    #[test]
    fn regressions_tampering_and_incompatible_evidence_fail_closed() {
        let profile = profile();
        let profile_hash = word_render_profile_sha256(&profile).expect("profile");
        let before = output("before", vec![0; 16], &profile_hash);
        let mut changed = output("changed", vec![255; 16], &profile_hash);
        changed.clipping_count = 1;
        let report = compare_word_page_images("comparison", &profile, &before, &changed)
            .expect("comparison report");
        assert!(!report.machine_checks_passed);
        assert!(report.human_review_required);
        assert!(!report.pages[0].passed);

        let mut tampered = before.clone();
        tampered.pages[0].rgba[0] = 1;
        assert_eq!(
            compare_word_page_images("comparison", &profile, &tampered, &changed),
            Err(WordVisualComparisonError::InvalidInput)
        );
        let mut other_platform = before.clone();
        other_platform.platform = WordRenderPlatform::Ubuntu;
        assert_eq!(
            compare_word_page_images("comparison", &profile, &before, &other_platform),
            Err(WordVisualComparisonError::IncompatibleEvidence)
        );
        let mut invalid_size = before.clone();
        invalid_size.pages[0].width = MAX_PAGE_DIMENSION + 1;
        assert_eq!(
            compare_word_page_images("comparison", &profile, &invalid_size, &changed),
            Err(WordVisualComparisonError::InvalidInput)
        );
    }
}
