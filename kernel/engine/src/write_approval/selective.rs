//! Exact hunk-level selection over one reviewed text change (Decision 0107).
//!
//! Selection narrows a proposed complete postimage to the hunks a user accepts.
//! It grants nothing: the selected postimage re-enters the existing exact write
//! preview, approval and grant path. A preimage that changed since review is
//! refused, so a concurrent human edit is revalidated instead of overwritten.
//! Hunks are syntactic line ranges; selection does not prove the result builds.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use serde::Serialize;
use sha2::{Digest, Sha256};

/// Largest preimage or proposal accepted for hunk selection.
pub const MAX_SELECTIVE_BYTES: usize = 1024 * 1024;
/// Largest line count on either side.
pub const MAX_SELECTIVE_LINES: usize = 20_000;
/// Largest line edit distance; larger changes keep whole-file review.
pub const MAX_SELECTIVE_EDIT_DISTANCE: usize = 2_048;
/// Largest hunk count presented for one file.
pub const MAX_SELECTIVE_HUNKS: usize = 512;
/// Largest bounded rendering; longer output is truncated with an explicit marker.
pub const MAX_SELECTIVE_RENDER_BYTES: usize = 256 * 1024;

/// Content-free reason hunk selection cannot be offered or applied.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectiveChangeError {
    /// Either side is not UTF-8 text or contains a NUL byte.
    NotText,
    /// Either side exceeds the byte or line bound.
    TooLarge,
    /// The edit distance or hunk count exceeds its bound.
    TooComplex,
    /// A selection names a hunk that this exact change does not contain.
    UnknownHunk,
    /// Current bytes differ from the reviewed preimage.
    PreimageDrift,
}

impl SelectiveChangeError {
    /// Stable diagnostic without paths or content.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::NotText => "write.selective.not-text",
            Self::TooLarge => "write.selective.too-large",
            Self::TooComplex => "write.selective.too-complex",
            Self::UnknownHunk => "write.selective.unknown-hunk",
            Self::PreimageDrift => "write.selective.preimage-drift",
        }
    }
}

/// One maximal run of changed lines between unchanged lines.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextHunk {
    hunk_id: String,
    old_start: usize,
    old_lines: usize,
    new_start: usize,
    new_lines: usize,
}

impl TextHunk {
    /// Identity bound to the exact preimage, proposal and line ranges.
    #[must_use]
    pub fn hunk_id(&self) -> &str {
        &self.hunk_id
    }

    /// Zero-based first preimage line and its count.
    #[must_use]
    pub const fn old_range(&self) -> (usize, usize) {
        (self.old_start, self.old_lines)
    }

    /// Zero-based first proposal line and its count.
    #[must_use]
    pub const fn new_range(&self) -> (usize, usize) {
        (self.new_start, self.new_lines)
    }
}

#[derive(Serialize)]
struct HunkIdentity<'a> {
    schema_version: u16,
    preimage_sha256: &'a str,
    proposal_sha256: &'a str,
    old_start: usize,
    old_lines: usize,
    new_start: usize,
    new_lines: usize,
}

#[derive(Serialize)]
struct SelectionIdentity<'a> {
    schema_version: u16,
    preimage_sha256: &'a str,
    proposal_sha256: &'a str,
    accepted_hunk_ids: &'a [String],
    rejected_hunk_ids: &'a [String],
    postimage_sha256: &'a str,
}

/// An exact reviewed preimage/proposal pair divided into selectable hunks.
#[derive(Clone, PartialEq, Eq)]
pub struct HunkedTextChange {
    preimage_sha256: String,
    proposal_sha256: String,
    preimage_lines: Vec<String>,
    proposal_lines: Vec<String>,
    hunks: Vec<TextHunk>,
}

impl std::fmt::Debug for HunkedTextChange {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("HunkedTextChange")
            .field("preimage_sha256", &self.preimage_sha256)
            .field("proposal_sha256", &self.proposal_sha256)
            .field("hunk_count", &self.hunks.len())
            .finish_non_exhaustive()
    }
}

