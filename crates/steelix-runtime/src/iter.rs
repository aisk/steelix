//! Native loops: the iterator of a `for` loop and the check that CPython's
//! eval loop makes at the end of every iteration.

use std::sync::atomic::{AtomicUsize, Ordering};

use pyo3::ffi;
use pyo3::prelude::*;
use pyo3::types::PyIterator;

/// `for x in xs`: the items of `xs`, making the [`checkpoint`] before each
/// item after the first, where CPython's eval loop makes it at the end of
/// every iteration (so Ctrl-C stops a native loop with `KeyboardInterrupt`
/// and other threads get to run, design §12). Like `xs.try_iter()?`, every
/// item is a `PyResult`.
///
/// ```ignore
/// for x in rt::iter(&xs)? {
///     let x = x?;
///     ..
/// }
/// ```
pub fn iter<'py, T>(iterable: &Bound<'py, T>) -> PyResult<Iter<'py>> {
    Ok(Iter {
        iterator: iterable.as_any().try_iter()?,
        started: false,
    })
}

/// See [`iter`].
pub struct Iter<'py> {
    iterator: Bound<'py, PyIterator>,
    started: bool,
}

impl<'py> Iterator for Iter<'py> {
    type Item = PyResult<Bound<'py, PyAny>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.started {
            if let Err(err) = checkpoint(self.iterator.py()) {
                return Some(Err(err));
            }
        } else {
            self.started = true;
        }
        self.iterator.next()
    }
}

/// The end of an iteration of a native loop (`while`, or a `for` loop that
/// does not use [`iter`]), where CPython's eval loop checks its "eval
/// breaker":
///
/// ```ignore
/// while !done.is_truthy()? {
///     ..
///     rt::checkpoint(py)?;
/// }
/// ```
///
/// - Another thread waiting for the GIL longer than `sys.getswitchinterval()`
///   asks the running thread to drop it. The loop then lets it run, as the
///   eval loop does, instead of keeping the GIL until it calls something that
///   releases it (a loop waiting for a flag that another thread sets would
///   never end).
/// - Pending signal handlers run (`PyErr_CheckSignals`), so Ctrl-C stops the
///   loop with `KeyboardInterrupt`, and so does a scheduled garbage
///   collection.
///
/// Nothing needs doing almost always: then it costs a read of the thread
/// state, cheaper than calling `PyErr_CheckSignals` every time.
#[inline]
pub fn checkpoint(py: Python<'_>) -> PyResult<()> {
    let breaker = eval_breaker(py);
    if breaker & EVENTS == 0 {
        return Ok(());
    }
    handle_events(py, breaker)
}

/// Lets another thread take the GIL if one asked for it, as CPython does
/// when a function starts or a generator resumes (`RecursionGuard::enter`,
/// `GenExpr`), so that recursion without loops does not starve the other
/// threads. Unlike [`checkpoint`] it runs no signal handlers: a
/// `KeyboardInterrupt` raised there would not be in the function's frame,
/// so signals stay with the loops, where they always were.
#[inline]
pub(crate) fn switch_threads(py: Python<'_>) {
    if eval_breaker(py) & GIL_DROP_REQUEST != 0 {
        drop_gil(py);
    }
}

#[cold]
#[inline(never)]
fn handle_events(py: Python<'_>, breaker: usize) -> PyResult<()> {
    if breaker & GIL_DROP_REQUEST != 0 {
        drop_gil(py);
    }
    // Signals, and a scheduled garbage collection, which
    // `PyErr_CheckSignals` runs as well. Other events (pending calls, an
    // exception set with `PyThreadState_SetAsyncExc`) have no public API
    // to handle them; they wait for the eval loop, like before.
    py.check_signals()
}

#[cold]
#[inline(never)]
fn drop_gil(py: Python<'_>) {
    // With the drop request set, releasing the GIL waits until another
    // thread has taken it (CPython's `FORCE_SWITCHING`); taking it back
    // then waits for its turn like any thread.
    py.detach(|| ());
}

/// Bits of `PyThreadState.eval_breaker`, from CPython's
/// `Include/internal/pycore_ceval.h`: the low eight bits are events
/// (`_PY_EVAL_EVENTS_MASK`), the others the instrumentation version.
const GIL_DROP_REQUEST: usize = 1 << 0;
const EVENTS: usize = 0xff;

/// The start of `struct _ts` (`PyThreadState`), as declared by the public
/// header `Include/cpython/pystate.h` since CPython 3.13.
#[repr(C)]
struct ThreadStateHead {
    prev: *mut ffi::PyThreadState,
    next: *mut ffi::PyThreadState,
    interp: *mut ffi::PyInterpreterState,
    eval_breaker: AtomicUsize,
}

/// The current thread's `eval_breaker`, which the eval loop reads on every
/// back edge and other threads set atomically (the GIL drop request, signals).
#[inline]
fn eval_breaker(_py: Python<'_>) -> usize {
    // SAFETY: the token proves the thread is attached, so it has a thread
    // state, which starts with the fields of `ThreadStateHead`; CPython
    // only accesses `eval_breaker` atomically, as this does.
    unsafe {
        let tstate = ffi::PyThreadState_GetUnchecked().cast::<ThreadStateHead>();
        (*tstate).eval_breaker.load(Ordering::Relaxed)
    }
}
