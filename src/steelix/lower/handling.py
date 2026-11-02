"""Lower `try`, `raise` and `with` (design §9).

Rust has no `try` block. Code whose exceptions are caught runs in an
immediately called closure (a region, see `frontend.regions`), and the
`PyResult` it returns is matched against the handlers:

    match (|| -> PyResult<()> {
        ..;
        Ok(())
    })() {
        Ok(()) => {}
        Err(err) if err.is_instance_of::<PyKeyError>(py) => { .. }
        Err(err) => return Err(err),
    }

A `try` whose body evaluates a single expression (`x = int(s)`, `return
f()`, `log(x)`) needs no closure: the `match` is on that expression. A
`finally` block runs after the closure, with the outcome kept in
`frame.finally(result)` and resumed afterwards. `with` hands the body as a
closure to `frame.with`, which implements the protocol.

`return`, `break` and `continue` can not leave a closure, so a closure that
contains them returns an `rt::Flow` saying what to do next, and the code
after it does that (`Ok(rt::Flow::Break) => break,`).
"""

from __future__ import annotations

import ast
import builtins
from dataclasses import dataclass, field
from typing import TYPE_CHECKING

from steelix.frontend.regions import Region, expression_body
from steelix.frontend.resolve import Lookup, resolve_global
from steelix.lower.expr import Kind, Value
from steelix.lower.rustflow import diverges, statement_line
from steelix.rustast import nodes as rs
from steelix.rustast.pyo3 import BUILTIN_EXCEPTIONS
from steelix.rustast.rewrite import map_exprs

if TYPE_CHECKING:
    from steelix.lower.stmt import StmtLowering

BOUND = "Bound<'py, PyAny>"
FRAME = rs.Path("frame")
JUMPS = ("return", "break", "continue")


@dataclass
class Closure:
    """A closure region being lowered."""

    loops: int
    """Number of loops open outside the closure: jumps to them return a
    `Flow` instead."""
    jumps: set[str] = field(default_factory=set)
    """`return`, `break`, `continue`: the jumps out of the closure in it."""


@dataclass
class Lowered:
    """A lowered closure region and what it can do next."""

    call: rs.Expr
    """The closure (called, for a `try`)."""
    mode: str
    """`unit` (`PyResult<()>`), `value` (it always returns: `PyResult<Bound>`)
    or `flow` (`PyResult<rt::Flow>`)."""
    jumps: set[str]
    next: bool
    """Whether it can end normally (continue after the statement)."""


def _frame(method: str, *args: rs.Expr) -> rs.MethodCall:
    return rs.MethodCall(FRAME, method, args)


def _ok(expr: rs.Expr) -> rs.Call:
    return rs.Call(rs.Path("Ok"), (expr,))


def _err(expr: rs.Expr) -> rs.Return:
    return rs.Return(rs.Call(rs.Path("Err"), (expr,)))


def _flow(variant: str, *args: rs.Expr) -> rs.Expr:
    path = rs.Path(f"rt::Flow::{variant}")
    return rs.Call(path, args) if args else path


