"""Lower `match` statements (design §8).

Each `case` becomes an `if` whose condition is a let chain: one link per
level of the pattern, in the order CPython checks them, so that evaluation
order and short-circuiting are CPython's. A runtime helper fetches all the
values of one level at once, like the instruction CPython uses for it
(`rt::match_seq` for `UNPACK_SEQUENCE`, `rt::match_map` for `MATCH_KEYS`,
`rt::match_class` for `MATCH_CLASS`), and the links after it check what is
inside:

    if let Some([item, direction]) = rt::match_seq(&subject)?
        && item.eq("go")?
        && exits.contains(&direction)?
    { .. }

The cases are an `if`/`else if` chain. A case whose captures are stored
(module level names, or locals used outside the case) before a guard that
may fail needs code between the pattern and the guard, so then every case
is an `if` of its own in a labeled block, left with `break 'match_`.

Captures are stored once the whole pattern matched, before the guard, in
CPython's order. A capture used only in the guard and body of the cases
binding it (`LocalsInfo.case_scoped`) is the variable the chain binds.
"""

from __future__ import annotations

import ast
from dataclasses import dataclass, field
from typing import TYPE_CHECKING

from steelix.frontend.patterns import pattern_captures
from steelix.lower.comprehension import unpack_pattern
from steelix.lower.expr import Kind, Value
from steelix.lower.rustflow import diverges
from steelix.rustast import nodes as rs

if TYPE_CHECKING:
    from steelix.lower.stmt import StmtLowering

# PyO3 converts tuples of up to this many items.
_MAX_TUPLE = 12


def _rt(name: str, *args: rs.Expr) -> rs.Expr:
    return rs.Try(rs.Call(rs.Path(f"rt::{name}"), args))


def _wildcard(pattern: ast.pattern) -> bool:
    match pattern:
        case ast.MatchAs(pattern=None, name=None) | ast.MatchStar(name=None):
            return True
    return False


@dataclass
class _Link:
    """One link of a let chain: a `rs.LetExpr` or a Rust `bool`."""

    expr: rs.Expr
    line: int
    """The line CPython reports for errors raised there."""


@dataclass
class _Pattern:
    """The lowered pattern of one case."""

    links: list[_Link] = field(default_factory=list)
    stores: list[tuple[str, Value]] = field(default_factory=list)
    """Captures stored once the pattern matched, in CPython's order."""
    lets: list[tuple[str, Value]] = field(default_factory=list)
    """Case scoped captures of a value the chain does not bind itself (the
    subject, `case x:`), by Rust name."""


