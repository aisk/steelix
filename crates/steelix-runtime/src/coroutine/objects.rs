//! The Python objects: [`Coroutine`], [`Generator`] and
//! [`CoroutineWrapper`], and their creation.

use std::rc::Rc;
use std::sync::Mutex;

use pyo3::prelude::*;
use pyo3::types::PyString;
use pyo3::{PyTraverseError, PyVisit};

use super::awaits::being_awaited;
use super::finalize::{COROUTINE_DEALLOC, GENERATOR_DEALLOC, install_finalizer};
use super::state::{Body, Core, Flavor, Input, Names, State, Step, stop_iteration};
use super::{AsyncBody, Locals};
use crate::Module;

/// Creates the coroutine (or generator) object of a call of an `async def`
/// (or generator function): the body does not run yet. Called by
/// [`Function`](crate::Function) for the bodies that `rt::def!(m, async ..)`
/// and `rt::def!(m, gen ..)` store.
pub(crate) fn create<'py>(
    module: &Module<'py>,
    closure: &[Bound<'py, PyAny>],
    args: &[Bound<'py, PyAny>],
    body: AsyncBody,
    generator: bool,
    name: Py<PyString>,
    qualname: Py<PyString>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = module.py();
    // SAFETY: the token only lives in the future, which is only polled and
    // dropped with the thread attached (module documentation).
    let forever = unsafe { Python::assume_attached() };
    let rebind = |object: &Bound<'py, PyAny>| object.clone().unbind().into_bound(forever);
    let closure: Vec<_> = closure.iter().map(rebind).collect();
    let locals = Rc::new(Locals {
        module: Module::from_globals(module.globals().clone().unbind().into_bound(forever)),
        closure: (body.capture)(&closure)?,
        args: args.iter().map(rebind).collect(),
    });
    let body = Body::new((body.start)(Rc::clone(&locals)), locals);
    let flavor = if generator {
        Flavor::Generator
    } else {
        Flavor::Coroutine
    };
    let core = Core {
        flavor,
        state: Mutex::new(State::Created(body)),
        held: Mutex::new(Vec::new()),
        names: Mutex::new(Names { name, qualname }),
    };
    if generator {
        crate::plain_type_name::<Generator>(py, c"generator");
        let object = Bound::new(py, Generator(core))?;
        install_finalizer(&object, &GENERATOR_DEALLOC);
        Ok(object.into_any())
    } else {
        crate::plain_type_name::<Coroutine>(py, c"coroutine");
        let object = Bound::new(py, Coroutine(core))?;
        install_finalizer(&object, &COROUTINE_DEALLOC);
        Ok(object.into_any())
    }
}

/// The object of a call of a native `async def`.
///
/// It behaves like a CPython coroutine where callers can tell: `send`,
/// `throw`, `close`, `__await__`, the errors for running it re-entrantly, for
/// reusing it and for a `StopIteration` raised in its body (PEP 479),
/// `cr_running`, `cr_suspended`, `__name__`, `__qualname__`, `repr`, closing
/// on deallocation and the warning for a coroutine that was never awaited.
/// `asyncio` accepts it (`collections.abc.Coroutine`); it is not a
/// `types.CoroutineType` (design §12).
#[pyclass(frozen, weakref, name = "coroutine")]
pub struct Coroutine(pub(super) Core);

#[pymethods]
impl Coroutine {
    /// None, like CPython's coroutine objects, instead of the Rust
    /// documentation above.
    #[classattr]
    #[pyo3(name = "__doc__")]
    fn doc(py: Python<'_>) -> Py<PyAny> {
        py.None()
    }

    fn send<'py>(&self, value: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        self.0.send(value)
    }

    #[pyo3(signature = (typ, val = None, tb = None))]
    fn throw<'py>(
        &self,
        typ: &Bound<'py, PyAny>,
        val: Option<&Bound<'py, PyAny>>,
        tb: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        self.0.throw(typ, val, tb)
    }

    fn close<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        self.0.close(py)
    }

    /// Like `await` on a CPython coroutine (`_PyEval_GetAwaitable`): a
    /// coroutine that is suspended at an `await` already has someone
    /// awaiting it. (CPython checks this in `await`, not in `__await__`,
    /// which `await` calls for this type.)
    pub(super) fn __await__(slf: &Bound<'_, Self>) -> PyResult<CoroutineWrapper> {
        if slf.get().0.suspended() {
            return Err(being_awaited());
        }
        crate::plain_type_name::<CoroutineWrapper>(slf.py(), c"coroutine_wrapper");
        Ok(CoroutineWrapper {
            coroutine: slf.clone().unbind(),
        })
    }

    fn __repr__(slf: &Bound<'_, Self>) -> String {
        slf.get().0.repr(slf.as_any())
    }

    #[getter]
    fn __name__(&self, py: Python<'_>) -> Py<PyString> {
        self.0.names().name.clone_ref(py)
    }

    #[setter(__name__)]
    fn set_name(&self, value: &Bound<'_, PyAny>) -> PyResult<()> {
        self.0.set_name(value, "__name__")
    }

    #[getter]
    fn __qualname__(&self, py: Python<'_>) -> Py<PyString> {
        self.0.names().qualname.clone_ref(py)
    }

    #[setter(__qualname__)]
    fn set_qualname(&self, value: &Bound<'_, PyAny>) -> PyResult<()> {
        self.0.set_name(value, "__qualname__")
    }

    #[getter]
    fn cr_running(&self) -> bool {
        self.0.running()
    }

    #[getter]
    fn cr_suspended(&self) -> bool {
        self.0.suspended()
    }

    /// The body runs without a Python frame (design §12).
    #[getter]
    fn cr_frame(&self, py: Python<'_>) -> Py<PyAny> {
        py.None()
    }

    #[getter]
    fn cr_code(&self, py: Python<'_>) -> Py<PyAny> {
        py.None()
    }

    #[getter]
    fn cr_await(&self, py: Python<'_>) -> Py<PyAny> {
        py.None()
    }

    /// `sys.set_coroutine_origin_tracking_depth` is not supported.
    #[getter]
    fn cr_origin(&self, py: Python<'_>) -> Py<PyAny> {
        py.None()
    }

    fn __traverse__(&self, visit: PyVisit<'_>) -> Result<(), PyTraverseError> {
        self.0.traverse(visit)
    }

    fn __clear__(&self) {
        self.0.clear();
    }
}