/// A complete postimage built from exactly the accepted hunks.
#[derive(Clone, PartialEq, Eq)]
pub struct SelectedTextChange {
    preimage_sha256: String,
    proposal_sha256: String,
    accepted_hunk_ids: Vec<String>,
    rejected_hunk_ids: Vec<String>,
    postimage: Vec<u8>,
    postimage_sha256: String,
    selection_sha256: String,
}

impl std::fmt::Debug for SelectedTextChange {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SelectedTextChange")
            .field("selection_sha256", &self.selection_sha256)
            .field("accepted", &self.accepted_hunk_ids.len())
            .field("rejected", &self.rejected_hunk_ids.len())
            .finish_non_exhaustive()
    }
}

impl SelectedTextChange {
    /// Complete postimage to submit to the existing exact write preview.
    #[must_use]
    pub fn postimage(&self) -> &[u8] {
        &self.postimage
    }

    /// Digest of the complete selected postimage.
    #[must_use]
    pub fn postimage_sha256(&self) -> &str {
        &self.postimage_sha256
    }

    /// Reviewed preimage digest the selection applies to.
    #[must_use]
    pub fn preimage_sha256(&self) -> &str {
        &self.preimage_sha256
    }

    /// Proposal digest the hunks were derived from.
    #[must_use]
    pub fn proposal_sha256(&self) -> &str {
        &self.proposal_sha256
    }

    /// Sorted accepted hunk identities.
    #[must_use]
    pub fn accepted_hunk_ids(&self) -> &[String] {
        &self.accepted_hunk_ids
    }

    /// Sorted rejected hunk identities.
    #[must_use]
    pub fn rejected_hunk_ids(&self) -> &[String] {
        &self.rejected_hunk_ids
    }

    /// Canonical digest of the whole selection record; not an approval.
    #[must_use]
    pub fn selection_sha256(&self) -> &str {
        &self.selection_sha256
    }

    /// Whether the selection leaves the reviewed preimage unchanged.
    #[must_use]
    pub fn is_unchanged(&self) -> bool {
        self.postimage_sha256 == self.preimage_sha256
    }
}

impl HunkedTextChange {
    /// Divides an exact text change into hunks. Unsupported input keeps the
    /// caller's existing whole-file review; nothing here reads or writes files.
    pub fn new(preimage: &[u8], proposed: &[u8]) -> Result<Self, SelectiveChangeError> {
        let preimage_lines = text_lines(preimage)?;
        let proposal_lines = text_lines(proposed)?;
        let preimage_sha256 = hex_sha256(preimage);
        let proposal_sha256 = hex_sha256(proposed);
        let ranges = changed_ranges(&preimage_lines, &proposal_lines)?;
        if ranges.len() > MAX_SELECTIVE_HUNKS {
            return Err(SelectiveChangeError::TooComplex);
        }
        let hunks = ranges
            .into_iter()
            .map(|(old_start, old_lines, new_start, new_lines)| TextHunk {
                hunk_id: canonical_sha256(&HunkIdentity {
                    schema_version: 1,
                    preimage_sha256: &preimage_sha256,
                    proposal_sha256: &proposal_sha256,
                    old_start,
                    old_lines,
                    new_start,
                    new_lines,
                }),
                old_start,
                old_lines,
                new_start,
                new_lines,
            })
            .collect();
        Ok(Self {
            preimage_sha256,
            proposal_sha256,
            preimage_lines,
            proposal_lines,
            hunks,
        })
    }

    /// Ordered hunks; identities are stable for the same exact pair.
    #[must_use]
    pub fn hunks(&self) -> &[TextHunk] {
        &self.hunks
    }

    /// Reviewed preimage digest.
    #[must_use]
    pub fn preimage_sha256(&self) -> &str {
        &self.preimage_sha256
    }

    /// Proposal digest.
    #[must_use]
    pub fn proposal_sha256(&self) -> &str {
        &self.proposal_sha256
    }

    /// Refuses current bytes that differ from the reviewed preimage.
    pub fn verify_current(&self, current: &[u8]) -> Result<(), SelectiveChangeError> {
        if hex_sha256(current) == self.preimage_sha256 {
            Ok(())
        } else {
            Err(SelectiveChangeError::PreimageDrift)
        }
    }

