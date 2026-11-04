//! The channel between `send` (or `throw`, `close`) and the awaits of the
//! body for one poll, the waker that leads to it, and what the futures of
//! the body keep in the coroutine object ([`Held`]).

use std::cell::Cell;
use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

use pyo3::exceptions::PyRuntimeError;
use pyo3::ffi;
use pyo3::prelude::*;

use super::state::{Body, Core, Input};

/// Carries values between the `send` (or `throw`, `close`) that polls the
/// body and the await where it is suspended, for one poll. It lives on the
/// stack of the poll; the awaits find it through the waker.
pub(super) struct Channel {
    /// The value or exception for the await that suspended last time.
    pub(super) input: Cell<Option<Input>>,
    /// The value of the await that suspends now, for the caller.
    pub(super) output: Cell<Option<Py<PyAny>>>,
    /// The coroutine being polled, whose `resume` is on the stack below the
    /// poll (see [`with_polled`]).
    pub(super) owner: *const Core,
    /// Whether the body handles an exception of its own in this poll (it
    /// suspended in an `except` or `finally` block).
    pub(super) own_handled: Cell<bool>,
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
    /// poll and the caller's is put back. A block that spans suspensions
    /// puts back, when it ends, what was handled when it started, which may
    /// be the caller's of an earlier poll (design §12).
    pub(super) fn poll(
        &self,
        py: Python<'_>,
        body: &mut Body,
        handled: &mut Option<Py<PyAny>>,
    ) -> Poll<PyResult<Bound<'static, PyAny>>> {
        let caller = handled_exception(py);
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
    /// gets the exception the body is handling there as `__context__`, and
    /// none if the body handles no exception of its own (the caller's does
    /// not count).
    pub(super) fn thrown(&self, py: Python<'_>, err: PyErr) -> PyErr {
        if !self.own_handled.get() {
            return err;
        }
        chain_handled(py, err)
    }

    /// Takes the exception thrown in before the body started, if any.
    fn take_thrown(&self) -> Option<PyErr> {
        match self.input.take() {
            Some(Input::Throw(err, _)) => Some(err),
            other => {
                self.input.set(other);
                None
            }
        }
    }
}

/// Gives `err` the exception being handled as `__context__`, as `raise`
/// would (`PyErr_SetObject`, which also breaks cycles).
fn chain_handled(py: Python<'_>, err: PyErr) -> PyErr {
    let exception = err.into_value(py).into_bound(py);
    // SAFETY: attached; `exception` is an exception instance. The raised
    // exception is taken back right away.
    unsafe {
        ffi::PyErr_SetObject(ffi::Py_TYPE(exception.as_ptr()).cast(), exception.as_ptr());
        PyErr::from_value(Bound::from_owned_ptr(py, ffi::PyErr_GetRaisedException()))
    }
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
/// suspensions: the iterator an [`await_`](super::await_) delegates to, the frame object
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
    pub(super) fn new(core: &Core, object: Bound<'_, PyAny>) -> Self {
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
pub(super) fn channel<'a>(cx: &Context<'a>) -> Option<&'a Channel> {
    let waker = cx.waker();
    let data = waker.data().cast::<Channel>();
    if !std::ptr::eq(waker.vtable(), &WAKER) || data.is_null() || data != CURRENT.get() {
        return None;
    }
    // SAFETY: `CURRENT` is the channel of the poll running on this thread,
    // which lives on the stack of `Channel::poll` until the poll returns.
    Some(unsafe { &*data })
}

pub(super) fn outside_coroutine() -> PyErr {
    PyRuntimeError::new_err(
        "rt::await_ or rt::yield_ polled outside a coroutine or generator driven by Python",
    )
}

pub(super) fn handled_exception(py: Python<'_>) -> Option<Bound<'_, PyAny>> {
    // SAFETY: attached; returns a new reference or NULL.
    unsafe { Bound::from_owned_ptr_or_opt(py, ffi::PyErr_GetHandledException()) }
}

pub(super) fn set_handled_exception(_py: Python<'_>, exception: Option<&Bound<'_, PyAny>>) {
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
