//! Native coroutines (`async def`) and generator functions (design §8,
//! "协程和生成器").
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
//! `Channel` of the current poll and returns `Poll::Pending`, which goes up
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
//! without a Python object in between: `rt::delegate(add(m, &x, &y)).await?`
//! ([`delegate`]). A generator function is the same with `rt::yield_`
//! (`rt::def!(m, gen count(n))`).
//!
//! The parts: `state` (the state shared by both objects, and resuming the
//! body), `channel` (the channel of a poll, the waker, `Held`), `awaits`
//! (the futures the body awaits), `objects` (the Python classes) and
//! `finalize` (closing on deallocation).
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

mod awaits;
mod channel;
mod finalize;
mod objects;
mod state;

use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

use pyo3::prelude::*;
use pyo3::{PyTraverseError, PyVisit};

use crate::Module;

pub(crate) use awaits::check_async_with_awaitable;
pub use awaits::{Await, Yield, aiter, anext, await_, delegate, yield_, yield_from};
pub(crate) use channel::{Held, thrown_at_start};
pub(crate) use objects::create;
pub use objects::{Coroutine, CoroutineWrapper, Generator};

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
