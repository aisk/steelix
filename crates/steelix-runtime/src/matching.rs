//! Structural pattern matching: the checks of a `case` pattern.
//!
//! Generated code turns each `case` into an `if let .. && ..` chain (design
//! §8). Each helper here does what one CPython instruction does for one
//! level of the pattern (`MATCH_SEQUENCE` with the length check and the
//! unpacking, `MATCH_MAPPING` with `MATCH_KEYS`, `MATCH_CLASS`) and hands back
//! all the values of that level at once, as an array the chain destructures.
//! The chain then checks the subpatterns in order, so evaluation order,
//! short-circuiting and error messages are CPython's (`Python/codegen.c`,
//! `Python/ceval.c` of 3.14).
//!
//! ```ignore
//! // case ["go", direction]:
//! if let Some([item, direction]) = rt::match_seq(&subject)? && item.eq("go")? { .. }
//! // case Point(x=0, y=y):
//! if let Some([x, y]) = rt::match_class(&subject, &m.global("Point")?, 0, ["x", "y"])?
//!     && x.eq(0)?
//! { .. }
//! ```

use pyo3::IntoPyObjectExt;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::ffi;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PySet, PyString, PyTuple, PyType};

use crate::interned;

/// Set on the builtin types whose single positional subpattern matches the
/// subject itself (`case int(x)`), CPython's `_Py_TPFLAGS_MATCH_SELF`.
const TPFLAGS_MATCH_SELF: std::ffi::c_ulong = 1 << 22;

fn has_flag(value: &Bound<'_, PyAny>, flag: std::ffi::c_ulong) -> bool {
    // SAFETY: the type of a valid object.
    unsafe { ffi::PyType_GetFlags(value.get_type_ptr()) & flag != 0 }
}

/// `MATCH_SEQUENCE`: the type has `Py_TPFLAGS_SEQUENCE` (`list`, `tuple`,
/// `range`, `collections.abc.Sequence` subclasses and registered classes;
/// not `str`, `bytes` or `bytearray`).
fn is_sequence(value: &Bound<'_, PyAny>) -> bool {
    has_flag(value, ffi::Py_TPFLAGS_SEQUENCE)
}

/// `MATCH_MAPPING`: the type has `Py_TPFLAGS_MAPPING`.
fn is_mapping(value: &Bound<'_, PyAny>) -> bool {
    has_flag(value, ffi::Py_TPFLAGS_MAPPING)
}

/// `[a, b]`: a sequence of exactly `N` items, unpacked by iterating it
/// (`UNPACK_SEQUENCE`).
///
/// ```ignore
/// if let Some([key, value]) = rt::match_seq(&subject)? { .. }
/// ```
pub fn match_seq<'py, const N: usize>(
    subject: &Bound<'py, PyAny>,
) -> PyResult<Option<[Bound<'py, PyAny>; N]>> {
    if !is_sequence(subject) || subject.len()? != N {
        return Ok(None);
    }
    crate::unpack(subject).map(Some)
}

/// `[a, *rest, b]`: a sequence of at least `B + A` items, unpacked like
/// `a, *rest, b = subject` (`UNPACK_EX`). The starred name gets a list.
///
/// ```ignore
/// if let Some(([first], rest, [])) = rt::match_seq_star(&subject)? { .. }
/// ```
#[allow(clippy::type_complexity)]
pub fn match_seq_star<'py, const B: usize, const A: usize>(
    subject: &Bound<'py, PyAny>,
) -> PyResult<
    Option<(
        [Bound<'py, PyAny>; B],
        Bound<'py, PyAny>,
        [Bound<'py, PyAny>; A],
    )>,
> {
    if !match_seq_min(subject, B + A)? {
        return Ok(None);
    }
    crate::unpack_star(subject).map(Some)
}

/// A sequence of exactly `len` items, for patterns that bind nothing from
/// it: `[]`, `[_, _]`.
pub fn match_seq_len(subject: &Bound<'_, PyAny>, len: usize) -> PyResult<bool> {
    Ok(is_sequence(subject) && subject.len()? == len)
}

/// A sequence of at least `len` items: `[_, *_]`, and patterns with `*_`
/// whose other items are then read by index (`subject.get_item(0)?`,
/// `rt::item_from_end(&subject, 1)?`), as CPython does for them. A pattern
/// that is only `[*_]` (`len` 0) does not call `len()`.
pub fn match_seq_min(subject: &Bound<'_, PyAny>, len: usize) -> PyResult<bool> {
    Ok(is_sequence(subject) && (len == 0 || subject.len()? >= len))
}

/// `subject[len(subject) - back]`: an item after `*_` in a sequence pattern,
/// read with a nonnegative index like CPython does (the subject may not
/// support negative ones). `len()` is called again each time.
pub fn item_from_end<'py>(subject: &Bound<'py, PyAny>, back: usize) -> PyResult<Bound<'py, PyAny>> {
    let len = subject.len()? as isize;
    subject.get_item(len - back as isize)
}

