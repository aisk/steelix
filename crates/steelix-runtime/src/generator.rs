//! `GenExpr`: the Python-visible object of a transpiled generator
//! expression (design §8).
//!
//! A generator expression is a restricted generator: one `for` clause (more
//! are kept as Python), `if` clauses and a single yielded value. CPython
//! evaluates the iterable of the `for` clause and calls `iter()` on it when
//! the expression is evaluated; everything else runs lazily, one item per
//! `next()`. The generated code mirrors that: the expression creates the
//! iterator and a `GenExpr` around it ([`genexpr!`](crate::genexpr)), and
//! the body is a plain Rust fn, the *step*, holding the `for` loop. Each
//! `next()` calls the step, which goes on iterating where the previous call
//! left off (the iterator keeps its position) and returns the next value, or
//! `None` once the iterator is exhausted:
//!
//! ```ignore
//! // (word.upper() for word in words if word)
//! #[rt::frame(m, "<genexpr>", 3)]
//! fn genexpr<'py>(
//!     m: &rt::Module<'py>,
//!     words: &Bound<'py, PyIterator>,
//! ) -> PyResult<Option<Bound<'py, PyAny>>> {
//!     for word in words {
//!         let word = word?;
//!         if word.is_truthy()? {
//!             return Ok(Some(word.call_method0("upper")?));
//!         }
//!         m.py().check_signals()?;
//!     }
//!     Ok(None)
//! }
//! let upper = rt::genexpr!(m, genexpr(words.try_iter()?))?;
//! ```
//!
//! Variables of the enclosing function are captured like in a nested
//! function (`genexpr[n, &count](..)`, see [`function!`](crate::function)),
//! so they are read when the step runs (late binding).

use std::sync::{Mutex, MutexGuard};

use pyo3::exceptions::{
    PyDeprecationWarning, PyRuntimeError, PyStopIteration, PyTypeError, PyValueError,
};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyIterator, PyString, PyTraceback, PyTuple, PyType};
use pyo3::{PyTraverseError, PyVisit, ffi};

use crate::Module;

/// Signature of the function that the [`genexpr!`](crate::genexpr) macro
/// generates around the step: the module, the closure cells and the
/// iterator. It returns the next value, or `None` when the iterator is
/// exhausted.
pub type Step = for<'py> fn(
    &Module<'py>,
    &[Bound<'py, PyAny>],
    &Bound<'py, PyIterator>,
) -> PyResult<Option<Bound<'py, PyAny>>>;

enum State {
    /// Created, `next()` not called yet.
    Created(Py<PyIterator>),
    /// Suspended after producing a value.
    Suspended(Py<PyIterator>),
    /// The step is running.
    Running,
    /// Exhausted, failed or closed: `next()` raises `StopIteration`.
    Closed,
}

struct Names {
    name: Py<PyString>,
    qualname: Py<PyString>,
}

/// The generator object of a generator expression.
///
/// It behaves like a CPython generator where callers can tell: iteration,
/// `send`, `throw`, `close`, `gi_running`, `gi_suspended`, the `ValueError`
/// when it runs re-entrantly, the `RuntimeError` replacing a `StopIteration`
/// raised in its body (PEP 479), `__name__`, `__qualname__` and `repr`. It
/// is not a `types.GeneratorType` (design §12).
#[pyclass(frozen, weakref, name = "generator")]
pub struct GenExpr {
    step: Step,
    globals: Py<PyDict>,
    closure: Option<Py<PyTuple>>,
    /// Locked only briefly, never while Python code runs: dropping the
    /// iterator may run arbitrary code, including this generator's methods.
    state: Mutex<State>,
    names: Mutex<Names>,
}

