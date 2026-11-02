"""Render Rust AST nodes into source text.

The output is valid Rust but only roughly laid out; `fmt.rustfmt` takes care of
the final formatting.
"""

from __future__ import annotations

import math

from steelix.rustast import nodes as rs

INDENT = "    "

# Precedence levels, higher binds tighter.
_BLOCKLIKE = 0
_OR = 1
_AND = 2
_COMPARE = 3
_UNARY = 4
_POSTFIX = 5

_BINARY_PRECEDENCE = {"||": _OR, "&&": _AND}


def print_file(file: rs.File) -> str:
    out: list[str] = []
    for line in file.inner_doc:
        out.append(f"//! {line}".rstrip())
    for attr in file.inner_attrs:
        out.append(f"#![{attr}]")
    if out:
        out.append("")
    previous: rs.Item | None = None
    for item in file.items:
        # Group consecutive `use` / `mod` lines, separate everything else.
        if previous is not None and not (
            type(item) is type(previous) and isinstance(item, rs.Use | rs.ModDecl)
        ):
            out.append("")
        out.append(print_item(item))
        previous = item
    return "\n".join(out) + "\n"


def print_item(item: rs.Item) -> str:
    match item:
        case rs.Use(path):
            return f"use {path};"
        case rs.ModDecl(name, public):
            return f"{'pub ' if public else ''}mod {name};"
        case rs.Fn():
            return _print_fn(item)
    raise TypeError(f"unknown item {item!r}")


def _print_fn(fn: rs.Fn) -> str:
    lines: list[str] = []
    lines += [f"// {c}".rstrip() for c in fn.comments]
    lines += [f"/// {d}".rstrip() for d in fn.doc]
    lines += [f"#[{a}]" for a in fn.attrs]
    generics = f"<{', '.join(fn.lifetimes)}>" if fn.lifetimes else ""
    params = ", ".join(f"{p.name}: {p.ty}" for p in fn.params)
    vis = "pub " if fn.public else ""
    lines.append(f"{vis}fn {fn.name}{generics}({params}) -> {fn.ret} {print_block(fn.body, 0)}")
    return "\n".join(lines)


def print_block(block: rs.Block, level: int) -> str:
    inner = level + 1
    lines = ["{"]
    for stmt in block.stmts:
        lines.append(INDENT * inner + print_stmt(stmt, inner))
    if block.tail is not None:
        lines.append(INDENT * inner + print_expr(block.tail))
    lines.append(INDENT * level + "}")
    return "\n".join(lines)


def print_stmt(stmt: rs.Stmt, level: int) -> str:
    match stmt:
        case rs.Let(name, mutable, ty, init):
            text = "let " + ("mut " if mutable else "") + name
            if ty is not None:
                text += f": {ty}"
            if init is not None:
                text += f" = {print_expr(init)}"
            return text + ";"
        case rs.Assign(target, value):
            return f"{print_expr(target)} = {print_expr(value)};"
        case rs.ExprStmt(expr):
            return f"{print_expr(expr)};"
        case rs.Return(value):
            return "return;" if value is None else f"return {print_expr(value)};"
        case rs.If():
            return _print_if(stmt, level)
        case rs.While(cond, body):
            return f"while {print_expr(cond)} {print_block(body, level)}"
        case rs.Loop(body):
            return f"loop {print_block(body, level)}"
        case rs.For(pat, iter_, body):
            return f"for {pat} in {print_expr(iter_)} {print_block(body, level)}"
        case rs.Break(label):
            return "break;" if label is None else f"break {label};"
        case rs.Continue():
            return "continue;"
        case rs.LabeledBlock(label, body):
            return f"{label}: {print_block(body, level)}"
        case rs.Comment(text):
            return f"// {text}".rstrip()
    raise TypeError(f"unknown statement {stmt!r}")


def _print_if(stmt: rs.If, level: int) -> str:
    text = f"if {print_expr(stmt.cond)} {print_block(stmt.then, level)}"
    match stmt.orelse:
        case None:
            return text
        case rs.If():
            return f"{text} else {_print_if(stmt.orelse, level)}"
        case rs.Block():
            return f"{text} else {print_block(stmt.orelse, level)}"
    raise TypeError(f"unknown else branch {stmt.orelse!r}")


