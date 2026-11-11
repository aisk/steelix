//! `rt::function!` and `rt::def!`, which create function objects.

/// Creates a Python function object backed by a Rust function, like
/// evaluating a `def` statement (without binding the name) or a `lambda`.
///
/// The first argument is where the function is defined: the module `m`, the
/// namespace `ns` of a class body, or in a function body its `frame`. A
/// function defined in a class body is a method: its `__qualname__` is
/// `<class qualname>.<name>` and its annotations see the class namespace.
/// One defined in the body of the function `f` is `f.<locals>.<name>`.
///
/// The parameter list is written like the Python one, using the Rust names of
/// the parameters. Defaults are evaluated right away, in order, and stored in
/// `__defaults__` / `__kwdefaults__`; annotations are kept as Python source and
/// evaluated lazily (PEP 649). A Rust name that is not the Python one gets it
/// with `as`, for the function as for its parameters (`r#` is stripped
/// without that).
///
/// The Rust function takes the module, the captured variables and then one
/// argument per parameter, all as `&Bound` (`*args` and `**kwargs` as
/// `&Bound<PyTuple>` and `&Bound<PyDict>`, captured cells as `&rt::Cell`).
/// It returns a `PyResult` of the object, or of `()` when the function only
/// ever returns `None` (not a coroutine or generator).
///
/// ```ignore
/// rt::function!(m, greet(name, greeting = "Hello"))?;      // fn greet(m, name, greeting)
/// rt::function!(m, wrap(f, /, *args, **kwargs))?;          // fn wrap(m, f, args, kwargs)
/// rt::function!(m, area(r: "float") -> "float")?;          // annotations
/// rt::function!(ns, label_ as "label"(slf as "self"))?;    // Python names differ
/// // A nested function capturing `n` by value and the shared cell `count`:
/// rt::function!(frame, add[n, &count](x))?;
/// ```
///
/// Decorators come before the name, as in Python. `@name` and
/// `@name.attr.attr` read the name the way code where the function is defined
/// does (a global, or a name in the class body); any other expression is
/// written `@(expr)`. The decorator expressions are evaluated first, top to
/// bottom, then the function is created, then the decorators are applied
/// bottom-up, and the result is the decorated value:
///
/// ```ignore
/// rt::function!(ns, @property length(slf as "self"))?;
/// rt::function!(m, @(m.global("functools")?.method1("wraps", (f,))?) wrapper(*args))?;
/// ```
///
/// Options: `qualname = ".."` (defaults to the name, `<class>.<name>` for a
/// method or `<function>.<locals>.<name>` in a function body), `name = ".."`
/// (like `as`) and `doc = ".."`.
#[macro_export]
macro_rules! function {
    // --- decorators, collected in order: `{name a b}` for `@a.b`,
    // `{expr e}` for `@(e)` ---
    (@decorators $h:tt [$($d:tt)*] @ ($e:expr) $($rest:tt)*) => {
        $crate::function!(@decorators $h [$($d)* {expr $e}] $($rest)*)
    };
    (@decorators $h:tt [$($d:tt)*] @ $first:ident $($rest:tt)*) => {
        $crate::function!(@dotted $h [$($d)*] [$first] $($rest)*)
    };
    (@dotted $h:tt [$($d:tt)*] [$($p:ident)*] . $attr:ident $($rest:tt)*) => {
        $crate::function!(@dotted $h [$($d)*] [$($p)* $attr] $($rest)*)
    };
    (@dotted $h:tt [$($d:tt)*] [$($p:ident)*] $($rest:tt)*) => {
        $crate::function!(@decorators $h [$($d)* {name $($p)*}] $($rest)*)
    };
    // --- what kind of function: `async def`, a generator, or plain ---
    (@decorators $h:tt [$($d:tt)*] async $($rest:tt)*) => {
        $crate::function!(@signature $h [$($d)*] coroutine $($rest)*)
    };
    (@decorators $h:tt [$($d:tt)*] gen $($rest:tt)*) => {
        $crate::function!(@signature $h [$($d)*] generator $($rest)*)
    };
    (@decorators $h:tt [$($d:tt)*] $($rest:tt)*) => {
        $crate::function!(@signature $h [$($d)*] function $($rest)*)
    };
    (@signature ($mode:ident, $definer:expr) [$($d:tt)*] $kind:ident
        $f:ident $(as $n:literal)? $([$($caps:tt)*])? ($($params:tt)*) $(-> $ret:literal)?
        $(, $key:ident = $value:literal)* $(,)?) => {{
        let definer = $definer;
        $crate::function!(@$mode definer
            ($kind, $f, [$($n)?], [$($($caps)*)?], [$($params)*], [$($ret)?], [$($key = $value),*])
            $($d)*)
    }};
    (@value $definer:ident $sig:tt $($d:tt)*) => {
        $crate::function!(@decorate $definer $sig $($d)*)
    };
    (@bind $definer:ident ($kind:ident, $f:ident, $n:tt, $caps:tt, $params:tt, $ret:tt, $opts:tt) $($d:tt)*) => {
        $crate::function::Definer::bind(
            $definer,
            $crate::function::def_name(
                $crate::function!(@pyname $n $opts),
                $crate::function!(@opt qualname $opts),
                stringify!($f),
            ),
            $crate::function!(@decorate $definer ($kind, $f, $n, $caps, $params, $ret, $opts) $($d)*)?,
        )
    };
    // Evaluates the first decorator, then (recursively) the others and the
    // function, and applies the first decorator last.
    (@decorate $definer:ident $sig:tt {$($d:tt)*} $($rest:tt)*) => {{
        use $crate::pyo3::types::PyAnyMethods as _;
        let decorator = $crate::function!(@decorator $definer $($d)*);
        let function = $crate::function!(@decorate $definer $sig $($rest)*)?;
        decorator.call1((function,))
    }};
    (@decorate $definer:ident ($kind:ident, $f:ident, $n:tt, [$($caps:tt)*], [$($params:tt)*], $ret:tt, $opts:tt)) => {
        $crate::function!(@caps ($definer, $kind, $f, $n, [$($params)*], $ret, $opts) [] $($caps)*)
    };
    (@decorator $definer:ident expr $e:expr) => { $e };
    (@decorator $definer:ident name $first:ident $($attr:ident)*) => {{
        use $crate::Attributes as _;
        $crate::function::Definer::lookup($definer, stringify!($first))? $(.attr(stringify!($attr))?)*
    }};
    // --- captures: `n` by value, `&count` as a shared cell ---
    (@caps $h:tt [$($c:tt)*] & $id:ident $(as $n:literal)? $(, $($rest:tt)*)?) => {
        $crate::function!(@caps $h [$($c)* {cell $id, $crate::function!(@name $id $($n)?)}] $($($rest)*)?)
    };
    (@caps $h:tt [$($c:tt)*] $id:ident $(as $n:literal)? $(, $($rest:tt)*)?) => {
        $crate::function!(@caps $h [$($c)* {value $id, $crate::function!(@name $id $($n)?)}] $($($rest)*)?)
    };
    (@caps ($module:expr, $kind:ident, $f:ident, $fname:tt, [$($params:tt)*], $ret:tt, $opts:tt) [$($c:tt)*]) => {
        $crate::function!(@params ($module, $kind, $f, $fname, [$($c)*], $ret, $opts) [] [] [] [] [] pos $($params)*)
    };
    // --- parameters, sorted into positional-only, positional, `*args`,
    // keyword-only and `**kwargs` ---
    (@params $h:tt [$($po:tt)*] [$($p:tt)*] [$($v:tt)*] [$($k:tt)*] [$($vk:tt)*] $mode:ident / $(, $($rest:tt)*)?) => {
        $crate::function!(@params $h [$($p)*] [] [$($v)*] [$($k)*] [$($vk)*] $mode $($($rest)*)?)
    };
    (@params $h:tt [$($po:tt)*] [$($p:tt)*] [$($v:tt)*] [$($k:tt)*] [] $mode:ident
        * * $id:ident $(as $n:literal)? $(: $ann:literal)? $(, $($rest:tt)*)?) => {
        $crate::function!(@params $h [$($po)*] [$($p)*] [$($v)*] [$($k)*]
            [{$id, $crate::function!(@name $id $($n)?), [], [$($ann)?]}] $mode $($($rest)*)?)
    };
    (@params $h:tt [$($po:tt)*] [$($p:tt)*] [] [$($k:tt)*] [$($vk:tt)*] pos
        * $id:ident $(as $n:literal)? $(: $ann:literal)? $(, $($rest:tt)*)?) => {
        $crate::function!(@params $h [$($po)*] [$($p)*]
            [{$id, $crate::function!(@name $id $($n)?), [], [$($ann)?]}] [$($k)*] [$($vk)*] kw $($($rest)*)?)
    };
    (@params $h:tt [$($po:tt)*] [$($p:tt)*] [] [$($k:tt)*] [$($vk:tt)*] pos * $(, $($rest:tt)*)?) => {
        $crate::function!(@params $h [$($po)*] [$($p)*] [] [$($k)*] [$($vk)*] kw $($($rest)*)?)
    };
    (@params $h:tt [$($po:tt)*] [$($p:tt)*] [$($v:tt)*] [$($k:tt)*] [$($vk:tt)*] pos
        $id:ident $(as $n:literal)? $(: $ann:literal)? $(= $default:expr)? $(, $($rest:tt)*)?) => {
        $crate::function!(@params $h [$($po)*]
            [$($p)* {$id, $crate::function!(@name $id $($n)?), [$($default)?], [$($ann)?]}]
            [$($v)*] [$($k)*] [$($vk)*] pos $($($rest)*)?)
    };
    (@params $h:tt [$($po:tt)*] [$($p:tt)*] [$($v:tt)*] [$($k:tt)*] [$($vk:tt)*] kw
        $id:ident $(as $n:literal)? $(: $ann:literal)? $(= $default:expr)? $(, $($rest:tt)*)?) => {
        $crate::function!(@params $h [$($po)*] [$($p)*] [$($v)*]
            [$($k)* {$id, $crate::function!(@name $id $($n)?), [$($default)?], [$($ann)?]}]
            [$($vk)*] kw $($($rest)*)?)
    };
    (@params $h:tt [$($po:tt)*] [$($p:tt)*] [$($v:tt)*] [$($k:tt)*] [$($vk:tt)*] $mode:ident) => {
        $crate::function!(@build $h [$($po)*] [$($p)*] [$($v)*] [$($k)*] [$($vk)*])
    };
    // --- the function object ---
    (@build ($module:expr, $kind:ident, $f:ident, $fname:tt, [$({$ck:ident $c:ident, $cn:expr})*], [$($ret:literal)?], [$($key:ident = $value:literal),*])
        [$({$po:ident, $pon:expr, [$($pod:expr)?], [$($poa:literal)?]})*]
        [$({$p:ident, $pn:expr, [$($pd:expr)?], [$($pa:literal)?]})*]
        [$({$v:ident, $vn:expr, [], [$($va:literal)?]})?]
        [$({$k:ident, $kn:expr, [$($kd:expr)?], [$($ka:literal)?]})*]
        [$({$vk:ident, $vkn:expr, [], [$($vka:literal)?]})?]
    ) => {{
        static SIGNATURE: $crate::Signature = $crate::Signature {
            names: &[$($pon,)* $($pn,)* $($vn,)? $($kn,)* $($vkn,)?],
            posonly: <[&str]>::len(&[$($pon),*]),
            positional: <[&str]>::len(&[$($pon,)* $($pn),*]),
            varargs: <[&str]>::len(&[$($vn)?]) == 1,
            kwonly: <[&str]>::len(&[$($kn),*]),
            varkw: <[&str]>::len(&[$($vkn)?]) == 1,
        };
        static ANNOTATIONS: &[(&str, &str)] = &[
            $($(($pon, $poa),)?)* $($(($pn, $pa),)?)* $($(($vn, $va),)?)?
            $($(($kn, $ka),)?)* $($(($vkn, $vka),)?)? $(("return", $ret),)?
        ];
        let definer = $module;
        let module: &$crate::Module<'_> = $crate::function::Definer::module(definer);
        let py = module.py();
        let defaults = ::std::vec![
            $($($crate::pyo3::IntoPyObjectExt::into_bound_py_any($pod, py)?,)?)*
            $($($crate::pyo3::IntoPyObjectExt::into_bound_py_any($pd, py)?,)?)*
        ];
        let kwdefaults = ::std::vec![
            $($(($kn, $crate::pyo3::IntoPyObjectExt::into_bound_py_any($kd, py)?),)?)*
        ];
        let closure = ::std::vec![$($crate::function!(@capture $ck $c),)*];
        static CLOSURE_NAMES: &[&str] = &[$($cn),*];
        let body = $crate::function!(@body $kind, $f, [$({$ck $c, $cn})*],
            [$($po)* $($p)*], [$($v)?], [$($k)*], [$($vk)?]);
        $crate::Function::create(module, $crate::FunctionDef {
            name: $crate::function!(@pyname $fname [$($key = $value),*]),
            qualname: $crate::function!(@opt qualname [$($key = $value),*]),
            rust_name: stringify!($f),
            doc: $crate::function!(@opt doc [$($key = $value),*]),
            signature: &SIGNATURE,
            defaults,
            kwdefaults,
            closure,
            closure_names: CLOSURE_NAMES,
            annotations: ANNOTATIONS,
            classdict: $crate::function::Definer::classdict(definer),
            class_qualname: $crate::function::Definer::class_qualname(definer),
            enclosing_function: $crate::function::Definer::enclosing_function(definer),
            body,
        })
    }};
    // --- the body: a closure that hands the bound arguments to the Rust fn ---
    (@body function, $f:ident, [$({$ck:ident $c:ident, $cn:expr})*],
        [$($p:ident)*], [$($v:ident)?], [$($k:ident)*], [$($vk:ident)?]) => {
        $crate::function::Kind::Function(|module, closure, args| {
            // Named before the parameters, which may shadow the function.
            let target = $f;
            let [$($c),*] = closure else {
                unreachable!("the closure has one cell per captured variable")
            };
            $($crate::function!(@captured $ck $c, $cn);)*
            let [$($p,)* $($v,)? $($k,)* $($vk,)?] = args else {
                unreachable!("`Function` binds one value per parameter")
            };
            $(let $v = $crate::function::as_tuple($v);)?
            $(let $vk = $crate::function::as_dict($vk);)?
            $crate::function!(@call target(module; $($c),*; $($p,)* $($v,)? $($k,)* $($vk,)?))
        })
    };
    // A coroutine or generator: the Rust fn is an `async fn`. At the call,
    // `capture` reads the captured variables; the coroutine object keeps
    // them with the module and the arguments (`rt::coroutine::Locals`), and
    // `start` returns a future that keeps an `Rc` of them and awaits the fn
    // with references into it.
    (@body $kind:ident, $f:ident, [$({$ck:ident $c:ident, $cn:expr})*],
        [$($p:ident)*], [$($v:ident)?], [$($k:ident)*], [$($vk:ident)?]) => {
        $crate::function!(@kind $kind)($crate::coroutine::AsyncBody {
            capture: |closure| {
                let [$($c),*] = closure else {
                    unreachable!("the closure has one cell per captured variable")
                };
                ::std::result::Result::Ok(::std::vec![$($crate::function!(@capture_async $ck $c, $cn)),*])
            },
            start: |locals| {
                let target = $f;
                ::std::boxed::Box::pin(async move {
                    let [$($c),*] = &locals.closure[..] else {
                        unreachable!("one value per captured variable")
                    };
                    $(let $c = $crate::function!(@captured_ref $ck $c);)*
                    let [$($p,)* $($v,)? $($k,)* $($vk,)?] = &locals.args[..] else {
                        unreachable!("`Function` binds one value per parameter")
                    };
                    $(let $v = $crate::function::as_tuple($v);)?
                    $(let $vk = $crate::function::as_dict($vk);)?
                    target(&locals.module, $($c,)* $($p,)* $($v,)? $($k,)* $($vk,)?).await
                })
            },
        })
    };
    (@kind coroutine) => { $crate::function::Kind::Coroutine };
    (@kind generator) => { $crate::function::Kind::Generator };
    (@capture_async value $c:ident, $cn:expr) => {
        $crate::coroutine::Captured::Value($crate::function::cell_contents($c, $cn)?)
    };
    (@capture_async cell $c:ident, $cn:expr) => {
        $crate::coroutine::Captured::Cell($crate::Cell::free($c, $cn))
    };
    (@captured_ref value $c:ident) => { $c.value() };
    (@captured_ref cell $c:ident) => { $c.cell() };
    (@name $id:ident) => { stringify!($id) };
    (@name $id:ident $n:literal) => { $n };
    (@capture value $c:ident) => { $crate::function::capture(&$c)? };
    (@capture cell $c:ident) => { $crate::Cell::object(&$c) };
    (@captured value $c:ident, $cn:expr) => {
        let $c = $crate::function::cell_contents($c, $cn)?;
    };
    (@captured cell $c:ident, $cn:expr) => {
        let $c = $crate::Cell::free($c, $cn);
    };
    // `module` is passed in as a token so that it keeps the hygiene context
    // of the closure parameter above. The fn returns an object or `()`.
    (@call $f:ident($module:ident; $($c:ident),*; $($p:ident,)*)) => {
        $f($module, $(&$c,)* $($p),*)
            .map(|value| $crate::function::IntoReturn::into_return(value, $module.py()))
    };
    (@opt name [name = $value:literal $(, $($rest:tt)*)?]) => { ::std::option::Option::Some($value) };
    (@opt qualname [qualname = $value:literal $(, $($rest:tt)*)?]) => { ::std::option::Option::Some($value) };
    (@opt doc [doc = $value:literal $(, $($rest:tt)*)?]) => { ::std::option::Option::Some($value) };
    (@opt $want:ident [$key:ident = $value:literal $(, $($rest:tt)*)?]) => {
        $crate::function!(@opt $want [$($($rest)*)?])
    };
    (@opt $want:ident []) => { ::std::option::Option::None };
    (@pyname [$n:literal] $opts:tt) => { ::std::option::Option::Some($n) };
    (@pyname [] $opts:tt) => { $crate::function!(@opt name $opts) };
    // --- entry point ---
    ($definer:expr, $($rest:tt)*) => {
        $crate::function!(@decorators (value, $definer) [] $($rest)*)
    };
}

/// Executes a `def` statement: creates the function like [`function!`],
/// applies its decorators and binds the result to the function's name, in
/// the module's globals or in the namespace of a class body.
///
/// ```ignore
/// rt::def!(m, fib(n))?;
/// rt::def!(m, @(trace(m, &rt::str(py, "outer"))?) greet(name), doc = "Say hello.")?;
/// rt::def!(ns, __init__(slf as "self", x, y = 0))?;          // `Point.__init__`
/// rt::def!(ns, @label.setter label_ as "label"(slf as "self", value))?;
/// ```
#[macro_export]
macro_rules! def {
    ($definer:expr, $($rest:tt)*) => {
        $crate::function!(@decorators (bind, $definer) [] $($rest)*)
    };
}