impl GenExpr {
    /// Creates the generator of a generator expression whose `for` clause
    /// iterates `iter`. Use it through [`genexpr!`](crate::genexpr).
    #[doc(hidden)]
    pub fn create<'py>(
        module: &Module<'py>,
        qualname: &'static str,
        iter: Bound<'py, PyIterator>,
        closure: Vec<Bound<'py, PyAny>>,
        step: Step,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = module.py();
        let name = qualname.rsplit('.').next().unwrap_or(qualname);
        let closure = (!closure.is_empty())
            .then(|| PyTuple::new(py, closure))
            .transpose()?;
        let generator = GenExpr {
            step,
            globals: module.globals().clone().unbind(),
            closure: closure.map(Bound::unbind),
            state: Mutex::new(State::Created(iter.unbind())),
            names: Mutex::new(Names {
                name: PyString::new(py, name).unbind(),
                qualname: PyString::new(py, qualname).unbind(),
            }),
        };
        crate::plain_type_name::<GenExpr>(py, c"generator");
        Ok(Bound::new(py, generator)?.into_any())
    }

    fn state(&self) -> MutexGuard<'_, State> {
        // A panic while holding the lock can only come from a bug here.
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn names(&self) -> MutexGuard<'_, Names> {
        self.names
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Takes the iterator out of the state to run the step, which then is
    /// `Running`. `Ok(None)` if the generator is closed.
    fn start(&self) -> PyResult<Option<Py<PyIterator>>> {
        let mut state = self.state();
        match std::mem::replace(&mut *state, State::Running) {
            State::Created(iter) | State::Suspended(iter) => Ok(Some(iter)),
            State::Running => Err(already_executing()),
            State::Closed => {
                *state = State::Closed;
                Ok(None)
            }
        }
    }

    /// Ends the generator. The iterator, if any, is dropped after the lock
    /// is released.
    fn finish(&self) -> Option<Py<PyIterator>> {
        match std::mem::replace(&mut *self.state(), State::Closed) {
            State::Created(iter) | State::Suspended(iter) => Some(iter),
            State::Running | State::Closed => None,
        }
    }

    /// Runs the step for the next value: `None` once the generator is
    /// exhausted (or closed).
    fn resume<'py>(&self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyAny>>> {
        let Some(iter) = self.start()? else {
            return Ok(None);
        };
        let iter = iter.into_bound(py);
        let module = Module::from_globals(self.globals.bind(py).clone());
        let closure = match &self.closure {
            Some(closure) => closure.bind(py).iter().collect(),
            None => Vec::new(),
        };
        let result = (self.step)(&module, &closure, &iter);
        if let Ok(Some(_)) = result {
            *self.state() = State::Suspended(iter.unbind());
        } else {
            *self.state() = State::Closed;
            drop(iter);
        }
        result.map_err(|err| stop_iteration_to_runtime_error(py, err, "generator"))
    }
}

#[pymethods]
impl GenExpr {
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

