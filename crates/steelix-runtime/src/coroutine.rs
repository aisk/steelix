//! Native coroutines (`async def`) and generator functions (design §8,
//! "协程和生成器"). PoC: the transpiler does not emit these yet.
//!
//! The body of an `async def` (or of a function containing `yield`) is a
//! Rust `async fn`, and the state machine is the one rustc generates for it.
//! There is no executor: nothing here waits for anything. The future is
//! driven from Python only, by `send`, `throw` and `close` on the
//! [`Coroutine`] (or [`Generator`]) object, each of which polls it once.
//!
//! The awaits in the body are [`await_`], [`yield_`] and [`yield_from`]. When
//! one of them has a value for whoever drives the coroutine (the future that
//! `asyncio.sleep` yields, a generator's next item) it leaves it in the
//! [`Channel`] of the current poll and returns `Poll::Pending`, which goes up
//! through every `.await` of the body to the `send` that polled it. `send`
//! returns that value to Python. The next `send(value)` (or `throw(exc)`)
//! puts the value (or exception) in the channel and polls again: the same
//! await is polled, takes it, and the body continues. `Poll::Pending` thus
//! never means "not ready yet, wake me": the waker does nothing.
//!
//! ```ignore
//! // async def fetch(session, url):
//! //     resp = await session.get(url)
//! //     return await resp.text()
//! pub async fn fetch<'py>(
//!     m: &rt::Module<'py>,
//!     session: &Bound<'py, PyAny>,
//!     url: &Bound<'py, PyAny>,
//! ) -> PyResult<Bound<'py, PyAny>> {
//!     rt::frame!(m, "fetch", 1, async |_| {
//!         let resp = rt::await_(session.method1("get", (url,))?).await?;
//!         rt::await_(resp.method0("text")?).await
//!     })
//! }
//! rt::def!(m, async fetch(session, url))?;
//! ```
//!
//! The parameters are references like in every generated function. The
//! coroutine object owns the module, the captured variables and the
//! arguments of the call ([`Locals`]), and the future that `rt::def!` stores
//! awaits the `async fn` with references to them. A native coroutine
//! awaiting another one it can call directly awaits the Rust future itself,
//! without a Python object in between: `add(m, &x, &y).await?`. A generator
//! function is the same with `rt::yield_` (`rt::def!(m, gen count(n))`).
//!
//! # What the GC sees
//!
//! The future is opaque to the GC, so what it holds itself is invisible:
//! a reference that the GC does not see keeps its object alive whatever
//! else refers to it, and a cycle through it is never collected. So the
//! objects a suspended body keeps for the runtime are stored in the
//! coroutine object and visited by its `__traverse__`: the module's globals,
//! the captured variables and the arguments ([`Locals`], which the future
//! only borrows), the exception the body is handling, the object an
//! [`await_`] or [`yield_from`] delegates to and the frame objects of the
//! body's traceback frames (`Held`). What the body's own code holds across
//! a suspension (local variables, temporaries, the exception of an `except`
//! block, the context manager of a `with` block) stays in the future and is
//! not seen (design §12). `tp_clear` drops the future first, then the rest.
//!
//! # Lifetimes and safety
//!
//! The future outlives the call that created it: it is stored in the Python
//! object. Generated code is generic over `'py` like all PyO3 code, so the
//! body is instantiated with `'py = 'static` (see [`AsyncBody`] and
//! `create`), the only place that makes a `Python<'static>`. That token
//! claims that the thread is attached for ever, which is true wherever the
//! future runs: it is only polled inside `send`, `throw` and `close` (and the
//! finalizer), which Python calls with the thread attached, and only dropped
//! there or when the object is deallocated or cleared by the GC, also with
//! the thread attached. The body can not keep the token or its objects
//! beyond the future, because it is generic over `'py`, and objects only
//! leave the future as `Py<PyAny>` (yielded values) or are rebound to the
//! caller's token (the result). Free-threaded builds are rejected at compile
//! time (`lib.rs`), so "attached" also means "holding the GIL".
//!
//! The object, and with it the future, may be resumed or dropped on another
//! thread than the one that created or last polled it, as CPython allows
//! (asyncio's `run_coroutine_threadsafe`, a generator shared by threads).
//! The future is not `Send` (it holds `Bound`s), and is declared `Send` here
//! on the premise that everything it holds across a suspension is a Python
//! object or `Send`, which is true of generated code. Code rewritten by hand
//! (design §15) must keep it: no `Rc`, `MutexGuard` or other thread-bound
//! value may be held across an `.await`. There is no cheap check for it (see
//! `Body`).

use std::cell::Cell;
use std::ffi::c_char;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, Once, OnceLock};
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

use pyo3::exceptions::{
    PyDeprecationWarning, PyGeneratorExit, PyRuntimeError, PyRuntimeWarning, PyStopIteration,
    PyTypeError, PyValueError,
};
use pyo3::prelude::*;
use pyo3::types::PyString;
use pyo3::{PyClass, PyTraverseError, PyVisit, ffi};

use crate::Module;

/// What [`rt::def!`](crate::def) stores for an `async def` or a generator
/// function. Both functions are generic over `'py`, which is what keeps the
/// body from relying on the `'static` lifetime it is actually run with.
#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct AsyncBody {
    /// Reads the captured variables out of the closure cells when the
    /// function is called: a variable captured by value is read then, one
    /// captured as a cell stays the cell (like [`Body`](crate::Body) does).
    pub capture: for<'py> fn(&[Bound<'py, PyAny>]) -> PyResult<Vec<Captured<'py>>>,
    /// Creates the future of the body, which borrows the module, the
    /// captured variables and the arguments from the [`Locals`] it keeps.
    pub start: for<'py> fn(Rc<Locals<'py>>) -> BodyFuture<'py>,
}