/// `{}` and the start of mapping patterns with keys that are not literals:
/// a mapping with at least `len` items (`len()` is not called for 0).
pub fn match_mapping(subject: &Bound<'_, PyAny>, len: usize) -> PyResult<bool> {
    Ok(is_mapping(subject) && (len == 0 || subject.len()? >= len))
}

/// `{"a": x, "b": y}`: a mapping with at least `N` items that has all the
/// `keys` (an array or a tuple of them), and their values. Like
/// `match_mapping` followed by `match_keys`.
///
/// ```ignore
/// if let Some([x, y]) = rt::match_map(&subject, ["a", "b"])? { .. }
/// ```
pub fn match_map<'py, K, const N: usize>(
    subject: &Bound<'py, PyAny>,
    keys: K,
) -> PyResult<Option<[Bound<'py, PyAny>; N]>>
where
    K: IntoPyObject<'py>,
{
    if !match_mapping(subject, N)? {
        return Ok(None);
    }
    match_keys(subject, keys)
}

/// `MATCH_KEYS`: the values of `keys` in the mapping `subject`, looked up
/// with `subject.get(key, sentinel)` (no `__missing__`, no `KeyError`) and
/// stopping at the first missing one. A key that is there twice (two value
/// patterns that are equal) raises `ValueError`.
pub fn match_keys<'py, K, const N: usize>(
    subject: &Bound<'py, PyAny>,
    keys: K,
) -> PyResult<Option<[Bound<'py, PyAny>; N]>>
where
    K: IntoPyObject<'py>,
{
    let py = subject.py();
    let keys = keys.into_bound_py_any(py)?;
    let mut values = Vec::with_capacity(N);
    if N > 0 {
        let get = subject.getattr(interned(py, "get"))?;
        let seen = PySet::empty(py)?;
        // SAFETY: `PyBaseObject_Type` is a static type object.
        let object =
            unsafe { Bound::from_borrowed_ptr(py, (&raw mut ffi::PyBaseObject_Type).cast()) };
        let missing = object.call0()?;
        for key in keys.try_iter()? {
            let key = key?;
            if seen.contains(&key)? {
                return Err(crate::raised(PyValueError::new_err(format!(
                    "mapping pattern checks duplicate key ({})",
                    key.repr()?
                ))));
            }
            seen.add(&key)?;
            let value = get.call1((&key, &missing))?;
            if value.is(&missing) {
                return Ok(None);
            }
            values.push(value);
        }
    }
    let count = values.len();
    match values.try_into() {
        Ok(values) => Ok(Some(values)),
        Err(_) => panic!("rt::match_keys: {count} keys for {N} values"),
    }
}

/// `**rest` in a mapping pattern: a new dict with the items of `subject`
/// (`dict.update`) without the `keys` the pattern matched.
pub fn match_rest<'py, K>(subject: &Bound<'py, PyAny>, keys: K) -> PyResult<Bound<'py, PyAny>>
where
    K: IntoPyObject<'py>,
{
    let py = subject.py();
    let keys = keys.into_bound_py_any(py)?;
    let rest = PyDict::new(py);
    crate::display::update_dict(&rest, subject)?;
    for key in keys.try_iter()? {
        rest.as_any().del_item(key?)?;
    }
    Ok(rest.into_any())
}

/// `Point()`: `cls` is a class (else `TypeError`) and `subject` an instance
/// of it.
pub fn match_instance(subject: &Bound<'_, PyAny>, cls: &Bound<'_, PyAny>) -> PyResult<bool> {
    let cls = class(cls)?;
    subject.is_instance(cls.as_any())
}

