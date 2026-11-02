"""Lower Python statements.

Each statement is lowered together with the prelude of temporaries its
expressions need (see `expr.py`), which is emitted right before it.
"""

from __future__ import annotations

import ast
from dataclasses import dataclass

from steelix.frontend.bindings import is_constant_true
from steelix.frontend.desugar import annotation_source, binding_name
from steelix.frontend.fallback import first_line, python_runs, python_source
from steelix.lower.context import NAMESPACE, Context
from steelix.lower.expr import IN_PLACE, ExprLowering, Kind, Value, negate
from steelix.lower.function import cell_declared_at, lower_nested
from steelix.lower.handling import HandlingLowering
from steelix.lower.rustflow import diverges, fallible, statement_line
from steelix.rustast import nodes as rs
from steelix.rustast.rewrite import map_exprs


def _rt(name: str, *args: rs.Expr) -> rs.Call:
    return rs.Call(rs.Path(f"rt::{name}"), args)


def _try_stmt(expr: rs.Expr) -> rs.Stmt:
    return rs.ExprStmt(rs.Try(expr))


@dataclass
class _Loop:
    """A loop being lowered, as a target of `break` and `continue`."""

    else_label: str | None
    """The label of the block around the loop and its `else`, which `break`
    leaves, if there is one."""
    blocks: int
    """Number of labeled blocks open around the loop."""
    label: str | None = None
    """The loop's own label, given once a jump needs it."""
    header: int | None = None
    """The line of the loop header, if its next iteration (the test of a
    `while`, the next item of a `for`) can raise: `continue` and the end of
    the body set the traceback line back to it."""