def print_expr(expr: rs.Expr, level: int = 0) -> str:
    match expr:
        case rs.Path(text) | rs.Raw(text):
            return text
        case rs.IntLit(value, suffix):
            return f"{value}{suffix}"
        case rs.FloatLit(value):
            return _float(value)
        case rs.StrLit(value):
            return string_literal(value)
        case rs.ByteStrLit(value):
            return byte_string_literal(value)
        case rs.BoolLit(value):
            return "true" if value else "false"
        case rs.Call(func, args):
            return f"{_operand(func, _POSTFIX)}({_args(args)})"
        case rs.MethodCall(receiver, method, args):
            return f"{_operand(receiver, _POSTFIX)}.{method}({_args(args)})"
        case rs.MacroCall(path, args):
            return f"{path}!({_args(args)})"
        case rs.Try(inner):
            return f"{_operand(inner, _POSTFIX)}?"
        case rs.Ref(inner):
            return f"&{_operand(inner, _UNARY)}"
        case rs.Tuple(elems):
            if len(elems) == 1:
                return f"({print_expr(elems[0])},)"
            return f"({_args(elems)})"
        case rs.Array(elems):
            return f"[{_args(elems)}]"
        case rs.Unary(op, inner):
            return f"{op}{_operand(inner, _UNARY)}"
        case rs.Binary(op, lhs, rhs):
            prec = _BINARY_PRECEDENCE.get(op, _COMPARE)
            # Comparisons do not chain in Rust, so both sides bind tighter.
            right = prec + 1
            left = prec if prec != _COMPARE else prec + 1
            # A block on the right needs no parentheses: `a && { .. }`.
            rhs_text = (
                print_expr(rhs)
                if isinstance(rhs, rs.BlockExpr | rs.IfExpr)
                else _operand(rhs, right)
            )
            return f"{_operand(lhs, left)} {op} {rhs_text}"
        case rs.BlockExpr(block):
            return print_block(block, level)
        case rs.IfExpr(cond, then, orelse):
            text = f"if {print_expr(cond)} {print_block(then, level)} else "
            if isinstance(orelse, rs.IfExpr):
                return text + print_expr(orelse, level)
            return text + print_block(orelse, level)
    raise TypeError(f"unknown expression {expr!r}")


def _args(args: tuple[rs.Expr, ...]) -> str:
    return ", ".join(print_expr(a) for a in args)


def _precedence(expr: rs.Expr) -> int:
    match expr:
        case rs.BlockExpr() | rs.IfExpr():
            return _BLOCKLIKE
        case rs.Binary(op):
            return _BINARY_PRECEDENCE.get(op, _COMPARE)
        case rs.Ref() | rs.Unary():
            return _UNARY
        case rs.IntLit(value) if value < 0:
            return _UNARY
        case rs.FloatLit(value) if math.copysign(1.0, value) < 0:
            return _UNARY
    return _POSTFIX


def _operand(expr: rs.Expr, needed: int) -> str:
    text = print_expr(expr)
    return f"({text})" if _precedence(expr) < needed else text


def _float(value: float) -> str:
    if math.isnan(value):
        return "f64::NAN"
    if math.isinf(value):
        return "f64::INFINITY" if value > 0 else "f64::NEG_INFINITY"
    # `repr` gives the shortest string that round-trips, which Rust parses to
    # the same f64. It always contains `.` or `e`, so Rust reads it as a float.
    return repr(value)


_ESCAPES = {"\\": "\\\\", '"': '\\"', "\n": "\\n", "\r": "\\r", "\t": "\\t", "\0": "\\0"}


def string_literal(value: str) -> str:
    """A Rust `&str` literal. `value` must not contain lone surrogates."""
    out = []
    for ch in value:
        if ch in _ESCAPES:
            out.append(_ESCAPES[ch])
        elif ch.isprintable() or ch == " ":
            out.append(ch)
        else:
            out.append(f"\\u{{{ord(ch):x}}}")
    return '"' + "".join(out) + '"'


def byte_string_literal(value: bytes) -> str:
    out = []
    for byte in value:
        ch = chr(byte)
        if ch in _ESCAPES:
            out.append(_ESCAPES[ch])
        elif 0x20 <= byte < 0x7F:
            out.append(ch)
        else:
            out.append(f"\\x{byte:02x}")
    return 'b"' + "".join(out) + '"'
