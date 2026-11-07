// SPDX-License-Identifier: Apache-2.0
//! Conservative adapters for observed project test output. No runner or authority.

use super::*;
use std::collections::BTreeMap;

#[derive(Default)]
struct Counts {
    passed: u32,
    failed: u32,
    skipped: u32,
    names: BTreeSet<String>,
}

fn number(s: &str) -> Option<u32> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}

fn seconds(s: &str) -> Option<()> {
    let (whole, fraction) = s.split_once('.')?;
    number(whole)?;
    if fraction.is_empty() || !fraction.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some(())
}

impl Counts {
    fn event(&mut self, name: &str, status: &str) -> Option<()> {
        if !valid_test_name(name) || !self.names.insert(name.into()) {
            return None;
        }
        let count = match status {
            "ok" => &mut self.passed,
            "failed" => &mut self.failed,
            "skipped" => &mut self.skipped,
            _ => return None,
        };
        *count = count.checked_add(1)?;
        Some(())
    }

    fn total(&self) -> Option<u32> {
        self.passed
            .checked_add(self.failed)?
            .checked_add(self.skipped)
    }
}

pub(super) fn parse(
    template: &ValidationTemplate,
    command: &CommandReceipt,
    observation: &ValidationObservation,
) -> Option<NormalizedFields> {
    if !template.kind.is_test() || !observation.artifacts.is_empty() {
        return None;
    }
    // Cargo reports compilation on stderr, before libtest can produce events.
    // A recognized compiler summary can never coexist with a test pass. Keep
    // this narrow: mixed/partial test output and other Cargo failures refuse.
    if template.parser == ValidationParserKind::RustLibtestTextV1
        && std::str::from_utf8(&observation.stderr)
            .ok()?
            .lines()
            .any(|line| line.starts_with("error: could not compile "))
    {
        if !observation.stdout.is_empty()
            || command.exit_code != Some(101)
            || !cargo_compile_failure(&observation.stderr)
        {
            return None;
        }
        return Some(empty_fields(ValidationStatus::CompileFailed));
    }
    let bytes = match template.parser {
        ValidationParserKind::RustLibtestTextV1 => &observation.stdout,
        ValidationParserKind::PythonUnittestTextV1 => &observation.stderr,
        _ => return None,
    };
    if bytes.is_empty() || bytes.len() > MAX_RESULT_BYTES || !bytes.ends_with(b"\n") {
        return None;
    }
    let source = std::str::from_utf8(bytes).ok()?;
    // CR, ANSI, non-ASCII names and partial final lines are not admitted by
    // these text protocols. Raw channels remain hash-bound by the normalizer.
    if !source.is_ascii() || source.contains('\r') {
        return None;
    }
    let (counts, failed_names) = match template.parser {
        ValidationParserKind::RustLibtestTextV1 => rust(source)?,
        ValidationParserKind::PythonUnittestTextV1 => python(source)?,
        _ => return None,
    };
    if failed_names.len() > MAX_FAILED_NAMES {
        return None;
    }
    let status = if counts.failed > 0 {
        if command.exit_code.is_none_or(|exit| exit == 0) {
            return None;
        }
        ValidationStatus::AssertionFailed
    } else {
        if command.exit_code != Some(0) {
            return None;
        }
        if counts.passed == 0 && counts.skipped > 0 {
            ValidationStatus::SkippedOnly
        } else if counts.passed < template.minimum_test_count {
            ValidationStatus::ZeroTests
        } else {
            ValidationStatus::Passed
        }
    };
    Some(NormalizedFields {
        status,
        passed: counts.passed,
        failed: counts.failed,
        skipped: counts.skipped,
        failed_names,
        retry_count: 0,
        initial_failure_sha256: None,
        reported_duration_ms: None,
    })
}