class _PatternLowering:
    """Lower the pattern of one case into the links of its chain."""

    def __init__(self, lowering: StmtLowering, temps: list[str]) -> None:
        self.lowering = lowering
        self.ctx = lowering.ctx
        self.exprs = lowering.exprs
        self.out = _Pattern()
        self.temps = temps
        """Rust names bound by the chain, released after the case."""

    def temp(self, base: str) -> str:
        name = self.ctx.temp(base)
        self.temps.append(name)
        return name

    def link(self, expr: rs.Expr, node: ast.AST) -> None:
        self.out.links.append(_Link(expr, node.lineno))

    # --- names ---

    def scoped(self, name: str) -> bool:
        fn = self.ctx.function
        return fn is not None and name in fn.locals.case_scoped

    def binding(self, pattern: ast.pattern, base: str) -> str:
        """The Rust name the chain binds a value matched against `pattern`
        to: `_` for a wildcard, the local itself for a case scoped capture,
        else a temporary."""
        if _wildcard(pattern):
            return "_"
        name = pattern.name if isinstance(pattern, ast.MatchAs | ast.MatchStar) else None
        if name is not None:
            if self.scoped(name):
                fn = self.ctx.function
                assert fn is not None
                if isinstance(pattern, ast.MatchAs) and pattern.pattern is not None:
                    return fn.names[name]
                return fn.names[name] if self.lowering.case_reads(name) else "_"
            return self.temp(name)
        return self.temp(base)

    def capture(self, name: str, value: Value, bound: str | None) -> None:
        """`name` captures `value`, which the chain has bound to `bound`
        (or not, for the subject itself)."""
        if bound is not None:
            if not self.scoped(name):
                self.out.stores.append((name, value))
            return
        if self.scoped(name):
            if self.lowering.case_reads(name):
                fn = self.ctx.function
                assert fn is not None
                self.out.lets.append((fn.names[name], value))
        else:
            self.out.stores.append((name, value))

    # --- patterns ---

    def match(self, pattern: ast.pattern, subject: Value, bound: str | None = None) -> None:
        """The links checking `subject` against `pattern`. `bound` is the
        Rust name the chain bound `subject` to, if it did."""
        match pattern:
            case ast.MatchAs(sub, name):
                if sub is not None:
                    self.match(sub, subject)
                if name is not None:
                    self.capture(name, subject, bound)
            case ast.MatchStar(name):
                if name is not None:
                    self.capture(name, subject, bound)
            case ast.MatchValue(value):
                rhs = self.value(value)
                self.link(self.exprs.compare_bool(ast.Eq(), subject, rhs), pattern)
            case ast.MatchSingleton(value):
                rhs = self.exprs.constant(value, pattern)  # type: ignore[arg-type]
                self.link(self.exprs.compare_bool(ast.Is(), subject, rhs), pattern)
            case ast.MatchSequence(patterns):
                self.sequence(pattern, patterns, subject)
            case ast.MatchMapping(keys, patterns, rest):
                self.mapping(pattern, keys, patterns, rest, subject)
            case ast.MatchClass(cls, patterns, kwd_attrs, kwd_patterns):
                self.class_(pattern, cls, patterns, kwd_attrs, kwd_patterns, subject)
            case ast.MatchOr(alternatives):
                self.alternatives(pattern, alternatives, subject)
            case _:
                raise self.ctx.unsupported(type(pattern).__name__, pattern)

    def value(self, expr: ast.expr) -> Value:
        """A value pattern or mapping key: a literal (CPython folds `-1` and
        `1+2j`) or a dotted name, evaluated where the pattern reaches it."""
        if not isinstance(expr, ast.Constant | ast.Attribute | ast.Name):
            try:
                folded = ast.literal_eval(expr)
            except ValueError:
                pass
            else:
                return self.exprs.constant(folded, expr)
        with self.exprs.fresh() as prelude:
            value = self.exprs.value(expr)
        if prelude:
            block = rs.BlockExpr(rs.Block(tuple(prelude), value.owned()))
            return Value(block, Kind.OWNED)
        return value

    def element(self, pattern: ast.pattern, name: str) -> None:
        """Match the value the chain bound to `name` against `pattern`."""
        if name != "_":
            self.match(pattern, Value(rs.Path(name), Kind.PLACE, temp=True), name)

    def sequence(self, node: ast.pattern, patterns: list[ast.pattern], subject: Value) -> None:
        """A sequence pattern, with the strategy CPython's compiler picks for
        its shape (`codegen_pattern_sequence`): only wildcards check the
        length; with `*_` the other items are read by index; otherwise the
        subject is unpacked by iterating it."""
        size = len(patterns)
        star = next((i for i, p in enumerate(patterns) if isinstance(p, ast.MatchStar)), None)
        target = subject.ref()
        if all(_wildcard(p) for p in patterns):
            if star is None:
                self.link(_rt("match_seq_len", target, rs.IntLit(size)), node)
            else:
                self.link(_rt("match_seq_min", target, rs.IntLit(size - 1)), node)
            return
        if star is not None and _wildcard(patterns[star]):
            self.link(_rt("match_seq_min", target, rs.IntLit(size - 1)), node)
            for i, sub in enumerate(patterns):
                if _wildcard(sub):
                    continue
                name = self.binding(sub, "item")
                if i < star:
                    item = rs.Try(subject.method("get_item", rs.IntLit(i)))
                else:
                    item = _rt("item_from_end", target, rs.IntLit(size - i))
                self.link(rs.LetExpr(name, item), node)
                self.element(sub, name)
            return
        names = [self.binding(sub, "item") for sub in patterns]
        helper = "match_seq" if star is None else "match_seq_star"
        pattern = f"Some({unpack_pattern(names, star)})"
        self.link(rs.LetExpr(pattern, _rt(helper, target)), node)
        for sub, name in zip(patterns, names, strict=True):
            self.element(sub, name)

    def mapping(
        self,
        node: ast.pattern,
        keys: list[ast.expr],
        patterns: list[ast.pattern],
        rest: str | None,
        subject: Value,
    ) -> None:
        """A mapping pattern (`codegen_pattern_mapping`): a mapping with at
        least as many items as keys, then the keys (evaluated after that
        check), their values, and the subpatterns."""
        target = subject.ref()
        if not keys:
            self.link(_rt("match_mapping", target, rs.IntLit(0)), node)
        if keys:
            names = [
                self.binding(sub, _key_base(key)) for key, sub in zip(keys, patterns, strict=True)
            ]
            literal = all(_is_literal(key) for key in keys)
            if not literal:
                self.link(_rt("match_mapping", target, rs.IntLit(len(keys))), node)
            values = [self.value(key) for key in keys]
            if rest is not None and not literal:
                # Evaluated once, for the lookups and for `**rest`.
                held = []
                for key, value in zip(keys, values, strict=True):
                    if _is_literal(key):
                        held.append(value)
                        continue
                    name = self.temp("key")
                    self.link(rs.LetExpr(name, value.owned()), key)
                    held.append(Value(rs.Path(name), Kind.PLACE, temp=True))
                values = held
            key_tuple = _keys(values)
            pattern = f"Some([{', '.join(names)}])"
            helper = "match_map" if literal else "match_keys"
            self.link(rs.LetExpr(pattern, _rt(helper, target, key_tuple)), node)
            for sub, name in zip(patterns, names, strict=True):
                self.element(sub, name)
        else:
            key_tuple = rs.Tuple(())
        if rest is not None:
            rest_pattern = ast.MatchAs(None, rest)
            name = self.binding(rest_pattern, "rest")
            self.link(rs.LetExpr(name, _rt("match_rest", target, key_tuple)), node)
            self.capture(rest, Value(rs.Path(name), Kind.PLACE, temp=True), name)

    def class_(
        self,
        node: ast.pattern,
        cls: ast.expr,
        patterns: list[ast.pattern],
        kwd_attrs: list[str],
        kwd_patterns: list[ast.pattern],
        subject: Value,
    ) -> None:
        """A class pattern (`MATCH_CLASS`): the class is evaluated first,
        then all the attributes are fetched, then the subpatterns run."""
        with self.exprs.fresh() as prelude:
            cls_value = self.exprs.value(cls)
        assert not prelude, "a class pattern names a class"
        target = subject.ref()
        if not patterns and not kwd_patterns:
            self.link(_rt("match_instance", target, cls_value.ref()), node)
            return
        names = [self.binding(sub, "item") for sub in patterns]
        keywords = zip(kwd_attrs, kwd_patterns, strict=True)
        names += [self.binding(sub, attr) for attr, sub in keywords]
        attrs = rs.Array(tuple(rs.StrLit(attr) for attr in kwd_attrs))
        call = _rt("match_class", target, cls_value.ref(), rs.IntLit(len(patterns)), attrs)
        self.link(rs.LetExpr(f"Some([{', '.join(names)}])", call), node)
        for sub, name in zip([*patterns, *kwd_patterns], names, strict=True):
            self.element(sub, name)

    def alternatives(
        self, node: ast.pattern, alternatives: list[ast.pattern], subject: Value
    ) -> None:
        """An OR pattern without captures: each alternative as a Rust `bool`,
        tried in order. An alternative that needs to bind values on the way
        is a `matches!` with the rest of its links as the guard."""
        conds: list[rs.Expr] = []
        for alt in alternatives:
            inner = _PatternLowering(self.lowering, self.temps)
            inner.match(alt, subject)
            assert not inner.out.stores and not inner.out.lets, "OR patterns capture nothing"
            conds.append(_as_bool([link.expr for link in inner.out.links]))
        cond = conds[0]
        for other in conds[1:]:
            cond = rs.Binary("||", cond, other)
        self.link(cond, node)


