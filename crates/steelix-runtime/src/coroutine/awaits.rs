//! The awaits of a body: `await` ([`await_`]), `yield` ([`yield_`]),
//! `yield from` ([`yield_from`]), direct delegation ([`delegate`]) and the
//! steps of `async for` ([`aiter`], [`anext`]).

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use pyo3::exceptions::{
    PyGeneratorExit, PyRuntimeError, PyStopAsyncIteration, PyStopIteration, PyTypeError,
};
use pyo3::ffi;
use pyo3::prelude::*;
use pyo3::types::PyTuple;

use super::channel::{Channel, Held, channel, outside_coroutine};
use super::finalize::PyErr_FormatUnraisable;
use super::objects::{Coroutine, Generator};
use super::state::{Input, Step};

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

/// `await f(x)` or `yield from g(x)` where `f` is a native coroutine function
/// (or `g` a native generator function) called directly: runs its future in
/// place, without a coroutine object in between, and returns its result.
///
/// ```ignore
/// let total = rt::delegate(add(m, &x, &y)).await?;
/// let rest = rt::delegate(Box::pin(countdown(m, &n))).await?;
/// ```
///
/// What the caller sends and throws goes to the awaits of the delegate as
/// they are, which is what CPython does with a delegate too, except for
/// `close()` (and `throw(GeneratorExit)`): CPython closes the delegate first
/// (`gen_close_iter`), discarding what it returns, and then raises
/// `GeneratorExit` at the `yield from` or `await` regardless, or the error
/// of closing the delegate (`RuntimeError: generator ignored GeneratorExit`
/// if it yields again). This does the same. An exception that comes out of
/// the delegate when something is thrown in enters the delegating body like
/// a thrown one, with the exception the body handles there as `__context__`
/// (`_gen_throw`), like [`await_`] does. The delegate shares the exception
/// state of the delegating body: where it handles none of its own it sees
/// the body's, CPython's would see the caller's (design §12).
pub async fn delegate<'py, F>(future: F) -> PyResult<Bound<'py, PyAny>>
where
    F: Future<Output = PyResult<Bound<'py, PyAny>>>,
{
    let mut future = std::pin::pin!(future);
    // Whether the delegate suspended in the last poll: the input of the
    // next one is for it.
    let mut suspended = false;
    std::future::poll_fn(move |cx| {
        let Some(channel) = channel(cx) else {
            return future.as_mut().poll(cx);
        };
        Python::attach(|py| {
            // A chain of direct delegations nests these polls on the C stack
            // within one `resume`: too deep raises `RecursionError`.
            let _stack = match crate::RecursionGuard::enter(py) {
                Ok(guard) => guard,
                Err(err) => return Poll::Ready(Err(err)),
            };
            let poll = match channel.input.take() {
                // What comes out enters the body like a thrown exception.
                Some(Input::Throw(err, args)) if suspended => {
                    let poll = if err.is_instance_of::<PyGeneratorExit>(py) {
                        Poll::Ready(Err(close_delegate(py, channel, future.as_mut(), cx, err)))
                    } else {
                        channel.input.set(Some(Input::Throw(err, args)));
                        future.as_mut().poll(cx)
                    };
                    match poll {
                        Poll::Ready(Err(err)) => Poll::Ready(Err(channel.thrown(py, err))),
                        poll => poll,
                    }
                }
                input => {
                    channel.input.set(input);
                    future.as_mut().poll(cx)
                }
            };
            suspended = poll.is_pending();
            poll
        })
    })
    .await
}

/// `close()` or `throw(GeneratorExit)` of a body suspended in [`delegate`]:
/// closes the delegate like `gen_close` (a new `GeneratorExit` where it is
/// suspended) and returns the exception to raise in the delegating body,
/// `exit` or the error of closing the delegate.
fn close_delegate<'py, F>(
    py: Python<'_>,
    channel: &Channel,
    mut future: Pin<&mut F>,
    cx: &mut Context<'_>,
    exit: PyErr,
) -> PyErr
where
    F: Future<Output = PyResult<Bound<'py, PyAny>>>,
{
    // SAFETY: the owner of the running poll's channel is the `Core` whose
    // `resume` is on the stack below this poll.
    let noun = unsafe { &*channel.owner }.flavor.noun();
    let ignored = || {
        crate::raised(PyRuntimeError::new_err(format!(
            "{noun} ignored GeneratorExit"
        )))
    };
    // `gen_close`: `Ok(true)` if the delegate yielded again.
    let mut close = || -> PyResult<bool> {
        let new_exit = PyGeneratorExit::new_err(());
        channel.input.set(Some(Input::Throw(new_exit, None)));
        let poll = future.as_mut().poll(cx);
        channel.input.set(None);
        match poll {
            Poll::Pending => {
                drop(channel.output.take());
                Ok(true)
            }
            Poll::Ready(Err(err)) if !err.is_instance_of::<PyGeneratorExit>(py) => Err(err),
            Poll::Ready(_) => Ok(false),
        }
    };
    match close() {
        Ok(false) => exit,
        Err(err) => err,
        Ok(true) => {
            // CPython drops the delegate as this error leaves the `yield
            // from`, and its finalizer closes it once more.
            let again = match close() {
                Ok(false) => None,
                Ok(true) => Some(ignored()),
                Err(err) => Some(err),
            };
            if let Some(again) = again {
                again.restore(py);
                let message = format!("Exception ignored while closing {noun}");
                let message = std::ffi::CString::new(message).expect("no NUL in the message");
                // SAFETY: an exception is set; the message has no `%`.
                unsafe { PyErr_FormatUnraisable(message.as_ptr()) };
            }
            ignored()
        }
    }
}