    /// Builds the complete postimage from exactly the accepted hunks. `current`
    /// must still equal the reviewed preimage, so a concurrent edit is refused
    /// as drift before anything is selected. Every identity must belong to this
    /// change; unlisted hunks keep the preimage.
    pub fn select(
        &self,
        current: &[u8],
        accepted: &BTreeSet<String>,
    ) -> Result<SelectedTextChange, SelectiveChangeError> {
        self.verify_current(current)?;
        if accepted
            .iter()
            .any(|id| !self.hunks.iter().any(|hunk| &hunk.hunk_id == id))
        {
            return Err(SelectiveChangeError::UnknownHunk);
        }
        let mut postimage = String::new();
        let mut cursor = 0;
        let mut accepted_hunk_ids = Vec::new();
        let mut rejected_hunk_ids = Vec::new();
        for hunk in &self.hunks {
            postimage.extend(
                self.preimage_lines[cursor..hunk.old_start]
                    .iter()
                    .map(String::as_str),
            );
            if accepted.contains(&hunk.hunk_id) {
                accepted_hunk_ids.push(hunk.hunk_id.clone());
                postimage.extend(
                    self.proposal_lines[hunk.new_start..hunk.new_start + hunk.new_lines]
                        .iter()
                        .map(String::as_str),
                );
            } else {
                rejected_hunk_ids.push(hunk.hunk_id.clone());
                postimage.extend(
                    self.preimage_lines[hunk.old_start..hunk.old_start + hunk.old_lines]
                        .iter()
                        .map(String::as_str),
                );
            }
            cursor = hunk.old_start + hunk.old_lines;
        }
        postimage.extend(self.preimage_lines[cursor..].iter().map(String::as_str));
        accepted_hunk_ids.sort();
        rejected_hunk_ids.sort();
        let postimage = postimage.into_bytes();
        let postimage_sha256 = hex_sha256(&postimage);
        let selection_sha256 = canonical_sha256(&SelectionIdentity {
            schema_version: 1,
            preimage_sha256: &self.preimage_sha256,
            proposal_sha256: &self.proposal_sha256,
            accepted_hunk_ids: &accepted_hunk_ids,
            rejected_hunk_ids: &rejected_hunk_ids,
            postimage_sha256: &postimage_sha256,
        });
        Ok(SelectedTextChange {
            preimage_sha256: self.preimage_sha256.clone(),
            proposal_sha256: self.proposal_sha256.clone(),
            accepted_hunk_ids,
            rejected_hunk_ids,
            postimage,
            postimage_sha256,
            selection_sha256,
        })
    }

    /// Bounded unified-style display with up to `context` unchanged lines around
    /// each hunk. Characters are escaped as in the whole-file preview (see
    /// `push_display_line`); a missing final newline is marked. Output over the
    /// bound ends with a truncation marker.
    #[must_use]
    pub fn render(&self, context: usize) -> String {
        let mut output = String::new();
        let mut shown_until = 0;
        for (index, hunk) in self.hunks.iter().enumerate() {
            let mut block = String::new();
            let before = hunk.old_start.saturating_sub(context).max(shown_until);
            let after = (hunk.old_start + hunk.old_lines)
                .saturating_add(context)
                .min(self.preimage_lines.len());
            let _ = writeln!(
                block,
                "@@ -{},{} +{},{} @@ hunk {}",
                hunk.old_start + 1,
                hunk.old_lines,
                hunk.new_start + 1,
                hunk.new_lines,
                &hunk.hunk_id[..12]
            );
            for line in &self.preimage_lines[before..hunk.old_start] {
                push_display_line(&mut block, ' ', line);
            }
            for line in &self.preimage_lines[hunk.old_start..hunk.old_start + hunk.old_lines] {
                push_display_line(&mut block, '-', line);
            }
            for line in &self.proposal_lines[hunk.new_start..hunk.new_start + hunk.new_lines] {
                push_display_line(&mut block, '+', line);
            }
            let next_start = self
                .hunks
                .get(index + 1)
                .map_or(usize::MAX, |next| next.old_start.saturating_sub(context));
            let trailing_end = after.min(next_start.max(hunk.old_start + hunk.old_lines));
            for line in &self.preimage_lines[hunk.old_start + hunk.old_lines..trailing_end] {
                push_display_line(&mut block, ' ', line);
            }
            shown_until = trailing_end;
            if output.len() + block.len() > MAX_SELECTIVE_RENDER_BYTES {
                let _ = writeln!(
                    output,
                    "... display truncated; {} of {} hunks not shown",
                    self.hunks.len() - index,
                    self.hunks.len()
                );
                return output;
            }
            output.push_str(&block);
        }
        output
    }
}

