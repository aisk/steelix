"""Lower calls: plain calls, `rt::call!` for keywords and unpacking,
method calls, direct calls of module functions (design §7) and builtins
with a direct PyO3 spelling."""

from __future__ import annotations

import ast
from typing import TYPE_CHECKING

from steelix.frontend.module_bindings import is_immutable_constant
from steelix.frontend.resolve import Direct, Lookup, resolve_call
from steelix.frontend.scope import default_of, function_kind, params_of
from steelix.lower.names import rust_ident
from steelix.lower.values import MAX_CALL_ARGS, Kind, Operand, Value
from steelix.rustast import nodes as rs
from steelix.rustast.printer import string_literal
from steelix.rustast.pyo3 import INFALLIBLE_LEN

if TYPE_CHECKING:
    from steelix.lower.expr import ExprLowering

# Builtins with a direct PyO3 spelling, by name and number of arguments. Only
# used when the name can only be the builtin (design §12).
# Builtins with a function of the same name in `rt::builtins`, called with
# positional arguments only. Keep in sync with the runtime's `builtins.rs`.
_SHORTCUT_BUILTINS = frozenset(
    """
    all any callable chr dict divmod enumerate filter float frozenset getattr
    hasattr hash id int issubclass list map max min next open ord print range
    reversed round set setattr sorted sum tuple zip
    """.split()
)

_SPECIAL_BUILTINS = {
    ("len", 1),
    ("isinstance", 2),
    ("str", 1),
    ("repr", 1),
    ("bool", 1),
    ("abs", 1),
    ("type", 1),
    ("iter", 1),
}


def _try(expr: rs.Expr) -> rs.Expr:
    return rs.Try(expr)


def _rt(name: str, *args: rs.Expr) -> rs.Call:
    return rs.Call(rs.Path(f"rt::{name}"), args)


