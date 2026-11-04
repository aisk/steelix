//! Calls with keyword arguments and `*` / `**` unpacking, see [`call!`].
//!
//! [`call!`]: crate::call!

use pyo3::exceptions::PyTypeError;
use pyo3::ffi;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyString, PyTuple};

/// How CPython names a callable in call errors (`_PyObject_FunctionStr`):
/// `module.qualname()`, or `qualname()` for builtins.
pub fn function_str(function: &Bound<'_, PyAny>) -> PyResult<String> {
    let Some(qualname) = function.getattr_opt("__qualname__")? else {
        return Ok(function.str()?.to_string());
    };
    match function.getattr_opt("__module__")? {
        Some(module) if !module.is_none() && module.ne("builtins")? => {
            Ok(format!("{}.{}()", module.str()?, qualname.str()?))
        }
        _ => Ok(format!("{}()", qualname.str()?)),
    }
}

/// The interned string for a keyword argument spelled in the source.
#[doc(hidden)]
pub fn keyword<'py>(py: Python<'py>, name: &'static str) -> Borrowed<'static, 'py, PyString> {
    crate::interned(py, name)
}

/// `function(args.., name=value..)` without `*` or `**`: a vectorcall
/// whose last `kwnames.len()` arguments are the keyword values, so that no
/// dict is built (and none taken apart again by a Python callee).
#[doc(hidden)]
pub fn vectorcall<'py>(
    function: &Bound<'py, PyAny>,
    args: &[Bound<'py, PyAny>],
    kwnames: &Bound<'py, PyTuple>,
) -> PyResult<Bound<'py, PyAny>> {
    let positional = args.len() - kwnames.len();
    // SAFETY: attached; `Bound` is a transparent object pointer, so `args`
    // is an array of `args.len()` objects, which outlive the call;
    // `kwnames` is a tuple of strings. The call returns a new reference or
    // NULL with an exception set.
    unsafe {
        let result = ffi::PyObject_Vectorcall(
            function.as_ptr(),
            args.as_ptr().cast(),
            positional,
            kwnames.as_ptr(),
        );
        Bound::from_owned_ptr_or_err(function.py(), result)
    }
}

/// Whether `value` can be iterated, by the test CPython uses after an
/// iteration failed.
fn is_iterable(value: &Bound<'_, PyAny>) -> bool {
    // SAFETY: valid object and type pointers.
    unsafe {
        let ty = ffi::Py_TYPE(value.as_ptr());
        !ffi::PyType_GetSlot(ty, ffi::Py_tp_iter).is_null()
            || ffi::PySequence_Check(value.as_ptr()) != 0
    }
}

/// `f(*args)` with nothing else positional: the value is converted to a tuple
/// when the call happens (`CALL_FUNCTION_EX`).
pub fn star_args<'py>(
    function: &Bound<'py, PyAny>,
    value: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyTuple>> {
    if let Ok(tuple) = value.cast_exact::<PyTuple>() {
        return Ok(tuple.clone());
    }
    if !is_iterable(value) {
        return Err(crate::raised(PyTypeError::new_err(format!(
            "{} argument after * must be an iterable, not {}",
            function_str(function)?,
            crate::type_name(value)
        ))));
    }
    // SAFETY: valid object pointer; returns a new `tuple` reference or NULL.
    unsafe {
        Bound::from_owned_ptr_or_err(value.py(), ffi::PySequence_Tuple(value.as_ptr()))
            .map(|tuple| tuple.cast_into_unchecked())
    }
}

/// `*value` among other positional arguments (`LIST_EXTEND`).
pub fn extend_args(args: &Bound<'_, PyList>, value: &Bound<'_, PyAny>) -> PyResult<()> {
    match args.call_method1(crate::interned(args.py(), "extend"), (value,)) {
        Ok(_) => Ok(()),
        Err(err) if err.is_instance_of::<PyTypeError>(value.py()) && !is_iterable(value) => {
            Err(crate::raised(PyTypeError::new_err(format!(
                "Value after * must be an iterable, not {}",
                crate::type_name(value)
            ))))
        }
        Err(err) => Err(err),
    }
}

