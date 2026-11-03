// DRAFT of a `#[pyclass]` for class `Size` (shapes.py:41),
// written by `steelix attrs app.py --pyclass shapes.Size`.
//
// This is advice for a human, not generated code: it is not part of the build,
// every body is `todo!()` and nothing checks it against the program. Design §15
// (类的改写, step 2) describes how to finish it, design §10 why steelix does
// not do it.
//
// The class is generated as `pub mod size` in src/shapes.rs; the struct and
// its `#[pymethods]` replace that mod, and in `exec_module` the
// `rt::class!(..)` becomes `m.set_global("Size", py.get_type::<Size>())`.
//
// What blocks or complicates the upgrade:
// - `@dataclass`: the generated `__init__`, `__repr__`, `__eq__` (and `__hash__`, ordering,
//   `frozen`) have to be written by hand, stubs below.
// - 2 dynamic accesses in the project with a receiver of unknown class may concern it
//   (app:show, shapes:Odd.__getattr__).
// - names of its attributes are also used through receivers of unknown class: `width` 1x in
//   app. See `steelix attrs --class shapes.Size`.
//
// The analysis only sees the project's modules: code outside the project, tests and `eval` may
// also read, write or subclass the class.

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTuple, PyType};

#[pyclass(module = "shapes", name = "Size")]
pub struct Size {
    /// `self.width`.
    // TODO: choose the Rust type (and `#[pyo3(get, set)]` if Python reads it).
    width: Py<PyAny>,
    /// `self.height`.
    // TODO: choose the Rust type (and `#[pyo3(get, set)]` if Python reads it).
    height: Py<PyAny>,
}

#[pymethods]
impl Size {
    /// The `__init__` that `@dataclass` generates, from the fields in order.
    // TODO: `height` defaults to `0` in Python.
    // Called in <module> (app.py:53).
    #[new]
    #[pyo3(signature = (width, height = None))]
    fn new<'py>(
        py: Python<'py>,
        width: &Bound<'py, PyAny>,
        height: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Self> {
        todo!()
    }

    /// `@dataclass` repr: `Size(width=.., height=..)`, each field with `repr()`.
    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        todo!()
    }

    /// `@dataclass` eq: compares the fields as a tuple, `NotImplemented` for
    /// other types. Without `frozen=True` (or `unsafe_hash`) instances are
    /// unhashable (`__hash__ = None`).
    fn __eq__<'py>(&self, py: Python<'py>, other: &Bound<'py, PyAny>) -> PyResult<Py<PyAny>> {
        todo!()
    }

    // pickle and `copy`: a `#[pyclass]` has no `__dict__` and does not pickle by
    // default. Add `__reduce__` returning the type and the arguments that rebuild
    // the instance (the type must stay reachable as `shapes.Size`).
    // fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyTuple>> {
    //     todo!()
    // }
}