class StmtLowering(HandlingLowering):
    def __init__(self, ctx: Context, line: int | None = None) -> None:
        self.ctx = ctx
        self.exprs = ExprLowering(ctx)
        self.loops: list[_Loop] = []
        """The enclosing loops, innermost last."""
        self.blocks = 0
        """Number of labeled blocks open around the statement being lowered."""
        self.closures: list = []
        """The closure regions being lowered (`handling.Closure`), innermost last."""
        self.line = line
        """The traceback line of the frame at this point of the generated
        code, if known: `frame.line(n)` only goes where it changes."""
        self.markers = 0
        """Number of `frame.line(n)` emitted so far."""

    # --- traceback lines ---

    def at_line(self, line: int) -> list[rs.Stmt]:
        """`frame.line(line);`, unless the frame is known to be there."""
        if self.line == line:
            return []
        self.line = line
        self.markers += 1
        return [_line_marker(line)]

    def mark(self, line: int, rust: list[rs.Stmt]) -> list[rs.Stmt]:
        """`at_line` for code on `line` that may raise."""
        return self.at_line(line) if fallible(rust) else []

    def block(self, stmts: list[ast.stmt]) -> rs.Block:
        out: list[rs.Stmt] = []
        for run in python_runs(self.reachable(stmts), self.ctx.module.plan):
            if isinstance(run, list):
                out += self.exec_python(run)
            else:
                out += self.stmt(run)
        return rs.Block(tuple(out))

    def exec_python(self, stmts: list[ast.stmt]) -> list[rs.Stmt]:
        """`rt::exec_python!(m, line = N, r#"..."#)?;` for statements kept as
        Python (design §13), after a comment saying where they are and why
        they are kept."""
        module = self.ctx.module
        features: dict[str, str] = {}
        for stmt in stmts:
            for issue in module.plan.python[stmt]:
                location = issue.location
                features.setdefault(issue.feature, f"{location.line}:{location.col + 1}")
        reasons = ", ".join(f"{feature} at {where}" for feature, where in features.items())
        line = first_line(stmts[0])
        args: list[rs.Expr] = [rs.Path(self.ctx.namespace()), rs.Tokens((f"line = {line}",))]
        if module.future_annotations:
            args.append(rs.Tokens(("future_annotations",)))
        annotations = [
            module.annotation_index[node]
            for stmt in stmts
            for node in ast.walk(stmt)
            if isinstance(node, ast.AnnAssign) and node in module.annotation_index
        ]
        if annotations:
            args.append(rs.Tokens((f"first_annotation = {min(annotations)}",)))
        pad = self.ctx.class_body is not None
        args.append(rs.RawStrLit("\n" + python_source(module.source_lines, stmts, pad)))
        call = rs.Try(rs.MacroCall("rt::exec_python", tuple(args)))
        comments = [rs.Comment(text) for text in module.leading_comments.get(stmts[0], [])]
        header = rs.Comment(f"{module.filename}:{line}, kept as Python: {reasons}")
        return [*comments, header, rs.ExprStmt(call)]

    def reachable(self, stmts: list[ast.stmt]) -> list[ast.stmt]:
        return [s for s in stmts if s not in self.ctx.flow.unreachable]

    def stmt(self, stmt: ast.stmt) -> list[rs.Stmt]:
        comments = self.comments(stmt)
        if stmt in self.ctx.flow.unreachable:
            return comments
        if isinstance(stmt, ast.ClassDef):
            from steelix.lower.classes import lower_class_def

            return comments + lower_class_def(self, stmt)
        if isinstance(stmt, ast.Break | ast.Continue):
            # Not worth a line of its own: the signal check before the next
            # iteration then reports the line before.
            return comments + self._stmt(stmt)
        if isinstance(stmt, _COMPOUND):
            # They mark the line of their header themselves.
            with self.exprs.fresh() as prelude:
                out = self._stmt(stmt)
            return comments + self.flush_items() + prelude + out
        with self.exprs.fresh() as prelude:
            out = self._stmt(stmt)
        items = self.flush_items()
        rust = prelude + out
        if isinstance(stmt, ast.FunctionDef):
            rust = _without_def_macros(rust)
        return comments + items + self.mark(statement_line(stmt), rust) + prelude + out

    def comments(self, stmt: ast.stmt) -> list[rs.Stmt]:
        if isinstance(stmt, ast.FunctionDef):
            return []  # they go to the Rust fn
        return [rs.Comment(text) for text in self.ctx.module.comments.get(stmt, [])]

    def flush_items(self) -> list[rs.Stmt]:
        """Nested functions lowered while lowering the current statement."""
        items: list[rs.Stmt] = [rs.ItemStmt(item) for item in self.ctx.items]
        self.ctx.items.clear()
        return items

    def _stmt(self, stmt: ast.stmt) -> list[rs.Stmt]:
        match stmt:
            case ast.Expr(value):
                return self.expr_stmt(value)
            case ast.Assign(targets, value):
                return self.assign(stmt, targets, value)
            case ast.AugAssign(target, op, value):
                return self.aug_assign(stmt, target, op, value)
            case ast.Delete(targets):
                return [s for target in targets for s in self.delete(target)]
            case ast.Return(value):
                return [self.return_stmt(value)]
            case ast.If():
                return self.if_stmt(stmt)
            case ast.While():
                return self.while_stmt(stmt)
            case ast.For():
                return self.for_stmt(stmt)
            case ast.Break():
                return self.jump("break")
            case ast.Continue():
                return self.jump("continue")
            case ast.Import(names):
                return [s for alias in names for s in self.import_(stmt, alias)]
            case ast.ImportFrom(module, names):
                assert module is not None
                return self.import_from(stmt, module, names)
            case ast.Assert(test, msg):
                return [self.assert_stmt(test, msg)]
            case ast.FunctionDef():
                return self.function_def(stmt)
            case ast.Raise(exc, cause):
                return [self.raise_stmt(exc, cause)]
            case ast.Try():
                return self.try_stmt(stmt)
            case ast.With():
                return self.with_stmt(stmt)
            case ast.AnnAssign():
                return self.class_annotation(stmt)
            case ast.Pass() | ast.Global() | ast.Nonlocal():
                return []
        raise self.ctx.unsupported(type(stmt).__name__, stmt)

    # --- functions ---

    def function_def(self, stmt: ast.FunctionDef) -> list[rs.Stmt]:
        """A nested `def` or a method: a Rust fn item, then the function
        object bound to the local name (in a class body, the namespace)."""
        if self.ctx.class_body is not None and self.ctx.function is None:
            from steelix.lower.classes import lower_method

            rust = lower_method(self.ctx, stmt)
        else:
            rust = lower_nested(self.ctx, stmt)
        value = self.exprs.function_value(stmt, rust)
        return [self.store_name(stmt, binding_name(stmt), value)]

    def class_annotation(self, stmt: ast.AnnAssign) -> list[rs.Stmt]:
        """`name: annotation [= value]` in a class body. The annotation goes
        into the class's `__annotate__` (`rt::class!(.., annotations = ..)`),
        or with `from __future__ import annotations` into `__annotations__`
        as a string, after the value is assigned."""
        if self.ctx.class_body is None or not isinstance(stmt.target, ast.Name):
            raise self.ctx.unsupported("annotated assignment", stmt)
        out: list[rs.Stmt] = []
        if stmt.value is not None:
            out += self.store(stmt, stmt.target, self.exprs.value(stmt.value))
        if self.ctx.module.future_annotations:
            text = rs.StrLit(annotation_source(stmt.annotation))
            call = rs.MethodCall(rs.Path(NAMESPACE), "annotate", (rs.StrLit(stmt.target.id), text))
            out.append(_try_stmt(call))
        return out

    def module_def(self, stmt: ast.FunctionDef, rust: str) -> list[rs.Stmt]:
        """A module level `def`, whose Rust fn is an item of the module."""
        with self.exprs.fresh() as prelude:
            if stmt.decorator_list:
                out = [self.store_name(stmt, stmt.name, self.exprs.function_value(stmt, rust))]
            else:
                out = [rs.ExprStmt(self.exprs.function_value(stmt, rust, bind=True).expr)]
        items = self.flush_items()
        line = self.mark(stmt.lineno, _without_def_macros(prelude + out))
        return items + line + prelude + out

    # --- expressions as statements ---

    def expr_stmt(self, value: ast.expr) -> list[rs.Stmt]:
        match value:
            case ast.Constant():
                return []  # docstrings and other bare constants do nothing
            case ast.BoolOp(op, values):
                # `a or b()` as a statement: run `b()` only if `a` is false.
                cond = self.exprs.condition(values[0])
                if isinstance(op, ast.Or):
                    cond = negate(cond)
                rest = values[1] if len(values) == 2 else ast.BoolOp(op, values[1:])
                return [rs.If(cond, self.nested(lambda: self.expr_stmt(rest)))]
            case ast.IfExp(test, body, orelse):
                cond = self.exprs.condition(test)
                then = self.nested(lambda: self.expr_stmt(body))
                other = self.nested(lambda: self.expr_stmt(orelse))
                return [rs.If(cond, then, other if other.stmts else None)]
        lowered = self.exprs.value(value)
        if lowered.is_pure:
            return []  # reading an assigned local has no effect
        if isinstance(lowered.expr, rs.Binary | rs.Unary | rs.IfExpr | rs.BlockExpr):
            return [rs.Let("_", init=lowered.expr)]
        return [rs.ExprStmt(lowered.expr)]

    def nested(self, lower) -> rs.Block:
        """A block for code that runs conditionally, with its own prelude."""
        with self.exprs.fresh() as prelude:
            out = lower()
        return rs.Block(tuple(prelude + out))

    def return_value(self, value: ast.expr | None) -> rs.Expr:
        if value is None:
            return rs.Call(
                rs.Path("Ok"), (rs.Call(rs.Path("rt::none"), (rs.Path(self.ctx.py()),)),)
            )
        return self.exprs.value(value).result()

    def return_stmt(self, value: ast.expr | None) -> rs.Stmt:
        if not self.closures:
            return rs.Return(self.return_value(value))
        if value is None:
            none = rs.Call(rs.Path("rt::none"), (rs.Path(self.ctx.py()),))
            return self.return_jump(Value(none, Kind.OWNED))
        return self.return_jump(self.exprs.value(value))

    def jump(self, kind: str) -> list[rs.Stmt]:
        """`break` or `continue` of the innermost loop."""
        if self.leaves_closure():
            return [self.loop_jump(kind)]
        loop = self.loops[-1]
        if kind == "break":
            if loop.else_label is not None:
                return [rs.Break(loop.else_label)]
            return [rs.Break(self.jump_label(loop))]
        line = self.line
        out = [*self.next_iteration(loop), rs.Continue(self.jump_label(loop))]
        self.line = line  # for code after the jump, which runs on another path
        return out

    # --- control flow ---

    def if_stmt(self, stmt: ast.If) -> list[rs.Stmt]:
        cond = self.exprs.condition(stmt.test)
        items = self.flush_items()
        self.exprs.prelude[:0] = self.mark(stmt.lineno, [*self.exprs.prelude, rs.ExprStmt(cond)])
        start = self.line
        then = self.block(stmt.body)
        then_end = self.line
        self.line = start
        orelse: rs.Block | rs.If | None = None
        if len(stmt.orelse) == 1 and isinstance(stmt.orelse[0], ast.If):
            # `elif`: the test may need statements first, then it is `else { .. }`.
            inner = self.stmt(stmt.orelse[0])
            if len(inner) == 1 and isinstance(inner[0], rs.If):
                orelse = inner[0]
            else:
                orelse = rs.Block(tuple(inner))
        elif stmt.orelse:
            orelse = self.block(stmt.orelse)
        # The line after the `if` is where the branches that go on ended.
        ends = [] if diverges(then.stmts) else [then_end]
        if orelse is None or not _diverges_branch(orelse):
            ends.append(self.line)
        self.line = ends[0] if ends and all(end == ends[0] for end in ends) else None
        return [*items, rs.If(cond, then, orelse)]

    def loop_label(self, stmt: ast.While | ast.For, base: str) -> str | None:
        """The label of the block around a loop with `else`, if `break` needs it."""
        if not self.reachable(stmt.orelse):
            return None
        for node in _loop_breaks(stmt.body):
            if node not in self.ctx.flow.unreachable:
                return self.ctx.label(base)
        return None

    def jump_label(self, loop: _Loop) -> str | None:
        """The label a `break` or `continue` needs to reach `loop`.

        Rust rejects unlabeled jumps out of a labeled block (E0695), which a
        jump from a loop's `else` to the enclosing loop is, so only those
        label the loop.
        """
        if self.blocks == loop.blocks:
            return None
        if loop.label is None:
            loop.label = self.ctx.label("outer")
        return loop.label

    def loop_body(
        self, stmts: list[ast.stmt], else_label: str | None, header: int | None
    ) -> tuple[rs.Block, _Loop]:
        """The body of a loop. With `header`, the line of a header that can
        raise when the loop goes on, the body ends by going back to it."""
        loop = _Loop(else_label, self.blocks, header=header)
        self.loops.append(loop)
        body = self.block(stmts)
        if not diverges(body.stmts):
            body = rs.Block((*body.stmts, *self.next_iteration(loop)))
        self.loops.pop()
        return body, loop

    def next_iteration(self, loop: _Loop) -> list[rs.Stmt]:
        """What runs before the next iteration of `loop` (at the end of the
        body or at `continue`): CPython checks for signals there, on the line
        the iteration ended on, then the header line is back."""
        reset = [] if loop.header is None else self.at_line(loop.header)
        return [self.check_signals(), *reset]

    def with_else(
        self, stmt: ast.While | ast.For, loop: rs.Stmt, label: str | None
    ) -> list[rs.Stmt]:
        if label is None:
            # Without a `break` the `else` block always runs after the loop.
            return [loop, *self.block(stmt.orelse).stmts]
        self.blocks += 1
        orelse = self.block(stmt.orelse)
        self.blocks -= 1
        return [rs.LabeledBlock(label, rs.Block((loop, *orelse.stmts)))]

    def while_stmt(self, stmt: ast.While) -> list[rs.Stmt]:
        label = self.loop_label(stmt, "while_else")
        constant_true = is_constant_true(stmt.test)
        with self.exprs.fresh() as prelude:
            cond = None if constant_true else self.exprs.condition(stmt.test)
        items = self.flush_items()
        markers = self.markers
        test_fails = cond is not None and fallible([*prelude, rs.ExprStmt(cond)])
        header: int | None = None
        before: list[rs.Stmt] = []
        if test_fails and not prelude:
            # `frame.line(n); while test { ..; frame.line(n); }`
            before = self.at_line(stmt.lineno)
            header = stmt.lineno
        elif test_fails:
            # The test is evaluated at the top of a `loop`, where the line is set.
            self.line = None
            prelude = [*self.at_line(stmt.lineno), *prelude]
        elif _changes_line(stmt.body, self.line):
            self.line = None
        body, target = self.loop_body(stmt.body, label, header)
        if header is None and self.markers != markers:
            self.line = None  # for the `else` block
        if cond is None:
            loop: rs.Stmt = rs.Loop(body, target.label)
        elif not prelude:
            loop = rs.While(cond, body, target.label)
        else:
            # The test needs statements before it on every iteration.
            exit_ = rs.If(negate(cond), rs.Block((rs.Break(),)))
            loop = rs.Loop(rs.Block((*prelude, exit_, *body.stmts)), target.label)
        out = items + before + self.with_else(stmt, loop, label)
        if self.markers != markers:
            self.line = None
        return out

    def check_signals(self) -> rs.Stmt:
        """`py.check_signals()?;` at the end of every loop iteration, where
        CPython's eval loop would run pending signal handlers: without it, a
        native loop can not be stopped with Ctrl-C (`KeyboardInterrupt`)."""
        return _try_stmt(rs.MethodCall(rs.Path(self.ctx.py()), "check_signals", ()))

    def for_stmt(self, stmt: ast.For) -> list[rs.Stmt]:
        label = self.loop_label(stmt, "for_else")
        iterable = self.exprs.value(stmt.iter)
        # Functions defined by the header, which the loop body must not hide.
        items = self.flush_items()
        iterator = rs.Try(iterable.method("try_iter"))
        markers = self.markers
        # Getting the iterator and each item can raise.
        self.exprs.prelude[:0] = self.at_line(stmt.lineno)
        fn = self.ctx.function
        target = stmt.target
        scoped = fn is not None and any(
            fn.locals.loop_scoped.get(name) is stmt for name in _names(target)
        )
        head: list[rs.Stmt]
        if scoped and isinstance(target, ast.Name):
            # `for x in xs.try_iter()? { let x = x?; .. }`
            assert fn is not None
            rust = fn.names[target.id]
            pat = rust
            head = [rs.Let(rust, init=rs.Try(rs.Path(rust)))]
        else:
            pat = self.ctx.temp("item")
            item = Value(rs.Try(rs.Path(pat)), Kind.OWNED)
            with self.exprs.fresh() as prelude:
                head = self.store(stmt, target, item, declare=scoped)
            head = prelude + head
        body, target = self.loop_body(stmt.body, label, stmt.lineno)
        loop = rs.For(pat, iterator, rs.Block((*head, *body.stmts)), target.label)
        out = items + self.with_else(stmt, loop, label)
        if self.markers != markers:
            self.line = None
        return out

    def assert_stmt(self, test: ast.expr, msg: ast.expr | None) -> rs.Stmt:
        self.use_exception("AssertionError")
        cond = self.exprs.condition(test)
        with self.exprs.fresh() as prelude:
            match msg:
                case None:
                    args: rs.Expr = rs.Tuple(())
                case ast.Constant(str() as text) if (
                    self.exprs.constant(text, msg).kind is Kind.LITERAL
                ):
                    args = rs.StrLit(text)
                case _:
                    value = self.exprs.value(msg)
                    args = rs.Tuple((rs.MethodCall(value.owned(), "unbind"),))
        error = rs.Call(rs.Path("PyAssertionError::new_err"), (args,))
        raised = rs.MethodCall(rs.Path("frame"), "raise", (error,))
        raise_ = rs.Return(rs.Call(rs.Path("Err"), (raised,)))
        return rs.If(negate(cond), rs.Block((*prelude, raise_)))

    # --- assignment ---

    def assign(self, stmt: ast.Assign, targets: list[ast.expr], value: ast.expr) -> list[rs.Stmt]:
        exprs = self.exprs
        if len(targets) == 1:
            target = targets[0]
            match target:
                case ast.Subscript(obj, index):
                    # Python evaluates the value first, Rust `obj.set_item(index, value)`
                    # last.
                    v, o, i = exprs.in_rust_order(
                        exprs.operands([value, obj, index]), [1, 2, 0], ["value", "obj", "index"]
                    )
                    return [_try_stmt(o.method("set_item", i.arg(), v.arg()))]
                case ast.Attribute(obj, attr):
                    v, o = exprs.in_rust_order(
                        exprs.operands([value, obj]), [1, 0], ["value", "obj"]
                    )
                    setter = o.method("setattr", rs.StrLit(attr), v.arg())
                    return [_try_stmt(setter)]
            return self.store(stmt, target, exprs.value(value))
        # `a = b = value`: evaluate once, then assign left to right.
        lowered = exprs.value(value)
        if lowered.kind not in (Kind.PLACE, Kind.REF) or not lowered.is_pure:
            temp = self.ctx.temp("value")
            exprs.prelude.append(rs.Let(temp, init=lowered.owned()))
            lowered = Value(rs.Path(temp), Kind.PLACE)
        out: list[rs.Stmt] = []
        for target in targets:
            out += self.nested(lambda target=target: self.store(stmt, target, lowered)).stmts
        return out

    def store(
        self, stmt: ast.stmt, target: ast.expr, value: Value, declare: bool = False
    ) -> list[rs.Stmt]:
        """Assign an already evaluated `value` to `target`.

        `declare` declares the names of a flat tuple target with `let`.
        """
        exprs = self.exprs
        match target:
            case ast.Name(id):
                return [self.store_name(stmt, id, value)]
            case ast.Subscript(obj, index):
                o, i = exprs.operands([obj, index])
                return [_try_stmt(o.method("set_item", i.arg(), value.arg()))]
            case ast.Attribute(obj, attr):
                o = exprs.value(obj)
                setter = o.method("setattr", rs.StrLit(attr), value.arg())
                return [_try_stmt(setter)]
            case ast.Tuple(elts) | ast.List(elts):
                return self.unpack(stmt, elts, value, declare)
        raise self.ctx.unsupported(type(target).__name__, target)

    def store_name(self, stmt: ast.stmt, name: str, value: Value) -> rs.Stmt:
        fn = self.ctx.function
        if fn is None or fn.scope.is_global(name):
            return _try_stmt(self.exprs.scope_call("set", rs.StrLit(name), value.arg()))
        rust = fn.names[name]
        if fn.is_cell(name):
            if cell_declared_at(fn, name, stmt):
                cell = rs.Call(
                    rs.Path("rt::Cell::new"), (rs.Path(self.ctx.py()), rs.StrLit(name), value.arg())
                )
                return rs.Let(rust, init=rs.Try(cell))
            return _try_stmt(rs.MethodCall(rs.Path(rust), "set", (value.arg(),)))
        info = fn.locals
        init = value.owned()
        if name in info.optional:
            init = rs.Call(rs.Path("Some"), (init,))
        if info.declared_at.get(name) is stmt:
            # An `Option` local declared here is deleted later, so it is `mut`.
            mutable = name in info.mutable or name in info.optional
            return rs.Let(rust, mutable=mutable, init=init)
        return rs.Assign(rs.Path(rust), init)

    def unpack(
        self, stmt: ast.stmt, elts: list[ast.expr], value: Value, declare: bool
    ) -> list[rs.Stmt]:
        """`a, b = value` and `a, *rest = value`.

        With `declare`, names are declared by the `let` pattern itself (loop
        variables scoped to the loop body).
        """
        star = next((i for i, e in enumerate(elts) if isinstance(e, ast.Starred)), None)
        call = rs.Try(_rt("unpack" if star is None else "unpack_star", value.ref()))
        targets = [e.value if isinstance(e, ast.Starred) else e for e in elts]

        fn = self.ctx.function
        names = [t.id for t in targets if isinstance(t, ast.Name)]
        plain = (
            fn is not None
            and len(names) == len(targets)
            and len(set(names)) == len(names)
            and all(
                fn.scope.is_local(n) and not fn.is_cell(n) and n not in fn.locals.optional
                for n in names
            )
        )
        if plain and not declare:
            assert fn is not None
            info = fn.locals
            if all(info.declared_at.get(n) is stmt for n in names):
                # `let [a, b] = rt::unpack(&value)?;`
                pattern = _pattern(
                    [("mut " if n in info.mutable else "") + fn.names[n] for n in names], star
                )
                return [rs.Let(pattern, init=call)]
            if not any(info.declared_at.get(n) is stmt for n in names):
                # Destructuring assignment to locals declared earlier.
                target = _pattern_expr([fn.names[n] for n in names], star)
                return [rs.Assign(target, call)]
        # Unpack into a pattern of declared names and temporaries, then assign
        # the temporaries to their targets in order.
        parts: list[str] = []
        rest: list[tuple[ast.expr, str]] = []
        for target in targets:
            if declare and isinstance(target, ast.Name):
                assert fn is not None
                parts.append(fn.names[target.id])
            else:
                temp = self.ctx.temp(_target_base(target))
                parts.append(temp)
                rest.append((target, temp))
        out: list[rs.Stmt] = [rs.Let(_pattern(parts, star), init=call)]
        for target, temp in rest:
            # Each temporary is used exactly once, so it can be moved.
            item = Value(rs.Path(temp), Kind.OWNED, pure=True)
            out += self.nested(
                lambda target=target, item=item: self.store(stmt, target, item, declare)
            ).stmts
        return out

    def aug_assign(
        self, stmt: ast.AugAssign, target: ast.expr, op: ast.operator, value: ast.expr
    ) -> list[rs.Stmt]:
        """`x += v`: the target's parts are evaluated once, then read, then `v`."""
        exprs = self.exprs
        helper = IN_PLACE[type(op)]
        match target:
            case ast.Name(id):
                current, rhs = exprs.operands([value], [exprs.name(id, target)])
                result = Value(rs.Try(_rt(helper, current.ref(), rhs.arg())), Kind.OWNED)
                return [self.store_name(stmt, id, result)]
            case ast.Subscript(obj, index):
                o, i = exprs.hoist_all(exprs.operands([obj, index]), ["target", "index"])
                getter = o.method("get_item", i.arg())
                current = Value(rs.Try(getter), Kind.OWNED, base="item")
                current, rhs = exprs.operands([value], [current])
                result = rs.Try(_rt(helper, current.ref(), rhs.arg()))
                return [_try_stmt(o.method("set_item", i.arg(), result))]
            case ast.Attribute(obj, attr):
                o = exprs.hoist(exprs.value(obj), "target")
                getter = o.method("getattr", rs.StrLit(attr))
                current = Value(rs.Try(getter), Kind.OWNED, base=attr)
                current, rhs = exprs.operands([value], [current])
                result = rs.Try(_rt(helper, current.ref(), rhs.arg()))
                return [_try_stmt(o.method("setattr", rs.StrLit(attr), result))]
        raise self.ctx.unsupported(type(target).__name__, target)

    def delete(self, target: ast.expr) -> list[rs.Stmt]:
        exprs = self.exprs
        match target:
            case ast.Name(id):
                fn = self.ctx.function
                if fn is None or fn.scope.is_global(id):
                    return [_try_stmt(exprs.scope_call("del", rs.StrLit(id)))]
                rust = fn.names[id]
                if fn.is_cell(id):
                    return [_try_stmt(rs.MethodCall(rs.Path(rust), "delete"))]
                if target in fn.locals.assigned_deletes:
                    return [rs.Assign(rs.Path(rust), rs.Path("None"))]
                place = rs.Ref(rs.Path(rust), mutable=True)
                return [_try_stmt(_rt("del_local", place, rs.StrLit(id)))]
            case ast.Subscript(obj, index):
                o, i = exprs.operands([obj, index])
                return [_try_stmt(o.method("del_item", i.arg()))]
            case ast.Attribute(obj, attr):
                o = exprs.value(obj)
                return [_try_stmt(o.method("delattr", rs.StrLit(attr)))]
            case ast.Tuple(elts) | ast.List(elts):
                out: list[rs.Stmt] = []
                for elt in elts:
                    out += self.nested(lambda elt=elt: self.delete(elt)).stmts
                return out
        raise self.ctx.unsupported(type(target).__name__, target)

    # --- imports ---

    def import_(self, stmt: ast.Import, alias: ast.alias) -> list[rs.Stmt]:
        context = rs.Path(self.ctx.context())
        name = rs.StrLit(alias.name)
        if alias.asname is not None and "." in alias.name:
            # `import a.b as c` binds the submodule itself.
            module = _rt("import_module", context, name)
        else:
            # `import a.b` binds the top-level package `a`.
            module = _rt("import", context, name)
        bound = alias.asname or alias.name.split(".")[0]
        return [self.store_name(stmt, bound, Value(rs.Try(module), Kind.OWNED))]

    def import_from(
        self, stmt: ast.ImportFrom, module: str, names: list[ast.alias]
    ) -> list[rs.Stmt]:
        temp = self.ctx.temp(module.rsplit(".", 1)[-1])
        fromlist = rs.Ref(rs.Array(tuple(rs.StrLit(a.name) for a in names)))
        load = _rt("import_names", rs.Path(self.ctx.context()), rs.StrLit(module), fromlist)
        out: list[rs.Stmt] = [rs.Let(temp, init=rs.Try(load))]
        for alias in names:
            value = rs.Try(_rt("import_from", rs.Ref(rs.Path(temp)), rs.StrLit(alias.name)))
            out.append(self.store_name(stmt, alias.asname or alias.name, Value(value, Kind.OWNED)))
        return out