/// The future of a coroutine or generator body, returning the value of its
/// `return` statement.
#[doc(hidden)]
pub type BodyFuture<'py> = Pin<Box<dyn Future<Output = PyResult<Bound<'py, PyAny>>> + 'py>>;

/// The module, the captured variables and the arguments of one call of a
/// native coroutine or generator function. The coroutine object keeps them,
/// where the GC sees them; the future holds another `Rc` and only borrows
/// them, so it adds no reference the GC can not see.
#[doc(hidden)]
pub struct Locals<'py> {
    pub module: Module<'py>,
    pub closure: Vec<Captured<'py>>,
    pub args: Vec<Bound<'py, PyAny>>,
}

/// A captured variable of a coroutine or generator: its value, or its cell
/// if the variable is reassigned after the capture (`rt::function!`'s
/// `[n, &count]`).
#[doc(hidden)]
pub enum Captured<'py> {
    Value(Bound<'py, PyAny>),
    Cell(crate::Cell<'py>),
}

impl<'py> Captured<'py> {
    pub fn value(&self) -> &Bound<'py, PyAny> {
        match self {
            Captured::Value(value) => value,
            Captured::Cell(_) => unreachable!("`rt::function!` captured this variable as a cell"),
        }
    }

    pub fn cell(&self) -> &crate::Cell<'py> {
        match self {
            Captured::Cell(cell) => cell,
            Captured::Value(_) => unreachable!("`rt::function!` captured this variable by value"),
        }
    }

    fn object(&self) -> &Bound<'py, PyAny> {
        match self {
            Captured::Value(value) => value,
            Captured::Cell(cell) => cell.as_object(),
        }
    }
}

impl Locals<'_> {
    fn traverse(&self, visit: &PyVisit<'_>) -> Result<(), PyTraverseError> {
        visit.call(self.module.globals().as_unbound())?;
        for captured in &self.closure {
            visit.call(captured.object().as_unbound())?;
        }
        for arg in &self.args {
            visit.call(arg.as_unbound())?;
        }
        Ok(())
    }
}

/// The body of a coroutine object: its future, and the locals the future
/// borrows. The future is dropped first.
struct Body {
    future: BodyFuture<'static>,
    locals: Rc<Locals<'static>>,
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
enum Flavor {
    Coroutine,
    Generator,
}

impl Flavor {
    fn noun(self) -> &'static str {
        match self {
            Flavor::Coroutine => "coroutine",
            Flavor::Generator => "generator",
        }
    }
}

