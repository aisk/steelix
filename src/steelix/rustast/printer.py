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
                text += f" = {print_expr(init, level)}"
            return text + ";"
        case rs.Assign(target, value):
            return f"{print_expr(target)} = {print_expr(value)};"
        case rs.ExprStmt(rs.Match() as match):
            # A `match` statement needs no semicolon.
            return print_expr(match, level)
        case rs.ExprStmt(expr):
            return f"{print_expr(expr, level)};"
        case rs.Return(value):
            return "return;" if value is None else f"return {print_expr(value, level)};"
        case rs.If():
            return _print_if(stmt, level)
        case rs.While(cond, body, label):
            return f"{_label(label)}while {print_expr(cond)} {print_block(body, level)}"
        case rs.Loop(body, label):
            return f"{_label(label)}loop {print_block(body, level)}"
        case rs.For(pat, iter_, body, label):
            head = f"{_label(label)}for {pat} in {print_expr(iter_)}"
            return f"{head} {print_block(body, level)}"
        case rs.Break(label):
            return "break;" if label is None else f"break {label};"
        case rs.Continue(label):
            return "continue;" if label is None else f"continue {label};"
        case rs.LabeledBlock(label, body):
            return f"{label}: {print_block(body, level)}"
        case rs.Comment(text):
            return f"// {text}".rstrip()
        case rs.ItemStmt(item):
            return print_item(item)
    raise TypeError(f"unknown statement {stmt!r}")


def _label(label: str | None) -> str:
    return "" if label is None else f"{label}: "


_MACRO_LINE = 70


def _macro_call(path: str, args: tuple[rs.Expr, ...]) -> str:
    """rustfmt leaves alone macro calls whose arguments do not parse as Rust
    expressions, like the signatures in `rt::function!`, so long ones are
    broken into one argument per line here. rustfmt fixes the indentation
    and joins the lines again when it can format the call itself."""
    text = f"{path}!({_args(args)})"
    if len(text) <= _MACRO_LINE or "\n" in text:
        return text
    args_text = f",\n{INDENT}".join(print_expr(arg) for arg in args)
    return f"{path}!(\n{INDENT}{args_text}\n)"


def _print_if(stmt: rs.If, level: int) -> str:
    cond = print_expr(stmt.cond, level)
    if stmt.pattern is not None:
        cond = f"let {stmt.pattern} = {cond}"
    text = f"if {cond} {print_block(stmt.then, level)}"
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
        case rs.Path(text):
            return text
        case rs.Tokens(parts):
            return "".join(p if isinstance(p, str) else print_expr(p) for p in parts)
        case rs.IntLit(value, suffix):
            return f"{value}{suffix}"
        case rs.FloatLit(value):
            return _float(value)
        case rs.StrLit(value):
            return string_literal(value)
        case rs.RawStrLit(value):
            return raw_string_literal(value)
        case rs.ByteStrLit(value):
            return byte_string_literal(value)
        case rs.BoolLit(value):
            return "true" if value else "false"
        case rs.Call(func, args):
            return f"{_operand(func, _POSTFIX)}({_args(args)})"
        case rs.MethodCall(receiver, method, args):
            return f"{_operand(receiver, _POSTFIX)}.{method}({_args(args)})"
        case rs.MacroCall(path, args):
            return _macro_call(path, args)
        case rs.Try(inner):
            return f"{_operand(inner, _POSTFIX)}?"
        case rs.Ref(inner, mutable):
            return f"&{'mut ' if mutable else ''}{_operand(inner, _UNARY)}"
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
        case rs.Closure(params, body, ret):
            head = f"|{', '.join(params)}|"
            if ret is not None:
                return f"{head} -> {ret} {print_block(body, level)}"
            if not body.stmts and body.tail is not None:
                return f"{head} {print_expr(body.tail, level)}"
            return f"{head} {print_block(body, level)}"
        case rs.Match(scrutinee, arms):
            inner = INDENT * (level + 1)
            lines = [f"match {print_expr(scrutinee, level)} {{"]
            for arm in arms:
                head = arm.pattern
                if arm.guard is not None:
                    head += f" if {print_expr(arm.guard, level + 1)}"
                lines.append(f"{inner}{head} => {_arm_body(arm.body, level + 1)}")
            lines.append(INDENT * level + "}")
            return "\n".join(lines)
    raise TypeError(f"unknown expression {expr!r}")


def _arm_body(body: rs.Block, level: int) -> str:
    """A jump or a single expression without braces, else a block."""
    if len(body.stmts) == 1 and body.tail is None:
        match body.stmts[0]:
            case rs.Return() | rs.Break() | rs.Continue() | rs.Assign() as simple:
                # `Ok(value) => x = value,`: an assignment is an expression too.
                return print_stmt(simple, level).removesuffix(";") + ","
            case rs.ExprStmt(rs.MacroCall("unreachable") as never):
                return print_expr(never, level) + ","
    if not body.stmts and body.tail is not None:
        return print_expr(body.tail, level) + ","
    return print_block(body, level)


def _args(args: tuple[rs.Expr, ...]) -> str:
    return ", ".join(print_expr(a) for a in args)


def _precedence(expr: rs.Expr) -> int:
    match expr:
        case rs.BlockExpr() | rs.IfExpr() | rs.Closure() | rs.Match():
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


def raw_string_literal(value: str) -> str:
    """A Rust raw string literal, with as many `#` as it takes to hold `value`."""
    if "\r" in value:
        raise ValueError("a raw string literal can not hold a carriage return")
    hashes = "#"
    while '"' + hashes in value:
        hashes += "#"
    return f'r{hashes}"{value}"{hashes}'