def _as_bool(links: list[rs.Expr]) -> rs.Expr:
    """The links of a chain as one Rust `bool`: `let` links become
    `matches!(value, pattern if rest)`."""
    result: rs.Expr | None = None
    for link in reversed(links):
        if isinstance(link, rs.LetExpr):
            result = rs.Matches(link.value, link.pattern, result)
        else:
            result = link if result is None else rs.Binary("&&", link, result)
    assert result is not None, "an alternative checks something"
    return result


def _is_literal(key: ast.expr) -> bool:
    return not isinstance(key, ast.Attribute | ast.Name)


def _key_base(key: ast.expr) -> str:
    """A name for the value of a mapping key: the key if it is a word."""
    match key:
        case ast.Constant(str() as text) if text.isidentifier() and text.isascii():
            return text
        case ast.Attribute(_, attr):
            return attr.lower()
    return "value"


def _keys(values: list[Value]) -> rs.Expr:
    """The keys of a mapping pattern for `rt::match_map`: an array of string
    literals, else a tuple (or an array of objects, past PyO3's tuples)."""
    args = [value.arg() for value in values]
    if all(isinstance(arg, rs.StrLit) for arg in args):
        return rs.Array(tuple(args))
    if len(args) <= _MAX_TUPLE:
        return rs.Tuple(tuple(args))
    return rs.Array(tuple(value.owned() for value in values))