enum State {
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

struct Names {
    name: Py<PyString>,
    qualname: Py<PyString>,
}

/// What resumes the body at its await: a value (`send`) or an exception
/// (`throw`, `close`).
enum Input {
    Send(Py<PyAny>),
    Throw(PyErr),
}

/// What happened when the body ran: it suspended with a value for the
/// caller, or returned.
enum Step<'py> {
    Yielded(Bound<'py, PyAny>),
    Returned(Bound<'py, PyAny>),
}

/// The state shared by [`Coroutine`] and [`Generator`].
struct Core {
    flavor: Flavor,
    /// Locked only briefly, never while Python code runs (also the locks
    /// below).
    state: Mutex<State>,
    /// What the futures of the body keep across its suspensions (see
    /// [`Held`]), by key.
    held: Mutex<Vec<(u64, Py<PyAny>)>>,
    names: Mutex<Names>,
}

impl Core {
    fn state(&self) -> MutexGuard<'_, State> {
        // A panic while holding the lock can only come from a bug here.
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn held(&self) -> MutexGuard<'_, Vec<(u64, Py<PyAny>)>> {
        self.held
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn names(&self) -> MutexGuard<'_, Names> {
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
    fn resume<'py>(&self, py: Python<'py>, input: Input) -> PyResult<Step<'py>> {
        // Coroutines resuming each other through Python nest their polls on
        // the C stack. The depth in Python frames is counted by the frames
        // of the body (`rt::frame!`, see `Depth`).
        let _stack = crate::RecursionGuard::enter(py)?;
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
                    Input::Throw(err) => (body, None, Some(Input::Throw(err))),
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
        let finish = FinishOnUnwind(self);
        let channel = Channel {
            input: Cell::new(input),
            output: Cell::new(None),
            owner: self,
            caller: Cell::new(std::ptr::null_mut()),
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
    fn clear(&self) {
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
            (Flavor::Generator, Input::Throw(err)) => Err(err),
        }
    }

    fn send<'py>(&self, value: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        match self.resume(value.py(), Input::Send(value.clone().unbind()))? {
            Step::Yielded(value) => Ok(value),
            Step::Returned(value) => Err(stop_iteration(value)),
        }
    }

    fn throw<'py>(
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
        match self.resume(py, Input::Throw(PyErr::from_value(exception)))? {
            Step::Yielded(value) => Ok(value),
            Step::Returned(value) => Err(stop_iteration(value)),
        }
    }

    /// Raises `GeneratorExit` where the body is suspended, so that its
    /// `finally` blocks and `with` exits run. Returns what the body returns
    /// if it catches the exception and returns (3.13).
    fn close<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
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
        match self.resume(py, Input::Throw(exit)) {
            Ok(Step::Yielded(_)) => Err(PyRuntimeError::new_err(format!(
                "{} ignored GeneratorExit",
                self.flavor.noun()
            ))),
            Ok(Step::Returned(value)) => Ok(value),
            Err(err) if err.is_instance_of::<PyGeneratorExit>(py) => Ok(py.None().into_bound(py)),
            Err(err) => Err(err),
        }
    }

    /// `tp_finalize`, like CPython's `_PyGen_Finalize`: a coroutine that was
    /// never started warns, a suspended coroutine or generator is closed.
    /// Errors are reported as unraisable, with CPython's messages.
    fn finalize(&self, object: &Bound<'_, PyAny>) {
        let py = object.py();
        let created = match &*self.state() {
            State::Created(_) => true,
            State::Suspended { .. } => false,
            State::Running | State::Finished => return,
        };
        if created && self.flavor == Flavor::Coroutine {
            if let Err(err) = warn_never_awaited(object) {
                write_unraisable(
                    py,
                    err,
                    c"Exception ignored while finalizing coroutine %R",
                    object,
                );
            }
        } else if let Err(err) = self.close(py) {
            let message = match self.flavor {
                Flavor::Coroutine => c"Exception ignored while closing coroutine %R",
                Flavor::Generator => c"Exception ignored while closing generator %R",
            };
            write_unraisable(py, err, message, object);
        }
    }

    fn running(&self) -> bool {
        matches!(*self.state(), State::Running)
    }

    fn suspended(&self) -> bool {
        matches!(*self.state(), State::Suspended { .. })
    }

    fn repr(&self, object: &Bound<'_, PyAny>) -> String {
        // `str()` of a `str` subclass may run Python code: not under the lock.
        let qualname = self.names().qualname.clone_ref(object.py());
        format!(
            "<{} object {} at {:#x}>",
            self.flavor.noun(),
            qualname.bind(object.py()),
            object.as_ptr() as usize
        )
    }

    fn set_name(&self, value: &Bound<'_, PyAny>, which: &str) -> PyResult<()> {
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

    fn traverse(&self, visit: PyVisit<'_>) -> Result<(), PyTraverseError> {
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
fn stop_iteration(value: Bound<'_, PyAny>) -> PyErr {
    if value.is_none() {
        PyStopIteration::new_err(())
    } else {
        PyStopIteration::new_err((value.unbind(),))
    }
}

fn stop_iteration_to_runtime_error(py: Python<'_>, err: PyErr, flavor: Flavor) -> PyErr {
    crate::generator::stop_iteration_to_runtime_error(py, err, flavor.noun())
}

/// The `RuntimeWarning` of a coroutine that is deallocated without ever
/// running, through `warnings._warn_unawaited_coroutine` like CPython.
fn warn_never_awaited(coroutine: &Bound<'_, PyAny>) -> PyResult<()> {
    let py = coroutine.py();
    let warnings = py.import("warnings")?;
    match warnings.getattr_opt("_warn_unawaited_coroutine")? {
        Some(warn) => warn.call1((coroutine,)).map(drop),
        None => {
            let qualname = coroutine.getattr("__qualname__")?;
            let message =
                std::ffi::CString::new(format!("coroutine '{qualname}' was never awaited"))?;
            PyErr::warn(py, &py.get_type::<PyRuntimeWarning>(), &message, 1)
        }
    }
}

unsafe extern "C" {
    /// `PyErr_FormatUnraisable` (3.13), not in `pyo3-ffi`.
    fn PyErr_FormatUnraisable(format: *const c_char, ...);
}

/// Reports `err` like CPython reports an exception in a finalizer: through
/// `sys.unraisablehook`, with `message` formatted with the object.
fn write_unraisable(
    py: Python<'_>,
    err: PyErr,
    message: &std::ffi::CStr,
    object: &Bound<'_, PyAny>,
) {
    err.restore(py);
    // SAFETY: an exception is set; the format has one `%R`, for a valid
    // object pointer.
    unsafe { PyErr_FormatUnraisable(message.as_ptr(), object.as_ptr()) };
}

// --- the channel between `send` and the awaits of the body ---

/// Carries values between the `send` (or `throw`, `close`) that polls the
/// body and the await where it is suspended, for one poll. It lives on the
/// stack of the poll; the awaits find it through the waker.
struct Channel {
    /// The value or exception for the await that suspended last time.
    input: Cell<Option<Input>>,
    /// The value of the await that suspends now, for the caller.
    output: Cell<Option<Py<PyAny>>>,
    /// The coroutine being polled, whose `resume` is on the stack below the
    /// poll (see [`with_polled`]).
    owner: *const Core,
    /// The exception the caller is handling during this poll (only
    /// compared, the poll keeps it alive), or null.
    caller: Cell<*mut ffi::PyObject>,
    /// Whether the body has an exception of its own to handle in this poll
    /// (it suspended in an `except` or `finally` block).
    own_handled: Cell<bool>,
}

thread_local! {
    /// The channel of the poll running on this thread, if any: polls of
    /// different coroutines nest when one awaits another through Python.
    static CURRENT: Cell<*const Channel> = const { Cell::new(std::ptr::null()) };
}

/// The waker passed to the body: it does nothing, nothing waits to be woken.
/// Its data is the address of the channel.
static WAKER: RawWakerVTable =
    RawWakerVTable::new(|data| RawWaker::new(data, &WAKER), |_| {}, |_| {}, |_| {});

impl Channel {
    /// Polls `body` once with this channel.
    ///
    /// `handled` is the body's exception state, like a CPython generator's
    /// `gi_exc_state`: while the body runs, `sys.exception()` is the
    /// exception it is handling, or else the caller's. When it suspends
    /// inside an `except` or `finally`, that exception is kept for the next
    /// poll and the caller's is put back.
    fn poll(
        &self,
        py: Python<'_>,
        body: &mut Body,
        handled: &mut Option<Py<PyAny>>,
    ) -> Poll<PyResult<Bound<'static, PyAny>>> {
        let caller = handled_exception(py);
        self.caller
            .set(caller.as_ref().map_or(std::ptr::null_mut(), Bound::as_ptr));
        if let Some(own) = handled.take() {
            set_handled_exception(py, Some(own.bind(py)));
            self.own_handled.set(true);
        }
        let restore = Restore {
            py,
            caller: caller.clone(),
            previous: CURRENT.replace(self),
        };
        // SAFETY: the vtable's functions never use the data pointer, so the
        // waker is valid on any thread and for any time.
        let waker = unsafe { Waker::from_raw(RawWaker::new((self as *const Self).cast(), &WAKER)) };
        let poll = body.future.as_mut().poll(&mut Context::from_waker(&waker));
        if poll.is_pending() {
            let now = handled_exception(py);
            *handled = match now {
                Some(now) if !caller.as_ref().is_some_and(|caller| caller.is(&now)) => {
                    Some(now.unbind())
                }
                _ => None,
            };
        }
        drop(restore);
        poll
    }

    /// An exception thrown in (`throw`, `close`) on its way into the body,
    /// where the body suspended: like CPython's `_PyErr_ChainStackItem`, it
    /// gets the exception the body is handling there as `__context__`
    /// (unless it is that exception, breaking cycles like `raise` does).
    /// Nothing changes when the body handles none.
    fn thrown(&self, py: Python<'_>, err: PyErr) -> PyErr {
        if !self.own_handled.get() {
            return err;
        }
        let exception = err.into_value(py).into_bound(py);
        // SAFETY: attached; `exception` is an exception instance. The poll
        // made the body's exception the one being handled, so
        // `PyErr_SetObject` chains it as `raise` would; the raised exception
        // is taken back right away.
        unsafe {
            ffi::PyErr_SetObject(ffi::Py_TYPE(exception.as_ptr()).cast(), exception.as_ptr());
            PyErr::from_value(Bound::from_owned_ptr(py, ffi::PyErr_GetRaisedException()))
        }
    }

    /// Takes the exception thrown in before the body started, if any.
    fn take_thrown(&self) -> Option<PyErr> {
        match self.input.take() {
            Some(Input::Throw(err)) => Some(err),
            other => {
                self.input.set(other);
                None
            }
        }
    }
}

/// For [`Handling`](crate::Handling) in the body of a coroutine or
/// generator: the coroutine being polled on this thread and the exception
/// its caller is handling (null if none), as addresses.
pub(crate) fn polling() -> Option<(*const (), *mut ffi::PyObject)> {
    let channel = CURRENT.get();
    // SAFETY: a non-null `CURRENT` is the live channel of the running poll.
    unsafe { channel.as_ref() }.map(|channel| (channel.owner.cast(), channel.caller.get()))
}

/// Runs `f` with the coroutine `owner` if it is the one being polled on this
/// thread right now, and only then: a future of its body that is dropped
/// anywhere else (the object being deallocated or cleared, maybe in the poll
/// of another coroutine) must not touch it.
fn with_polled<R>(owner: *const Core, f: impl FnOnce(&Core) -> R) -> Option<R> {
    let channel = CURRENT.get();
    // SAFETY: a non-null `CURRENT` is the live channel of the running poll.
    let channel = unsafe { channel.as_ref() }?;
    if !std::ptr::eq(channel.owner, owner) {
        return None;
    }
    // SAFETY: the owner of the running poll's channel is the `Core` whose
    // `resume` (borrowing it) is on the stack below this poll.
    Some(f(unsafe { &*owner }))
}

/// An object that a future in the body of a coroutine keeps across
/// suspensions: the iterator an [`await_`] delegates to, the frame object
/// of a traceback frame. It is stored in the coroutine object, whose
/// `__traverse__` visits it, so that it adds no reference the GC can not
/// see; the future keeps this handle.
///
/// The object is only reachable in a poll of that coroutine: elsewhere
/// [`Held::get`] returns `None`. Dropped in such a poll, the handle removes
/// the object; a body dropped anywhere else is dropped as a whole, and the
/// coroutine drops all it held then.
pub(crate) struct Held {
    owner: *const Core,
    key: u64,
}

impl Held {
    fn new(core: &Core, object: Bound<'_, PyAny>) -> Self {
        static NEXT_KEY: AtomicU64 = AtomicU64::new(0);
        let key = NEXT_KEY.fetch_add(1, Ordering::Relaxed);
        core.held().push((key, object.unbind()));
        Held { owner: core, key }
    }

    /// Keeps `object` in the coroutine being polled on this thread. `None`
    /// outside a poll.
    pub(crate) fn in_current(object: &Bound<'_, PyAny>) -> Option<Self> {
        let channel = CURRENT.get();
        // SAFETY: a non-null `CURRENT` is the live channel of the running poll.
        let owner = unsafe { channel.as_ref() }?.owner;
        with_polled(owner, |core| Held::new(core, object.clone()))
    }

    /// The object, in a poll of its coroutine; `None` elsewhere, or if the
    /// coroutine was cleared.
    pub(crate) fn get<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyAny>> {
        with_polled(self.owner, |core| {
            let held = core.held();
            let (_, object) = held.iter().find(|(key, _)| *key == self.key)?;
            Some(object.clone_ref(py).into_bound(py))
        })
        .flatten()
    }

    /// Removes the object from its coroutine, if this is a poll of it.
    fn release(&self) -> Option<Py<PyAny>> {
        with_polled(self.owner, |core| {
            let mut held = core.held();
            let index = held.iter().position(|(key, _)| *key == self.key)?;
            Some(held.swap_remove(index).1)
        })
        .flatten()
    }
}

impl Drop for Held {
    fn drop(&mut self) {
        // Dropped after the lock is released: it may run `__del__`.
        let object = self.release();
        drop(object);
    }
}

/// One level of Python recursion for a poll of a frame of a native
/// coroutine or generator ([`frame!`](crate::frame) with an `async`
/// closure), like CPython counts the frame of a generator each time it is
/// resumed: a chain of generators delegating to each other (`yield from`,
/// `await`, through Python objects or direct awaits) raises `RecursionError:
/// maximum recursion depth exceeded` at `sys.getrecursionlimit()` frames.
/// Also checks the C stack, like [`RecursionGuard`](crate::RecursionGuard).
pub(crate) struct Depth<'py> {
    _stack: crate::RecursionGuard<'py>,
    #[cfg(python_recursion)]
    counted: *mut ThreadState,
}

