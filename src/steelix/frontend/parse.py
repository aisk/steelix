"""Read Python source and build the `ast` tree."""

from __future__ import annotations

import ast
import importlib.util
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


def read_source(path: Path) -> str:
    """The text of a Python source file, decoded like the import system
    does: UTF-8 unless a coding cookie says otherwise, universal newlines."""
    return importlib.util.decode_source(path.read_bytes())


def parse_file(path: Path) -> ParsedModule:
    source = read_source(path)
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
