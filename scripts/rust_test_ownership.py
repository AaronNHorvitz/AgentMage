"""Exact test-only ownership of whole Rust source files for developer scans.

Not a Rust parser or permission boundary. A file is test-only only when it is
referenced at least once and every recognized reference to it comes from
test-only code: an inline test module, an exact `#[cfg(test)]` module
declaration, or another test-only file. Everything else stays production, so
an unrecognized form causes over-scanning, never an exemption. Any reference
this module cannot resolve with certainty disables every file exemption.
The native compiler and separate closed-boundary checks remain required.
"""

from __future__ import annotations

import re
from pathlib import Path
from typing import Iterable, Mapping

from scripts.rust_source_audit import _code_mask


# The shared audit pattern plus an optional visibility qualifier, which does
# not change a `cfg(test)` module's compilation. Kept local so the strict-local
# and effect-boundary scans keep their existing exemption set.
TEST_MODULE = re.compile(
    r"(?m)^#\[cfg\((?:test|all\(test,\s*target_os\s*=\s+\))\)\]"
    r"\s*\n(?:pub(?:\((?:crate|super)\))?[ \t]+)?mod\s+[A-Za-z_][A-Za-z_0-9]*\s*\{"
)
ATTRIBUTE = re.compile(r"#\[[^\[\]\n]*\]")
DECLARATION = re.compile(
    r"(?P<attributes>(?:#\[[^\[\]\n]*\][ \t]*\n?[ \t]*)*)"
    r"(?<![A-Za-z0-9_])(?:pub(?:\([a-z]+\))?[ \t]+)?"
    r"(?P<keyword>mod)[ \t]+(?P<name>[A-Za-z_][A-Za-z_0-9]*)[ \t]*;"
)
PATH_ATTRIBUTE = re.compile(r'#\[path[ \t]*=[ \t]*"(?P<value>[^"\n]*)"\]')
INCLUDE = re.compile(r'include!\([ \t]*"(?P<value>[^"\n]*)"[ \t]*\)')
PLAIN_FILE = re.compile(r"[A-Za-z0-9_]+\.rs")
# Every such token in code must belong to a recognized reference.
FILE_MODULE_TOKEN = re.compile(r"(?<![A-Za-z0-9_])mod\s+[A-Za-z_][A-Za-z_0-9]*\s*;")
INCLUDE_TOKEN = re.compile(r"(?<![A-Za-z0-9_])include!")
PATH_TOKEN = re.compile(r"#\[\s*path(?![A-Za-z0-9_])")
MOD_RS_FILES = frozenset({"lib.rs", "main.rs", "mod.rs"})


class AmbiguousReference(ValueError):
    """A module or include reference could not be resolved with certainty."""


def _test_module_spans(mask: str) -> list[tuple[int, int]] | None:
    """Complete inline test modules; None when one is unbalanced."""
    spans: list[tuple[int, int]] = []
    cursor = 0
    for module in TEST_MODULE.finditer(mask):
        if module.start() < cursor:
            continue
        index = module.end()
        depth = 1
        while index < len(mask) and depth:
            depth += (mask[index] == "{") - (mask[index] == "}")
            index += 1
        if depth:
            return None
        spans.append((module.start(), index))
        cursor = index
    return spans


def _depth(mask: str, offset: int) -> int:
    return mask.count("{", 0, offset) - mask.count("}", 0, offset)


def _is_code(source: str, mask: str, offset: int) -> bool:
    return mask[offset] == source[offset] and not mask[offset].isspace()


