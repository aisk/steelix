//! `Function`: the Python-visible object for a transpiled `def`.
//!
//! `wrap_pyfunction!` produces a `builtin_function_or_method`, which can not
//! take attributes, can not have its `__name__` rewritten by
//! `functools.wraps`, and does not bind `self` when stored on a class. This
//! type behaves like a Python function instead: it has a `__dict__`, writable
//! metadata, and implements `__get__`.

use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTuple};
use pyo3::{PyTraverseError, PyVisit};

/// Signature shared by all function bodies stored in a [`Function`].
///
/// `args` has already been checked against the parameter list. The `def!`
/// macro generates a small adapter of this type around each plain Rust fn.
pub type Body = for<'py> fn(
    Python<'py>,
    &Bound<'py, PyDict>,
    &[Bound<'py, PyAny>],
) -> PyResult<Bound<'py, PyAny>>;

#[pyclass(frozen, dict, name = "function", module = "steelix_runtime")]
pub struct Function {
    params: &'static [&'static str],
    body: Body,
    globals: Py<PyDict>,
}

impl Function {
    /// Creates a function the way executing a `def` statement does and binds
    /// it to `name` in `globals`.
    ///
    /// `__name__`, `__qualname__`, `__doc__` and `__module__` live in the
    /// instance `__dict__`, so they are writable just like on a Python function.
    pub fn define<'py>(
        globals: &Bound<'py, PyDict>,
        name: &str,
        params: &'static [&'static str],
        doc: Option<&str>,
        body: Body,
    ) -> PyResult<()> {
        let py = globals.py();
        // Rust keywords are written as raw identifiers, which `stringify!` keeps.
        let name = name.trim_start_matches("r#");
        let function = Bound::new(
            py,
            Function {
                params,
                body,
                globals: globals.clone().unbind(),
            },
        )?;
        function.setattr("__name__", name)?;
        function.setattr("__qualname__", name)?;
        function.setattr("__doc__", doc)?;
        function.setattr("__module__", globals.get_item("__name__")?)?;
        globals.set_item(name, function)
    }
}

#[pymethods]
impl Function {
    #[pyo3(signature = (*args, **kwargs))]
    fn __call__<'py>(
        slf: &Bound<'py, Self>,
        args: &Bound<'py, PyTuple>,
        kwargs: Option<&Bound<'py, PyDict>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let this = slf.get();
        if kwargs.is_some_and(|kwargs| !kwargs.is_empty()) {
            return Err(PyTypeError::new_err(format!(
                "{}() does not accept keyword arguments yet (steelix limitation)",
                qualname(slf)?
            )));
        }
        let args: Vec<Bound<'py, PyAny>> = args.iter().collect();
        check_arity(slf, this.params, args.len())?;
        (this.body)(slf.py(), this.globals.bind(slf.py()), &args)
    }

    fn __get__<'py>(
        slf: &Bound<'py, Self>,
        instance: Option<&Bound<'py, PyAny>>,
        _owner: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        match instance {
            Some(instance) if !instance.is_none() => {
                let method_type = slf.py().import("types")?.getattr("MethodType")?;
                method_type.call1((slf, instance))
            }
            _ => Ok(slf.clone().into_any()),
        }
    }

    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        Ok(format!(
            "<function {} at {:#x}>",
            qualname(slf)?,
            slf.as_ptr() as usize
        ))
    }

    #[getter]
    fn __globals__<'py>(&self, py: Python<'py>) -> Bound<'py, PyDict> {
        self.globals.bind(py).clone()
    }

    fn __traverse__(&self, visit: PyVisit<'_>) -> Result<(), PyTraverseError> {
        visit.call(&self.globals)
    }
}

fn qualname(function: &Bound<'_, Function>) -> PyResult<String> {
    function.getattr("__qualname__")?.str()?.extract()
}

/// Raises the same `TypeError` as CPython when the argument count is wrong.
fn check_arity(function: &Bound<'_, Function>, params: &[&str], given: usize) -> PyResult<()> {
    let expected = params.len();
    if given > expected {
        let plural = if expected == 1 { "" } else { "s" };
        let verb = if given == 1 { "was" } else { "were" };
        return Err(PyTypeError::new_err(format!(
            "{}() takes {expected} positional argument{plural} but {given} {verb} given",
            qualname(function)?
        )));
    }
    if given < expected {
        let missing = &params[given..];
        let plural = if missing.len() == 1 { "" } else { "s" };
        return Err(PyTypeError::new_err(format!(
            "{}() missing {} required positional argument{plural}: {}",
            qualname(function)?,
            missing.len(),
            join_names(missing)
        )));
    }
    Ok(())
}

/// Formats names like CPython: `'a'`, `'a' and 'b'`, `'a', 'b', and 'c'`.
fn join_names(names: &[&str]) -> String {
    let quoted: Vec<String> = names
        .iter()
        .map(|name| format!("'{}'", name.trim_start_matches("r#")))
        .collect();
    match quoted.as_slice() {
        [] => String::new(),
        [only] => only.clone(),
        [first, second] => format!("{first} and {second}"),
        [rest @ .., last] => format!("{}, and {last}", rest.join(", ")),
    }
}