impl<'py> Depth<'py> {
    pub(crate) fn enter(py: Python<'py>) -> PyResult<Self> {
        let stack = crate::RecursionGuard::enter(py)?;
        #[cfg(python_recursion)]
        {
            let counted = thread_state();
            if !counted.is_null() {
                // SAFETY: the current thread state, attached; the fields are
                // CPython 3.14's (checked by `thread_state`). This is
                // `_Py_EnterRecursivePy` and `_Py_CheckRecursiveCallPy`; with
                // the headroom for handling a recursion error (50 frames)
                // used up CPython aborts, this raises again.
                unsafe {
                    let remaining = (*counted).py_recursion_remaining;
                    if remaining <= 0 && ((*counted).recursion_headroom == 0 || remaining <= -50) {
                        return Err(crate::raised(pyo3::exceptions::PyRecursionError::new_err(
                            "maximum recursion depth exceeded",
                        )));
                    }
                    (*counted).py_recursion_remaining = remaining - 1;
                }
            }
            Ok(Self {
                _stack: stack,
                counted,
            })
        }
        #[cfg(not(python_recursion))]
        Ok(Self { _stack: stack })
    }
}

#[cfg(python_recursion)]
impl Drop for Depth<'_> {
    fn drop(&mut self) {
        if !self.counted.is_null() {
            // SAFETY: the thread state counted in `enter`, on this thread
            // (the guard lives in one poll).
            unsafe { (*self.counted).py_recursion_remaining += 1 };
        }
    }
}

