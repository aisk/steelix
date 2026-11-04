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

from steelix.frontend.desugar import AsyncWith
from steelix.frontend.regions import Region, expression_body, suspends
from steelix.frontend.resolve import Lookup, resolve_global
from steelix.lower.function import BOUND
from steelix.lower.rustflow import children, diverges, returns_as_tail, statement_line
from steelix.lower.values import MAX_CALL_ARGS, Kind, Value
from steelix.rustast import nodes as rs
from steelix.rustast.pyo3 import BUILTIN_EXCEPTIONS
from steelix.rustast.rewrite import map_exprs

if TYPE_CHECKING:
    from steelix.lower.stmt import StmtLowering

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

    def suspends(self: StmtLowering, region: Region) -> bool:
        """Whether the closure of `region` is `async`: it awaits or yields
        (design §8). Other regions of a coroutine stay plain closures."""
        fn = self.ctx.function
        return fn is not None and fn.is_async and suspends(region.nodes())

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
            stmts, tail = returns_as_tail(stmts)
        elif jumps:
            stmts, tail = returns_as_tail(stmts)
            mode = "flow"
            if next_:
                tail = _ok(_flow("Next"))
        else:
            mode = "unit"
            if next_:
                tail = _ok(rs.Tuple(()))
            else:
                stmts, tail = returns_as_tail(stmts)
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
                out.append(rs.Let(rust, name in fn.locals.mutable, fn.local_type(name)))
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
        closure = lowered.call
        assert isinstance(closure, rs.Closure)
        inlined = None if self.suspends(region) else _inlined(closure)
        out: list[rs.Stmt]
        if self.suspends(region):
            # `let result: PyResult<..> = async { .. }.await;`
            awaited = rs.Await(rs.AsyncBlock(closure.body))
            out = [rs.Let(result, ty=closure.ret, init=awaited)]
        elif inlined is not None:
            # Nothing to catch: `let result: PyResult<..> = Ok(..);`
            before, outcome = inlined
            out = [*before, rs.Let(result, ty=closure.ret, init=outcome)]
        else:
            out = [rs.Let(result, init=rs.Call(closure, ()))]
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
        if stmt in self.ctx.flow.infallible_tries:
            # The handlers never run (`try: x = 1`).
            return [*self.block(stmt.body).stmts, *self.block(stmt.orelse).stmts]
        expr = expression_body(stmt)
        prelude: list[rs.Stmt] = []
        async_outcome: str | None = None
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
            if _has_await(scrutinee) and (inner or _has_try(scrutinee)):
                # `match async { .. }.await`: the type of `Ok(..)` must be
                # spelled out, a `PyResult` returned by PyO3 or the runtime
                # tells it.
                if _is_ok(scrutinee):
                    assert isinstance(scrutinee, rs.Call)
                    scrutinee = rs.Call(rs.Path("Ok::<_, PyErr>"), scrutinee.args)
                scrutinee = rs.Await(rs.AsyncBlock(rs.Block(tuple(inner), scrutinee)))
            elif inner:
                # The expression needs statements first: they can raise too.
                scrutinee = rs.Call(rs.Closure((), rs.Block(tuple(inner), scrutinee), ret), ())
            elif _has_try(scrutinee):
                # So can parts of the expression. `Ok(..)` says nothing about
                # the error type, a `PyResult` returned by PyO3 does.
                typed = _is_ok(scrutinee)
                closure = rs.Closure((), rs.Block((), scrutinee), ret if typed else None)
                scrutinee = rs.Call(closure, ())
            elif _is_ok(scrutinee):
                # It can not raise, but the analysis did not see that (a
                # global target, a captured variable): `Ok` needs its type.
                assert isinstance(scrutinee, rs.Call)
                scrutinee = rs.Call(rs.Path("Ok::<_, PyErr>"), scrutinee.args)
            handler_line = self.line
            ok_arms = self.expression_arms(stmt, body)
        else:
            region = Region(stmt, "body")
            closure, stmts = self.closure([region], lambda: list(self.block(stmt.body).stmts))
            lowered = self.finish_closure(closure, stmts)
            assert isinstance(lowered.call, rs.Closure)
            inlined = None if self.suspends(region) else _inlined(lowered.call)
            if self.suspends(region):
                # `match async { .. }.await` can not tell the error type of
                # a `?` in it: `let result: PyResult<..> = async { .. }.await;`
                outcome = self.ctx.temp("result")
                awaited = rs.Await(rs.AsyncBlock(lowered.call.body))
                prelude = [rs.Let(outcome, ty=lowered.call.ret, init=awaited)]
                scrutinee = rs.Path(outcome)
                async_outcome = outcome
            elif inlined is not None:
                # Only the outcome can be an error (`try: raise E`): the
                # statements run before the `match` on it.
                prelude, scrutinee = inlined
                if isinstance(scrutinee, rs.BlockExpr):
                    # With `let`s that must go away before the handlers run.
                    outcome = self.ctx.temp("result")
                    prelude = [rs.Let(outcome, ty=lowered.call.ret, init=scrutinee)]
                    scrutinee = rs.Path(outcome)
            else:
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
            arm, handler_line = self.handler_arm(handler, handler_line)
            arms.append(arm)
            ends.append(self.line)
            if handler.type is None:
                catch_all = True
        if not catch_all:
            err = self.ctx.binding("err")
            arms.append(rs.Arm(f"Err({err})", rs.Block((_err(rs.Path(err)),))))
        if async_outcome is not None and all(arm.pattern != "Err(_)" for arm in arms):
            # The `match` moves the exception out, nothing is left to drop
            # (with `Err(_)` the statement gets a block of its own instead).
            self.ctx.pinned.add(async_outcome)
        self.line = ends[0] if all(end == ends[0] for end in ends) else None
        match_ = rs.Match(scrutinee, tuple(arms))
        fn = self.ctx.function
        body = stmt.body[0]
        if (
            fn is not None
            and isinstance(body, ast.Assign)
            and isinstance(body.targets[0], ast.Name)
            and fn.locals.declared_at.get(body.targets[0].id) is stmt
        ):
            # `let x = match int.call1((s,)) { Ok(value) => value, .. };`
            name = body.targets[0].id
            rust = fn.names[name]
            valued = tuple(rs.Arm(a.pattern, _assigned_value(a.body, rust), a.guard) for a in arms)
            mutable = name in fn.locals.mutable
            return [*prelude, rs.Let(rust, mutable, init=rs.Match(scrutinee, valued))]
        return [*prelude, rs.ExprStmt(match_)]

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
                self.ctx.pinned.add(hoisted[name][0])  # `builtins` keeps it
            return rs.Path(hoisted[name][0])

        rewritten = map_exprs(rs.ExprStmt(expr), visit).expr
        if _has_try(rewritten):
            return expr, []
        lets: list[rs.Stmt] = [rs.Let(temp, init=rs.Try(call)) for temp, call in hoisted.values()]
        return rewritten, lets

    def handler_arm(
        self: StmtLowering, handler: ast.ExceptHandler, line: int | None
    ) -> tuple[rs.Arm, int | None]:
        """The `match` arm of `handler`, and the traceback line once its
        test ran (`line` before it)."""
        err = self.ctx.binding("err")
        fn = self.ctx.function
        name = handler.name
        if name is not None and fn is not None and name in fn.locals.handler_scoped:
            if fn.names[name] == "err":
                # `except E as err:` binds its own `err`: `let err = frame.catch(err);`
                err = "err"
        guard: rs.Expr | None = None
        if handler.type is not None:
            guard = self.handler_test(handler, rs.Path(err))
            if _has_try(guard):
                # `frame.matches` moved the line to the `except`.
                line = self.line = handler.lineno
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
        return rs.Arm(pattern, rs.Block((*head, *body)), guard), line

    def handler_test(self: StmtLowering, handler: ast.ExceptHandler, err: rs.Path) -> rs.Expr:
        """Whether the exception `err` matches `handler`: a test of its type
        for builtin exceptions, else `frame.matches(&err, line, || handler)?`,
        which evaluates the handler expression while `err` is being handled."""
        type_ = handler.type
        assert type_ is not None
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
            value = self.exprs.value(type_)
        if value.kind is Kind.PLACE and not value.temp:
            # A local: the handler may run again (in a loop), keep it.
            classes = rs.Call(rs.Path("Ok"), (value.owned(),))
        else:
            classes = value.result()
        closure = rs.Closure((), rs.Block(tuple(prelude), classes))
        line = handler.lineno
        if not self.ctx.module.traceback_lines and self.frame_line is not None:
            # Without line markers the traceback line stays the def line, also
            # after the test: an `except` line there would be stale.
            line = self.frame_line
        return rs.Try(_frame("matches", rs.Ref(err), rs.IntLit(line), closure))

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
                if len(args) <= MAX_CALL_ARGS:
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
        self.exprs.line = line
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
            stmts = body + list(self.block(stmt.body).stmts)
            tail: rs.Expr | None
            if not diverges(stmts):
                tail = _ok(_flow("Next") if closure.jumps else rs.Tuple(()))
            else:
                stmts, tail = returns_as_tail(stmts)
                if not closure.jumps:
                    # A body that always raises says nothing about the type.
                    ret = "PyResult<()>"
            block = rs.Block(tuple(stmts), tail)
        if isinstance(stmt, AsyncWith):
            # `frame.async_with(&manager, async |value| ..).await`
            closure_ = rs.Closure((param,), block, ret, is_async=True)
            return before, rs.Await(_frame("async_with", manager_ref, closure_)), entry
        if self.suspends(region):
            closure_ = rs.Closure((param,), block, ret, is_async=True)
            return before, rs.Await(_frame("with_async", manager_ref, closure_)), entry
        call = _frame("with", manager_ref, rs.Closure((param,), block, ret))
        return before, call, entry