class HandlingLowering:
    """The part of `StmtLowering` for exceptions."""

    # --- jumps out of closures ---

    def return_jump(self: StmtLowering, value: Value) -> rs.Stmt:
        """`return value`, from the function or out of the closure."""
        if not self.closures:
            return rs.Return(value.result())
        self.closures[-1].jumps.add("return")
        return rs.Return(_ok(_flow("Return", value.owned())))

    def leaves_closure(self: StmtLowering) -> bool:
        """Whether a `break` or `continue` of the innermost loop has to leave a closure."""
        return bool(self.closures) and self.closures[-1].loops >= len(self.loops)

    def loop_jump(self: StmtLowering, kind: str) -> rs.Stmt:
        """`break` or `continue` out of the closure."""
        self.closures[-1].jumps.add(kind)
        return rs.Return(_ok(_flow(kind.capitalize())))

    # --- closures ---

    def closure(self: StmtLowering, regions: list[Region], lower):  # type: ignore[no-untyped-def]
        """Lower the code of a closure region with `lower()`, which returns
        its statements."""
        closure = Closure(len(self.loops))
        self.closures.append(closure)
        declarations = [d for region in regions for d in self.region_declarations(region)]
        try:
            stmts = declarations + lower()
        finally:
            self.closures.pop()
        return closure, stmts

    def finish_closure(
        self: StmtLowering,
        closure: Closure,
        stmts: list[rs.Stmt],
    ) -> Lowered:
        """The closure expression for lowered `stmts`."""
        next_ = not diverges(stmts)
        jumps = closure.jumps
        tail: rs.Expr | None = None
        if jumps == {"return"} and not next_:
            mode = "value"
            stmts = [_unflow(s) for s in stmts]
            stmts, tail = _return_as_tail(stmts)
        elif jumps:
            stmts, tail = _return_as_tail(stmts)
            mode = "flow"
            if next_:
                tail = _ok(_flow("Next"))
        else:
            mode = "unit"
            if next_:
                tail = _ok(rs.Tuple(()))
        lifetime = self.ctx.lifetime()
        result = {
            "unit": "()",
            "value": f"Bound<{lifetime}, PyAny>",
            "flow": f"rt::Flow<{lifetime}>",
        }[mode]
        body = rs.Closure((), rs.Block(tuple(stmts), tail), f"PyResult<{result}>")
        return Lowered(body, mode, set(jumps), next_)

    def region_declarations(self: StmtLowering, region: Region) -> list[rs.Stmt]:
        """`let` for the locals that live in a closure region and are not
        declared by an assignment."""
        fn = self.ctx.function
        if fn is None:
            return []
        out: list[rs.Stmt] = []
        for name in fn.locals.region_top.get(region, []):
            rust = fn.names[name]
            if name in fn.locals.optional:
                out.append(rs.Let(rust, True, f"Option<{BOUND}>", rs.Path("None")))
            else:
                out.append(rs.Let(rust, name in fn.locals.mutable, BOUND))
        return out

    def dispatch(
        self: StmtLowering, lowered: Lowered, next_body: list[rs.Stmt], wrap: bool
    ) -> list[tuple[str, list[rs.Stmt]]]:
        """`(pattern, body)` arms doing what the closure says to do next.
        With `wrap`, the patterns match the closure's `PyResult` (`Ok(..)`),
        else its `?`ed value."""

        def pattern(inner: str) -> str:
            return f"Ok({inner})" if wrap else inner

        match lowered.mode:
            case "unit" if lowered.next:
                return [(pattern("()"), next_body)]
            case "unit":
                # The body always raises.
                return [(pattern("()"), [rs.ExprStmt(rs.MacroCall("unreachable", ()))])]
            case "value":
                value = self.ctx.binding("value")
                jump = self.return_jump(Value(rs.Path(value), Kind.PLACE, temp=True))
                return [(pattern(value), [jump])]
        arms: list[tuple[str, list[rs.Stmt]]] = []
        for kind in JUMPS:
            if kind not in lowered.jumps:
                continue
            if kind == "return":
                value = self.ctx.binding("value")
                jump = [self.return_jump(Value(rs.Path(value), Kind.PLACE, temp=True))]
                arms.append((pattern(f"rt::Flow::Return({value})"), jump))
            else:
                arms.append((pattern(f"rt::Flow::{kind.capitalize()}"), self.jump(kind)))
        if lowered.next:
            covered = len(arms) == len(JUMPS)
            arms.append((pattern("rt::Flow::Next" if covered else "_"), next_body))
        else:
            # `Next` never happens: the last jump covers it.
            last, body = arms[-1]
            arms[-1] = (pattern("_"), body)
        return arms

    def flow_stmts(self: StmtLowering, lowered: Lowered, value: rs.Expr) -> list[rs.Stmt]:
        """Do what the `Flow` (or value) `value` of a closure says."""
        if lowered.mode == "unit" and lowered.next:
            return [rs.ExprStmt(rs.Try(value))]
        if lowered.mode == "unit":
            # It always raises (or never ends).
            return [rs.ExprStmt(rs.Try(value)), rs.ExprStmt(rs.MacroCall("unreachable", ()))]
        if lowered.mode == "value":
            return [self.return_jump(Value(rs.Try(value), Kind.OWNED))]
        arms = self.dispatch(lowered, [], wrap=False)
        rest = [(p, b) for p, b in arms if b]
        if len(rest) == 1 and len(arms) == 2 and arms[-1][0] == "_":
            # `if let rt::Flow::Return(value) = frame.with(..)? { return Ok(value); }`
            pattern, body = rest[0]
            return [rs.If(rs.Try(value), rs.Block(tuple(body)), pattern=pattern)]
        match_arms = tuple(rs.Arm(p, rs.Block(tuple(b))) for p, b in arms)
        return [rs.ExprStmt(rs.Match(rs.Try(value), match_arms))]

    # --- try ---

    def try_stmt(self: StmtLowering, stmt: ast.Try) -> list[rs.Stmt]:
        if not stmt.finalbody:
            return self.try_except(stmt)
        if stmt.handlers:
            region = Region(stmt, "handled")
            closure, stmts = self.closure([region], lambda: self.try_except(stmt))
        else:
            region = Region(stmt, "body")
            closure, stmts = self.closure([region], lambda: list(self.block(stmt.body).stmts))
        lowered = self.finish_closure(closure, stmts)
        result = self.ctx.temp("result")
        finally_ = self.ctx.temp("finally")
        out: list[rs.Stmt] = [rs.Let(result, init=rs.Call(lowered.call, ()))]
        start = len(out)
        # The line is unknown: an exception may come from anywhere.
        self.line = None
        final = self.block(stmt.finalbody).stmts
        if diverges(final):
            # `return` or `break` in `finally` drops the outcome (and any
            # pending exception).
            finally_ = "_" + finally_
            resume: list[rs.Stmt] = []
        else:
            resume = self.flow_stmts(lowered, rs.MethodCall(rs.Path(finally_), "resume"))
        init = _frame("finally", rs.Path(result))
        out.insert(start, rs.Let(finally_, init=init))
        return out + list(final) + resume

    def try_except(self: StmtLowering, stmt: ast.Try) -> list[rs.Stmt]:
        """`try` with handlers (and `else`), without the `finally` part."""
        expr = expression_body(stmt)
        prelude: list[rs.Stmt] = []
        if expr is not None:
            body = stmt.body[0]
            with self.exprs.fresh() as inner:
                scrutinee = self.exprs.value(expr).result()
            items = self.flush_items()
            scrutinee, hoisted = self.hoist_builtins(scrutinee, inner)
            prelude = items + self.mark(statement_line(body), [*inner, rs.Return(scrutinee)])
            prelude += hoisted
            lifetime = self.ctx.lifetime()
            ret = f"PyResult<Bound<{lifetime}, PyAny>>"
            if inner:
                # The expression needs statements first: they can raise too.
                scrutinee = rs.Call(rs.Closure((), rs.Block(tuple(inner), scrutinee), ret), ())
            elif _has_try(scrutinee):
                # So can parts of the expression. `Ok(..)` says nothing about
                # the error type, a `PyResult` returned by PyO3 does.
                typed = isinstance(scrutinee, rs.Call) and scrutinee.func == rs.Path("Ok")
                closure = rs.Closure((), rs.Block((), scrutinee), ret if typed else None)
                scrutinee = rs.Call(closure, ())
            handler_line = self.line
            ok_arms = self.expression_arms(stmt, body)
        else:
            closure, stmts = self.closure(
                [Region(stmt, "body")], lambda: list(self.block(stmt.body).stmts)
            )
            lowered = self.finish_closure(closure, stmts)
            scrutinee = rs.Call(lowered.call, ())
            handler_line = None
            ok_arms = self.dispatch(lowered, [], wrap=True)
            if lowered.next:
                # The `else` block runs where the body ended normally.
                pattern, _ = ok_arms[-1]
                ok_arms[-1] = (pattern, list(self.block(stmt.orelse).stmts))
        ends = [self.line]
        arms = [rs.Arm(p, rs.Block(tuple(b))) for p, b in ok_arms]
        catch_all = False
        for handler in stmt.handlers:
            self.line = handler_line
            arms.append(self.handler_arm(handler))
            ends.append(self.line)
            if handler.type is None:
                catch_all = True
        if not catch_all:
            err = self.ctx.binding("err")
            arms.append(rs.Arm(f"Err({err})", rs.Block((_err(rs.Path(err)),))))
        self.line = ends[0] if all(end == ends[0] for end in ends) else None
        return [*prelude, rs.ExprStmt(rs.Match(scrutinee, tuple(arms)))]

    def expression_arms(
        self: StmtLowering, stmt: ast.Try, body: ast.stmt
    ) -> list[tuple[str, list[rs.Stmt]]]:
        """The `Ok` arm of a `try` whose body is one expression."""
        match body:
            case ast.Return():
                value = self.ctx.binding("value")
                result = Value(rs.Path(value), Kind.PLACE, temp=True)
                return [(f"Ok({value})", [self.return_jump(result)])]
            case ast.Assign([ast.Name(name)]):
                value = self.ctx.binding("value")
                result = Value(rs.Path(value), Kind.PLACE, temp=True)
                with self.exprs.fresh() as prelude:
                    store = self.store_name(body, name, result)
                orelse = self.block(stmt.orelse).stmts
                return [(f"Ok({value})", [*prelude, store, *orelse])]
        return [("Ok(_)", list(self.block(stmt.orelse).stmts))]

    def hoist_builtins(
        self: StmtLowering, expr: rs.Expr, prelude: list[rs.Stmt]
    ) -> tuple[rs.Expr, list[rs.Stmt]]:
        """`let int = rt::builtin(py, "int")?;` before the `match` on
        `int(s)`, so that the call is the only thing that can raise in it.

        Looking up a builtin that exists (`int`) only fails if someone
        deleted it from `builtins`; doing it before the `try` keeps the
        common `try: x = int(s)` readable (design §9). Names that are not
        builtins of this Python version are left alone: `try: unicode
        except NameError:` must catch their `NameError`."""
        if prelude:
            return expr, []
        hoisted: dict[str, tuple[str, rs.Expr]] = {}

        def visit(node: rs.Expr) -> rs.Expr:
            match node:
                case rs.Try(rs.Call(rs.Path("rt::builtin"), (rs.Path(), rs.StrLit(name))) as call):
                    pass
                case rs.Try(rs.MethodCall(rs.Path(), "global", (rs.StrLit(name),)) as call):
                    pass
                case _:
                    return node
            if not hasattr(builtins, name):
                return node
            if name not in hoisted:
                hoisted[name] = (self.ctx.temp(name), call)
            return rs.Path(hoisted[name][0])

        rewritten = map_exprs(rs.ExprStmt(expr), visit).expr
        if _has_try(rewritten):
            return expr, []
        lets: list[rs.Stmt] = [rs.Let(temp, init=rs.Try(call)) for temp, call in hoisted.values()]
        return rewritten, lets

    def handler_arm(self: StmtLowering, handler: ast.ExceptHandler) -> rs.Arm:
        err = self.ctx.binding("err")
        fn = self.ctx.function
        name = handler.name
        if name is not None and fn is not None and name in fn.locals.handler_scoped:
            if fn.names[name] == "err":
                # `except E as err:` binds its own `err`: `let err = frame.catch(err);`
                err = "err"
        guard: rs.Expr | None = None
        if handler.type is not None:
            guard = self.handler_test(handler.type, rs.Path(err))
        trivial = name is None and _trivial(handler.body, self)
        head: list[rs.Stmt] = []
        tail: list[rs.Stmt] = []
        if not trivial:
            catch = _frame("catch", rs.Path(err))
            handling = rs.Path("rt::Handling::new")
            if name is None:
                head.append(rs.Let("_handling", init=rs.Call(handling, (rs.Ref(catch),))))
            elif fn is not None and not fn.scope.is_global(name):
                if name not in fn.locals.handler_scoped:
                    raise self.ctx.unsupported("except-as name used outside its handler", handler)
                rust = fn.names[name]
                head.append(rs.Let(rust, init=catch))
                head.append(rs.Let("_handling", init=rs.Call(handling, (rs.Ref(rs.Path(rust)),))))
            else:
                exception = self.ctx.binding("exception")
                head.append(rs.Let(exception, init=catch))
                setter = self.exprs.scope_call("set", rs.StrLit(name), rs.Ref(rs.Path(exception)))
                head.append(rs.ExprStmt(rs.Try(setter)))
                head.append(
                    rs.Let("_handling", init=rs.Call(handling, (rs.Ref(rs.Path(exception)),)))
                )
                # The implicit `del name` at the end of the handler.
                unbind = self.exprs.scope_call("unbind", rs.StrLit(name))
                tail.append(rs.ExprStmt(rs.Try(unbind)))
        body = list(self.block(handler.body).stmts)
        if tail and not diverges(body):
            body += tail
        pattern = f"Err({err})" if guard is not None or not trivial else "Err(_)"
        return rs.Arm(pattern, rs.Block((*head, *body)), guard)

    def handler_test(self: StmtLowering, type_: ast.expr, err: rs.Path) -> rs.Expr:
        """Whether the exception `err` matches `except type_`."""
        elts = type_.elts if isinstance(type_, ast.Tuple) else [type_]
        builtin = [self.builtin_exception(e) for e in elts]
        if all(builtin):
            tests = [
                rs.MethodCall(
                    err, f"is_instance_of::<{self.use_exception(name)}>", (self.exprs.py(),)
                )
                for name in builtin
                if name is not None
            ]
            test = tests[0]
            for other in tests[1:]:
                test = rs.Binary("||", test, other)
            return test
        with self.exprs.fresh() as prelude:
            classes = self.exprs.value(type_)
            test: rs.Expr = rs.Try(rs.Call(rs.Path("rt::matches"), (rs.Ref(err), classes.ref())))
        if prelude:
            return rs.BlockExpr(rs.Block(tuple(prelude), test))
        return test

    def builtin_exception(self: StmtLowering, expr: ast.expr) -> str | None:
        """The name of the builtin exception class `expr` refers to, if it
        has a PyO3 type."""
        if not isinstance(expr, ast.Name) or expr.id not in BUILTIN_EXCEPTIONS:
            return None
        fn = self.ctx.function
        if fn is not None and not fn.scope.is_global(expr.id):
            if fn.scope.is_local(expr.id) or fn.scope.is_free(expr.id):
                return None
        if resolve_global(self.ctx.module.bindings, expr.id) is not Lookup.BUILTIN:
            return None
        return expr.id

    def use_exception(self: StmtLowering, name: str) -> str:
        rust = BUILTIN_EXCEPTIONS[name]
        self.exprs.use(f"pyo3::exceptions::{rust}")
        return rust

    # --- raise ---

    def raise_stmt(self: StmtLowering, exc: ast.expr | None, cause: ast.expr | None) -> rs.Stmt:
        if exc is None:
            return _err(_frame("reraise"))
        exception = self.exception(exc)
        if cause is None:
            return _err(_frame("raise", exception))
        if isinstance(cause, ast.Constant) and cause.value is None:
            cause_expr: rs.Expr = rs.MethodCall(self.exprs.py(), "None", ())
        else:
            value = self.exprs.value(cause)
            cause_expr = value.ref() if value.kind is Kind.REF else value.arg()
        return _err(_frame("raise_from", exception, cause_expr))

    def exception(self: StmtLowering, exc: ast.expr) -> rs.Expr:
        """What `frame.raise` takes for `raise exc`: `PyValueError::new_err(..)`
        for a builtin exception class with positional arguments, else the
        object."""
        match exc:
            case ast.Name() if (name := self.builtin_exception(exc)) is not None:
                return self.new_err(name, [])
            case ast.Call(ast.Name() as func, args, []) if (
                name := self.builtin_exception(func)
            ) is not None and not any(isinstance(a, ast.Starred) for a in args):
                return self.new_err(name, args)
        value = self.exprs.value(exc)
        if value.kind is Kind.PLACE:
            return rs.Ref(value.expr)
        if value.kind is Kind.REF:
            return value.ref()
        return value.owned()

    def new_err(self: StmtLowering, name: str, args: list[ast.expr]) -> rs.Expr:
        """`PyValueError::new_err("message")`, or with the arguments in a tuple."""
        rust = self.use_exception(name)
        match args:
            case []:
                arguments: rs.Expr = rs.Tuple(())
            case [ast.Constant(str() as text) as arg] if (
                self.exprs.constant(text, arg).kind is Kind.LITERAL
            ):
                arguments = rs.StrLit(text)
            case _:
                values = self.exprs.operands(list(args))
                arguments = rs.Tuple(tuple(rs.MethodCall(v.owned(), "unbind") for v in values))
        return rs.Call(rs.Path(f"{rust}::new_err"), (arguments,))

    # --- with ---

    def with_stmt(self: StmtLowering, stmt: ast.With) -> list[rs.Stmt]:
        closure = Closure(len(self.loops))
        self.closures.append(closure)
        try:
            prelude, call, entry = self.with_item(stmt, 0, closure)
        finally:
            self.closures.pop()
        lowered = Lowered(call, "flow" if closure.jumps else "unit", set(closure.jumps), True)
        # `frame.with` puts back the line it was called on for `__exit__`.
        self.line = entry
        if lowered.mode == "unit":
            return [*prelude, rs.ExprStmt(rs.Try(call))]
        return prelude + self.flow_stmts(lowered, call)

    def with_item(
        self: StmtLowering, stmt: ast.With, index: int, closure: Closure
    ) -> tuple[list[rs.Stmt], rs.Expr, int | None]:
        """The statements before `frame.with(&manager, |value| ..)` for item
        `index`, the call, and the line it is called on."""
        item = stmt.items[index]
        line = item.context_expr.lineno
        with self.exprs.fresh() as prelude:
            manager = self.exprs.value(item.context_expr)
            manager_ref = manager.ref()
        before = [*self.flush_items(), *self.mark(line, [*prelude, rs.ExprStmt(manager_ref)])]
        before += prelude
        entry = self.line
        fn = self.ctx.function
        target = item.optional_vars
        region = Region(stmt, index)
        body: list[rs.Stmt] = list(self.region_declarations(region))
        if target is None:
            param = "_"
        elif isinstance(target, ast.Name) and fn is not None and target.id in fn.locals.with_params:
            param = fn.names[target.id]
        else:
            param = self.ctx.binding("value")
            with self.exprs.fresh() as store_prelude:
                store = self.store(stmt, target, Value(rs.Path(param), Kind.REF))
            body += [*store_prelude, *store]
        ret: str | None = None
        if index + 1 < len(stmt.items):
            inner_before, inner, _ = self.with_item(stmt, index + 1, closure)
            block = rs.Block((*body, *inner_before), inner)
        else:
            stmts, tail = _return_as_tail(body + list(self.block(stmt.body).stmts))
            if tail is not None:
                pass
            elif not diverges(stmts):
                tail = _ok(_flow("Next") if closure.jumps else rs.Tuple(()))
            elif not closure.jumps:
                # A body that always raises says nothing about the type.
                ret = "PyResult<()>"
            block = rs.Block(tuple(stmts), tail)
        call = _frame("with", manager_ref, rs.Closure((param,), block, ret))
        return before, call, entry