/// The start of CPython 3.14's `PyThreadState` (`Include/cpython/pystate.h`,
/// public but not part of the limited API), up to the counters of Python
/// recursion, which no function exposes: `Py_EnterRecursiveCall` checks the
/// C stack since 3.14.
#[cfg(python_recursion)]
#[repr(C)]
struct ThreadState {
    prev: *mut std::ffi::c_void,
    next: *mut std::ffi::c_void,
    interp: *mut std::ffi::c_void,
    eval_breaker: usize,
    status: std::ffi::c_uint,
    holds_gil: std::ffi::c_int,
    whence: std::ffi::c_int,
    state: std::ffi::c_int,
    py_recursion_remaining: std::ffi::c_int,
    py_recursion_limit: std::ffi::c_int,
    recursion_headroom: std::ffi::c_int,
}

/// The current thread state as [`ThreadState`], or null if its layout does
/// not look like 3.14's (then only the C stack is checked).
#[cfg(python_recursion)]
fn thread_state() -> *mut ThreadState {
    static LAYOUT: OnceLock<bool> = OnceLock::new();
    // SAFETY: attached (the caller holds a `Python`), so there is a current
    // thread state.
    let state = unsafe { ffi::PyThreadState_Get() }.cast::<ThreadState>();
    // The layout is checked once against what the public API says: the
    // thread's limit is `sys.getrecursionlimit()`, and the depth is within
    // it.
    let matches = *LAYOUT.get_or_init(|| {
        // SAFETY: as above; reading plain `int` fields of the thread state.
        unsafe {
            let limit = (*state).py_recursion_limit;
            let remaining = (*state).py_recursion_remaining;
            limit == ffi::Py_GetRecursionLimit() && (-50..=limit).contains(&remaining)
        }
    });
    if matches { state } else { std::ptr::null_mut() }
}

/// Puts back the thread's state after a poll, also when it panics.
struct Restore<'py> {
    py: Python<'py>,
    caller: Option<Bound<'py, PyAny>>,
    previous: *const Channel,
}

impl Drop for Restore<'_> {
    fn drop(&mut self) {
        CURRENT.set(self.previous);
        set_handled_exception(self.py, self.caller.as_ref());
    }
}

/// The channel of the poll that is polling `cx` right now. `None` when the
/// future is polled by anything else (a Rust executor).
fn channel<'a>(cx: &Context<'a>) -> Option<&'a Channel> {
    let waker = cx.waker();
    let data = waker.data().cast::<Channel>();
    if !std::ptr::eq(waker.vtable(), &WAKER) || data.is_null() || data != CURRENT.get() {
        return None;
    }
    // SAFETY: `CURRENT` is the channel of the poll running on this thread,
    // which lives on the stack of `Channel::poll` until the poll returns.
    Some(unsafe { &*data })
}

fn outside_coroutine() -> PyErr {
    PyRuntimeError::new_err(
        "rt::await_ or rt::yield_ polled outside a coroutine or generator driven by Python",
    )
}