class CallLowering:
    """The part of `ExprLowering` for calls."""

    def call(
        self: ExprLowering,
        func: ast.expr,
        args: list[ast.expr],
        keywords: list[ast.keyword],
        node: ast.Call,
    ) -> Value:
        plain = not keywords and not any(isinstance(a, ast.Starred) for a in args)
        if (
            self.ctx.class_body is not None
            and self.ctx.function is None
            and isinstance(func, ast.Name)
            and func.id in _FRAME_BUILTINS
            and len(args) + len(keywords) < (2 if func.id in ("exec", "eval") else 1)
        ):
            # They look at the calling frame, which a native class body does not have.
            raise self.ctx.unsupported(f"{func.id}() in a class body", node)
        if isinstance(func, ast.Name) and self.is_global(func.id):
            special = self.global_call(func.id, node, plain)
            if special is not None:
                return special
        if len(args) > MAX_CALL_ARGS and not any(isinstance(a, ast.Starred) for a in args):
            # A tuple of them would not implement PyO3's `PyCallArgs`.
            raise self.ctx.unsupported(f"call with more than {MAX_CALL_ARGS} arguments", node)
        if not plain:
            return self.call_macro(func, args, keywords)
        if isinstance(func, ast.Attribute):
            return self.method_call(func, args)
        callee, *values = self.operands([func, *args])
        return self.call_value(callee, values)

    def call_value(self: ExprLowering, callee: Value, values: list[Value]) -> Value:
        if values:
            tuple_ = rs.Tuple(tuple(v.arg() for v in values))
            call = callee.method("call1", tuple_)
        else:
            call = callee.method("call0")
        return Value(_try(call), Kind.OWNED)

    def call_macro(
        self: ExprLowering, func: ast.expr, args: list[ast.expr], keywords: list[ast.keyword]
    ) -> Value:
        """A call with keyword arguments or unpacking: `rt::call!(f, x, *xs, k = v)`.

        Python evaluates the callee, the positional arguments, then the
        keyword arguments, which is the order the macro evaluates them in.
        """
        if isinstance(func, ast.Attribute):
            # `o.m(..)` looks the method up first.
            def method() -> Value:
                obj = self.value(func.value)
                getattr_ = obj.method("attr", rs.StrLit(func.attr))
                return Value(_try(getattr_), Kind.OWNED, base=func.attr)

            first: Operand = method
        else:
            first = func
        items: list[Operand] = [first]
        items += [a.value if isinstance(a, ast.Starred) else a for a in args]
        items += [k.value for k in keywords]
        callee, *values = self.operands(items)
        parts: list[rs.Expr] = []
        for arg, value in zip(args, values, strict=False):
            if isinstance(arg, ast.Starred):
                parts.append(rs.Tokens(("*", value.bound())))
            else:
                parts.append(value.arg())
        for keyword, value in zip(keywords, values[len(args) :], strict=True):
            if keyword.arg is None:
                parts.append(rs.Tokens(("**", value.bound())))
            else:
                parts.append(rs.Tokens((f"{_keyword(keyword.arg)} = ", value.arg())))
        macro = rs.MacroCall("rt::call", (callee.bound(), *parts))
        return Value(_try(macro), Kind.OWNED)

    def method_call(self: ExprLowering, func: ast.Attribute, args: list[ast.expr]) -> Value:
        """`o.m(args)`: `method1` when that keeps Python's order, which
        looks the method up before evaluating the arguments."""
        obj = self.value(func.value)
        start = len(self.prelude)
        getattr_ = obj.method("attr", rs.StrLit(func.attr))
        method = Value(_try(getattr_), Kind.OWNED, base=func.attr)
        callee, *values = self.operands(list(args), [method])
        if len(self.prelude) == start and all(v.is_pure for v in values):
            name = rs.StrLit(func.attr)
            if values:
                tuple_ = rs.Tuple(tuple(v.arg() for v in values))
                call = obj.method("method1", name, tuple_)
            else:
                call = obj.method("method0", name)
            return Value(_try(call), Kind.OWNED)
        return self.call_value(callee, values)

    def is_global(self: ExprLowering, name: str) -> bool:
        """Whether `name` is looked up in the module globals (and builtins)."""
        if self.comprehension_variable(name) is not None:
            return False
        fn = self.ctx.function
        if fn is None:
            return self.ctx.class_body is None or bool(self.comprehensions)
        return fn.scope.is_global(name)

    def global_call(self: ExprLowering, name: str, node: ast.Call, plain: bool) -> Value | None:
        """Calls of module functions and builtins with a direct Rust spelling."""
        bindings = self.ctx.module.bindings
        args = node.args
        if name == "super" and not args and not node.keywords:
            if not bindings.is_module_bound(name):
                return self.zero_arg_super(node)
        if self.comprehensions and _reads_frame(name, node, bindings.is_module_bound(name)):
            # In a comprehension the calling frame has the comprehension's
            # variables as locals too.
            raise self.ctx.unsupported(f"{name}() in a comprehension", node)
        if not bindings.is_module_bound(name) and not args and not node.keywords:
            # At module level the local namespace is the globals dict.
            namespace_name = name == "globals" or (
                name in ("locals", "vars") and self.ctx.at_module_level
            )
            if namespace_name:
                # `&Bound<PyDict>`, owned by the module: no clone to read it.
                namespace = self.module_call("globals")
                return Value(namespace, Kind.REF, typed="PyDict", pure=True)
            if name == "dir" and self.ctx.at_module_level:
                return Value(_try(self.module_call("dir")), Kind.OWNED)
            if name in ("locals", "vars", "dir"):
                # In a function they read the calling frame, which transpiled
                # code does not have.
                raise self.ctx.unsupported(f"{name}() in a function", node)
        if (
            name in ("exec", "eval")
            and not bindings.is_module_bound(name)
            and len(args) == 1
            and not isinstance(args[0], ast.Starred)
            and not node.keywords
        ):
            # Without a namespace they run in the calling frame, which
            # transpiled code does not have. At module level that is the
            # globals dict, which can be passed explicitly.
            if not self.ctx.at_module_level or bindings.is_module_bound("globals"):
                raise self.ctx.unsupported(f"{name}() without a namespace in a function", node)
            namespace = ast.copy_location(ast.Call(ast.Name("globals", ast.Load()), [], []), node)
            explicit = ast.copy_location(ast.Call(node.func, [args[0], namespace], []), node)
            return self.call(node.func, explicit.args, [], explicit)
        resolution = resolve_call(bindings, name, node, module_index=self.ctx.statement_index)
        if isinstance(resolution, Direct):
            return self.direct_call(resolution, node)
        if resolution is not Lookup.BUILTIN or not plain:
            return None
        if (name, len(args)) in _SPECIAL_BUILTINS:
            return self.builtin_call(name, self.operands(list(args)))
        if name in _SHORTCUT_BUILTINS and len(args) <= MAX_CALL_ARGS:
            values = self.operands(list(args))
            tuple_ = rs.Tuple(tuple(v.arg() for v in values))
            call = rs.Call(rs.Path(f"rt::builtins::{name}"), (self.py(), tuple_))
            return Value(_try(call), Kind.OWNED)
        return None

    def zero_arg_super(self: ExprLowering, node: ast.Call) -> Value:
        """`super()` in a method: `rt::super_(&__class__, slf)`, the class
        from the `__class__` cell and the first argument of the function."""
        fn = self.ctx.function
        if fn is None or not fn.scope.is_free("__class__"):
            raise self.ctx.unsupported("super() without arguments outside a method", node)
        positional = fn.node.args.posonlyargs + fn.node.args.args
        if not positional:
            raise self.ctx.unsupported("super() without arguments and no first argument", node)
        first = self.name(positional[0].arg, node)
        cell = rs.Path(fn.names["__class__"])
        return Value(_try(_rt("super_", cell, first.ref())), Kind.OWNED)

    def builtin_call(self: ExprLowering, name: str, args: list[Value]) -> Value:
        """Builtins whose PyO3 method does exactly what the builtin does."""
        arg = args[0]
        match name:
            case "len":
                length: rs.Expr = arg.method("len")
                if arg.typed not in INFALLIBLE_LEN:
                    length = _try(length)
                return Value(length, Kind.NATIVE, _rt("int", self.py(), length), base="len")
            case "isinstance":
                check = _try(arg.method("is_instance", args[1].ref()))
                return self.bool_value(check)
            case "bool":
                return self.bool_value(arg.truthy())
            case "type":
                return Value(arg.method("get_type"), Kind.OWNED, typed="PyType")
            case "iter":
                call = _try(arg.method("try_iter"))
                return Value(call, Kind.OWNED, typed="PyIterator")
            case "str" | "repr":
                call = _try(arg.method(name))
                return Value(call, Kind.OWNED, typed="PyString")
            case "abs":
                return Value(_try(arg.method("abs")), Kind.OWNED)
        raise AssertionError(name)

    def direct_call(self: ExprLowering, target: Direct, node: ast.Call) -> Value:
        """A call of a module level function bound at transpile time.

        Omitted parameters read their default from the function object
        (`m.default(..)`), after all arguments are evaluated, like CPython.
        Immutable literal defaults are written in place instead, unless the
        module assigns `__defaults__` or `__kwdefaults__` (design §6).
        """
        module = self.ctx.module
        fn = target.fn
        params = params_of(fn.args)
        n_args = len(node.args)
        values = self.operands([*node.args, *(k.value for k in node.keywords)])
        roles = ["value"] * len(values)
        order: list[int] = []
        per_slot: list[list[int]] = []
        for param, slot in zip(params, target.slots, strict=True):
            match slot.kind:
                case "arg" | "varargs":
                    indices = list(slot.indices)
                case "keyword" | "varkw":
                    indices = [n_args + j for j in slot.indices]
                case _:
                    default = default_of(fn.args, param)
                    if not module.bindings.writes_defaults and is_immutable_constant(default):
                        # `greet("Bob")` passes the literal default `"Hello"`.
                        assert isinstance(default, ast.Constant)
                        values.append(self.constant(default.value, default))
                    else:
                        read = self.module_call("default", rs.StrLit(fn.name), rs.StrLit(param))
                        values.append(Value(_try(read), Kind.OWNED))
                    roles.append(param)
                    indices = [len(values) - 1]
            for i in indices:
                roles[i] = param
            order += indices
            per_slot.append(indices)
        values = self.in_rust_order(values, order, roles)

        call_args: list[rs.Expr] = [self.module()]
        for slot, indices in zip(target.slots, per_slot, strict=True):
            used = [values[i] for i in indices]
            match slot.kind:
                case "varargs":
                    call_args.append(rs.Ref(self.tuple_of(used)))
                case "varkw":
                    names = [node.keywords[j].arg or "" for j in slot.indices]
                    call_args.append(rs.Ref(self.dict_of(names, used)))
                case _:
                    call_args.append(used[0].ref())
        call: rs.Expr = rs.Call(rs.Path(module.fn_names[fn.name]), tuple(call_args))
        if function_kind(fn) == "function":
            return Value(_try(call), Kind.OWNED)
        # `await f(x)` or `yield from g(x)` runs the future in place.
        caller = self.ctx.function
        if caller is not None and module.callgraph.in_cycle(caller.node, fn):
            # A recursive future would contain itself.
            call = rs.Call(rs.Path("Box::pin"), (call,))
        return Value(_try(rs.Await(_rt("delegate", call))), Kind.OWNED, awaited=True)

    def tuple_of(self: ExprLowering, values: list[Value]) -> rs.Expr:
        self.use("pyo3::types::PyTuple")
        if not values:
            return rs.Call(rs.Path("PyTuple::empty"), (self.py(),))
        array = rs.Array(tuple(self.elements(values)))
        return _try(rs.Call(rs.Path("PyTuple::new"), (self.py(), array)))

    def dict_of(self: ExprLowering, names: list[str], values: list[Value]) -> rs.Expr:
        if not values:
            self.use("pyo3::types::PyDict")
            return rs.Call(rs.Path("PyDict::new"), (self.py(),))
        self.use("pyo3::types::IntoPyDict")
        pairs = rs.Array(
            tuple(
                rs.Tuple((rs.StrLit(name), v))
                for name, v in zip(names, self.elements(values), strict=True)
            )
        )
        return _try(rs.MethodCall(pairs, "into_py_dict", (self.py(),)))


_FRAME_BUILTINS = frozenset({"locals", "vars", "dir", "exec", "eval", "globals", "super"})
"""Builtins that use the namespace of the calling frame when called without one."""


def _reads_frame(name: str, node: ast.Call, shadowed: bool) -> bool:
    """`locals()`, `vars()`, `dir()`, or `exec` and `eval` without a namespace."""
    if shadowed:
        return False
    if name in ("locals", "vars", "dir"):
        return not node.args and not node.keywords
    if name in ("exec", "eval"):
        return len(node.args) + len(node.keywords) < 2
    return False


def keyword_arg(name: str) -> str:
    return _keyword(name)


def _keyword(name: str) -> str:
    """A keyword argument name in `rt::call!`: an identifier if possible."""
    rust = rust_ident(name)
    if rust.removeprefix("r#") == name:
        return rust
    return string_literal(name)