def _return_as_tail(stmts: list[rs.Stmt]) -> tuple[list[rs.Stmt], rs.Expr | None]:
    """A final `return Ok(..);` as the tail expression `Ok(..)`."""
    last = stmts[-1] if stmts else None
    if isinstance(last, rs.Return) and isinstance(last.value, rs.Call):
        if last.value.func == rs.Path("Ok"):
            return stmts[:-1], last.value
    return stmts, None


def _unflow(stmt: rs.Stmt) -> rs.Stmt:
    """`return Ok(rt::Flow::Return(x))` as `return Ok(x)`, for a closure that
    returns the value itself, not looking into nested closures."""

    def visit(node):  # type: ignore[no-untyped-def]
        match node:
            case rs.Return(
                rs.Call(rs.Path("Ok"), (rs.Call(rs.Path("rt::Flow::Return"), (value,)),))
            ):
                return rs.Return(_ok(value))
            case rs.Closure() | rs.Fn():
                return node
            case rs.Block(stmts, tail):
                return rs.Block(tuple(visit(s) for s in stmts), tail)
            case rs.If(cond, then, orelse, pattern):
                other = None if orelse is None else visit(orelse)
                return rs.If(cond, visit(then), other, pattern)
            case rs.While(cond, body, label):
                return rs.While(cond, visit(body), label)
            case rs.Loop(body, label):
                return rs.Loop(visit(body), label)
            case rs.For(pat, iter_, body, label):
                return rs.For(pat, iter_, visit(body), label)
            case rs.LabeledBlock(label, body):
                return rs.LabeledBlock(label, visit(body))
            case rs.ExprStmt(rs.Match(scrutinee, arms)):
                arms = tuple(rs.Arm(a.pattern, visit(a.body), a.guard) for a in arms)
                return rs.ExprStmt(rs.Match(scrutinee, arms))
        return node

    return visit(stmt)


def _has_try(expr: rs.Expr) -> bool:
    from steelix.lower.rustflow import walk

    return any(isinstance(node, rs.Try) for node in walk(expr))


def _trivial(body: list[ast.stmt], lowering: StmtLowering) -> bool:
    """A handler that runs no Python code and does not look at the
    exception: it needs neither the traceback entry nor `sys.exception()`."""
    fn = lowering.ctx.function
    for stmt in body:
        match stmt:
            case ast.Pass() | ast.Break() | ast.Continue():
                continue
            case ast.Return(None | ast.Constant()):
                continue
            case ast.Assign([ast.Name(name)], ast.Constant()) if (
                fn is not None and fn.scope.is_local(name) and not fn.is_cell(name)
            ):
                continue
        return False
    return True