fn push_display_line(output: &mut String, marker: char, line: &str) {
    let (content, newline) = match line.strip_suffix('\n') {
        Some(content) => (content, true),
        None => (line, false),
    };
    output.push(marker);
    // Escape exactly as the whole-file preview's `{:?}` does, so this display is
    // never weaker than it: control, format, line and paragraph separator,
    // private-use, unassigned and non-ASCII space characters, a leading
    // combining mark and backslash. Only tab and the ASCII quotes are shown as
    // themselves; they cannot reorder, hide or break a displayed line.
    let mut escaped = content.escape_debug();
    while let Some(character) = escaped.next() {
        if character != '\\' {
            output.push(character);
            continue;
        }
        match escaped.next() {
            Some(quote @ ('\'' | '"')) => output.push(quote),
            Some('t') => output.push('\t'),
            Some(other) => {
                output.push('\\');
                output.push(other);
            }
            None => output.push('\\'),
        }
    }
    output.push('\n');
    if !newline {
        output.push_str("\\ no newline at end of file\n");
    }
}

fn text_lines(bytes: &[u8]) -> Result<Vec<String>, SelectiveChangeError> {
    if bytes.len() > MAX_SELECTIVE_BYTES {
        return Err(SelectiveChangeError::TooLarge);
    }
    if bytes.contains(&0) {
        return Err(SelectiveChangeError::NotText);
    }
    let text = std::str::from_utf8(bytes).map_err(|_| SelectiveChangeError::NotText)?;
    let lines: Vec<String> = text.split_inclusive('\n').map(str::to_owned).collect();
    if lines.len() > MAX_SELECTIVE_LINES {
        return Err(SelectiveChangeError::TooLarge);
    }
    Ok(lines)
}

type Range = (usize, usize, usize, usize);

/// Returns maximal changed (old_start, old_lines, new_start, new_lines) ranges
/// from a shortest line edit script, bounded by `MAX_SELECTIVE_EDIT_DISTANCE`.
fn changed_ranges(old: &[String], new: &[String]) -> Result<Vec<Range>, SelectiveChangeError> {
    let prefix = old
        .iter()
        .zip(new)
        .take_while(|(left, right)| left == right)
        .count();
    let suffix = old[prefix..]
        .iter()
        .rev()
        .zip(new[prefix..].iter().rev())
        .take_while(|(left, right)| left == right)
        .count();
    let a = &old[prefix..old.len() - suffix];
    let b = &new[prefix..new.len() - suffix];
    let equal = shortest_equal_pairs(a, b)?;
    let mut ranges = Vec::new();
    let (mut x, mut y) = (0, 0);
    for (next_x, next_y) in equal.into_iter().chain(std::iter::once((a.len(), b.len()))) {
        if next_x > x || next_y > y {
            ranges.push((prefix + x, next_x - x, prefix + y, next_y - y));
        }
        x = next_x + 1;
        y = next_y + 1;
    }
    Ok(ranges)
}