def _has_await(expr: rs.Expr) -> bool:
    """Whether `expr` awaits, not looking into closures."""
    match expr:
        case rs.Await():
            return True
        case rs.Closure() | rs.AsyncBlock() | rs.Fn():
            return False
    return any(_has_await(child) for child in children(expr))


def _unflow(stmt: rs.Stmt) -> rs.Stmt:
    """`return Ok(rt::Flow::Return(x))` as `return Ok(x)`, for a closure that
    returns the value itself, not looking into nested closures."""

    def visit(node):  # type: ignore[no-untyped-def]
        match node:
            case rs.Return(
                rs.Call(rs.Path("Ok"), (rs.Call(rs.Path("rt::Flow::Return"), (value,)),))
            ):
                # `Ok(f()?)` is `f()`, as `Value.result` writes it.
                return rs.Return(value.expr if isinstance(value, rs.Try) else _ok(value))
            case rs.Closure() | rs.AsyncBlock() | rs.Fn():
                return node
            case rs.Block(stmts, tail):
                return rs.Block(tuple(visit(s) for s in stmts), tail)
            case rs.If(cond, then, orelse, pattern):
                other = None if orelse is None else visit(orelse)
                return rs.If(cond, visit(then), other, pattern)
            case rs.While(cond, body, label, pattern):
                return rs.While(cond, visit(body), label, pattern)
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


