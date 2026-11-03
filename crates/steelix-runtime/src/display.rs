//! List, tuple, set and dict displays with `*` and `**` unpacking, see
//! [`list!`], [`tuple!`], [`set!`] and [`dict!`].
//!
//! The macros evaluate their arguments in source order and do with each what
//! CPython's bytecode does at that point: `*value` is iterated right away
//! (`LIST_EXTEND`, `SET_UPDATE`), `**value` is merged right away
//! (`DICT_UPDATE`), and the plain elements between them are evaluated as a
//! group before they are inserted (`BUILD_SET`, `BUILD_MAP`), so hashing a
//! key never happens before a later element of its group is evaluated.
//!
//! [`list!`]: crate::list!
//! [`tuple!`]: crate::tuple!
//! [`set!`]: crate::set!
//! [`dict!`]: crate::dict!

use pyo3::exceptions::{PyAttributeError, PyTypeError};
use pyo3::ffi;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PySet};

/// `{.., *value}` (`SET_UPDATE`): adds the items of an iterable.
pub fn update_set(set: &Bound<'_, PySet>, value: &Bound<'_, PyAny>) -> PyResult<()> {
    set.call_method1("update", (value,)).map(drop)
}

/// `{.., **value}` (`DICT_UPDATE`): merges a mapping, raising `TypeError`
/// like CPython when it has no `keys`.
pub fn update_dict(dict: &Bound<'_, PyDict>, value: &Bound<'_, PyAny>) -> PyResult<()> {
    let py = dict.py();
    // SAFETY: valid object pointers; `dict` is a dict.
    if unsafe { ffi::PyDict_Update(dict.as_ptr(), value.as_ptr()) } == 0 {
        return Ok(());
    }
    let err = PyErr::fetch(py);
    if err.is_instance_of::<PyAttributeError>(py) {
        return Err(crate::raised(PyTypeError::new_err(format!(
            "'{}' object is not a mapping",
            crate::type_name(value)
        ))));
    }
    Err(err)
}

/// A list display with `*` unpacking, written like the Python display.
///
/// ```ignore
/// rt::list!(py, first, *rest)?  // [first, *rest]
/// ```
#[macro_export]
macro_rules! list {
    ($py:expr $(, $($items:tt)*)?) => {{
        let list = $crate::pyo3::types::PyList::empty($py);
        $crate::list!(@items list, $($($items)*)?);
        ::std::result::Result::<_, $crate::pyo3::PyErr>::Ok(list)
    }};
    (@items $list:ident, * $value:expr $(, $($rest:tt)*)?) => {
        {
            // Bound first so that `?` inside `$value` keeps its own type, and
            // borrowed so that a local variable is not moved.
            let value = &$value;
            $crate::call::extend_args(&$list, value)?;
        }
        $crate::list!(@items $list, $($($rest)*)?);
    };
    (@items $list:ident, $value:expr $(, $($rest:tt)*)?) => {
        $crate::pyo3::types::PyListMethods::append(&$list, $value)?;
        $crate::list!(@items $list, $($($rest)*)?);
    };
    (@items $list:ident, ) => {};
}

/// A tuple display with `*` unpacking: [`list!`](crate::list!), then a tuple.
///
/// ```ignore
/// rt::tuple!(py, *args, last)?  // (*args, last)
/// ```
#[macro_export]
macro_rules! tuple {
    ($py:expr $(, $($items:tt)*)?) => {
        $crate::list!($py $(, $($items)*)?)
            .map(|list| $crate::pyo3::types::PyListMethods::to_tuple(&list))
    };
}

/// A set display with `*` unpacking.
///
/// ```ignore
/// rt::set!(py, *a, b)?  // {*a, b}
/// ```
#[macro_export]
macro_rules! set {
    ($py:expr $(, $($items:tt)*)?) => {{
        let py = $py;
        let set = $crate::pyo3::types::PySet::empty(py)?;
        $crate::set!(@items py, set, [] $($($items)*)?);
        ::std::result::Result::<_, $crate::pyo3::PyErr>::Ok(set)
    }};
    (@items $py:ident, $set:ident, [$($group:expr),*] * $value:expr $(, $($rest:tt)*)?) => {
        $crate::set!(@add $py, $set, [$($group),*]);
        {
            let value = &$value;
            $crate::display::update_set(&$set, value)?;
        }
        $crate::set!(@items $py, $set, [] $($($rest)*)?);
    };
    (@items $py:ident, $set:ident, [$($group:expr),*] $value:expr $(, $($rest:tt)*)?) => {
        $crate::set!(@items $py, $set, [$($group,)* $value] $($($rest)*)?);
    };
    (@items $py:ident, $set:ident, [$($group:expr),*]) => {
        $crate::set!(@add $py, $set, [$($group),*]);
    };
    (@add $py:ident, $set:ident, []) => {};
    (@add $py:ident, $set:ident, [$($group:expr),+]) => {
        // `BUILD_SET`: the elements are evaluated, then added.
        for item in [$($crate::pyo3::IntoPyObjectExt::into_bound_py_any($group, $py)?),+] {
            $crate::pyo3::types::PySetMethods::add(&$set, item)?;
        }
    };
}

/// A dict display with `**` unpacking, written like the Python display with
/// `(key, value)` for its items.
///
/// ```ignore
/// rt::dict!(py, **defaults, ("debug", true))?  // {**defaults, "debug": True}
/// ```
#[macro_export]
macro_rules! dict {
    ($py:expr $(, $($items:tt)*)?) => {{
        let py = $py;
        let dict = $crate::pyo3::types::PyDict::new(py);
        $crate::dict!(@items py, dict, [] $($($items)*)?);
        ::std::result::Result::<_, $crate::pyo3::PyErr>::Ok(dict)
    }};
    (@items $py:ident, $dict:ident, [$(($k:expr, $v:expr))*] * * $value:expr $(, $($rest:tt)*)?) => {
        $crate::dict!(@add $py, $dict, [$(($k, $v))*]);
        {
            let value = &$value;
            $crate::display::update_dict(&$dict, value)?;
        }
        $crate::dict!(@items $py, $dict, [] $($($rest)*)?);
    };
    (@items $py:ident, $dict:ident, [$(($k:expr, $v:expr))*] ($key:expr, $value:expr) $(, $($rest:tt)*)?) => {
        $crate::dict!(@items $py, $dict, [$(($k, $v))* ($key, $value)] $($($rest)*)?);
    };
    (@items $py:ident, $dict:ident, [$(($k:expr, $v:expr))*]) => {
        $crate::dict!(@add $py, $dict, [$(($k, $v))*]);
    };
    (@add $py:ident, $dict:ident, []) => {};
    (@add $py:ident, $dict:ident, [$(($k:expr, $v:expr))+]) => {
        // `BUILD_MAP`: the keys and values are evaluated, then inserted.
        for (key, value) in [$((
            $crate::pyo3::IntoPyObjectExt::into_bound_py_any($k, $py)?,
            $crate::pyo3::IntoPyObjectExt::into_bound_py_any($v, $py)?,
        )),+] {
            $crate::pyo3::types::PyDictMethods::set_item(&$dict, key, value)?;
        }
    };
}
