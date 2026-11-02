"""Read Python source and build the `ast` tree."""

from __future__ import annotations

import ast
import io
import tokenize
from dataclasses import dataclass, field
from pathlib import Path


@dataclass
class ParsedModule:
    """One Python module ready for analysis."""

    name: str
    """Module name as used by `import`, e.g. `fib`."""
    filename: str
    """File name used in diagnostics and source location comments."""
    source: str
    tree: ast.Module
    comments: dict[int, str] = field(default_factory=dict)
    """Comments by line number, kept for later source mapping."""
    path: Path | None = None
    """The file as it was named on the command line, if read from disk."""


def parse_source(source: str, name: str, filename: str | None = None) -> ParsedModule:
    filename = filename or f"{name}.py"
    tree = ast.parse(source, filename=filename)
    return ParsedModule(name, filename, source, tree, _collect_comments(source))


def parse_file(path: Path) -> ParsedModule:
    source = path.read_text(encoding="utf-8")
    parsed = parse_source(source, path.stem, path.name)
    parsed.path = path
    return parsed


def _collect_comments(source: str) -> dict[int, str]:
    comments: dict[int, str] = {}
    tokens = tokenize.generate_tokens(io.StringIO(source).readline)
    for tok in tokens:
        if tok.type == tokenize.COMMENT:
            comments[tok.start[0]] = tok.string.removeprefix("#").strip()
    return comments