def _references(path: Path, source: str) -> list[tuple[Path, bool]]:
    """Return (target, test_context) pairs for one file's references."""
    mask = _code_mask(source)
    if mask is None:
        raise AmbiguousReference(f"{path}: malformed lexical source")
    spans = _test_module_spans(mask)
    if spans is None:
        raise AmbiguousReference(f"{path}: unbalanced test module")

    def in_test_module(offset: int) -> bool:
        return any(start <= offset < end for start, end in spans)

    references: list[tuple[Path, bool]] = []
    declared_at: set[int] = set()
    path_attributes_at: set[int] = set()
    for match in DECLARATION.finditer(source):
        keyword = match.start("keyword")
        if not _is_code(source, mask, keyword):
            continue
        declared_at.add(keyword)
        attributes = list(ATTRIBUTE.finditer(match["attributes"]))
        start = match.start("attributes")
        path_attributes_at.update(
            start + item.start() for item in attributes if item.group(0).startswith("#[path")
        )
        if any(not _is_code(source, mask, start + item.start()) for item in attributes):
            raise AmbiguousReference(f"{path}: module attribute is not plain code")
        texts = [item.group(0) for item in attributes]
        paths = [PATH_ATTRIBUTE.fullmatch(text) for text in texts]
        declared = [value["value"] for value in paths if value]
        if any(text.startswith("#[path") for text, value in zip(texts, paths) if not value):
            raise AmbiguousReference(f"{path}: unrecognized path attribute")
        if len(declared) > 1:
            raise AmbiguousReference(f"{path}: repeated path attribute")
        depth = _depth(mask, keyword)
        if depth and not in_test_module(keyword):
            # Inline production modules change relative resolution; never guess it.
            raise AmbiguousReference(f"{path}: nested production module declaration")
        if declared:
            if not PLAIN_FILE.fullmatch(declared[0]):
                raise AmbiguousReference(f"{path}: non-sibling path attribute")
            target = path.parent / declared[0]
        elif depth:
            # Only reachable inside a test module; its default file is test-owned.
            target = path.parent / path.stem / f"{match['name']}.rs"
        elif path.name in MOD_RS_FILES:
            target = path.parent / f"{match['name']}.rs"
        else:
            target = path.parent / path.stem / f"{match['name']}.rs"
        test_context = in_test_module(keyword) or (depth == 0 and "#[cfg(test)]" in texts)
        references.append((target, test_context))
    included_at: set[int] = set()
    for match in INCLUDE.finditer(source):
        if not _is_code(source, mask, match.start()):
            continue
        if not PLAIN_FILE.fullmatch(match["value"]):
            raise AmbiguousReference(f"{path}: non-sibling include")
        included_at.add(match.start())
        references.append((path.parent / match["value"], in_test_module(match.start())))
    for pattern, recognized, kind in (
        (FILE_MODULE_TOKEN, declared_at, "module declaration"),
        (INCLUDE_TOKEN, included_at, "include"),
        (PATH_TOKEN, path_attributes_at, "path attribute"),
    ):
        for token in pattern.finditer(mask):
            if token.start() not in recognized:
                raise AmbiguousReference(f"{path}: unrecognized {kind}")
    return references


def test_only_files(sources: Mapping[Path, str]) -> frozenset[Path]:
    """Return files owned only by test code; raise when any reference is ambiguous."""
    edges: list[tuple[Path, Path, bool]] = []
    for path, source in sources.items():
        edges.extend((path, target, test) for target, test in _references(path, source))
    test_only: set[Path] = set()
    while True:
        referenced: dict[Path, bool] = {}
        for owner, target, test in edges:
            in_test = test or owner in test_only
            referenced[target] = referenced.get(target, True) and in_test
        found = {
            path for path, only_test in referenced.items() if only_test and path in sources
        }
        if found == test_only:
            return frozenset(test_only)
        test_only = found


def _production_text(source: str) -> str:
    """Blank complete inline test modules, keeping line numbers and trailing items."""
    mask = _code_mask(source)
    spans = None if mask is None else _test_module_spans(mask)
    if not spans:
        return source
    result: list[str] = []
    cursor = 0
    for start, end in spans:
        result.append(source[cursor:start])
        result.append("\n" * source[start:end].count("\n"))
        cursor = end
    result.append(source[cursor:])
    return "".join(result)


def production_sources(paths: Iterable[Path]) -> dict[Path, str]:
    """Map each path to its production text for lexical neutrality scans.

    Exactly owned test-only files map to an empty string. Other files keep all
    production items, including any after an inline test module. If ownership
    is ambiguous anywhere, no whole file is exempted.
    """
    sources = {path: path.read_text(encoding="utf-8") for path in paths}
    try:
        exempt = test_only_files(sources)
    except AmbiguousReference:
        exempt = frozenset()
    return {
        path: "" if path in exempt else _production_text(source)
        for path, source in sources.items()
    }