_COMPOUND = (ast.If, ast.While, ast.For, ast.Try, ast.With)


def _without_def_macros(stmts: list[rs.Stmt]) -> list[rs.Stmt]:
    """`stmts` with `rt::def!(..)?` and `rt::function!(..)?` replaced by
    their arguments: creating the function object only fails when Python is
    out of memory, which is not worth a line marker before every `def`. The
    defaults and decorators it evaluates can raise."""

    def visit(node: rs.Expr) -> rs.Expr:
        match node:
            case rs.Try(rs.MacroCall("rt::def" | "rt::function", args)):
                return rs.Tuple(args)
        return node

    return [map_exprs(stmt, visit) for stmt in stmts]


def _line_marker(line: int) -> rs.Stmt:
    return rs.ExprStmt(rs.MethodCall(rs.Path("frame"), "line", (rs.IntLit(line),)))


def _changes_line(body: list[ast.stmt], line: int | None) -> bool:
    """Whether running `body` may move the traceback line away from `line`."""
    return any(
        isinstance(node, ast.stmt) and node.lineno != line
        for stmt in body
        for node in ast.walk(stmt)
    )


def _diverges_branch(branch: rs.Block | rs.If) -> bool:
    if isinstance(branch, rs.If):
        return diverges((branch,))
    return diverges(branch.stmts)