/// Myers' greedy shortest edit script, recording only the diagonal window each
/// step reads so memory grows with the square of the bounded edit distance.
/// Returns the ordered (old, new) indices of unchanged lines.
fn shortest_equal_pairs(
    a: &[String],
    b: &[String],
) -> Result<Vec<(usize, usize)>, SelectiveChangeError> {
    let (n, m) = (a.len() as isize, b.len() as isize);
    let limit = (a.len() + b.len()).min(MAX_SELECTIVE_EDIT_DISTANCE) as isize;
    let offset = limit + 1;
    let mut v = vec![0_isize; (2 * limit + 3) as usize];
    let mut trace: Vec<Vec<isize>> = Vec::new();
    let at = |k: isize| (offset + k) as usize;
    for d in 0..=limit {
        trace.push(v[at(-d - 1)..=at(d + 1)].to_vec());
        for k in (-d..=d).step_by(2) {
            let mut x = if k == -d || (k != d && v[at(k - 1)] < v[at(k + 1)]) {
                v[at(k + 1)]
            } else {
                v[at(k - 1)] + 1
            };
            let mut y = x - k;
            while x < n && y < m && a[x as usize] == b[y as usize] {
                x += 1;
                y += 1;
            }
            v[at(k)] = x;
            if x >= n && y >= m {
                return Ok(backtrack(&trace, a, b, d));
            }
        }
    }
    Err(SelectiveChangeError::TooComplex)
}

fn backtrack(
    trace: &[Vec<isize>],
    a: &[String],
    b: &[String],
    depth: isize,
) -> Vec<(usize, usize)> {
    let mut pairs = Vec::new();
    let (mut x, mut y) = (a.len() as isize, b.len() as isize);
    for d in (0..=depth).rev() {
        // trace[d] holds diagonals -(d+1)..=(d+1) from before step d.
        let window = &trace[d as usize];
        let at = |k: isize| (k + d + 1) as usize;
        let k = x - y;
        let previous_k = if k == -d || (k != d && window[at(k - 1)] < window[at(k + 1)]) {
            k + 1
        } else {
            k - 1
        };
        let previous_x = window[at(previous_k)];
        let previous_y = previous_x - previous_k;
        while x > previous_x && y > previous_y {
            x -= 1;
            y -= 1;
            pairs.push((x as usize, y as usize));
        }
        if d > 0 {
            x = previous_x;
            y = previous_y;
        }
    }
    pairs.reverse();
    pairs
}

fn hex_sha256(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        let _ = write!(encoded, "{byte:02x}");
    }
    encoded
}