fn handled_exception(py: Python<'_>) -> Option<Bound<'_, PyAny>> {
    // SAFETY: attached; returns a new reference or NULL.
    unsafe { Bound::from_owned_ptr_or_opt(py, ffi::PyErr_GetHandledException()) }
}

fn set_handled_exception(_py: Python<'_>, exception: Option<&Bound<'_, PyAny>>) {
    let exception = exception.map_or(std::ptr::null_mut(), Bound::as_ptr);
    // SAFETY: attached; the function takes its own reference, NULL clears.
    unsafe { ffi::PyErr_SetHandledException(exception) };
}

/// For the traceback frame of a body (`rt::frame!` with an `async`
/// closure): the exception thrown into a coroutine or generator that has
/// not started, which CPython raises before the first statement.
pub(crate) fn thrown_at_start() -> impl Future<Output = Option<PyErr>> {
    std::future::poll_fn(|cx| Poll::Ready(channel(cx).and_then(Channel::take_thrown)))
}

// --- awaits ---

/// `await awaitable`: waits for `awaitable`, delegating to it like CPython
/// does (PEP 492, with the `send`/`throw`/`close` rules of PEP 380), and
/// returns its result.
///
/// ```ignore
/// let resp = rt::await_(session.method1("get", (&url,))?).await?;
/// rt::await_(&task).await?;
/// ```
pub fn await_<'py>(awaitable: impl AsRef<Bound<'py, PyAny>>) -> Await<'py> {
    let awaitable = awaitable.as_ref();
    Await {
        py: awaitable.py(),
        state: AwaitState::Start(awaitable.clone(), Delegate::Await),
    }
}

/// `yield from iterable` in a generator: yields what `iter(iterable)`
/// yields, forwarding `send`, `throw` and `close` to it (PEP 380), and
/// returns its return value.
///
/// ```ignore
/// let total = rt::yield_from(&parts).await?;
/// ```
pub fn yield_from<'py>(iterable: impl AsRef<Bound<'py, PyAny>>) -> Await<'py> {
    let iterable = iterable.as_ref();
    Await {
        py: iterable.py(),
        state: AwaitState::Start(iterable.clone(), Delegate::YieldFrom),
    }
}

/// The future of [`await_`] and [`yield_from`].
pub struct Await<'py> {
    py: Python<'py>,
    state: AwaitState<'py>,
}

enum AwaitState<'py> {
    Start(Bound<'py, PyAny>, Delegate),
    /// Suspended in the iterator, which yielded a value for the caller. The
    /// coroutine keeps the iterator, where the GC sees it.
    Delegating(Held),
    Done,
}

#[derive(Clone, Copy)]
enum Delegate {
    Await,
    YieldFrom,
}

impl<'py> Future for Await<'py> {
    type Output = PyResult<Bound<'py, PyAny>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let Some(channel) = channel(cx) else {
            return Poll::Ready(Err(outside_coroutine()));
        };
        let (iter, step) = match std::mem::replace(&mut self.state, AwaitState::Done) {
            AwaitState::Start(value, delegate) => {
                let iter = match delegate {
                    Delegate::Await => awaitable_iter(&value),
                    Delegate::YieldFrom => yield_from_iter(&value),
                };
                let iter = match iter {
                    Ok(iter) => iter,
                    Err(err) => return Poll::Ready(Err(err)),
                };
                let step = send(&iter, &iter.py().None().into_bound(iter.py()));
                (iter, step)
            }
            AwaitState::Delegating(held) => {
                let py = self.py;
                let Some(iter) = held.get(py) else {
                    return Poll::Ready(Err(PyRuntimeError::new_err(
                        "the coroutine was cleared while it was suspended",
                    )));
                };
                drop(held);
                let step = match channel.input.take() {
                    Some(Input::Send(value)) => send(&iter, value.bind(py)),
                    // Whatever comes out of the iterator then enters the
                    // body as a thrown exception (`_gen_throw`).
                    Some(Input::Throw(err)) => {
                        throw(&iter, err).map_err(|err| channel.thrown(py, err))
                    }
                    None => send(&iter, &py.None().into_bound(py)),
                };
                (iter, step)
            }
            AwaitState::Done => panic!("`rt::await_` polled after it completed"),
        };
        match step {
            Ok(Step::Yielded(value)) => {
                channel.output.set(Some(value.unbind()));
                let held = Held::in_current(&iter).expect("`channel` is the running poll's");
                self.state = AwaitState::Delegating(held);
                Poll::Pending
            }
            Ok(Step::Returned(value)) => Poll::Ready(Ok(value)),
            Err(err) => Poll::Ready(Err(err)),
        }
    }
}

/// The iterator `await value` delegates to, like CPython's
/// `_PyEval_GetAwaitable`: a native coroutine (or a generator decorated
/// with `types.coroutine`) itself, otherwise the result of `__await__`.
///
/// A CPython coroutine suspended at an `await` is being awaited by someone
/// else already, which is an error (so is a native one: its `__await__`
/// says so).
fn awaitable_iter<'py>(value: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    if is_coroutine(value)? {
        // SAFETY: valid object pointer.
        if unsafe { ffi::PyCoro_CheckExact(value.as_ptr()) } != 0
            && !value
                .getattr(crate::interned(value.py(), "cr_await"))?
                .is_none()
        {
            return Err(being_awaited());
        }
        return Ok(value.clone());
    }
    let Some(await_) = crate::frame::lookup_special(value, "__await__")? else {
        return Err(crate::raised(PyTypeError::new_err(format!(
            "'{}' object can't be awaited",
            crate::type_name(value)
        ))));
    };
    let iter = await_.call0()?;
    if is_coroutine(&iter)? {
        return Err(crate::raised(PyTypeError::new_err(
            "__await__() returned a coroutine",
        )));
    }
    // SAFETY: valid object pointer.
    if unsafe { ffi::PyIter_Check(iter.as_ptr()) } == 0 {
        return Err(crate::raised(PyTypeError::new_err(format!(
            "__await__() returned non-iterator of type '{}'",
            crate::type_name(&iter)
        ))));
    }
    Ok(iter)
}

