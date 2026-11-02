"""Docstrings exactly as CPython stores them in `__doc__`."""

from __future__ import annotations

import ast

from steelix.diagnostics import SourceLocation, UnsupportedError


def clean_doc(doc: str) -> str:
    """Strip indentation the way the 3.13+ compiler does (`_PyCompile_CleanDoc`).

    Unlike `inspect.cleandoc`, blank lines at the start and end are kept, and
    the first line only loses its leading whitespace.
    """
    doc = doc.expandtabs()
    lines = doc.split("\n")
    margin = None
    for line in lines[1:]:
        stripped = line.lstrip(" ")
        if stripped:
            indent = len(line) - len(stripped)
            margin = indent if margin is None else min(margin, indent)
    margin = margin or 0
    out = [lines[0].lstrip(" ")]
    for line in lines[1:]:
        out.append(line[margin:] if len(line) > margin else "")
    return "\n".join(out)


def docstring(node: ast.FunctionDef | ast.Module, filename: str = "") -> str | None:
    raw = ast.get_docstring(node, clean=False)
    if raw is None:
        return None
    if any(0xD800 <= ord(ch) <= 0xDFFF for ch in raw):
        raise UnsupportedError("docstring with lone surrogates", SourceLocation.of(filename, node))
    return clean_doc(raw)
