use pyo3::exceptions::PyNameError;
use pyo3::exceptions::PyStopIteration;
use pyo3::exceptions::PyTypeError;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// builtin_shortcuts.py:6
#[rt::frame(m, "shortcuts", 6)]
pub fn shortcuts<'py>(m: &rt::Module<'py>, xs: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::builtins::print(
        py,
        (
            rt::builtins::list(py, ())?,
            rt::builtins::dict(py, ())?,
            rt::builtins::set(py, ())?,
            rt::builtins::tuple(py, ())?,
            rt::builtins::int(py, ())?,
            rt::builtins::float(py, ())?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            rt::builtins::sorted(py, (xs,))?,
            rt::builtins::max(py, (xs,))?,
            rt::builtins::min(py, (xs,))?,
            sum(m, xs)?,
            rt::builtins::list(py, (rt::builtins::reversed(py, (xs,))?,))?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            rt::builtins::list(py, (rt::builtins::enumerate(py, (xs,))?,))?,
            rt::builtins::list(py, (rt::builtins::zip(py, (xs, xs))?,))?,
            rt::builtins::list(py, (rt::builtins::map(py, (rt::builtin(py, "str")?, xs))?,))?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            rt::builtins::any(py, (xs,))?,
            rt::builtins::all(py, (xs,))?,
            rt::builtins::hash(py, (1,))?,
            rt::builtins::ord(py, ("a",))?,
            rt::builtins::chr(py, (98,))?,
            rt::builtins::divmod(py, (7, 2))?,
            rt::builtins::round(py, (2.5,))?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            rt::builtins::int(py, ("12",))?,
            rt::builtins::int(py, ("ff", 16))?,
            rt::builtins::float(py, ("1.5",))?,
            rt::builtins::max(py, (1, 2, 3))?,
            rt::builtins::next(py, (xs.try_iter()?, py.None()))?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            rt::builtins::hasattr(py, (xs, "append"))?,
            rt::builtins::getattr(py, (xs, "missing", "default"))?,
            rt::builtins::callable(py, (rt::builtin(py, "len")?,))?,
        ),
    )?;
    Ok(rt::none(py))
}

// builtin_shortcuts.py:15
#[rt::frame(m, "not_shortcuts", 15)]
pub fn not_shortcuts<'py>(
    m: &rt::Module<'py>,
    xs: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    // Keyword and star arguments keep the general call.
    rt::builtins::print(
        py,
        (
            rt::call!(rt::builtin(py, "sorted")?, xs, reverse = true)?,
            rt::call!(rt::builtin(py, "max")?, *xs)?,
            rt::call!(rt::builtin(py, "int")?, "7", base = 8)?,
            rt::call!(rt::builtin(py, "max")?, PyList::empty(py), default = 0)?,
        ),
    )?;
    Ok(rt::none(py))
}

// builtin_shortcuts.py:20
#[rt::frame(m, "errors", 20, frame)]
pub fn errors<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    match rt::builtins::int(py, ("x",)) {
        Ok(_) => {}
        Err(err) if err.is_instance_of::<PyValueError>(py) => {
            let e = frame.catch(err);
            rt::builtins::print(py, (e.get_type().attr("__name__")?, &e))?;
        }
        Err(err) => return Err(err),
    }
    match rt::builtins::max(py, ()) {
        Ok(_) => {}
        Err(err) if err.is_instance_of::<PyTypeError>(py) => {
            let e = frame.catch(err);
            rt::builtins::print(py, (e.get_type().attr("__name__")?, &e))?;
        }
        Err(err) => return Err(err),
    }
    match (|| rt::builtins::next(py, (PyList::empty(py).try_iter()?,)))() {
        Ok(_) => {}
        Err(err) if err.is_instance_of::<PyStopIteration>(py) => {
            let _caught = frame.catch(err);
            rt::builtins::print(py, ("stop",))?;
        }
        Err(err) => return Err(err),
    }
    Ok(rt::none(py))
}

// builtin_shortcuts.py:35
pub fn sum<'py>(m: &rt::Module<'py>, xs: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    Ok(rt::str(m.py(), "module level sum"))
}

/// Executes the body of `builtin_shortcuts.py` in the module `m`.
#[rt::frame(m, "<module>", 1, frame)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    m.set_global(
        "__doc__",
        "Builtins called through `rt::builtins`, and the calls that are not.",
    )?;
    m.set_global("builtins", rt::import(m, "builtins")?)?;
    rt::def!(m, shortcuts(xs))?;
    rt::def!(m, not_shortcuts(xs))?;
    rt::def!(m, errors())?;
    rt::def!(m, sum(xs))?;
    shortcuts(m, PyList::new(py, [3, 1, 2])?.as_any())?;
    not_shortcuts(m, PyList::new(py, [3, 1, 2])?.as_any())?;
    errors(m)?;
    m.set_global("original", m.global("builtins")?.attr("max")?)?;
    // builtin_shortcuts.py:44
    fn lambda<'py>(m: &rt::Module<'py>, args: &Bound<'py, PyTuple>) -> PyResult<Bound<'py, PyAny>> {
        Ok(rt::str(m.py(), "patched max"))
    }
    let value = rt::function!(m, lambda(*args), qualname = "<lambda>")?;
    m.global("builtins")?.set_attr("max", value)?;
    shortcuts(m, PyList::new(py, [3, 1, 2])?.as_any())?;
    let original = m.global("original")?;
    m.global("builtins")?.set_attr("max", original)?;
    m.global("builtins")?.del_attr("min")?;
    match (|| shortcuts(m, PyList::new(py, [3, 1, 2])?.as_any()))() {
        Ok(_) => {}
        Err(err) if err.is_instance_of::<PyNameError>(py) => {
            let exception = frame.catch(err);
            m.set_global("e", &exception)?;
            rt::builtins::print(
                py,
                (m.global("e")?.get_type().attr("__name__")?, m.global("e")?),
            )?;
            m.unbind_global("e")?;
        }
        Err(err) => return Err(err),
    }
    Ok(())
}