@dataclass
class _Case:
    links: list[rs.Expr]
    """The chain, with markers inside it."""
    before: list[rs.Stmt]
    """Line markers to put before the `if` of a case of its own."""
    stores: list[rs.Stmt]
    lets: list[rs.Stmt]
    guard: rs.Expr | None
    guard_before: list[rs.Stmt]
    """What the guard needs before it when it is an `if` of its own."""
    body: list[rs.Stmt]


class MatchLowering:
    """The part of `StmtLowering` for `match`."""

    def case_reads(self: StmtLowering, name: str) -> bool:
        """Whether the case being lowered reads the capture `name`."""
        case = self.cases[-1]
        parts: list[ast.AST] = [*case.body]
        if case.guard is not None:
            parts.append(case.guard)
        return any(
            isinstance(node, ast.Name) and node.id == name
            for part in parts
            for node in ast.walk(part)
        )

    def match_stmt(self: StmtLowering, stmt: ast.Match) -> list[rs.Stmt]:
        markers = self.markers
        subject = self.exprs.value(stmt.subject)
        items = self.flush_items()
        self.exprs.prelude[:0] = self.mark(
            stmt.subject.lineno, [*self.exprs.prelude, rs.ExprStmt(subject.expr)]
        )
        labeled = any(case.guard is not None and self.stores_captures(case) for case in stmt.cases)
        own: list[rs.Stmt] = []
        temp = None
        if not (
            subject.is_pure
            and isinstance(subject.expr, rs.Path)
            and subject.kind in (Kind.PLACE, Kind.REF)
            and not _rebound(stmt)
        ):
            temp = self.ctx.temp("subject")
            let = rs.Let(temp, init=subject.owned())
            if labeled:
                own.append(let)
            else:
                self.exprs.prelude.append(let)
            subject = Value(rs.Path(temp), Kind.PLACE)
        label = self.ctx.label("match_") if labeled else None
        if labeled:
            self.blocks += 1
        cases = []
        for index, case in enumerate(stmt.cases):
            if index:
                self.line = None  # where the chains before stopped
            cases.append(self.case(stmt, case, subject, items))
        if labeled:
            self.blocks -= 1
            if temp is not None:
                self.ctx.release(temp)  # in the block, not in the prelude
        if label is not None:
            out = self.labeled_cases(cases, label, own)
        else:
            out = self.chained_cases(cases)
        if self.markers != markers:
            self.line = None
        return items + out

    def stores_captures(self: StmtLowering, case: ast.match_case) -> bool:
        fn = self.ctx.function
        return any(
            fn is None or name not in fn.locals.case_scoped
            for name in pattern_captures(case.pattern)
        )

    def case(
        self: StmtLowering,
        stmt: ast.Match,
        case: ast.match_case,
        subject: Value,
        items: list[rs.Stmt],
    ) -> _Case:
        """Lower one case. A line marker for the first link of its chain goes
        before the `if` (clippy finds a block as the whole condition hard to
        read), markers for later links into them. Nested `fn` items of the
        pattern and the guard (generator expressions) go to `items`, before
        the `match`."""
        self.cases.append(case)
        temps: list[str] = []
        try:
            self.exprs.line = case.pattern.lineno
            pattern = _PatternLowering(self, temps)
            pattern.match(case.pattern, subject)
            lowered = pattern.out
            links: list[rs.Expr] = []
            before: list[rs.Stmt] = []
            for link in lowered.links:
                marker = self.mark(link.line, [rs.ExprStmt(link.expr)])
                if links:
                    links.append(_with_marker(link.expr, marker))
                else:
                    before = marker
                    links.append(link.expr)
            stores = [self.store_name(stmt, name, value) for name, value in lowered.stores]
            lets = [rs.Let(name, init=value.owned()) for name, value in lowered.lets]
            guard = None
            guard_before: list[rs.Stmt] = []
            if case.guard is not None:
                self.exprs.line = case.guard.lineno
                with self.exprs.fresh() as prelude:
                    cond = self.exprs.condition(case.guard)
                marker = self.mark(case.guard.lineno, [*prelude, rs.ExprStmt(cond)])
                guard_before = marker + prelude
                guard = cond
                if not stores:
                    # A link of the chain, after the scoped captures.
                    links += [rs.LetExpr(name, value) for name, value in _lets(lets)]
                    lets = []
                    if not links:
                        before = guard_before
                    elif guard_before:
                        guard = rs.BlockExpr(rs.Block(tuple(guard_before), cond))
                    guard_before = []
            items += self.flush_items()
            body = self.block(case.body).stmts
        finally:
            self.cases.pop()
            for name in temps:
                self.ctx.release(name)
        return _Case(links, before, stores, lets, guard, guard_before, list(body))

    def chained_cases(self: StmtLowering, cases: list[_Case]) -> list[rs.Stmt]:
        """`if .. {} else if .. {} else {}`, one branch per case. A case that
        needs a line marker first is `else { frame.line(n); if .. }`."""
        out: rs.If | rs.Block | None = None
        for case in reversed(cases):
            conds = case.links + ([case.guard] if case.guard is not None else [])
            then = rs.Block((*case.lets, *case.stores, *case.body))
            if not conds:
                # The last case matches anything: no `else {}` when it does nothing.
                out = then if then.stmts else None
                continue
            out = rs.If(_chain(conds), then, out)
            if case.before:
                out = rs.Block((*case.before, out))
        match out:
            case None:
                return []
            case rs.Block(stmts) if any(isinstance(s, rs.Let) for s in stmts):
                return [rs.ExprStmt(rs.BlockExpr(out))]
            case rs.Block(stmts):
                return list(stmts)
        return [out]

    def labeled_cases(
        self: StmtLowering, cases: list[_Case], label: str, own: list[rs.Stmt]
    ) -> list[rs.Stmt]:
        """Every case an `if` of its own in `'match_: { .. }`, the body
        leaving the block with `break 'match_`."""
        stmts = list(own)
        breaks = False
        for index, case in enumerate(cases):
            body = list(case.body)
            if index < len(cases) - 1 and not diverges(body):
                body.append(rs.Break(label))
                breaks = True
            if case.guard is not None and case.stores:
                inner = [*case.guard_before, rs.If(case.guard, rs.Block(tuple(body)))]
                then = [*case.lets, *case.stores, *inner]
                conds = case.links
            else:
                then = [*case.lets, *case.stores, *body]
                conds = case.links + ([case.guard] if case.guard is not None else [])
            if conds:
                stmts += [*case.before, rs.If(_chain(conds), rs.Block(tuple(then)))]
            else:
                stmts += then
        if breaks:
            return [rs.LabeledBlock(label, rs.Block(tuple(stmts)))]
        if own:
            return [rs.ExprStmt(rs.BlockExpr(rs.Block(tuple(stmts))))]
        return stmts


