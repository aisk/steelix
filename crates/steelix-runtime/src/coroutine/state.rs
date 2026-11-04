//! The state of a coroutine or generator object, shared by [`Coroutine`]
//! and [`Generator`], and how `send`, `throw` and `close` resume its body.
//!
//! [`Coroutine`]: super::Coroutine
//! [`Generator`]: super::Generator

use std::cell::Cell;
use std::rc::Rc;
use std::sync::{Mutex, MutexGuard};
use std::task::Poll;

use pyo3::exceptions::{
    PyDeprecationWarning, PyGeneratorExit, PyRuntimeError, PyStopIteration, PyTypeError,
    PyValueError,
};
use pyo3::prelude::*;
use pyo3::types::{PyString, PyTuple};
use pyo3::{PyTraverseError, PyVisit};

use super::channel::{Channel, handled_exception, set_handled_exception};
use super::{BodyFuture, Locals};

/// The body of a coroutine object: its future, and the locals the future
/// borrows. The future is dropped first.
pub(super) struct Body {
    pub(super) future: BodyFuture<'static>,
    locals: Rc<Locals<'static>>,
    /// Dropped last, see `Drop`.
    handled: KeepHandled,
}

impl Body {
    pub(super) fn new(future: BodyFuture<'static>, locals: Rc<Locals<'static>>) -> Self {
        Body {
            future,
            locals,
            handled: KeepHandled(None),
        }
    }
}

/// A body suspended in an `except` or `finally` block that is dropped (the
/// object is deallocated or cleared, or it ignored `close()`) drops the
/// block's [`Handling`](crate::Handling), which puts back the exception
/// handled when the block started. That is not the thread's state where
/// the body is dropped, so the exception handled there is kept across the
/// drop.
impl Drop for Body {
    fn drop(&mut self) {
        // Attached already (a body is only dropped attached, module
        // documentation), so this only gets the token.
        Python::attach(|py| self.handled.0 = handled_exception(py).map(Bound::unbind));
    }
}

/// The exception handled where a [`Body`] is dropped, made current again
/// once the future and the locals are gone.
struct KeepHandled(Option<Py<PyAny>>);

impl Drop for KeepHandled {
    fn drop(&mut self) {
        Python::attach(|py| set_handled_exception(py, self.0.as_ref().map(|e| e.bind(py))));
    }
}

// SAFETY: the future and the locals hold Python objects (which may be used
// from any thread that is attached) and Rust values such as `Cell`s, the
// traceback frame and the two `Rc`s of the locals (both in this `Body`, so
// they move together), which are only accessed from one thread at a time:
// the body is behind the object's `Mutex` and taken out of it while it is
// polled, and it is only polled or dropped with the thread attached (module
// documentation). A coroutine may be resumed from another thread, like in
// CPython, so this also relies on the body holding nothing else across a
// suspension that is bound to the thread it was created on (`Rc`, a
// `MutexGuard`, a thread-local handle). Generated code holds only Python
// objects and `Send` values; code rewritten by hand must keep that rule
// (design §15). It is not checked: the thread of a poll says nothing about
// what the body holds, and resuming on another thread is legitimate.
unsafe impl Send for Body {}

/// Coroutine or generator: the two objects share everything but the Python
/// interface and the wording of their errors.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Flavor {
    Coroutine,
    Generator,
}

impl Flavor {
    pub(super) fn noun(self) -> &'static str {
        match self {
            Flavor::Coroutine => "coroutine",
            Flavor::Generator => "generator",
        }
    }
}

pub(super) enum State {
    /// Created, not started.
    Created(Body),
    /// Suspended at an await. `handled` is the exception the body is
    /// handling there (it suspended inside an `except` or `finally`), which
    /// is the one `sys.exception()` returns when it resumes.
    Suspended {
        body: Body,
        handled: Option<Py<PyAny>>,
    },
    /// The body is being polled.
    Running,
    /// Returned, raised or closed.
    Finished,
}