    fn __next__<'py>(slf: &Bound<'py, Self>) -> PyResult<Option<Bound<'py, PyAny>>> {
        slf.get().resume(slf.py())
    }

    /// `send(None)` is `next()`. The value is the result of the `yield`
    /// expression, which a generator expression ignores, but like CPython a
    /// generator that has not started can only be sent `None`.
    fn send<'py>(slf: &Bound<'py, Self>, value: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        let this = slf.get();
        if !value.is_none() && matches!(*this.state(), State::Created(_)) {
            return Err(PyTypeError::new_err(
                "can't send non-None value to a just-started generator",
            ));
        }
        match this.resume(slf.py())? {
            Some(value) => Ok(value),
            None => Err(PyStopIteration::new_err(())),
        }
    }

    /// Raises the exception where the generator is suspended. A generator
    /// expression has no handlers, so it always comes out again and the
    /// generator is closed.
    #[pyo3(signature = (typ, val = None, tb = None))]
    fn throw<'py>(
        slf: &Bound<'py, Self>,
        typ: &Bound<'py, PyAny>,
        val: Option<&Bound<'py, PyAny>>,
        tb: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = slf.py();
        if val.is_some() || tb.is_some() {
            PyErr::warn(
                py,
                &py.get_type::<PyDeprecationWarning>(),
                c"the (type, exc, tb) signature of throw() is deprecated, use the single-arg signature instead.",
                1,
            )?;
        }
        let exception = thrown(typ, val, tb)?;
        let this = slf.get();
        match *this.state() {
            State::Running => return Err(already_executing()),
            State::Closed => return Err(PyErr::from_value(exception)),
            State::Created(_) | State::Suspended(_) => {}
        }
        drop(this.finish());
        Err(stop_iteration_to_runtime_error(
            py,
            PyErr::from_value(exception),
            "generator",
        ))
    }

    /// Closes the generator, releasing its iterator. A generator expression
    /// can not catch the `GeneratorExit` this raises in CPython, so nothing
    /// else happens.
    fn close(&self) -> PyResult<()> {
        if matches!(*self.state(), State::Running) {
            return Err(already_executing());
        }
        drop(self.finish());
        Ok(())
    }

    fn __repr__(slf: &Bound<'_, Self>) -> String {
        // `str()` of a `str` subclass may run Python code: not under the lock.
        let qualname = slf.get().names().qualname.clone_ref(slf.py());
        let qualname = qualname.bind(slf.py());
        format!(
            "<generator object {qualname} at {:#x}>",
            slf.as_ptr() as usize
        )
    }

    #[getter]
    fn __name__(&self, py: Python<'_>) -> Py<PyString> {
        self.names().name.clone_ref(py)
    }

    #[setter(__name__)]
    fn set_name(&self, value: &Bound<'_, PyAny>) -> PyResult<()> {
        let value = value
            .cast::<PyString>()
            .map_err(|_| PyTypeError::new_err("__name__ must be set to a string object"))?;
        // The old name is dropped after the lock is released: a `__del__`
        // may read the names again.
        let old = std::mem::replace(&mut self.names().name, value.clone().unbind());
        drop(old);
        Ok(())
    }

    #[getter]
    fn __qualname__(&self, py: Python<'_>) -> Py<PyString> {
        self.names().qualname.clone_ref(py)
    }

    #[setter(__qualname__)]
    fn set_qualname(&self, value: &Bound<'_, PyAny>) -> PyResult<()> {
        let value = value
            .cast::<PyString>()
            .map_err(|_| PyTypeError::new_err("__qualname__ must be set to a string object"))?;
        let old = std::mem::replace(&mut self.names().qualname, value.clone().unbind());
        drop(old);
        Ok(())
    }

    #[getter]
    fn gi_running(&self) -> bool {
        matches!(*self.state(), State::Running)
    }

    #[getter]
    fn gi_suspended(&self) -> bool {
        matches!(*self.state(), State::Suspended(_))
    }

    /// The step runs without a Python frame (design §12).
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
        visit.call(&self.globals)?;
        visit.call(&self.closure)?;
        // The locks are never held across Python code, so they are free
        // unless another thread is using the generator right now.
        if let Ok(state) = self.state.try_lock()
            && let State::Created(iter) | State::Suspended(iter) = &*state
        {
            visit.call(iter)?;
        }
        if let Ok(names) = self.names.try_lock() {
            visit.call(&names.name)?;
            visit.call(&names.qualname)?;
        }
        Ok(())
    }

    fn __clear__(&self) {
        // Called by the garbage collector, which does not run while the
        // step is running on this thread; the iterator may run code when it
        // is dropped, so not under the lock.
        if let Ok(mut state) = self.state.try_lock()
            && let State::Created(_) | State::Suspended(_) = &*state
        {
            let old = std::mem::replace(&mut *state, State::Closed);
            drop(state);
            drop(old);
        }
    }
}

fn already_executing() -> PyErr {
    crate::raised(PyValueError::new_err("generator already executing"))
}

/// PEP 479: a `StopIteration` leaving the body of a generator would look
/// like its end to the caller, so CPython replaces it by a `RuntimeError`
/// whose cause it is.
/// Also for native coroutines and generator functions, whose noun the
/// message uses.
pub(crate) fn stop_iteration_to_runtime_error(py: Python<'_>, err: PyErr, noun: &str) -> PyErr {
    if !err.is_instance_of::<PyStopIteration>(py) {
        return err;
    }
    let cause = err.into_value(py);
    let error = PyRuntimeError::new_err(format!("{noun} raised StopIteration"));
    let value = error.value(py);
    // SAFETY: exception instances. `PyException_SetContext` steals a
    // reference; `PyException_SetCause` steals one and sets
    // `__suppress_context__`, like `_PyErr_FormatFromCause`.
    unsafe {
        ffi::PyException_SetContext(value.as_ptr(), cause.clone_ref(py).into_ptr());
        ffi::PyException_SetCause(value.as_ptr(), cause.into_ptr());
    }
    error
}