def _lets(lets: list[rs.Stmt]) -> list[tuple[str, rs.Expr]]:
    pairs = []
    for let in lets:
        assert isinstance(let, rs.Let) and let.init is not None
        pairs.append((let.name, let.init))
    return pairs


def _chain(conds: list[rs.Expr]) -> rs.Expr:
    result = conds[0]
    for cond in conds[1:]:
        result = rs.Binary("&&", result, cond)
    return result


def _with_marker(link: rs.Expr, marker: list[rs.Stmt]) -> rs.Expr:
    """`link` setting the traceback line first: `let p = { frame.line(n); v }`."""
    if not marker:
        return link
    if isinstance(link, rs.LetExpr):
        return rs.LetExpr(link.pattern, rs.BlockExpr(rs.Block(tuple(marker), link.value)))
    return rs.BlockExpr(rs.Block(tuple(marker), link))


def _rebound(stmt: ast.Match) -> bool:
    """Whether the subject is a name that a pattern or guard may rebind
    before a later case checks it."""
    if not isinstance(stmt.subject, ast.Name):
        return True
    name = stmt.subject.id
    for case in stmt.cases:
        if name in pattern_captures(case.pattern):
            return True
        if case.guard is not None and any(
            isinstance(node, ast.Name) and node.id == name and not isinstance(node.ctx, ast.Load)
            for node in ast.walk(case.guard)
        ):
            return True
    return False