pub(super) struct Names {
    pub(super) name: Py<PyString>,
    pub(super) qualname: Py<PyString>,
}

/// What resumes the body at its await: a value (`send`) or an exception
/// (`throw`, `close`).
pub(super) enum Input {
    Send(Py<PyAny>),
    /// The exception, and the arguments `throw()` was called with, which
    /// an object delegated to that is not a generator gets as they are.
    Throw(PyErr, Option<Py<PyTuple>>),
}

/// What happened when the body ran: it suspended with a value for the
/// caller, or returned.
pub(super) enum Step<'py> {
    Yielded(Bound<'py, PyAny>),
    Returned(Bound<'py, PyAny>),
}

/// The state shared by [`Coroutine`](super::Coroutine) and [`Generator`](super::Generator).
pub(super) struct Core {
    pub(super) flavor: Flavor,
    /// Locked only briefly, never while Python code runs (also the locks
    /// below).
    pub(super) state: Mutex<State>,
    /// What the futures of the body keep across its suspensions (see
    /// [`Held`](super::Held)), by key.
    pub(super) held: Mutex<Vec<(u64, Py<PyAny>)>>,
    pub(super) names: Mutex<Names>,
}

impl Core {
    pub(super) fn state(&self) -> MutexGuard<'_, State> {
        // A panic while holding the lock can only come from a bug here.
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub(super) fn held(&self) -> MutexGuard<'_, Vec<(u64, Py<PyAny>)>> {
        self.held
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub(super) fn names(&self) -> MutexGuard<'_, Names> {
        self.names
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn already_executing(&self) -> PyErr {
        crate::raised(PyValueError::new_err(format!(
            "{} already executing",
            self.flavor.noun()
        )))
    }

    /// Runs the body until it suspends or ends.
    pub(super) fn resume<'py>(&self, py: Python<'py>, input: Input) -> PyResult<Step<'py>> {
        let (mut body, mut handled, input) = {
            let mut state = self.state();
            match std::mem::replace(&mut *state, State::Running) {
                State::Created(body) => match input {
                    Input::Send(value) if !value.is_none(py) => {
                        *state = State::Created(body);
                        return Err(PyTypeError::new_err(format!(
                            "can't send non-None value to a just-started {}",
                            self.flavor.noun()
                        )));
                    }
                    // Nothing waits for the first `None`; an exception is
                    // raised by the frame before the first statement.
                    Input::Send(_) => (body, None, None),
                    Input::Throw(err, args) => (body, None, Some(Input::Throw(err, args))),
                },
                State::Suspended { body, handled } => (body, handled, Some(input)),
                State::Running => {
                    *state = State::Running;
                    return Err(self.already_executing());
                }
                State::Finished => {
                    *state = State::Finished;
                    drop(state);
                    return self.resume_finished(py, input);
                }
            }
        };
        // Coroutines resuming each other through Python nest their polls on
        // the C stack: a chain too deep raises `RecursionError` here, from
        // the C stack check (design §12), and the body ends like a CPython
        // frame that fails to start. Direct delegation nests polls without a
        // resume and checks in `delegate`.
        let _stack = match crate::RecursionGuard::enter(py) {
            Ok(guard) => guard,
            Err(err) => {
                self.finish(body);
                return Err(err);
            }
        };
        let finish = FinishOnUnwind(self);
        let channel = Channel {
            input: Cell::new(input),
            output: Cell::new(None),
            owner: self,
            own_handled: Cell::new(false),
        };
        let poll = channel.poll(py, &mut body, &mut handled);
        std::mem::forget(finish);
        match poll {
            Poll::Pending => match channel.output.take() {
                Some(value) => {
                    *self.state() = State::Suspended { body, handled };
                    Ok(Step::Yielded(value.into_bound(py)))
                }
                None => {
                    self.finish(body);
                    Err(PyRuntimeError::new_err(format!(
                        "{} '{}' awaited a Rust future that is not driven by Python",
                        self.flavor.noun(),
                        self.names().qualname
                    )))
                }
            },
            Poll::Ready(result) => {
                self.finish(body);
                match result {
                    Ok(value) => Ok(Step::Returned(value.unbind().into_bound(py))),
                    Err(err) => Err(stop_iteration_to_runtime_error(py, err, self.flavor)),
                }
            }
        }
    }

    /// Marks the body finished and drops it, then what its futures held
    /// (they release it themselves when they are dropped in a poll, not
    /// here). Not under the lock: dropping objects may run `__del__`.
    fn finish(&self, body: Body) {
        *self.state() = State::Finished;
        drop(body);
        self.drop_held();
    }

    fn drop_held(&self) {
        let held = std::mem::take(&mut *self.held());
        drop(held);
    }

    /// `tp_clear`, when the GC breaks a cycle through the object: drops the
    /// future, then the locals it borrowed and the rest of what it held.
    /// The GC runs the finalizer first, which closes a suspended body, so
    /// this only drops a body that was never started or ignored the close.
    pub(super) fn clear(&self) {
        let state = {
            let mut state = self.state();
            // A running body is on the stack of its caller, which keeps the
            // object alive: the GC never clears it.
            if matches!(*state, State::Running) {
                return;
            }
            std::mem::replace(&mut *state, State::Finished)
        };
        drop(state);
        self.drop_held();
    }

    /// `send` or `throw` after the body ended.
    fn resume_finished<'py>(&self, py: Python<'py>, input: Input) -> PyResult<Step<'py>> {
        match (self.flavor, input) {
            (Flavor::Coroutine, _) => Err(PyRuntimeError::new_err(
                "cannot reuse already awaited coroutine",
            )),
            (Flavor::Generator, Input::Send(_)) => Ok(Step::Returned(py.None().into_bound(py))),
            (Flavor::Generator, Input::Throw(err, _)) => Err(err),
        }
    }

    pub(super) fn send<'py>(&self, value: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        match self.resume(value.py(), Input::Send(value.clone().unbind()))? {
            Step::Yielded(value) => Ok(value),
            Step::Returned(value) => Err(stop_iteration(value)),
        }
    }

    pub(super) fn throw<'py>(
        &self,
        typ: &Bound<'py, PyAny>,
        val: Option<&Bound<'py, PyAny>>,
        tb: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = typ.py();
        if val.is_some() || tb.is_some() {
            PyErr::warn(
                py,
                &py.get_type::<PyDeprecationWarning>(),
                c"the (type, exc, tb) signature of throw() is deprecated, use the single-arg signature instead.",
                1,
            )?;
        }
        let exception = crate::generator::thrown(typ, val, tb)?;
        // `_gen_throw` passes on what it got, up to the first missing one.
        let args = match (val, tb) {
            (_, Some(tb)) => PyTuple::new(py, [typ, val.unwrap_or(&py.None().into_bound(py)), tb])?,
            (Some(val), None) => PyTuple::new(py, [typ, val])?,
            (None, None) => PyTuple::new(py, [typ])?,
        };
        let input = Input::Throw(PyErr::from_value(exception), Some(args.unbind()));
        match self.resume(py, input)? {
            Step::Yielded(value) => Ok(value),
            Step::Returned(value) => Err(stop_iteration(value)),
        }
    }

    /// Raises `GeneratorExit` where the body is suspended, so that its
    /// `finally` blocks and `with` exits run. Returns what the body returns
    /// if it catches the exception and returns (3.13).
    pub(super) fn close<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let created = {
            let mut state = self.state();
            match &*state {
                State::Running => return Err(self.already_executing()),
                State::Finished => return Ok(py.None().into_bound(py)),
                State::Created(_) => Some(std::mem::replace(&mut *state, State::Finished)),
                State::Suspended { .. } => None,
            }
        };
        if let Some(created) = created {
            // Dropped after the lock is released: the arguments may run
            // `__del__`.
            drop(created);
            return Ok(py.None().into_bound(py));
        }
        let exit = crate::raised(PyGeneratorExit::new_err(()));
        match self.resume(py, Input::Throw(exit, None)) {
            Ok(Step::Yielded(_)) => Err(PyRuntimeError::new_err(format!(
                "{} ignored GeneratorExit",
                self.flavor.noun()
            ))),
            Ok(Step::Returned(value)) => Ok(value),
            Err(err) if err.is_instance_of::<PyGeneratorExit>(py) => Ok(py.None().into_bound(py)),
            Err(err) => Err(err),
        }
    }

    pub(super) fn running(&self) -> bool {
        matches!(*self.state(), State::Running)
    }

    pub(super) fn suspended(&self) -> bool {
        matches!(*self.state(), State::Suspended { .. })
    }

    pub(super) fn repr(&self, object: &Bound<'_, PyAny>) -> String {
        // `str()` of a `str` subclass may run Python code: not under the lock.
        let qualname = self.names().qualname.clone_ref(object.py());
        format!(
            "<{} object {} at {:#x}>",
            self.flavor.noun(),
            qualname.bind(object.py()),
            object.as_ptr() as usize
        )
    }

    pub(super) fn set_name(&self, value: &Bound<'_, PyAny>, which: &str) -> PyResult<()> {
        let value = value
            .cast::<PyString>()
            .map_err(|_| PyTypeError::new_err(format!("{which} must be set to a string object")))?
            .clone()
            .unbind();
        // The old name is dropped after the lock is released.
        let old = {
            let mut names = self.names();
            let slot = if which == "__name__" {
                &mut names.name
            } else {
                &mut names.qualname
            };
            std::mem::replace(slot, value)
        };
        drop(old);
        Ok(())
    }

    pub(super) fn traverse(&self, visit: PyVisit<'_>) -> Result<(), PyTraverseError> {
        // The future itself is opaque: what the body's code holds across a
        // suspension is not visited (module documentation, design §12). A
        // running body is not visited either, its caller keeps the object
        // alive. The locks are never held across Python code.
        if let Ok(state) = self.state.try_lock() {
            match &*state {
                State::Created(body) => body.locals.traverse(&visit)?,
                State::Suspended { body, handled } => {
                    body.locals.traverse(&visit)?;
                    if let Some(handled) = handled {
                        visit.call(handled)?;
                    }
                }
                State::Running | State::Finished => {}
            }
        }
        if let Ok(held) = self.held.try_lock() {
            for (_, object) in held.iter() {
                visit.call(object)?;
            }
        }
        if let Ok(names) = self.names.try_lock() {
            visit.call(&names.name)?;
            visit.call(&names.qualname)?;
        }
        Ok(())
    }
}

/// Marks the body finished if polling it panics, instead of leaving it
/// `Running` (and "already executing") for ever.
struct FinishOnUnwind<'a>(&'a Core);

impl Drop for FinishOnUnwind<'_> {
    fn drop(&mut self) {
        *self.0.state() = State::Finished;
        self.0.drop_held();
    }
}

/// `StopIteration(value)`, the end of a `send` or `throw`. Like CPython, a
/// `None` result gives a `StopIteration` without arguments, and a tuple or an
/// exception is not taken apart.
pub(super) fn stop_iteration(value: Bound<'_, PyAny>) -> PyErr {
    if value.is_none() {
        PyStopIteration::new_err(())
    } else {
        PyStopIteration::new_err((value.unbind(),))
    }
}

fn stop_iteration_to_runtime_error(py: Python<'_>, err: PyErr, flavor: Flavor) -> PyErr {
    crate::generator::stop_iteration_to_runtime_error(py, err, flavor.noun())
}