/// The object of a call of a native generator function. See [`Coroutine`];
/// it is not a `types.GeneratorType` (design §12).
#[pyclass(frozen, weakref, name = "generator")]
pub struct Generator(pub(super) Core);

#[pymethods]
impl Generator {
    /// None, like CPython's generator objects, instead of the Rust
    /// documentation above.
    #[classattr]
    #[pyo3(name = "__doc__")]
    fn doc(py: Python<'_>) -> Py<PyAny> {
        py.None()
    }

    fn __iter__<'py>(slf: &Bound<'py, Self>) -> Bound<'py, Self> {
        slf.clone()
    }

    fn __next__<'py>(&self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyAny>>> {
        match self.0.resume(py, Input::Send(py.None()))? {
            Step::Yielded(value) => Ok(Some(value)),
            Step::Returned(value) if value.is_none() => Ok(None),
            Step::Returned(value) => Err(stop_iteration(value)),
        }
    }

    fn send<'py>(&self, value: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        self.0.send(value)
    }

    #[pyo3(signature = (typ, val = None, tb = None))]
    fn throw<'py>(
        &self,
        typ: &Bound<'py, PyAny>,
        val: Option<&Bound<'py, PyAny>>,
        tb: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        self.0.throw(typ, val, tb)
    }

    fn close<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        self.0.close(py)
    }

    fn __repr__(slf: &Bound<'_, Self>) -> String {
        slf.get().0.repr(slf.as_any())
    }

    #[getter]
    fn __name__(&self, py: Python<'_>) -> Py<PyString> {
        self.0.names().name.clone_ref(py)
    }

    #[setter(__name__)]
    fn set_name(&self, value: &Bound<'_, PyAny>) -> PyResult<()> {
        self.0.set_name(value, "__name__")
    }

    #[getter]
    fn __qualname__(&self, py: Python<'_>) -> Py<PyString> {
        self.0.names().qualname.clone_ref(py)
    }

    #[setter(__qualname__)]
    fn set_qualname(&self, value: &Bound<'_, PyAny>) -> PyResult<()> {
        self.0.set_name(value, "__qualname__")
    }

    #[getter]
    fn gi_running(&self) -> bool {
        self.0.running()
    }

    #[getter]
    fn gi_suspended(&self) -> bool {
        self.0.suspended()
    }

    #[getter]
    fn gi_frame(&self, py: Python<'_>) -> Py<PyAny> {
        py.None()
    }

    #[getter]
    fn gi_code(&self, py: Python<'_>) -> Py<PyAny> {
        py.None()
    }

    #[getter]
    fn gi_yieldfrom(&self, py: Python<'_>) -> Py<PyAny> {
        py.None()
    }

    fn __traverse__(&self, visit: PyVisit<'_>) -> Result<(), PyTraverseError> {
        self.0.traverse(visit)
    }

    fn __clear__(&self) {
        self.0.clear();
    }
}

/// What `coroutine.__await__()` returns, like CPython's `coroutine_wrapper`:
/// an iterator that drives the coroutine.
#[pyclass(frozen, name = "coroutine_wrapper")]
pub struct CoroutineWrapper {
    coroutine: Py<Coroutine>,
}

#[pymethods]
impl CoroutineWrapper {
    /// CPython's, instead of the Rust documentation above.
    #[classattr]
    #[pyo3(name = "__doc__")]
    fn doc() -> &'static str {
        "A wrapper object implementing __await__ for coroutines."
    }

    fn __iter__<'py>(slf: &Bound<'py, Self>) -> Bound<'py, Self> {
        slf.clone()
    }

    fn __next__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        self.coroutine.get().0.send(&py.None().into_bound(py))
    }

    fn send<'py>(&self, value: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        self.coroutine.get().0.send(value)
    }

    #[pyo3(signature = (typ, val = None, tb = None))]
    fn throw<'py>(
        &self,
        typ: &Bound<'py, PyAny>,
        val: Option<&Bound<'py, PyAny>>,
        tb: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        self.coroutine.get().0.throw(typ, val, tb)
    }

    fn close<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        self.coroutine.get().0.close(py)
    }

    fn __traverse__(&self, visit: PyVisit<'_>) -> Result<(), PyTraverseError> {
        visit.call(&self.coroutine)
    }
}
