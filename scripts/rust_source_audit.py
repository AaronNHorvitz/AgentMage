"""Conservative lexical test-module exclusion for developer source audits.

Not a Rust parser or permission boundary. Keep all input on malformed lexical
material; never discard production items merely because a test module preceded
them. The native compiler and separate closed-boundary checks remain required.
"""

from __future__ import annotations

import re
from functools import lru_cache


# The optional exact target_os string is blank in the lexical mask. Both shapes
# require `test`; unknown boolean expressions remain scanned conservatively.
TEST_MODULE = re.compile(
    r"(?m)^#\[cfg\((?:test|all\(test,\s*target_os\s*=\s+\))\)\]"
    r"\s*\nmod\s+[A-Za-z_][A-Za-z_0-9]*\s*\{"
)
RAW_STRING = re.compile(r'(?:br|cr|r)(#{0,255})"')
CHARACTER = re.compile(r"'(?:\\(?:u\{[0-9a-fA-F_]+\}|x[0-9a-fA-F]{2}|[^\n])|[^'\\\n])'")


def _code_mask(source: str) -> str | None:
    """Blank strings/comments, preserving offsets and newlines for brace matching."""
    masked = list(source)
    index = 0
    while index < len(source):
        start = index
        if source.startswith("//", index):
            end = source.find("\n", index + 2)
            index = len(source) if end < 0 else end
        elif source.startswith("/*", index):
            index += 2
            depth = 1
            while index < len(source) and depth:
                if source.startswith("/*", index):
                    depth += 1
                    index += 2
                elif source.startswith("*/", index):
                    depth -= 1
                    index += 2
                else:
                    index += 1
            if depth:
                return None
        elif raw := RAW_STRING.match(source, index):
            closing = '"' + raw[1]
            end = source.find(closing, raw.end())
            if end < 0:
                return None
            index = end + len(closing)
        elif source[index] == '"':
            index += 1
            while index < len(source) and source[index] != '"':
                index += 2 if source[index] == "\\" else 1
            if index >= len(source):
                return None
            index += 1
        elif source[index] == "'" and (character := CHARACTER.match(source, index)):
            index = character.end()
        else:
            index += 1
            continue
        for position in range(start, index):
            if masked[position] != "\n":
                masked[position] = " "
    return "".join(masked)


@lru_cache(maxsize=1024)
def production_source(source: str) -> str:
    """Exclude complete known test-only modules; retain any trailing items."""
    mask = _code_mask(source)
    if mask is None:
        return source
    result: list[str] = []
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
            return source
        result.append(source[cursor:module.start()])
        result.append("\n" * source[module.start():index].count("\n"))
        cursor = index
    result.append(source[cursor:])
    return "".join(result)