fn being_awaited() -> PyErr {
    crate::raised(PyRuntimeError::new_err(
        "coroutine is being awaited already",
    ))
}

/// The iterator `yield from value` delegates to (`GET_YIELD_FROM_ITER`). A
/// generator suspended elsewhere is resumed from here, like in CPython, and
/// a running one raises `generator already executing` when it is sent to.
fn yield_from_iter<'py>(value: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    // SAFETY: valid object pointer.
    let native = unsafe { ffi::PyCoro_CheckExact(value.as_ptr()) } != 0;
    if native || value.is_instance_of::<Coroutine>() {
        return Err(crate::raised(PyTypeError::new_err(
            "cannot 'yield from' a coroutine object in a non-coroutine generator",
        )));
    }
    // SAFETY: valid object pointer.
    if unsafe { ffi::PyGen_CheckExact(value.as_ptr()) } != 0 {
        return Ok(value.clone());
    }
    Ok(value.try_iter()?.into_any())
}

/// A CPython coroutine, or a generator whose code has
/// `CO_ITERABLE_COROUTINE` (`types.coroutine`).
fn is_coroutine(value: &Bound<'_, PyAny>) -> PyResult<bool> {
    // SAFETY: valid object pointers.
    unsafe {
        if ffi::PyCoro_CheckExact(value.as_ptr()) != 0 {
            return Ok(true);
        }
        if ffi::PyGen_CheckExact(value.as_ptr()) == 0 {
            return Ok(false);
        }
    }
    let flags: u32 = value.getattr("gi_code")?.getattr("co_flags")?.extract()?;
    Ok(flags & 0x100 != 0)
}

/// `PyIter_Send`: one step of the iterator.
fn send<'py>(iter: &Bound<'py, PyAny>, value: &Bound<'py, PyAny>) -> PyResult<Step<'py>> {
    let py = iter.py();
    let mut result = std::ptr::null_mut();
    // SAFETY: valid object pointers; on success `result` is a new reference.
    let status = unsafe { ffi::PyIter_Send(iter.as_ptr(), value.as_ptr(), &mut result) };
    match status {
        ffi::PySendResult::PYGEN_ERROR => Err(PyErr::fetch(py)),
        // SAFETY: a new reference, see above.
        ffi::PySendResult::PYGEN_NEXT => {
            Ok(Step::Yielded(unsafe { Bound::from_owned_ptr(py, result) }))
        }
        ffi::PySendResult::PYGEN_RETURN => {
            Ok(Step::Returned(unsafe { Bound::from_owned_ptr(py, result) }))
        }
    }
}

/// Throws `err` into the iterator the body is delegating to, like CPython's
/// `_gen_throw` with a delegate: `GeneratorExit` closes the iterator and is
/// raised here, anything else goes to its `throw` method if it has one.
fn throw<'py>(iter: &Bound<'py, PyAny>, err: PyErr) -> PyResult<Step<'py>> {
    let py = iter.py();
    if err.is_instance_of::<PyGeneratorExit>(py) {
        close_iter(iter)?;
        return Err(err);
    }
    let Some(throw) = iter.getattr_opt(crate::interned(py, "throw"))? else {
        return Err(err);
    };
    match throw.call1((err.into_value(py),)) {
        Ok(value) => Ok(Step::Yielded(value)),
        Err(err) if err.is_instance_of::<PyStopIteration>(py) => {
            let value = err.value(py).getattr(crate::interned(py, "value"))?;
            Ok(Step::Returned(value))
        }
        Err(err) => Err(err),
    }
}

/// Closes the iterator a closed coroutine or generator was delegating to
/// (`gen_close_iter`).
fn close_iter(iter: &Bound<'_, PyAny>) -> PyResult<()> {
    let py = iter.py();
    let close = match iter.getattr_opt(crate::interned(py, "close")) {
        Ok(close) => close,
        Err(err) => {
            err.write_unraisable(py, Some(iter));
            None
        }
    };
    if let Some(close) = close {
        close.call0()?;
    }
    Ok(())
}

/// `yield value` in a generator: hands `value` to the caller and returns
/// what it sends next (`None` for `next()`), or raises what it throws in.
///
/// ```ignore
/// let x = rt::yield_(&total).await?;
/// ```
pub fn yield_<'py>(value: impl AsRef<Bound<'py, PyAny>>) -> Yield<'py> {
    let value = value.as_ref().clone();
    Yield {
        py: value.py(),
        value: Some(value),
    }
}

/// The future of [`yield_`].
pub struct Yield<'py> {
    py: Python<'py>,
    value: Option<Bound<'py, PyAny>>,
}

impl<'py> Future for Yield<'py> {
    type Output = PyResult<Bound<'py, PyAny>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let Some(channel) = channel(cx) else {
            return Poll::Ready(Err(outside_coroutine()));
        };
        let py = self.py;
        if let Some(value) = self.value.take() {
            channel.output.set(Some(value.unbind()));
            return Poll::Pending;
        }
        Poll::Ready(match channel.input.take() {
            Some(Input::Send(value)) => Ok(value.into_bound(py)),
            Some(Input::Throw(err)) => Err(channel.thrown(py, err)),
            None => Ok(py.None().into_bound(py)),
        })
    }
}

