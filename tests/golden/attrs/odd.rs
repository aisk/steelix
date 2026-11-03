// DRAFT of a `#[pyclass]` for class `Odd` (shapes.py:54),
// written by `steelix attrs app.py --pyclass shapes.Odd`.
//
// This is advice for a human, not generated code: it is not part of the build,
// every body is `todo!()` and nothing checks it against the program. Design §15
// (类的改写, step 2) describes how to finish it, design §10 why steelix does
// not do it.
//
// The class is generated as `pub mod odd` in src/shapes.rs; the struct and
// its `#[pymethods]` replace that mod, and in `exec_module` the
// `rt::class!(..)` becomes `m.set_global("Odd", py.get_type::<Odd>())`.
//
// What blocks or complicates the upgrade:
// - base `Base` is a project class (shapes.py:50): it must become a `#[pyclass(subclass)]`
//   first, and this one `#[pyclass(extends = Base)]`.
// - metaclass (`metaclass=Meta`): `#[pyclass]` has no metaclass.
// - class body logic at line 57: `if LIMIT > 5:`.
// - defines `__getattr__`: attribute access is dynamic.
// - `__slots__` (a, _b): the fields replace them; check whether anything relies on the slot
//   descriptors.
// - uses the instance `__dict__`: `vars(self)` in Odd.cached at shapes.py:83, `self.__dict__`
//   in Odd.__getattr__ at shapes.py:89; a `#[pyclass]` has no `__dict__` unless
//   `#[pyclass(dict)]`.
// - 2 dynamic accesses in the project with a receiver of unknown class may concern it
//   (app:show, shapes:Odd.__getattr__).
// - class attribute `LIMIT` is reassigned (in Odd.build at shapes.py:78, in reset at
//   app.py:50): it can not be a `#[classattr]`, keep it in the type's `__dict__`.
// - its attributes are used outside its methods by 1 function in 1 module (app): all of them
//   have to be rewritten with it.
//
// The analysis only sees the project's modules: code outside the project, tests and `eval` may
// also read, write or subclass the class.

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTuple, PyType};

#[pyclass(module = "shapes", name = "Odd", extends = Base)]
pub struct Odd {
    /// `self.a`, assigned in Odd.__init__.
    // TODO: choose the Rust type (and `#[pyo3(get, set)]` if Python reads it).
    a: Py<PyAny>,
    /// `self._b`, assigned in Odd.__init__, Odd.b, read in Odd.b.
    // TODO: choose the Rust type (and `#[pyo3(get, set)]` if Python reads it).
    _b: Py<PyAny>,
}

#[pymethods]
impl Odd {
    /// `LIMIT = 10` (line 56).
    // Reassigned at run time: keep it in the type's `__dict__` instead.
    #[classattr]
    fn LIMIT(py: Python<'_>) -> Py<PyAny> {
        todo!()
    }

    /// `big = ..` (line 57).
    #[classattr]
    fn big(py: Python<'_>) -> Py<PyAny> {
        todo!()
    }

    /// `def __init__(self, a, /, b=1, *rest, key, flag=False, **extra)` (line 60), generated as
    /// `odd::__init__` in src/shapes.rs.
    // `#[new]` is `__new__`: it builds the struct from the arguments instead of
    // assigning attributes of an existing instance.
    // Called in Odd.make (shapes.py:74), Odd.build (shapes.py:79).
    // TODO: `b` defaults to `1` in Python.
    // TODO: `flag` defaults to `False` in Python.
    #[new]
    #[pyo3(signature = (a, /, b = None, *rest, key, flag = None, **extra))]
    fn new<'py>(
        py: Python<'py>,
        a: &Bound<'py, PyAny>,
        b: Option<&Bound<'py, PyAny>>,
        rest: &Bound<'py, PyTuple>,
        key: &Bound<'py, PyAny>,
        flag: Option<&Bound<'py, PyAny>>,
        extra: Option<&Bound<'py, PyDict>>,
    ) -> PyResult<Self> {
        todo!()
    }

    /// `@property def b(self)` (line 65), generated as `odd::b` in src/shapes.rs.
    #[getter]
    fn b<'py>(&self, py: Python<'py>) -> PyResult<Py<PyAny>> {
        todo!()
    }

    /// `@b.setter def b(self, value)` (line 69), generated as `odd::b_` in src/shapes.rs.
    #[setter]
    fn set_b<'py>(&mut self, py: Python<'py>, value: &Bound<'py, PyAny>) -> PyResult<()> {
        todo!()
    }

    /// `@staticmethod def make(x)` (line 73), generated as `odd::make` in src/shapes.rs.
    #[staticmethod]
    fn make<'py>(py: Python<'py>, x: &Bound<'py, PyAny>) -> PyResult<Py<PyAny>> {
        todo!()
    }

    /// `@classmethod def build(cls, *args)` (line 77), generated as `odd::build` in
    /// src/shapes.rs.
    #[classmethod]
    #[pyo3(signature = (*args))]
    fn build<'py>(
        cls: &Bound<'py, PyType>,
        py: Python<'py>,
        args: &Bound<'py, PyTuple>,
    ) -> PyResult<Py<PyAny>> {
        todo!()
    }

    /// `@functools.cache def cached(self)` (line 82), generated as `odd::cached` in
    /// src/shapes.rs.
    // TODO: decorator `@functools.cache` has no `#[pymethods]` equivalent.
    fn cached<'py>(&self, py: Python<'py>) -> PyResult<Py<PyAny>> {
        todo!()
    }

    /// `def __hash__(self)` (line 85), generated as `odd::__hash__` in src/shapes.rs.
    fn __hash__<'py>(&self, py: Python<'py>) -> PyResult<isize> {
        todo!()
    }

    /// `def __getattr__(self, name)` (line 88), generated as `odd::__getattr__` in
    /// src/shapes.rs.
    fn __getattr__<'py>(&self, py: Python<'py>, name: &Bound<'py, PyAny>) -> PyResult<Py<PyAny>> {
        todo!()
    }

    // pickle and `copy`: a `#[pyclass]` has no `__dict__` and does not pickle by
    // default. Add `__reduce__` returning the type and the arguments that rebuild
    // the instance (the type must stay reachable as `shapes.Odd`).
    // fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyTuple>> {
    //     todo!()
    // }
}