/// The exception `throw(typ, val, tb)` raises, checked and normalized like
/// CPython's `_gen_throw`.
pub(crate) fn thrown<'py>(
    typ: &Bound<'py, PyAny>,
    val: Option<&Bound<'py, PyAny>>,
    tb: Option<&Bound<'py, PyAny>>,
) -> PyResult<Bound<'py, PyAny>> {
    let val = val.filter(|value| !value.is_none());
    let tb = tb.filter(|tb| !tb.is_none());
    if let Some(tb) = tb
        && !tb.is_instance_of::<PyTraceback>()
    {
        return Err(PyTypeError::new_err(
            "throw() third argument must be a traceback object",
        ));
    }
    let exception = if let Ok(class) = typ.cast::<PyType>()
        && class.is_subclass_of::<pyo3::exceptions::PyBaseException>()?
    {
        match val {
            Some(value) if value.is_instance(class)? => value.clone(),
            Some(value) => match value.cast::<PyTuple>() {
                Ok(args) => class.call1(args)?,
                Err(_) => class.call1((value,))?,
            },
            None => class.call0()?,
        }
    } else if typ.is_instance_of::<pyo3::exceptions::PyBaseException>() {
        if val.is_some() {
            return Err(PyTypeError::new_err(
                "instance exception may not have a separate value",
            ));
        }
        typ.clone()
    } else {
        return Err(PyTypeError::new_err(format!(
            "exceptions must be classes or instances deriving from BaseException, not {}",
            typ.get_type().name()?
        )));
    };
    if !exception.is_instance_of::<pyo3::exceptions::PyBaseException>() {
        return Err(PyTypeError::new_err(format!(
            "calling {} should have returned an instance of BaseException, not {}",
            typ.repr()?,
            exception.get_type().repr()?
        )));
    }
    if let Some(tb) = tb {
        exception.setattr("__traceback__", tb)?;
    }
    Ok(exception)
}

/// Evaluates a generator expression: creates a [`GenExpr`] whose `for`
/// clause iterates the given iterator, with the step `f`.
///
/// The step takes the module, the captured variables (like
/// [`function!`](crate::function): `n` by value, `&count` as a cell) and the
/// iterator, and returns `PyResult<Option<Bound<PyAny>>>`. The iterator
/// expression is evaluated right away, which is when CPython evaluates the
/// iterable of the first `for` clause and calls `iter()` on it.
///
/// ```ignore
/// rt::genexpr!(m, genexpr(xs.try_iter()?))?;
/// rt::genexpr!(m, genexpr[n, &count](xs.try_iter()?), qualname = "f.<locals>.<genexpr>")?;
/// ```
///
/// `qualname` defaults to `<genexpr>`.
#[macro_export]
macro_rules! genexpr {
    (@caps $h:tt [$($c:tt)*] & $id:ident $(as $n:literal)? $(, $($rest:tt)*)?) => {
        $crate::genexpr!(@caps $h [$($c)* {cell $id, $crate::function!(@name $id $($n)?)}] $($($rest)*)?)
    };
    (@caps $h:tt [$($c:tt)*] $id:ident $(as $n:literal)? $(, $($rest:tt)*)?) => {
        $crate::genexpr!(@caps $h [$($c)* {value $id, $crate::function!(@name $id $($n)?)}] $($($rest)*)?)
    };
    (@caps ($module:expr, $f:ident, $iter:expr, $qualname:expr) [$({$ck:ident $c:ident, $cn:expr})*]) => {{
        let definer = $module;
        let module: &$crate::Module<'_> = $crate::function::Definer::module(definer);
        let iter = $iter;
        let closure = ::std::vec![$($crate::function!(@capture $ck $c),)*];
        let step: $crate::generator::Step = |module, closure, iter| {
            // Named before the captured variables, which may shadow it.
            let target = $f;
            let [$($c),*] = closure else {
                unreachable!("the closure has one cell per captured variable")
            };
            $($crate::function!(@captured $ck $c, $cn);)*
            $crate::genexpr!(@call target(module; $($c),*; iter))
        };
        $crate::GenExpr::create(module, $qualname, iter, closure, step)
    }};
    (@call $f:ident($module:ident; $($c:ident),*; $iter:ident)) => {
        $f($module, $(&$c,)* $iter)
    };
    (@qualname) => { "<genexpr>" };
    (@qualname $q:literal) => { $q };
    ($module:expr, $f:ident $([$($caps:tt)*])? ($iter:expr) $(, qualname = $q:literal)? $(,)?) => {
        $crate::genexpr!(@caps ($module, $f, $iter, $crate::genexpr!(@qualname $($q)?)) [] $($($caps)*)?)
    };
}

/// The next item of `iter`, or `None` once it is exhausted: `next(iter)`
/// without the `StopIteration`.
pub fn next<'py>(iter: &Bound<'py, PyIterator>) -> PyResult<Option<Bound<'py, PyAny>>> {
    iter.clone().next().transpose()
}
