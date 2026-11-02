"""Lower Python statements."""

from __future__ import annotations

import ast

from steelix.frontend.bindings import is_constant_true
from steelix.lower.context import Context
from steelix.lower.expr import ExprLowering, Kind, Value
from steelix.rustast import nodes as rs


class StmtLowering:
    def __init__(self, ctx: Context) -> None:
        self.ctx = ctx
        self.exprs = ExprLowering(ctx)

    def block(self, stmts: list[ast.stmt]) -> rs.Block:
        out: list[rs.Stmt] = []
        for stmt in stmts:
            out += self.stmt(stmt)
        return rs.Block(tuple(out))

    def reachable(self, stmts: list[ast.stmt]) -> list[ast.stmt]:
        fn = self.ctx.function
        if fn is None:
            return stmts
        return [s for s in stmts if s not in fn.locals.unreachable]

    def stmt(self, stmt: ast.stmt) -> list[rs.Stmt]:
        fn = self.ctx.function
        if fn is not None and stmt in fn.locals.unreachable:
            return []
        match stmt:
            case ast.Expr(value):
                return self.expr_stmt(value)
            case ast.Assign(targets, value):
                return self.assign(stmt, targets, value)
            case ast.Return(value):
                return [rs.Return(self.return_value(value))]
            case ast.If():
                return [self.if_stmt(stmt)]
            case ast.While(test, body):
                block = self.block(self.reachable(body))
                if is_constant_true(test):
                    return [rs.Loop(block)]
                return [rs.While(self.exprs.condition(test), block)]
            case ast.Pass():
                return []
        raise self.ctx.unsupported(type(stmt).__name__, stmt)

    def expr_stmt(self, value: ast.expr) -> list[rs.Stmt]:
        if isinstance(value, ast.Constant):
            return []  # docstrings and other bare constants do nothing
        lowered = self.exprs.value(value)
        if lowered.kind in (Kind.PLACE, Kind.LITERAL) or (
            lowered.kind is Kind.REF and isinstance(lowered.expr, rs.Path)
        ):
            return []  # reading an assigned local has no effect
        return [rs.ExprStmt(lowered.expr)]

    def return_value(self, value: ast.expr | None) -> rs.Expr:
        if value is None:
            return rs.Call(
                rs.Path("Ok"), (rs.Call(rs.Path("rt::none"), (rs.Path(self.ctx.py()),)),)
            )
        return self.exprs.value(value).result()

    def if_stmt(self, stmt: ast.If) -> rs.If:
        cond = self.exprs.condition(stmt.test)
        then = self.block(self.reachable(stmt.body))
        orelse: rs.Block | rs.If | None = None
        if len(stmt.orelse) == 1 and isinstance(stmt.orelse[0], ast.If):
            orelse = self.if_stmt(stmt.orelse[0])
        elif stmt.orelse:
            orelse = self.block(self.reachable(stmt.orelse))
        return rs.If(cond, then, orelse)

    # --- assignment ---

    def assign(self, stmt: ast.Assign, targets: list[ast.expr], value: ast.expr) -> list[rs.Stmt]:
        names = [t.id for t in targets if isinstance(t, ast.Name)]
        lowered = self.exprs.value(value)
        if len(names) == 1:
            return [self.store(stmt, names[0], lowered)]
        # `a = b = value`: evaluate once, then assign left to right.
        temp = self.ctx.temp("value")
        out: list[rs.Stmt] = [rs.Let(temp, init=lowered.owned())]
        shared = Value(rs.Path(temp), Kind.PLACE)
        for name in names:
            out.append(self.store(stmt, name, shared))
        return out

    def store(self, stmt: ast.Assign, name: str, value: Value) -> rs.Stmt:
        fn = self.ctx.function
        if fn is None or not fn.scope.is_local(name):
            setter = rs.MethodCall(
                rs.Path(self.ctx.globals()), "set_item", (rs.StrLit(name), value.arg())
            )
            return rs.ExprStmt(rs.Try(setter))
        rust = fn.names[name]
        info = fn.locals
        if name in info.optional:
            return rs.Assign(rs.Path(rust), rs.Call(rs.Path("Some"), (value.owned(),)))
        if info.declared_at.get(name) is stmt:
            return rs.Let(rust, mutable=name in info.mutable, init=value.owned())
        return rs.Assign(rs.Path(rust), value.owned())