/// `**value` (`DICT_MERGE`): adds the items of a mapping to the keyword
/// arguments, refusing duplicates.
pub fn merge_kwargs<'py>(
    function: &Bound<'py, PyAny>,
    kwargs: &Bound<'py, PyDict>,
    value: &Bound<'py, PyAny>,
) -> PyResult<()> {
    let keys = if let Ok(dict) = value.cast::<PyDict>() {
        dict.keys().into_any()
    } else {
        match value.getattr_opt("keys")? {
            Some(keys) => keys.call0()?,
            None => {
                return Err(crate::raised(PyTypeError::new_err(format!(
                    "{} argument after ** must be a mapping, not {}",
                    function_str(function)?,
                    crate::type_name(value)
                ))));
            }
        }
    };
    for key in keys.try_iter()? {
        let key = key?;
        let item = value.get_item(&key)?;
        if kwargs.contains(&key)? {
            let message = if key.is_instance_of::<PyString>() {
                format!(
                    "{} got multiple values for keyword argument '{}'",
                    function_str(function)?,
                    key.str()?
                )
            } else {
                format!("{} keywords must be strings", function_str(function)?)
            };
            return Err(crate::raised(PyTypeError::new_err(message)));
        }
        kwargs.set_item(key, item)?;
    }
    Ok(())
}