fn canonical_sha256(value: &impl Serialize) -> String {
    hex_sha256(&serde_json::to_vec(value).expect("closed identity records serialize"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_ids(change: &HunkedTextChange) -> BTreeSet<String> {
        change
            .hunks()
            .iter()
            .map(|hunk| hunk.hunk_id().to_owned())
            .collect()
    }

    fn subset(change: &HunkedTextChange, mask: u32) -> BTreeSet<String> {
        change
            .hunks()
            .iter()
            .enumerate()
            .filter(|(index, _)| mask & (1 << index) != 0)
            .map(|(_, hunk)| hunk.hunk_id().to_owned())
            .collect()
    }

    #[test]
    fn accepting_every_hunk_is_the_proposal_and_rejecting_every_hunk_is_the_preimage() {
        for (preimage, proposal) in [
            ("", ""),
            ("", "a\n"),
            ("a\n", ""),
            ("same\n", "same\n"),
            ("a\nb\nc\n", "a\nB\nc\n"),
            ("a\nb\nc", "a\nb\nc\n"),
            ("x\r\ny\r\n", "x\r\nY\r\n"),
            ("1\n2\n3\n4\n5\n6\n", "0\n1\n2\n3x\n4\n6\n7\n"),
            ("only\n", "first\nonly\nlast"),
        ] {
            let change = HunkedTextChange::new(preimage.as_bytes(), proposal.as_bytes()).unwrap();
            assert_eq!(preimage == proposal, change.hunks().is_empty());
            let all = change
                .select(preimage.as_bytes(), &all_ids(&change))
                .unwrap();
            assert_eq!(
                all.postimage(),
                proposal.as_bytes(),
                "{preimage:?} -> {proposal:?}"
            );
            assert!(all.rejected_hunk_ids().is_empty());
            let none = change
                .select(preimage.as_bytes(), &BTreeSet::new())
                .unwrap();
            assert_eq!(none.postimage(), preimage.as_bytes());
            assert!(none.is_unchanged());
            assert!(none.accepted_hunk_ids().is_empty());
        }
    }

    #[test]
    fn each_subset_applies_exactly_the_accepted_hunks() {
        let preimage = "fn a() {}\nkeep 1\nfn b() {}\nkeep 2\nfn c() {}\n";
        let proposal = "fn a2() {}\nkeep 1\nfn b() {}\nnew line\nkeep 2\nfn c3() {}\n";
        let change = HunkedTextChange::new(preimage.as_bytes(), proposal.as_bytes()).unwrap();
        assert_eq!(change.hunks().len(), 3);
        assert_eq!(change.hunks()[0].old_range(), (0, 1));
        assert_eq!(change.hunks()[1].old_range(), (3, 0));
        assert_eq!(change.hunks()[1].new_range(), (3, 1));
        assert_eq!(change.hunks()[2].new_range(), (5, 1));
        let mut digests = BTreeSet::new();
        for mask in 0..8 {
            let selected = change
                .select(preimage.as_bytes(), &subset(&change, mask))
                .unwrap();
            let expected = [
                if mask & 1 != 0 {
                    "fn a2() {}\n"
                } else {
                    "fn a() {}\n"
                },
                "keep 1\nfn b() {}\n",
                if mask & 2 != 0 { "new line\n" } else { "" },
                "keep 2\n",
                if mask & 4 != 0 {
                    "fn c3() {}\n"
                } else {
                    "fn c() {}\n"
                },
            ]
            .concat();
            assert_eq!(
                String::from_utf8(selected.postimage().to_vec()).unwrap(),
                expected
            );
            assert_eq!(selected.postimage_sha256(), hex_sha256(expected.as_bytes()));
            assert_eq!(
                selected.accepted_hunk_ids().len() + selected.rejected_hunk_ids().len(),
                3
            );
            assert!(digests.insert(selected.selection_sha256().to_owned()));
        }
    }

    #[test]
    fn identities_are_stable_and_bound_to_the_exact_pair() {
        let preimage = b"a\nb\nc\nd\n";
        let first = HunkedTextChange::new(preimage, b"A\nb\nc\nD\n").unwrap();
        assert_eq!(
            first,
            HunkedTextChange::new(preimage, b"A\nb\nc\nD\n").unwrap()
        );
        let other = HunkedTextChange::new(preimage, b"A\nb\nc\nd2\n").unwrap();
        // The same first-line edit belongs to a different proposal and identity.
        assert_eq!(first.hunks()[0].old_range(), other.hunks()[0].old_range());
        assert_ne!(first.hunks()[0].hunk_id(), other.hunks()[0].hunk_id());
        assert_eq!(
            first.select(preimage, &all_ids(&other)).err(),
            Some(SelectiveChangeError::UnknownHunk)
        );
        let forged = BTreeSet::from(["0".repeat(64)]);
        assert_eq!(
            first.select(preimage, &forged).err(),
            Some(SelectiveChangeError::UnknownHunk)
        );
        assert_eq!(
            first.select(preimage, &subset(&first, 1)).unwrap(),
            first.select(preimage, &subset(&first, 1)).unwrap()
        );
        assert!(!format!("{first:?}").contains("b\\n"));
    }

    #[test]
    fn drift_non_text_and_bounds_are_refused() {
        let change = HunkedTextChange::new(b"a\n", b"b\n").unwrap();
        assert_eq!(change.verify_current(b"a\n"), Ok(()));
        assert_eq!(
            change.verify_current(b"a\nhuman edit\n"),
            Err(SelectiveChangeError::PreimageDrift)
        );
        // Selection itself refuses drift before any identity or postimage is used.
        for accepted in [all_ids(&change), BTreeSet::from(["0".repeat(64)])] {
            assert_eq!(
                change.select(b"a\nhuman edit\n", &accepted).err(),
                Some(SelectiveChangeError::PreimageDrift)
            );
        }
        assert_eq!(
            change
                .select(b"a\n", &all_ids(&change))
                .unwrap()
                .postimage(),
            b"b\n"
        );
        for (preimage, proposal) in [(&b"\xff\n"[..], &b"a\n"[..]), (b"a\n", b"a\0\n")] {
            assert_eq!(
                HunkedTextChange::new(preimage, proposal).err(),
                Some(SelectiveChangeError::NotText)
            );
        }
        let oversized = vec![b'a'; MAX_SELECTIVE_BYTES + 1];
        assert_eq!(
            HunkedTextChange::new(&oversized, b"").err(),
            Some(SelectiveChangeError::TooLarge)
        );
        let many_lines = "x\n".repeat(MAX_SELECTIVE_LINES + 1);
        assert_eq!(
            HunkedTextChange::new(many_lines.as_bytes(), b"").err(),
            Some(SelectiveChangeError::TooLarge)
        );
        let before: String = (0..1_100).map(|index| format!("a{index}\n")).collect();
        let after: String = (0..1_100).map(|index| format!("b{index}\n")).collect();
        assert_eq!(
            HunkedTextChange::new(before.as_bytes(), after.as_bytes()).err(),
            Some(SelectiveChangeError::TooComplex)
        );
        let before: String = (0..600)
            .map(|index| format!("keep\nold{index}\n"))
            .collect();
        let after: String = (0..600)
            .map(|index| format!("keep\nnew{index}\n"))
            .collect();
        assert_eq!(
            HunkedTextChange::new(before.as_bytes(), after.as_bytes()).err(),
            Some(SelectiveChangeError::TooComplex)
        );
        for error in [
            SelectiveChangeError::NotText,
            SelectiveChangeError::TooLarge,
            SelectiveChangeError::TooComplex,
            SelectiveChangeError::UnknownHunk,
            SelectiveChangeError::PreimageDrift,
        ] {
            assert!(error.code().starts_with("write.selective."));
        }
    }

    #[test]
    fn rendering_is_bounded_escaped_and_marks_missing_newlines() {
        let change = HunkedTextChange::new(b"a\nb\x1b[31m\nc", b"a\nB\nc\n").unwrap();
        let rendered = change.render(1);
        assert!(rendered.starts_with("@@ -2,2 +2,2 @@ hunk "), "{rendered}");
        assert!(
            rendered.contains("\n a\n-b\\u{1b}[31m\n-c\n\\ no newline at end of file\n+B\n+c\n")
        );
        assert!(!rendered.contains('\u{1b}'));
        // Adjacent hunks share context without repeating a line.
        let change = HunkedTextChange::new(b"1\n2\n3\n4\n5\n", b"1x\n2\n3\n4\n5x\n").unwrap();
        let rendered = change.render(3);
        assert_eq!(rendered.matches(" 3\n").count(), 1, "{rendered}");
        let long_line = "y".repeat(1_000);
        let before: String = (0..400)
            .map(|index| format!("k{index}\n{long_line}{index}\n"))
            .collect();
        let after: String = (0..400)
            .map(|index| format!("k{index}\nz{index}\n"))
            .collect();
        let change = HunkedTextChange::new(before.as_bytes(), after.as_bytes()).unwrap();
        let rendered = change.render(0);
        assert!(rendered.len() <= MAX_SELECTIVE_RENDER_BYTES + 128);
        assert!(
            rendered.ends_with("hunks not shown\n"),
            "{}",
            &rendered[rendered.len() - 80..]
        );
    }

    #[test]
    fn rendering_escapes_every_reordering_invisible_and_separator_character() {
        let hostile = [
            '\u{202e}', '\u{2066}', '\u{200b}', '\u{2028}', '\u{2029}', '\u{feff}', '\u{85}',
            '\u{7f}', '\u{9b}', '\u{a0}', '\u{e000}', '\u{1b}', '\r',
        ];
        let line: String = hostile.iter().collect();
        let proposal = format!("a\n{line}\n\u{301}lead e\u{301} \\u{{202e}} 'q' \"q\"\tend\n");
        let change = HunkedTextChange::new(b"a\n", proposal.as_bytes()).unwrap();
        let rendered = change.render(3);
        for character in hostile {
            assert!(!rendered.contains(character), "{:x}", u32::from(character));
        }
        assert!(rendered.contains("+\\u{202e}\\u{2066}\\u{200b}\\u{2028}\\u{2029}\\u{feff}"));
        assert!(rendered.contains("\\u{85}\\u{7f}\\u{9b}\\u{a0}\\u{e000}\\u{1b}\\r\n"));
        // A leading combining mark cannot merge with the marker; a later one is
        // ordinary text. A literal backslash escape stays distinguishable, and
        // tab and quotes are shown as themselves.
        assert!(rendered.contains("+\\u{301}lead e\u{301} \\\\u{202e} 'q' \"q\"\tend\n"));
        // Every displayed line starts with a marker, a header or the newline note.
        assert!(
            rendered
                .lines()
                .all(|line| line.starts_with(['@', ' ', '-', '+', '\\']))
        );
        // The whole-file preview's `{:?}` escapes at least as much as this display.
        let preview = format!("{proposal:?}");
        for character in hostile {
            assert!(!preview.contains(character));
        }
    }

    #[test]
    fn rendering_accepts_any_context_value_without_overflow() {
        let change = HunkedTextChange::new(b"1\n2\n3\n4\n", b"1\n2x\n3\n4\n").unwrap();
        let unbounded = change.render(usize::MAX);
        assert_eq!(unbounded, change.render(1 << 40));
        let expected = format!(
            "@@ -2,1 +2,1 @@ hunk {}\n 1\n-2\n+2x\n 3\n 4\n",
            &change.hunks()[0].hunk_id()[..12]
        );
        assert_eq!(unbounded, expected);
    }

    #[test]
    fn randomized_pairs_round_trip_and_subsets_compose_their_ranges() {
        let mut state = 0x9e37_79b9_7f4a_7c15_u64;
        let mut next = move |bound: u64| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state % bound
        };
        for _ in 0..300 {
            let preimage: Vec<String> = (0..next(30))
                .map(|_| format!("{}\n", ["a", "b", "c", "d", "e"][next(5) as usize]))
                .collect();
            let mut proposal = preimage.clone();
            for _ in 0..next(6) {
                let position = next(proposal.len() as u64 + 1) as usize;
                match next(3) {
                    0 => proposal.insert(position, format!("n{}\n", next(4))),
                    1 if position < proposal.len() => {
                        proposal.remove(position);
                    }
                    _ if position < proposal.len() => {
                        proposal[position] = format!("r{}\n", next(4))
                    }
                    _ => {}
                }
            }
            let (before, after) = (preimage.concat(), proposal.concat());
            let change = HunkedTextChange::new(before.as_bytes(), after.as_bytes()).unwrap();
            assert_eq!(
                change
                    .select(before.as_bytes(), &all_ids(&change))
                    .unwrap()
                    .postimage(),
                after.as_bytes()
            );
            assert_eq!(
                change
                    .select(before.as_bytes(), &BTreeSet::new())
                    .unwrap()
                    .postimage(),
                before.as_bytes()
            );
            let mask = next(1 << change.hunks().len().min(16)) as u32;
            let selected = change
                .select(before.as_bytes(), &subset(&change, mask))
                .unwrap();
            let mut expected = Vec::new();
            let mut cursor = 0;
            for (index, hunk) in change.hunks().iter().enumerate() {
                let (old_start, old_lines) = hunk.old_range();
                let (new_start, new_lines) = hunk.new_range();
                expected.extend_from_slice(&preimage[cursor..old_start]);
                if index < 16 && mask & (1 << index) != 0 {
                    expected.extend_from_slice(&proposal[new_start..new_start + new_lines]);
                } else {
                    expected.extend_from_slice(&preimage[old_start..old_start + old_lines]);
                }
                cursor = old_start + old_lines;
            }
            expected.extend_from_slice(&preimage[cursor..]);
            assert_eq!(selected.postimage(), expected.concat().as_bytes());
        }
    }
}
