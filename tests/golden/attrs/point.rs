// DRAFT of a `#[pyclass]` for class `Point` (shapes.py:7),
// written by `steelix attrs app.py --pyclass shapes.Point`.
//
// This is advice for a human, not generated code: it is not part of the build,
// every body is `todo!()` and nothing checks it against the program. Design §15
// (类的改写, step 2) describes how to finish it, design §10 why steelix does
// not do it.
//
// The class is generated as `pub mod point` in src/shapes.rs; the struct and
// its `#[pymethods]` replace that mod, and in `exec_module` the
// `rt::class!(..)` becomes `m.set_global("Point", py.get_type::<Point>())`.
//
// What blocks or complicates the upgrade:
// - subclassed in the project by shapes.Point3 (shapes.py:31): needs `#[pyclass(subclass)]`;
//   the subclasses' methods read and write the fields too.
// - dynamic attribute access on its instances: `setattr` in configure at app.py:45.
// - uses the instance `__dict__`: `vars(p)` in configure at app.py:43; a `#[pyclass]` has no
//   `__dict__` unless `#[pyclass(dict)]`.
// - 2 dynamic accesses in the project with a receiver of unknown class may concern it
//   (app:show, shapes:Odd.__getattr__).
// - class attribute `count` is reassigned (in Point.__init__ at shapes.py:15, in reset at
//   app.py:49): it can not be a `#[classattr]`, keep it in the type's `__dict__`.
// - instance attributes assigned outside the class's methods: `label`.
// - its attributes are used outside its methods by 5 functions in 2 modules (app, shapes): all
//   of them have to be rewritten with it.
// - names of its attributes are also used through receivers of unknown class: `x` 3x in app,
//   other; `y` 2x in app, other. See `steelix attrs --class shapes.Point`.
//
// The analysis only sees the project's modules: code outside the project, tests and `eval` may
// also read, write or subclass the class.

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTuple, PyType};

#[pyclass(module = "shapes", name = "Point", subclass)]
pub struct Point {
    /// `self.x`, assigned in Point.__init__, Point.move, scale, read in Point.norm,
    /// Point3.flat.
    // TODO: choose the Rust type (and `#[pyo3(get, set)]` if Python reads it).
    x: Py<PyAny>,
    /// `self.y`, assigned in Point.__init__, scale, read in Point.norm,
    /// Point.later.<locals>.inner, Point3.flat, scale, configure.
    // TODO: choose the Rust type (and `#[pyo3(get, set)]` if Python reads it).
    y: Py<PyAny>,
    /// `self._Point__secret`, assigned in Point.__init__, read in Point.later.
    // TODO: choose the Rust type (and `#[pyo3(get, set)]` if Python reads it).
    _Point__secret: Py<PyAny>,
    /// `self.label`, assigned in origin.
    // TODO: choose the Rust type (and `#[pyo3(get, set)]` if Python reads it).
    label: Py<PyAny>,
}

#[pymethods]
impl Point {
    /// `UNITS = ('m', 'km')` (line 8).
    #[classattr]
    fn UNITS(py: Python<'_>) -> Py<PyAny> {
        todo!()
    }

    /// `count = 0` (line 9).
    // Reassigned at run time: keep it in the type's `__dict__` instead.
    #[classattr]
    fn count(py: Python<'_>) -> Py<PyAny> {
        todo!()
    }

    /// `def __init__(self, x, y)` (line 11), generated as `point::__init__` in src/shapes.rs.
    // `#[new]` is `__new__`: it builds the struct from the arguments instead of
    // assigning attributes of an existing instance.
    // Called in Point3.flat (shapes.py:37), origin (app.py:15), maybe (app.py:21), closure
    // (app.py:28), <module> (app.py:54).
    #[new]
    fn new<'py>(py: Python<'py>, x: &Bound<'py, PyAny>, y: &Bound<'py, PyAny>) -> PyResult<Self> {
        todo!()
    }

    /// `def norm(self)` (line 17), generated as `point::norm` in src/shapes.rs.
    fn norm<'py>(&self, py: Python<'py>) -> PyResult<Py<PyAny>> {
        todo!()
    }

    /// `def move(self, dx)` (line 20), generated as `point::r#move` in src/shapes.rs.
    #[pyo3(name = "move")]
    fn r#move<'py>(&mut self, py: Python<'py>, dx: &Bound<'py, PyAny>) -> PyResult<Py<PyAny>> {
        todo!()
    }

    /// `def later(self)` (line 24), generated as `point::later` in src/shapes.rs.
    fn later<'py>(&self, py: Python<'py>) -> PyResult<Py<PyAny>> {
        todo!()
    }

    // pickle and `copy`: a `#[pyclass]` has no `__dict__` and does not pickle by
    // default. Add `__reduce__` returning the type and the arguments that rebuild
    // the instance (the type must stay reachable as `shapes.Point`).
    // fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyTuple>> {
    //     todo!()
    // }
}
