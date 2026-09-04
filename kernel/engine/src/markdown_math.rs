//! Source-preserving Markdown mathematics and offline-preview admission.
#![allow(missing_docs)]

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MathKind {
    Inline,
    Display,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MathSpan {
    pub kind: MathKind,
    pub start: usize,
    pub end: usize,
    pub source: String,
    pub label: Option<String>,
    pub references: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RendererManifest {
    pub component: String,
    pub version: String,
    pub binary_sha256: String,
    pub font_manifest_sha256: String,
    pub license_sha256: String,
    pub max_input_bytes: u64,
    pub max_output_bytes: u64,
    pub max_macro_depth: u16,
    pub max_render_ms: u64,
    pub runtime_downloads: bool,
    pub remote_assets: bool,
    pub executable_extensions: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Preview {
    pub source_sha256: String,
    pub renderer_sha256: String,
    pub cache_sha256: String,
    pub syntax_diagnostic_sha256: String,
    pub screen_reader_sha256: String,
    pub source_link_sha256: String,
    pub copyable_source: bool,
    pub keyboard_navigable: bool,
    pub zoom_reflow: bool,
    pub high_contrast: bool,
    pub cancelled: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MathError {
    UnclosedDelimiter,
    UnsafeCommand,
    InputTooLarge,
    InvalidRenderer,
    InvalidPreview,
}

const UNSAFE: &[&str] = &[
    "\\write18",
    "\\input",
    "\\include",
    "\\openout",
    "\\read",
    "\\usepackage",
    "\\href{http:",
    "\\href{https:",
    "\\def",
    "\\loop",
];
pub fn parse_markdown_math(source: &str, max_bytes: usize) -> Result<Vec<MathSpan>, MathError> {
    if source.len() > max_bytes {
        return Err(MathError::InputTooLarge);
    }
    if UNSAFE.iter().any(|token| source.contains(token)) {
        return Err(MathError::UnsafeCommand);
    }
    let bytes = source.as_bytes();
    let mut spans = Vec::new();
    let mut index = 0;
    let mut fenced = false;
    while index < bytes.len() {
        if bytes[index..].starts_with(b"```") {
            fenced = !fenced;
            index += 3;
            continue;
        }
        if fenced || bytes[index] != b'$' || (index > 0 && bytes[index - 1] == b'\\') {
            index += 1;
            continue;
        }
        let display = index + 1 < bytes.len() && bytes[index + 1] == b'$';
        let width = if display { 2 } else { 1 };
        let start = index;
        index += width;
        let body = index;
        loop {
            if index >= bytes.len() {
                return Err(MathError::UnclosedDelimiter);
            }
            if bytes[index] == b'$'
                && (index == 0 || bytes[index - 1] != b'\\')
                && (!display || index + 1 < bytes.len() && bytes[index + 1] == b'$')
            {
                break;
            }
            index += 1
        }
        let end = index + width;
        spans.push(MathSpan {
            kind: if display {
                MathKind::Display
            } else {
                MathKind::Inline
            },
            start,
            end,
            source: source[body..index].to_owned(),
            label: None,
            references: Vec::new(),
        });
        index = end;
    }
    Ok(spans)
}
pub fn validate_renderer(value: &RendererManifest) -> Result<(), MathError> {
    if !valid_id(&value.component)
        || !valid_id(&value.version)
        || [
            &value.binary_sha256,
            &value.font_manifest_sha256,
            &value.license_sha256,
        ]
        .into_iter()
        .any(|v| !valid_sha(v))
        || value.max_input_bytes == 0
        || value.max_input_bytes > 4 * 1024 * 1024
        || value.max_output_bytes == 0
        || value.max_output_bytes > 16 * 1024 * 1024
        || value.max_macro_depth == 0
        || value.max_macro_depth > 64
        || value.max_render_ms == 0
        || value.max_render_ms > 30_000
        || value.runtime_downloads
        || value.remote_assets
        || value.executable_extensions
    {
        return Err(MathError::InvalidRenderer);
    }
    Ok(())
}
pub fn validate_preview(value: &Preview) -> Result<(), MathError> {
    if [
        &value.source_sha256,
        &value.renderer_sha256,
        &value.cache_sha256,
        &value.syntax_diagnostic_sha256,
        &value.screen_reader_sha256,
        &value.source_link_sha256,
    ]
    .into_iter()
    .any(|v| !valid_sha(v))
        || !value.copyable_source
        || !value.keyboard_navigable
        || !value.zoom_reflow
        || !value.high_contrast
    {
        return Err(MathError::InvalidPreview);
    }
    Ok(())
}
fn valid_id(v: &str) -> bool {
    !v.is_empty()
        && v.len() <= 128
        && v.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}
fn valid_sha(v: &str) -> bool {
    v.len() == 64
        && v.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn math_spans_preserve_exact_source() {
        let s = "price \\$5 and $x+y$\n```\n$code$\n```\n$$z$$";
        let v = parse_markdown_math(s, 1024).unwrap();
        assert_eq!(v.len(), 2);
        assert_eq!(&s[v[0].start..v[0].end], "$x+y$");
        assert_eq!(v[1].source, "z");
    }
    #[test]
    fn unsafe_and_unclosed_math_fail_closed() {
        assert_eq!(
            parse_markdown_math("$\\input{x}$", 100),
            Err(MathError::UnsafeCommand)
        );
        assert_eq!(
            parse_markdown_math("$x", 100),
            Err(MathError::UnclosedDelimiter)
        );
    }
    #[test]
    fn renderer_is_offline_and_bounded() {
        let h = "a".repeat(64);
        let mut r = RendererManifest {
            component: "agentmage-math-reference".into(),
            version: "1.0.0".into(),
            binary_sha256: h.clone(),
            font_manifest_sha256: h.clone(),
            license_sha256: h,
            max_input_bytes: 1024,
            max_output_bytes: 4096,
            max_macro_depth: 16,
            max_render_ms: 1000,
            runtime_downloads: false,
            remote_assets: false,
            executable_extensions: false,
        };
        assert_eq!(validate_renderer(&r), Ok(()));
        r.remote_assets = true;
        assert_eq!(validate_renderer(&r), Err(MathError::InvalidRenderer));
    }
    #[test]
    fn preview_requires_accessible_controls() {
        let h = "b".repeat(64);
        let mut p = Preview {
            source_sha256: h.clone(),
            renderer_sha256: h.clone(),
            cache_sha256: h.clone(),
            syntax_diagnostic_sha256: h.clone(),
            screen_reader_sha256: h.clone(),
            source_link_sha256: h,
            copyable_source: true,
            keyboard_navigable: true,
            zoom_reflow: true,
            high_contrast: true,
            cancelled: false,
        };
        assert_eq!(validate_preview(&p), Ok(()));
        p.keyboard_navigable = false;
        assert_eq!(validate_preview(&p), Err(MathError::InvalidPreview));
    }
}