fn cargo_compile_failure(bytes: &[u8]) -> bool {
    fn summary(line: &str) -> Option<()> {
        let rest = line.strip_prefix("error: could not compile `")?;
        let (package, rest) = rest.split_once('`')?;
        if package.is_empty()
            || package.len() > 128
            || !package
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
        {
            return None;
        }
        let rest = rest.strip_prefix(" (")?;
        let (target, rest) = rest.split_once(") due to ")?;
        if !matches!(target, "lib" | "lib test" | "build script") {
            let target = target.strip_suffix(" test").unwrap_or(target);
            let (kind, name) = target.split_once(" \"")?;
            let name = name.strip_suffix('"')?;
            if !matches!(kind, "bin" | "example" | "test" | "bench")
                || name.is_empty()
                || name.len() > 128
                || !name
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
            {
                return None;
            }
        }
        let (errors, rest) = rest.split_once(" previous error")?;
        let errors = number(errors)?;
        if errors == 0 {
            return None;
        }
        let rest = if errors == 1 {
            rest
        } else {
            rest.strip_prefix('s')?
        };
        if !rest.is_empty() {
            let (warnings, suffix) = rest.strip_prefix("; ")?.split_once(' ')?;
            let warnings = number(warnings)?;
            if warnings == 0
                || suffix
                    != if warnings == 1 {
                        "warning emitted"
                    } else {
                        "warnings emitted"
                    }
            {
                return None;
            }
        }
        Some(())
    }
    if bytes.is_empty() || bytes.len() > MAX_RESULT_BYTES || !bytes.ends_with(b"\n") {
        return false;
    }
    let Ok(source) = std::str::from_utf8(bytes) else {
        return false;
    };
    if source.contains('\r') {
        return false;
    }
    let lines = source.lines().filter(|s| !s.is_empty()).collect::<Vec<_>>();
    let Some(last) = lines.last() else {
        return false;
    };
    if summary(last).is_none()
        || lines
            .iter()
            .filter(|s| s.starts_with("error: could not compile "))
            .count()
            != 1
    {
        return false;
    }
    // Require a separate rustc diagnostic, not just a Cargo termination label.
    lines[..lines.len() - 1].iter().any(|line| {
        line.strip_prefix("error[E").is_some_and(|rest| {
            rest.split_once("]: ").is_some_and(|(code, message)| {
                code.len() == 4 && code.bytes().all(|b| b.is_ascii_digit()) && !message.is_empty()
            })
        }) || line
            .strip_prefix("error: ")
            .is_some_and(|message| !message.is_empty())
    })
}

fn rust(source: &str) -> Option<(Counts, Vec<String>)> {
    let mut active: Option<(u32, Counts)> = None;
    let mut total = Counts::default();
    let mut failures = BTreeSet::new();
    let mut group = 0_u32;
    let mut diagnostics = false;
    for line in source.lines().filter(|line| !line.is_empty()) {
        if let Some(rest) = line.strip_prefix("running ") {
            if active.is_some() {
                return None;
            }
            let count = rest
                .strip_suffix(" tests")
                .or_else(|| rest.strip_suffix(" test"))?;
            active = Some((number(count)?, Counts::default()));
            group = group.checked_add(1)?;
            diagnostics = false;
        } else if let Some(rest) = line.strip_prefix("test result: ") {
            let (expected, counts) = active.take()?;
            let (status, rest) = rest.split_once(". ")?;
            if status != if counts.failed > 0 { "FAILED" } else { "ok" } {
                return None;
            }
            let (summary, duration) = rest.split_once("; finished in ")?;
            seconds(duration.strip_suffix('s')?)?;
            let fields = summary.split("; ").collect::<Vec<_>>();
            if fields.len() != 5
                || number(fields[0].strip_suffix(" passed")?)? != counts.passed
                || number(fields[1].strip_suffix(" failed")?)? != counts.failed
                || number(fields[2].strip_suffix(" ignored")?)? != counts.skipped
                || fields[3] != "0 measured"
                || fields[4] != "0 filtered out"
                || expected != counts.total()?
            {
                return None;
            }
            total.passed = total.passed.checked_add(counts.passed)?;
            total.failed = total.failed.checked_add(counts.failed)?;
            total.skipped = total.skipped.checked_add(counts.skipped)?;
        } else if let Some((_, counts)) = active.as_mut() {
            if line == "failures:" && counts.failed > 0 {
                diagnostics = true;
            } else if !diagnostics {
                let (name, status) = line.strip_prefix("test ")?.rsplit_once(" ... ")?;
                let status = match status {
                    "ok" => "ok",
                    "FAILED" => "failed",
                    "ignored" => "skipped",
                    _ => return None,
                };
                counts.event(name, status)?;
                if status == "failed" {
                    let name = format!("libtest[{group}]::{name}");
                    if !valid_test_name(&name) {
                        return None;
                    }
                    failures.insert(name);
                }
            }
        } else {
            return None;
        }
    }
    if active.is_some() || group == 0 || failures.len() != total.failed as usize {
        return None;
    }
    Some((total, failures.into_iter().collect()))
}