def _inlined(closure: rs.Closure) -> tuple[list[rs.Stmt], rs.Expr] | None:
    """The statements and the outcome of a `try` closure that does not
    need to be one: nothing in it can raise or jump out (no `?`, no
    `return`), only its outcome may be an `Err`. With `let`s, which must
    not live on after the `try` (a `frame.finally` guard, a temporary), it
    becomes a block instead."""
    body = closure.body
    if body.tail is None:
        return None

    def escapes(node: object) -> bool:
        match node:
            case rs.Try() | rs.Return():
                return True
            case rs.Closure() | rs.AsyncBlock() | rs.Fn():
                return False
        return any(escapes(child) for child in children(node))

    if any(escapes(stmt) for stmt in body.stmts) or escapes(body.tail):
        return None
    if any(isinstance(stmt, rs.Let) for stmt in body.stmts):
        return [], rs.BlockExpr(body)
    return list(body.stmts), body.tail


def _assigned_value(body: rs.Block, rust: str) -> rs.Block:
    """An arm ending in `x = value;` as one whose value is `value`; the
    others never end (the analysis made sure of it)."""
    stmts = body.stmts
    if stmts and isinstance(stmts[-1], rs.Assign) and stmts[-1].target == rs.Path(rust):
        return rs.Block(stmts[:-1], stmts[-1].value)
    assert diverges(stmts), stmts
    return body


def _is_ok(expr: rs.Expr) -> bool:
    return isinstance(expr, rs.Call) and expr.func == rs.Path("Ok")


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