/// `async for`: the asynchronous iterator of `iterable` (`GET_AITER`),
/// which [`anext`] steps through.
///
/// ```ignore
/// let iter = rt::aiter(&rows)?;
/// while let Some(row) = rt::anext(&iter).await? {
///     ..
/// }
/// ```
pub fn aiter<'py>(iterable: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let Some(method) = crate::frame::lookup_special(iterable, "__aiter__")? else {
        return Err(crate::raised(PyTypeError::new_err(format!(
            "'async for' requires an object with __aiter__ method, got {}",
            crate::type_name(iterable)
        ))));
    };
    let iterator = method.call0()?;
    if crate::frame::lookup_special(&iterator, "__anext__")?.is_none() {
        return Err(crate::raised(PyTypeError::new_err(format!(
            "'async for' received an object from __aiter__ that does not implement __anext__: {}",
            crate::type_name(&iterator)
        ))));
    }
    Ok(iterator)
}

/// The next item of an `async for` loop (`GET_ANEXT`): awaits what
/// `iterator.__anext__()` returns, `None` once that raises
/// `StopAsyncIteration`.
pub async fn anext<'py>(iterator: &Bound<'py, PyAny>) -> PyResult<Option<Bound<'py, PyAny>>> {
    let py = iterator.py();
    let next = async {
        let Some(method) = crate::frame::lookup_special(iterator, "__anext__")? else {
            return Err(crate::raised(PyTypeError::new_err(format!(
                "'async for' requires an iterator with __anext__ method, got {}",
                crate::type_name(iterator)
            ))));
        };
        let awaitable = method.call0()?;
        let iter = awaitable_iter(&awaitable).map_err(|cause| {
            let err = PyTypeError::new_err(format!(
                "'async for' received an invalid object from __anext__: {}",
                crate::type_name(&awaitable)
            ));
            // `_PyErr_FormatFromCause`: the cause is the context too.
            let value = err.value(py);
            let cause = cause.into_value(py).into_bound(py);
            // SAFETY: valid exception objects; both calls steal a reference.
            unsafe {
                ffi::PyException_SetContext(value.as_ptr(), cause.clone().into_ptr());
                ffi::PyException_SetCause(value.as_ptr(), cause.into_ptr());
            }
            err
        })?;
        Await {
            py,
            state: AwaitState::Start(iter, Delegate::Iterator),
        }
        .await
    };
    match next.await {
        Ok(item) => Ok(Some(item)),
        Err(err) if err.is_instance_of::<PyStopAsyncIteration>(py) => Ok(None),
        Err(err) => Err(err),
    }
}

/// Checks that `value`, which the `__aenter__` or `__aexit__` (`method`)
/// of an `async with` returned, can be awaited, with CPython's message
/// for one whose type has no `__await__`.
pub(crate) fn check_async_with_awaitable(value: &Bound<'_, PyAny>, method: &str) -> PyResult<()> {
    if is_coroutine(value)? || crate::frame::lookup_special(value, "__await__")?.is_some() {
        return Ok(());
    }
    Err(crate::raised(PyTypeError::new_err(format!(
        "'async with' received an object from {method} that does not implement __await__: {}",
        crate::type_name(value)
    ))))
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
    /// The iterator to delegate to, already taken from an awaitable.
    Iterator,
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
                    Delegate::Iterator => Ok(value),
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
                    Some(Input::Throw(err, args)) => {
                        throw(&iter, err, args).map_err(|err| channel.thrown(py, err))
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

pub(super) fn being_awaited() -> PyErr {
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
pub(super) fn send<'py>(
    iter: &Bound<'py, PyAny>,
    value: &Bound<'py, PyAny>,
) -> PyResult<Step<'py>> {
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
/// raised here, anything else goes to its `throw` method if it has one. A
/// generator or coroutine gets the exception, anything else the arguments
/// of the `throw()` call (`args`) if there was one.
pub(super) fn throw<'py>(
    iter: &Bound<'py, PyAny>,
    err: PyErr,
    args: Option<Py<PyTuple>>,
) -> PyResult<Step<'py>> {
    let py = iter.py();
    if err.is_instance_of::<PyGeneratorExit>(py) {
        close_iter(iter)?;
        return Err(err);
    }
    let Some(throw) = iter.getattr_opt(crate::interned(py, "throw"))? else {
        return Err(err);
    };
    // SAFETY: valid object pointer.
    let generator = unsafe {
        ffi::PyGen_CheckExact(iter.as_ptr()) != 0 || ffi::PyCoro_CheckExact(iter.as_ptr()) != 0
    } || iter.is_instance_of::<Coroutine>()
        || iter.is_instance_of::<Generator>()
        || iter.is_instance_of::<crate::GenExpr>();
    let result = match args {
        Some(args) if !generator => throw.call1(args.bind(py)),
        _ => throw.call1((err.into_value(py),)),
    };
    match result {
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
            Some(Input::Throw(err, _)) => Err(channel.thrown(py, err)),
            None => Ok(py.None().into_bound(py)),
        })
    }
}