/// `Point(x, y=0)` (`MATCH_CLASS`): `cls` is a class (else `TypeError`),
/// `subject` an instance of it, and it has the attributes named by the
/// first `positional` items of `cls.__match_args__` and by `keywords`. Their
/// values, positional ones first. A missing attribute (`AttributeError`) is
/// no match; other errors propagate. The builtins `bool`, `bytearray`,
/// `bytes`, `dict`, `float`, `frozenset`, `int`, `list`, `set`, `str` and
/// `tuple` (and their subclasses without `__match_args__`) take one
/// positional subpattern, which matches the subject itself.
///
/// ```ignore
/// if let Some([x, y]) = rt::match_class(&subject, &m.global("Point")?, 2, [])? { .. }
/// ```
pub fn match_class<'py, const K: usize, const N: usize>(
    subject: &Bound<'py, PyAny>,
    cls: &Bound<'py, PyAny>,
    positional: usize,
    keywords: [&'static str; K],
) -> PyResult<Option<[Bound<'py, PyAny>; N]>> {
    assert_eq!(positional + K, N, "rt::match_class: wrong number of values");
    let py = subject.py();
    let cls = class(cls)?;
    if !subject.is_instance(cls.as_any())? {
        return Ok(None);
    }
    let seen = PySet::empty(py)?;
    let mut values = Vec::with_capacity(N);
    if positional > 0 {
        let (names, match_self) = match cls.getattr_opt(interned(py, "__match_args__"))? {
            Some(names) => match names.cast_exact::<PyTuple>() {
                Ok(names) => (names.clone(), false),
                Err(_) => {
                    return Err(crate::raised(PyTypeError::new_err(format!(
                        "{}.__match_args__ must be a tuple (got {})",
                        type_name(&cls),
                        crate::type_name(&names)
                    ))));
                }
            },
            None => (PyTuple::empty(py), has_flag_of(&cls, TPFLAGS_MATCH_SELF)),
        };
        let allowed = if match_self { 1 } else { names.len() };
        if allowed < positional {
            let plural = if allowed == 1 { "" } else { "s" };
            return Err(crate::raised(PyTypeError::new_err(format!(
                "{}() accepts {allowed} positional sub-pattern{plural} ({positional} given)",
                type_name(&cls)
            ))));
        }
        if match_self {
            values.push(subject.clone());
        } else {
            for name in names.iter().take(positional) {
                let Ok(name) = name.cast_exact::<PyString>() else {
                    return Err(crate::raised(PyTypeError::new_err(format!(
                        "__match_args__ elements must be strings (got {})",
                        crate::type_name(&name)
                    ))));
                };
                match attribute(subject, &cls, name, &seen)? {
                    Some(value) => values.push(value),
                    None => return Ok(None),
                }
            }
        }
    }
    for name in keywords {
        match attribute(subject, &cls, &interned(py, name), &seen)? {
            Some(value) => values.push(value),
            None => return Ok(None),
        }
    }
    match values.try_into() {
        Ok(values) => Ok(Some(values)),
        Err(_) => unreachable!("one value per subpattern"),
    }
}

/// `cls` as a type, or the `TypeError` of a class pattern naming something else.
fn class<'py>(cls: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyType>> {
    match cls.cast::<PyType>() {
        Ok(cls) => Ok(cls.clone()),
        Err(_) => Err(crate::raised(PyTypeError::new_err(
            "called match pattern must be a class",
        ))),
    }
}

fn has_flag_of(cls: &Bound<'_, PyType>, flag: std::ffi::c_ulong) -> bool {
    // SAFETY: a valid type object.
    unsafe { ffi::PyType_GetFlags(cls.as_type_ptr()) & flag != 0 }
}

/// The `tp_name` of a class, which CPython's messages use (`Point`, `int`).
fn type_name(cls: &Bound<'_, PyType>) -> String {
    // SAFETY: a valid type object has a NUL-terminated `tp_name`.
    unsafe { std::ffi::CStr::from_ptr((*cls.as_type_ptr()).tp_name) }
        .to_string_lossy()
        .into_owned()
}

/// One attribute for a class pattern, `None` if it is missing. Naming an
/// attribute twice (positionally through `__match_args__` and as a keyword)
/// is a `TypeError`.
fn attribute<'py>(
    subject: &Bound<'py, PyAny>,
    cls: &Bound<'py, PyType>,
    name: &Bound<'py, PyString>,
    seen: &Bound<'py, PySet>,
) -> PyResult<Option<Bound<'py, PyAny>>> {
    if seen.contains(name)? {
        return Err(crate::raised(PyTypeError::new_err(format!(
            "{}() got multiple sub-patterns for attribute {}",
            type_name(cls),
            name.repr()?
        ))));
    }
    seen.add(name)?;
    subject.getattr_opt(name)
}