// --- the Python objects ---

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
    let body = Body {
        future: (body.start)(Rc::clone(&locals)),
        locals,
    };
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
        let object = Bound::new(py, Generator(core))?;
        install_finalizer(&object, &GENERATOR_DEALLOC);
        Ok(object.into_any())
    } else {
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
#[pyclass(frozen, weakref, name = "coroutine", module = "steelix_runtime")]
pub struct Coroutine(Core);

#[pymethods]
impl Coroutine {
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
    fn __await__(slf: &Bound<'_, Self>) -> PyResult<CoroutineWrapper> {
        if slf.get().0.suspended() {
            return Err(being_awaited());
        }
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
#[pyclass(frozen, weakref, name = "generator", module = "steelix_runtime")]
pub struct Generator(Core);

#[pymethods]
impl Generator {
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
#[pyclass(frozen, name = "coroutine_wrapper", module = "steelix_runtime")]
pub struct CoroutineWrapper {
    coroutine: Py<Coroutine>,
}

#[pymethods]
impl CoroutineWrapper {
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

// --- finalization ---

/// The classes whose objects close their body when they are deallocated.
trait Finalized: PyClass<Frozen = pyo3::pyclass::boolean_struct::True> + Sync {
    fn core(&self) -> &Core;
}

impl Finalized for Coroutine {
    fn core(&self) -> &Core {
        &self.0
    }
}

impl Finalized for Generator {
    fn core(&self) -> &Core {
        &self.0
    }
}

/// The `tp_dealloc` PyO3 gave each class, which [`dealloc`] calls.
static COROUTINE_DEALLOC: OnceLock<ffi::destructor> = OnceLock::new();
static GENERATOR_DEALLOC: OnceLock<ffi::destructor> = OnceLock::new();

/// Gives the class of `object` a finalizer (PEP 442), when its first object
/// is created.
///
/// CPython closes a suspended generator in `tp_finalize`, which runs before
/// the object is torn down and may run Python code that sees it (its
/// `repr` in an unraisable error, a `finally` block resurrecting it).
/// PyO3 classes can not declare `tp_finalize`, and PyO3's `tp_dealloc` does
/// not call finalizers, so both slots are completed here, like the
/// vectorcall slot of `Function`: `tp_finalize` (which the GC calls for
/// objects in cycles) and a `tp_dealloc` that calls the finalizer first, like
/// the one of Python classes, then PyO3's.
fn install_finalizer<T: Finalized>(
    object: &Bound<'_, T>,
    original: &'static OnceLock<ffi::destructor>,
) {
    static COROUTINE: Once = Once::new();
    static GENERATOR: Once = Once::new();
    let once = if std::ptr::eq(original, &COROUTINE_DEALLOC) {
        &COROUTINE
    } else {
        &GENERATOR
    };
    once.call_once(|| {
        // SAFETY: we are attached, and the only object of the class is this
        // new one, so no deallocation is using these slots yet. The class
        // can not be subclassed, so every object has this type.
        unsafe {
            let ty = object.as_any().get_type_ptr();
            let pyo3_dealloc = (*ty).tp_dealloc.expect("PyO3 classes have a tp_dealloc");
            original.get_or_init(|| pyo3_dealloc);
            (*ty).tp_dealloc = Some(dealloc::<T>);
            (*ty).tp_finalize = Some(finalize::<T>);
            ffi::PyType_Modified(ty);
        }
    });
}

fn original_dealloc<T: Finalized>() -> ffi::destructor {
    let original = if std::any::TypeId::of::<T>() == std::any::TypeId::of::<Coroutine>() {
        &COROUTINE_DEALLOC
    } else {
        &GENERATOR_DEALLOC
    };
    *original
        .get()
        .expect("the slot is replaced after saving the original")
}

/// `tp_dealloc`: runs the finalizer (unless the GC already did), which may
/// resurrect the object, then PyO3's `tp_dealloc`.
unsafe extern "C" fn dealloc<T: Finalized>(object: *mut ffi::PyObject) {
    // SAFETY: CPython calls `tp_dealloc` attached, with a reference count of
    // zero, which is what `PyObject_CallFinalizerFromDealloc` expects.
    unsafe {
        if ffi::PyObject_CallFinalizerFromDealloc(object) < 0 {
            return;
        }
        original_dealloc::<T>()(object);
    }
}

/// `tp_finalize`, through PyO3's trampoline so that PyO3 knows the thread is
/// attached (objects dropped in it are released right away).
unsafe extern "C" fn finalize<T: Finalized>(object: *mut ffi::PyObject) {
    unsafe fn run<T: Finalized>(
        py: Python<'_>,
        object: *mut ffi::PyObject,
    ) -> PyResult<*mut ffi::PyObject> {
        // SAFETY: CPython calls `tp_finalize` with a live object of the type
        // whose slot this is.
        let object = unsafe { Borrowed::from_ptr(py, object) };
        // SAFETY: see above.
        unsafe { object.cast_unchecked::<T>() }
            .get()
            .core()
            .finalize(&object);
        // SAFETY: a new reference to `None`.
        Ok(unsafe { ffi::Py_NewRef(ffi::Py_None()) })
    }
    use pyo3::impl_::trampoline::{MethodDef, unaryfunc};
    struct Run<T>(std::marker::PhantomData<T>);
    impl<T: Finalized> MethodDef<unaryfunc::Func> for Run<T> {
        const METH: unaryfunc::Func = run::<T>;
    }
    // SAFETY: CPython calls this attached. A finalizer must leave the
    // current exception alone: it is saved and restored around it. The
    // result is `None` or NULL (a panic).
    unsafe {
        let saved = ffi::PyErr_GetRaisedException();
        let result = unaryfunc::<Run<T>>(object);
        if result.is_null() {
            ffi::PyErr_WriteUnraisable(object);
        } else {
            ffi::Py_DECREF(result);
        }
        ffi::PyErr_SetRaisedException(saved);
    }
}
