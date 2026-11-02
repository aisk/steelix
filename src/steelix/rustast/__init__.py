"""A small Rust AST, only as rich as the generated code needs.

Planned modules:
    nodes.py    dataclasses for items, statements, expressions, types and paths
    printer.py  render nodes into Rust source text
    fmt.py      optional `rustfmt` pass over the rendered source
"""