def _pattern(names: list[str], star: int | None) -> str:
    """`[a, b]`, or `([a], rest, [b])` for `rt::unpack_star`."""
    if star is None:
        return f"[{', '.join(names)}]"
    before = ", ".join(names[:star])
    after = ", ".join(names[star + 1 :])
    return f"([{before}], {names[star]}, [{after}])"


def _pattern_expr(names: list[str], star: int | None) -> rs.Expr:
    """`_pattern` as the left side of a destructuring assignment."""
    paths = [rs.Path(name) for name in names]
    if star is None:
        return rs.Array(tuple(paths))
    before, after = rs.Array(tuple(paths[:star])), rs.Array(tuple(paths[star + 1 :]))
    return rs.Tuple((before, paths[star], after))


def _target_base(target: ast.expr) -> str:
    match target:
        case ast.Name(id):
            return id
        case ast.Attribute(_, attr):
            return attr
        case ast.Subscript():
            return "item"
    return "items"


def _names(target: ast.expr) -> list[str]:
    return [n.id for n in ast.walk(target) if isinstance(n, ast.Name)]


def _loop_breaks(body: list[ast.stmt]):
    """`break` statements that leave the loop whose body is `body`."""
    for stmt in body:
        match stmt:
            case ast.Break():
                yield stmt
            case ast.While() | ast.For():
                yield from ()  # breaks inside belong to the inner loop
                for inner in stmt.orelse:
                    yield from _loop_breaks([inner])
            case ast.If(_, then, orelse):
                yield from _loop_breaks(then)
                yield from _loop_breaks(orelse)
            case ast.With(body=body):
                yield from _loop_breaks(body)
            case ast.Try(body, handlers, orelse, finalbody):
                for block in (body, *(h.body for h in handlers), orelse, finalbody):
                    yield from _loop_breaks(block)