fn python(source: &str) -> Option<(Counts, Vec<String>)> {
    let mut lines = source.lines().filter(|line| !line.is_empty()).peekable();
    let mut counts = Counts::default();
    let mut failures = BTreeSet::new();
    // Verbose per-test events precede tracebacks and the final summary.
    while let Some(line) = lines.peek().copied() {
        if line == "----------------------------------------------------------------------"
            || line == "======================================================================"
        {
            break;
        }
        let (name, status) = lines.next()?.rsplit_once(" ... ")?;
        if !name.contains(" (") || !name.ends_with(')') {
            return None;
        }
        let status = match status {
            "ok" => "ok",
            "FAIL" | "ERROR" => "failed",
            value if value.starts_with("skipped '") && value.ends_with('\'') => "skipped",
            _ => return None,
        };
        counts.event(name, status)?;
        if status == "failed" {
            failures.insert(name.to_owned());
        }
    }
    // Diagnostics are untrusted prose. Their structured headers must match the
    // observed failed tests; a second runner or nested summary refuses.
    let mut headers = BTreeSet::new();
    let separator = "----------------------------------------------------------------------";
    let heading = "======================================================================";
    let mut next = lines.next()?;
    while next == heading {
        let header = lines.next()?;
        let name = header
            .strip_prefix("FAIL: ")
            .or_else(|| header.strip_prefix("ERROR: "))?;
        if !failures.contains(name)
            || !headers.insert(name.to_owned())
            || lines.next()? != separator
        {
            return None;
        }
        while let Some(line) = lines.peek().copied() {
            if line == heading || line == separator {
                break;
            }
            if line.starts_with("Ran ") || line.starts_with("OK") || line.starts_with("FAILED") {
                return None;
            }
            lines.next();
        }
        next = lines.next()?;
    }
    if next != separator {
        return None;
    }
    if headers != failures {
        return None;
    }
    let ran = lines.next()?.strip_prefix("Ran ")?;
    let (ran, duration) = ran.split_once(" in ")?;
    let ran = ran
        .strip_suffix(" tests")
        .or_else(|| ran.strip_suffix(" test"))?;
    if number(ran)? != counts.total()? {
        return None;
    }
    seconds(duration.strip_suffix('s')?)?;
    let outcome = lines.next()?;
    let expected_status = if counts.failed > 0 { "FAILED" } else { "OK" };
    let mut summary = BTreeMap::new();
    if outcome != expected_status {
        let details = outcome
            .strip_prefix(expected_status)?
            .strip_prefix(" (")?
            .strip_suffix(')')?;
        for entry in details.split(", ") {
            let (key, value) = entry.split_once('=')?;
            if !matches!(key, "failures" | "errors" | "skipped")
                || summary.insert(key, number(value)?).is_some()
            {
                return None;
            }
        }
    }
    if summary
        .get("failures")
        .copied()
        .unwrap_or(0)
        .checked_add(summary.get("errors").copied().unwrap_or(0))?
        != counts.failed
        || summary.get("skipped").copied().unwrap_or(0) != counts.skipped
        || lines.next().is_some()
    {
        return None;
    }
    Some((counts, failures.into_iter().collect()))
}