/// Calls a Python object with keyword arguments or `*` / `**` unpacking,
/// written like the Python call it comes from.
///
/// ```ignore
/// rt::call!(f, x, *rest, sep = ", ", **options)?  // f(x, *rest, sep=", ", **options)
/// ```
///
/// Arguments are evaluated in Python's order: the callee, the positional
/// arguments (iterating each `*` value right away), then the keyword
/// arguments (merging each `**` mapping right away). A lone `*args` is turned
/// into a tuple only when the call happens, like `CALL_FUNCTION_EX`.
#[macro_export]
macro_rules! call {
    (@parse $f:expr, [$($p:tt)*] [$($k:tt)*] $name:ident = $value:expr $(, $($rest:tt)*)?) => {
        $crate::call!(@parse $f, [$($p)*] [$($k)* (kw $name $value)] $($($rest)*)?)
    };
    (@parse $f:expr, [$($p:tt)*] [$($k:tt)*] $name:literal = $value:expr $(, $($rest:tt)*)?) => {
        $crate::call!(@parse $f, [$($p)*] [$($k)* (kw $name $value)] $($($rest)*)?)
    };
    (@parse $f:expr, [$($p:tt)*] [$($k:tt)*] * * $value:expr $(, $($rest:tt)*)?) => {
        $crate::call!(@parse $f, [$($p)*] [$($k)* (merge $value)] $($($rest)*)?)
    };
    (@parse $f:expr, [$($p:tt)*] [$($k:tt)*] * $value:expr $(, $($rest:tt)*)?) => {
        $crate::call!(@parse $f, [$($p)* (star $value)] [$($k)*] $($($rest)*)?)
    };
    (@parse $f:expr, [$($p:tt)*] [$($k:tt)*] $value:expr $(, $($rest:tt)*)?) => {
        $crate::call!(@parse $f, [$($p)* (arg $value)] [$($k)*] $($($rest)*)?)
    };
    // A lone `*args`: converted to a tuple after the keyword arguments.
    (@parse $f:expr, [(star $value:expr)] [$($k:tt)*]) => {{
        // In two steps, so that `$f` may also be a reference.
        let function = &$f;
        let function: &$crate::pyo3::Bound<'_, $crate::pyo3::PyAny> = function;
        // Borrowed, so that a local variable is not moved.
        let star = &$value;
        let kwargs = $crate::call!(@kwargs function, [$($k)*]);
        $crate::pyo3::types::PyAnyMethods::call(
            function,
            $crate::call::star_args(function, star)?,
            kwargs.as_ref(),
        )
    }};
    // Keyword arguments without `**`: their names are known, a tuple made
    // once for each call site.
    (@parse $f:expr, [$((arg $value:expr))*] [$((kw $name:tt $kwvalue:expr))+]) => {{
        // In two steps, so that `$f` may also be a reference.
        let function = &$f;
        let function: &$crate::pyo3::Bound<'_, $crate::pyo3::PyAny> = function;
        let py = $crate::pyo3::Bound::py(function);
        let args = [
            $($crate::pyo3::IntoPyObjectExt::into_bound_py_any($value, py)?,)*
            $($crate::pyo3::IntoPyObjectExt::into_bound_py_any($kwvalue, py)?,)+
        ];
        static KWNAMES: $crate::pyo3::sync::PyOnceLock<
            $crate::pyo3::Py<$crate::pyo3::types::PyTuple>,
        > = $crate::pyo3::sync::PyOnceLock::new();
        let kwnames = KWNAMES.get_or_try_init(py, || {
            let names = [$($crate::call::keyword(py, $crate::call!(@kwname $name))),+];
            $crate::pyo3::types::PyTuple::new(py, names).map($crate::pyo3::Bound::unbind)
        })?;
        $crate::call::vectorcall(function, &args, kwnames.bind(py))
    }};
    (@kwname $name:ident) => {
        $crate::function::python_name(stringify!($name))
    };
    (@kwname $name:literal) => {
        $name
    };
    (@parse $f:expr, [$((arg $value:expr))*] [$($k:tt)*]) => {{
        // In two steps, so that `$f` may also be a reference.
        let function = &$f;
        let function: &$crate::pyo3::Bound<'_, $crate::pyo3::PyAny> = function;
        let args = ($($value,)*);
        let kwargs = $crate::call!(@kwargs function, [$($k)*]);
        $crate::pyo3::types::PyAnyMethods::call(function, args, kwargs.as_ref())
    }};
    (@parse $f:expr, [$($p:tt)*] [$($k:tt)*]) => {{
        // In two steps, so that `$f` may also be a reference.
        let function = &$f;
        let function: &$crate::pyo3::Bound<'_, $crate::pyo3::PyAny> = function;
        let args = $crate::pyo3::types::PyList::empty($crate::pyo3::Bound::py(function));
        $($crate::call!(@arg args, $p);)*
        let kwargs = $crate::call!(@kwargs function, [$($k)*]);
        $crate::pyo3::types::PyAnyMethods::call(
            function,
            $crate::pyo3::types::PyListMethods::to_tuple(&args),
            kwargs.as_ref(),
        )
    }};
    (@arg $args:ident, (arg $value:expr)) => {
        $crate::pyo3::types::PyListMethods::append(&$args, $value)?
    };
    (@arg $args:ident, (star $value:expr)) => {{
        // Bound first so that `?` inside `$value` keeps its own type, and
        // borrowed so that a local variable is not moved.
        let value = &$value;
        $crate::call::extend_args(&$args, value)?
    }};
    (@kwargs $function:ident, []) => {
        ::std::option::Option::<$crate::pyo3::Bound<'_, $crate::pyo3::types::PyDict>>::None
    };
    (@kwargs $function:ident, [$($k:tt)+]) => {{
        let kwargs = $crate::pyo3::types::PyDict::new($crate::pyo3::Bound::py($function));
        $($crate::call!(@kwarg $function, kwargs, $k);)+
        ::std::option::Option::Some(kwargs)
    }};
    (@kwarg $function:ident, $kwargs:ident, (kw $name:ident $value:expr)) => {
        $crate::pyo3::types::PyDictMethods::set_item(
            &$kwargs,
            $crate::call::keyword(
                $crate::pyo3::Bound::py(&$kwargs),
                $crate::function::python_name(stringify!($name)),
            ),
            $value,
        )?
    };
    (@kwarg $function:ident, $kwargs:ident, (kw $name:literal $value:expr)) => {
        $crate::pyo3::types::PyDictMethods::set_item(
            &$kwargs,
            $crate::call::keyword($crate::pyo3::Bound::py(&$kwargs), $name),
            $value,
        )?
    };
    (@kwarg $function:ident, $kwargs:ident, (merge $value:expr)) => {{
        let value = &$value;
        $crate::call::merge_kwargs($function, &$kwargs, value)?
    }};
    ($f:expr $(, $($args:tt)*)?) => {
        $crate::call!(@parse $f, [] [] $($($args)*)?)
    };
}
