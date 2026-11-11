//! Traceback frames and exception handling for generated code (design §9).
//!
//! Native functions have no Python frame while they run. Like Cython, they
//! add one to the traceback of an exception on its way out instead: every
//! generated function has the attribute [`frame`](macro@crate::frame), which knows
//! the Python file, the function's qualified name and its `def` line.
//! Tracebacks of uncaught exceptions and `traceback.extract_tb` then have
//! CPython's frames, with the `def` line for native ones. Code generated with
//! `--traceback-lines` also keeps the line of the statement it is running up
//! to date with [`Frame::line`], so the lines are CPython's too.
//!
//! The same object implements the parts of `try`, `except`, `finally`,
//! `raise` and `with` that need to know about the frame: an exception gets an
//! entry for the frame when it is raised in it, or when it reaches a handler
//! or the end of the function, but not again when it is re-raised (bare
//! `raise`, the end of a `finally` block), exactly as in CPython.

use std::cell::{Cell, RefCell};
use std::ffi::CString;

use pyo3::BoundObject;
use pyo3::exceptions::{PyRuntimeError, PyTypeError};
use pyo3::ffi;
use pyo3::prelude::*;
use pyo3::sync::PyOnceLock;
use pyo3::types::{PyDict, PyTuple, PyType};

use crate::Module;

/// What [`Frame`] needs to create a frame object for tracebacks: the
/// function's qualified name and the line of its `def`. The code object is
/// created when the first exception passes through, and kept.
pub struct Code {
    qualname: &'static str,
    first_line: u32,
    recursive: bool,
    object: PyOnceLock<Py<PyAny>>,
}

impl Code {
    /// `qualname` is the function's `__qualname__`, or `<module>` for module
    /// level code.
    pub const fn new(qualname: &'static str, first_line: u32) -> Self {
        Self {
            qualname,
            first_line,
            recursive: false,
            object: PyOnceLock::new(),
        }
    }

    /// For a function that may call itself with direct Rust calls: each
    /// call checks the stack first ([`RecursionGuard`](crate::RecursionGuard)),
    /// so that unbounded recursion raises `RecursionError`.
    pub const fn recursive(mut self) -> Self {
        self.recursive = true;
        self
    }

    fn is_module(&self) -> bool {
        self.qualname == "<module>"
    }

    /// The name shown in tracebacks: the last part of the qualified name.
    fn name(&self) -> &'static str {
        self.qualname.rsplit('.').next().unwrap_or(self.qualname)
    }

    fn object<'py>(&self, module: &Module<'py>, class: bool) -> PyResult<&Bound<'py, PyAny>> {
        let py = module.py();
        self.object
            .get_or_try_init(py, || {
                let filename = CString::new(crate::python::filename(module)?.to_cow()?.as_ref())?;
                let name = CString::new(self.name())?;
                // SAFETY: valid C strings; the result is a new reference or NULL.
                let code = unsafe {
                    Bound::from_owned_ptr_or_err(
                        py,
                        ffi::PyCode_NewEmpty(
                            filename.as_ptr(),
                            name.as_ptr(),
                            self.first_line as std::ffi::c_int,
                        )
                        .cast(),
                    )?
                };
                let changes = PyDict::new(py);
                changes.set_item("co_qualname", self.qualname)?;
                if !self.is_module() && !class {
                    // CO_OPTIMIZED | CO_NEWLOCALS, like the code of a `def`.
                    changes.set_item("co_flags", 0x3)?;
                }
                Ok(code.call_method("replace", (), Some(&changes))?.unbind())
            })
            .map(|code| code.bind(py))
    }
}

/// The traceback frame of one call of a native function, or of the module
/// body. Created by [`#[rt::frame]`](macro@crate::frame).
pub struct Frame<'a, 'py> {
    /// Borrowed from the module: the frame of a suspended coroutine adds no
    /// reference to the globals that the GC can not see.
    globals: &'a Bound<'py, PyDict>,
    module: &'a Module<'py>,
    /// The namespace of a class body, which runs in its own frame.
    class_namespace: Option<&'a Bound<'py, PyAny>>,
    code: &'static Code,
    line: Cell<u32>,
    /// The frame object, created when the first exception needs it and
    /// shared by all entries of this call, like a Python frame.
    object: RefCell<FrameObject<'py>>,
    /// The frame of the body of a native coroutine or generator, which
    /// keeps its frame object in the coroutine object (see `object`).
    suspends: bool,
    /// An exception that already has its entry for this frame: raised here
    /// with [`Frame::raise`], or re-raised. It gets no second entry when it
    /// reaches a handler or leaves the function.
    ///
    /// Only addresses, see [`Entered`]: the generated code drops an
    /// exception whose handler does not need it without telling the frame,
    /// and holding it here would keep it (with its arguments and traceback)
    /// alive until the function returns, while CPython frees it when the
    /// handler ends.
    entered: Cell<Entered>,
}

/// Identifies the exception in [`Frame::entered`] without keeping it alive:
/// its address, and the address of the frame at the head of its traceback
/// then (this frame, or for a bare `raise` the frame whose handler is
/// running, which is still running too).
///
/// Once the exception is freed, a new one may get its address. That one
/// can only have reached this frame from a callee, so the head of its
/// traceback is another frame: whenever an exception gets an entry for this
/// frame, `entered` is updated.
/// Where a [`Frame`] keeps its frame object.
enum FrameObject<'py> {
    None,
    Owned(Bound<'py, PyAny>),
    /// In the coroutine whose body the frame belongs to, where the GC sees
    /// it, when the frame suspends with it.
    Held(crate::coroutine::Held),
}

#[derive(Clone, Copy)]
struct Entered {
    exception: *mut ffi::PyObject,
    head: *mut ffi::PyObject,
}

impl Entered {
    const NONE: Self = Self {
        exception: std::ptr::null_mut(),
        head: std::ptr::null_mut(),
    };

    fn of(exception: &Bound<'_, PyAny>) -> Self {
        Self {
            exception: exception.as_ptr(),
            head: head_frame(exception),
        }
    }

    fn is(self, exception: &Bound<'_, PyAny>) -> bool {
        self.exception == exception.as_ptr()
            && !self.head.is_null()
            && self.head == head_frame(exception)
    }
}

/// The frame of the first traceback entry of `exception`, or null. Only
/// compared: the traceback keeps it alive while the exception has it.
fn head_frame(exception: &Bound<'_, PyAny>) -> *mut ffi::PyObject {
    let py = exception.py();
    let head = || -> PyResult<*mut ffi::PyObject> {
        let traceback = exception.getattr(crate::interned(py, "__traceback__"))?;
        if traceback.is_none() {
            return Ok(std::ptr::null_mut());
        }
        Ok(traceback.getattr(crate::interned(py, "tb_frame"))?.as_ptr())
    };
    head().unwrap_or(std::ptr::null_mut())
}

/// What code runs in: a module (for the module body and functions) or the
/// namespace of a class body.
#[doc(hidden)]
pub trait Scope<'py> {
    fn module(&self) -> &Module<'py>;
    fn globals(&self) -> &Bound<'py, PyDict> {
        self.module().globals()
    }
    fn class_namespace(&self) -> Option<&Bound<'py, PyAny>> {
        None
    }
}

impl<'py> Scope<'py> for Module<'py> {
    fn module(&self) -> &Module<'py> {
        self
    }
}

impl<'py, S: Scope<'py>> Scope<'py> for &S {
    fn module(&self) -> &Module<'py> {
        S::module(self)
    }
    fn class_namespace(&self) -> Option<&Bound<'py, PyAny>> {
        S::class_namespace(self)
    }
}

impl<'py> Scope<'py> for crate::Namespace<'py> {
    fn module(&self) -> &Module<'py> {
        crate::Namespace::module(self)
    }
    fn class_namespace(&self) -> Option<&Bound<'py, PyAny>> {
        Some(self.mapping())
    }
}

/// Runs the body of a native function (or module, or class) in a traceback
/// frame. Use it through [`#[rt::frame]`](macro@crate::frame).
pub fn run<'py, T>(
    scope: &impl Scope<'py>,
    code: &'static Code,
    body: impl FnOnce(&Frame<'_, 'py>) -> PyResult<T>,
) -> PyResult<T> {
    // Before the frame: `RecursionError` is raised in the caller.
    let _guard = match code.recursive {
        true => Some(crate::RecursionGuard::enter(scope.globals().py())?),
        false => None,
    };
    let frame = Frame::new(scope, code, false);
    body(&frame).map_err(|err| PyErr::from_value(frame.take(err)))
}

/// Runs the body of a native coroutine or generator in a traceback frame:
/// [`#[rt::frame]`](macro@crate::frame) on an `async fn`.
///
/// The frame lives as long as the body, across its suspensions, so an
/// exception gets one entry for it however often the body was resumed. An
/// exception thrown in before the body started is raised here, before the
/// first statement, like CPython does.
pub async fn run_async<'py, T>(
    scope: &impl Scope<'py>,
    code: &'static Code,
    body: impl AsyncFnOnce(&Frame<'_, 'py>) -> PyResult<T>,
) -> PyResult<T> {
    let frame = Frame::new(scope, code, true);
    let result = match crate::coroutine::thrown_at_start().await {
        Some(err) => Err(err),
        None => body(&frame).await,
    };
    result.map_err(|err| PyErr::from_value(frame.take(err)))
}

impl<'f, 'py> Frame<'f, 'py> {
    fn new(scope: &'f impl Scope<'py>, code: &'static Code, suspends: bool) -> Self {
        Frame {
            module: scope.module(),
            globals: scope.globals(),
            class_namespace: scope.class_namespace(),
            code,
            line: Cell::new(code.first_line),
            object: RefCell::new(FrameObject::None),
            suspends,
            entered: Cell::new(Entered::NONE),
        }
    }

    pub fn py(&self) -> Python<'py> {
        self.globals.py()
    }

    /// The qualified name of the function whose body this is, for the
    /// names of the functions it defines (`f.<locals>.g`): `None` for a
    /// module or class body.
    fn function_qualname(&self) -> Option<&'static str> {
        (!self.code.is_module() && self.class_namespace.is_none()).then_some(self.code.qualname)
    }

    /// Records that the function is now running the statement on `line`
    /// of the Python file.
    pub fn line(&self, line: u32) {
        self.line.set(line);
    }

    /// An exception caught by an `except` block: its traceback gets this
    /// frame at the current line, unless it already has it, and it is the
    /// exception being handled until the returned [`Caught`] is dropped, at
    /// the end of the block.
    ///
    /// ```ignore
    /// Err(err) if err.is_instance_of::<PyValueError>(py) => {
    ///     let e = frame.catch(err);
    ///     rt::builtins::print(py, ("caught", &e))?;
    /// }
    /// ```
    pub fn catch(&self, err: PyErr) -> Caught<'py> {
        let exception = self.take(err);
        let handling = Handling::new(&exception);
        Caught {
            _handling: handling,
            exception,
        }
    }

    /// An exception that reached a `finally` block, the end of a `with`
    /// block or the end of the function: its traceback gets this frame at
    /// the current line, unless it already has it.
    fn take(&self, err: PyErr) -> Bound<'py, PyAny> {
        let py = self.py();
        let exception = err.into_value(py).into_bound(py).into_any();
        self.enter(&exception);
        self.entered.set(Entered::NONE);
        exception
    }

    /// Gives `exception` this frame's traceback entry at the current line,
    /// unless it already has it, and remembers that it has it.
    fn enter(&self, exception: &Bound<'py, PyAny>) {
        if self.entered.get().is(exception) {
            return;
        }
        let entered = if self.raised_by_kept_code(exception) {
            Entered::of(exception)
        } else {
            self.add_entry(exception)
        };
        self.entered.set(entered);
    }

    /// `except handler:`, where `handler` is not a builtin exception class:
    /// evaluates `handler` and tells whether `err` is an instance of it (or
    /// of a class in a tuple of classes).
    ///
    /// Like CPython, the handler expression runs with `err` as the exception
    /// being handled: an exception it raises, or the `TypeError` for a
    /// handler that is not an exception class, has `err` as its
    /// `__context__`.
    ///
    /// ```ignore
    /// Err(err) if frame.matches(&err, || m.global("ParseError"))? => { .. }
    /// ```
    pub fn matches(
        &self,
        err: &PyErr,
        handler: impl FnOnce() -> PyResult<Bound<'py, PyAny>>,
    ) -> PyResult<bool> {
        self.matches_at(err, self.line.get(), handler)
    }

    /// [`Frame::matches`] for an `except` on `line` (`--traceback-lines`):
    /// the handler expression is evaluated on that line, where its errors
    /// are reported, and the frame stays there.
    ///
    /// ```ignore
    /// Err(err) if frame.matches_at(&err, 7, || m.global("ParseError"))? => { .. }
    /// ```
    pub fn matches_at(
        &self,
        err: &PyErr,
        line: u32,
        handler: impl FnOnce() -> PyResult<Bound<'py, PyAny>>,
    ) -> PyResult<bool> {
        let exception = err.value(self.py()).as_any();
        self.enter(exception);
        self.line(line);
        let _handling = Handling::new(exception);
        let handler = handler()?;
        self.line(line);
        matches(err, &handler)
    }

    /// `raise exception`: an exception class or instance, or a `PyErr` such
    /// as `PyValueError::new_err("..")`. Raised here, so its traceback gets
    /// this frame at the current line.
    pub fn raise(&self, exception: impl IntoException<'py>) -> PyErr {
        self.raise_with(exception, None)
    }

    /// `raise exception from cause`, where `cause` may be `None`.
    pub fn raise_from(
        &self,
        exception: impl IntoException<'py>,
        cause: impl IntoPyObject<'py>,
    ) -> PyErr {
        match cause.into_pyobject(self.py()) {
            Ok(cause) => self.raise_with(exception, Some(cause.into_any().into_bound())),
            Err(err) => self.raise(err.into()),
        }
    }

    fn raise_with(
        &self,
        exception: impl IntoException<'py>,
        cause: Option<Bound<'py, PyAny>>,
    ) -> PyErr {
        let py = self.py();
        let exception = match exception
            .into_exception(py)
            .and_then(|exception| set_cause(&exception, cause).map(|()| exception))
        {
            Ok(exception) => exception,
            // Raising failed (a bad class, a bad cause): that is the exception.
            Err(err) => err.into_value(py).into_bound(py).into_any(),
        };
        // SAFETY: `exception` is an exception instance. `PyErr_SetObject`
        // chains the exception being handled as `__context__` (breaking
        // cycles) exactly like `raise` does, and the raised exception is taken
        // back right away.
        let exception = unsafe {
            ffi::PyErr_SetObject(ffi::Py_TYPE(exception.as_ptr()).cast(), exception.as_ptr());
            Bound::from_owned_ptr(py, ffi::PyErr_GetRaisedException())
        };
        self.entered.set(self.add_entry(&exception));
        PyErr::from_value(exception)
    }

    /// A bare `raise`: re-raises the exception being handled, keeping its
    /// traceback. Without one it raises `RuntimeError`.
    pub fn reraise(&self) -> PyErr {
        let py = self.py();
        // SAFETY: returns a new reference, or NULL when no exception is handled.
        let handled = unsafe { Bound::from_owned_ptr_or_opt(py, ffi::PyErr_GetHandledException()) };
        match handled {
            Some(exception) => self.reraise_exception(exception),
            None => self.raise(PyRuntimeError::new_err("No active exception to reraise")),
        }
    }

    /// Re-raises an exception that already went through this frame, without
    /// another traceback entry.
    pub(crate) fn reraise_exception(&self, exception: Bound<'py, PyAny>) -> PyErr {
        self.entered.set(Entered::of(&exception));
        PyErr::from_value(exception)
    }

    /// The start of a `finally` block, with the outcome of the `try` (and
    /// `except`) part. While it runs, a pending exception is the one being
    /// handled; [`Finally::resume`] continues with the outcome afterwards.
    ///
    /// ```ignore
    /// let result = (|| -> PyResult<()> { ..; Ok(()) })();
    /// let finally = frame.finally(result);
    /// file.call_method0("close")?;
    /// finally.resume()?;
    /// ```
    pub fn finally<'a, T>(&'a self, result: PyResult<T>) -> Finally<'a, 'py, T> {
        match result {
            Ok(value) => Finally {
                frame: self,
                outcome: Ok(value),
                _handling: None,
            },
            Err(err) => {
                let exception = self.take(err);
                let handling = Handling::new(&exception);
                Finally {
                    frame: self,
                    outcome: Err(exception),
                    _handling: Some(handling),
                }
            }
        }
    }

    /// `with manager as value: body`: calls `__enter__`, runs the body with
    /// its result, then `__exit__`, which may suppress an exception from the
    /// body. Both methods are looked up on the type, as CPython does.
    ///
    /// `__exit__` runs on the current line when this is called: the line of
    /// the `with` statement with `--traceback-lines`, else the `def` line.
    /// The body returns what the statement does next
    /// ([`Flow`] if it may jump out of it), the default after a suppressed
    /// exception.
    pub fn with<T: Default>(
        &self,
        manager: &Bound<'py, PyAny>,
        body: impl FnOnce(&Bound<'py, PyAny>) -> PyResult<T>,
    ) -> PyResult<T> {
        let py = self.py();
        let line = self.line.get();
        let exit = special_method(manager, "__exit__")?;
        let enter = special_method(manager, "__enter__")?;
        let value = enter.call0()?;
        let result = body(&value);
        match result {
            Ok(result) => {
                self.line(line);
                exit.call1((py.None(), py.None(), py.None()))?;
                Ok(result)
            }
            Err(err) => {
                let exception = self.take(err);
                self.line(line);
                let _handling = Handling::new(&exception);
                let traceback = exception.getattr("__traceback__")?;
                let suppress = exit
                    .call1((exception.get_type(), &exception, traceback))?
                    .is_truthy()?;
                if suppress {
                    Ok(T::default())
                } else {
                    Err(self.reraise_exception(exception))
                }
            }
        }
    }

    /// [`Frame::with`] for a `with` statement in a coroutine or generator
    /// whose body awaits or yields: the body is an `async` closure.
    ///
    /// ```ignore
    /// frame.with_async(&lock, async |_| {
    ///     rt::yield_(&item).await?;
    ///     Ok(())
    /// })
    /// .await?;
    /// ```
    pub async fn with_async<T: Default>(
        &self,
        manager: &Bound<'py, PyAny>,
        body: impl AsyncFnOnce(&Bound<'py, PyAny>) -> PyResult<T>,
    ) -> PyResult<T> {
        let py = self.py();
        let line = self.line.get();
        let exit = special_method(manager, "__exit__")?;
        let enter = special_method(manager, "__enter__")?;
        let value = enter.call0()?;
        let result = body(&value).await;
        match result {
            Ok(result) => {
                self.line(line);
                exit.call1((py.None(), py.None(), py.None()))?;
                Ok(result)
            }
            Err(err) => {
                let exception = self.take(err);
                self.line(line);
                let _handling = Handling::new(&exception);
                let traceback = exception.getattr("__traceback__")?;
                let suppress = exit
                    .call1((exception.get_type(), &exception, traceback))?
                    .is_truthy()?;
                if suppress {
                    Ok(T::default())
                } else {
                    Err(self.reraise_exception(exception))
                }
            }
        }
    }

    /// `async with manager as value: body` in a coroutine: [`Frame::with`]
    /// with `__aenter__` and `__aexit__`, whose results are awaited. The
    /// body is an `async` closure.
    ///
    /// ```ignore
    /// frame.async_with(&lock, async |_| {
    ///     total.set(total.get()?.add(1)?)?;
    ///     Ok(())
    /// })
    /// .await?;
    /// ```
    pub async fn async_with<T: Default>(
        &self,
        manager: &Bound<'py, PyAny>,
        body: impl AsyncFnOnce(&Bound<'py, PyAny>) -> PyResult<T>,
    ) -> PyResult<T> {
        let py = self.py();
        let line = self.line.get();
        let exit = async_special_method(manager, "__aexit__")?;
        let enter = async_special_method(manager, "__aenter__")?;
        let entered = enter.call0()?;
        crate::coroutine::check_async_with_awaitable(&entered, "__aenter__")?;
        let value = crate::await_(entered).await?;
        let result = body(&value).await;
        match result {
            Ok(result) => {
                self.line(line);
                let exited = exit.call1((py.None(), py.None(), py.None()))?;
                crate::coroutine::check_async_with_awaitable(&exited, "__aexit__")?;
                crate::await_(exited).await?;
                Ok(result)
            }
            Err(err) => {
                let exception = self.take(err);
                self.line(line);
                let _handling = Handling::new(&exception);
                let traceback = exception.getattr("__traceback__")?;
                let exited = exit.call1((exception.get_type(), &exception, traceback))?;
                crate::coroutine::check_async_with_awaitable(&exited, "__aexit__")?;
                let suppress = crate::await_(exited).await?.is_truthy()?;
                if suppress {
                    Ok(T::default())
                } else {
                    Err(self.reraise_exception(exception))
                }
            }
        }
    }

    /// Adds this frame's traceback entry at the current line to `exception`.
    ///
    /// Adding the entry can only fail when Python itself is in trouble (no
    /// memory, no stack left); the traceback then just lacks this frame, and
    /// the result remembers nothing.
    fn add_entry(&self, exception: &Bound<'py, PyAny>) -> Entered {
        let add = || -> PyResult<Entered> {
            let py = self.py();
            let frame = self.object()?;
            let next = exception.getattr("__traceback__")?;
            let entry = traceback_type(py)?.call1((next, &frame, -1, self.line.get()))?;
            exception.setattr("__traceback__", entry)?;
            Ok(Entered {
                exception: exception.as_ptr(),
                head: frame.as_ptr(),
            })
        };
        add().unwrap_or(Entered::NONE)
    }

    fn object(&self) -> PyResult<Bound<'py, PyAny>> {
        let py = self.py();
        match &*self.object.borrow() {
            FrameObject::Owned(frame) => return Ok(frame.clone()),
            // Gone if the coroutine was cleared (or this is not its
            // poll): a new one then.
            FrameObject::Held(held) => {
                if let Some(frame) = held.get(py) {
                    return Ok(frame);
                }
            }
            FrameObject::None => {}
        }
        let module = Module::from_globals(self.globals.clone());
        let code = self.code.object(&module, self.class_namespace.is_some())?;
        let locals = if let Some(namespace) = &self.class_namespace {
            namespace.as_ptr()
        } else if self.code.is_module() {
            self.globals.as_ptr()
        } else {
            std::ptr::null_mut()
        };
        // SAFETY: a code object and a dict, attached to the interpreter; the
        // result is a new reference or NULL.
        let frame = unsafe {
            Bound::from_owned_ptr_or_err(
                py,
                ffi::PyFrame_New(
                    ffi::PyThreadState_Get(),
                    code.as_ptr().cast(),
                    self.globals.as_ptr(),
                    locals,
                )
                .cast(),
            )?
        };
        let held = if self.suspends {
            crate::coroutine::Held::in_current(&frame)
        } else {
            None
        };
        let slot = match held {
            Some(held) => FrameObject::Held(held),
            None => FrameObject::Owned(frame.clone()),
        };
        // The old one (if the coroutine lost it) is dropped after the borrow.
        let old = std::mem::replace(&mut *self.object.borrow_mut(), slot);
        drop(old);
        Ok(frame)
    }

    /// Whether `exception` was raised by module level (or class body) code
    /// kept as Python (`rt::exec_python!`) in this module or class body: its
    /// traceback starts with that code's `<module>` (or class body) frame,
    /// which stands for this frame already.
    fn raised_by_kept_code(&self, exception: &Bound<'py, PyAny>) -> bool {
        if !self.code.is_module() && self.class_namespace.is_none() {
            return false;
        }
        let head = || -> PyResult<bool> {
            let traceback = exception.getattr("__traceback__")?;
            if traceback.is_none() {
                return Ok(false);
            }
            let frame = traceback.getattr("tb_frame")?;
            let code = frame.getattr("f_code")?;
            let same_code = if self.code.is_module() {
                code.getattr("co_name")?.eq("<module>")?
            } else {
                // Class body code, which has no `CO_OPTIMIZED` flag.
                code.getattr("co_qualname")?.eq(self.code.qualname)?
                    && code.getattr("co_flags")?.extract::<u32>()? & 0x1 == 0
            };
            Ok(frame.getattr("f_globals")?.is(self.globals) && same_code)
        };
        head().unwrap_or(false)
    }
}

/// The rest of a `try` statement after its `finally` block: see
/// [`Frame::finally`].
pub struct Finally<'a, 'py, T> {
    frame: &'a Frame<'a, 'py>,
    outcome: Result<T, Bound<'py, PyAny>>,
    _handling: Option<Handling<'py>>,
}

impl<T> Finally<'_, '_, T> {
    /// Continues after the `finally` block: re-raises a pending exception,
    /// or returns what the `try` part did.
    pub fn resume(self) -> PyResult<T> {
        match self.outcome {
            Ok(value) => Ok(value),
            Err(exception) => Err(self.frame.reraise_exception(exception)),
        }
    }
}

/// What a block that runs in a closure (the body of a `try` or a `with`)
/// does next: continue after the statement, or leave it with `break`,
/// `continue` or `return`.
#[derive(Default)]
pub enum Flow<'py> {
    #[default]
    Next,
    Break,
    Continue,
    Return(Bound<'py, PyAny>),
}

/// [`Flow`] in a function that returns `()`, where `return` has no value.
#[derive(Default)]
pub enum Jump {
    #[default]
    Next,
    Break,
    Continue,
    Return,
}

/// The exception of an `except` block, from [`Frame::catch`]: used like the
/// `Bound` it dereferences to, and the exception being handled (see
/// [`Handling`]) until it is dropped at the end of the block.
pub struct Caught<'py> {
    // Dropped first: the previous exception is restored before this one is
    // released, like CPython's implicit `del e` after the block.
    _handling: Handling<'py>,
    exception: Bound<'py, PyAny>,
}

impl<'py> std::ops::Deref for Caught<'py> {
    type Target = Bound<'py, PyAny>;

    fn deref(&self) -> &Bound<'py, PyAny> {
        &self.exception
    }
}

impl AsRef<Py<PyAny>> for Caught<'_> {
    fn as_ref(&self) -> &Py<PyAny> {
        self.exception.as_ref()
    }
}

impl<'a, 'py> IntoPyObject<'py> for &'a Caught<'py> {
    type Target = PyAny;
    type Output = Borrowed<'a, 'py, PyAny>;
    type Error = std::convert::Infallible;

    fn into_pyobject(self, _py: Python<'py>) -> Result<Self::Output, Self::Error> {
        Ok(self.exception.as_borrowed())
    }
}

/// The exception being handled while an `except` (or `finally`) block runs:
/// `sys.exception()` returns it, and exceptions raised meanwhile get it as
/// `__context__`. Restores the previous one when dropped, on every way out
/// of the block (`?`, `return`, `break`, a panic).
///
/// In the body of a native coroutine or generator the block may span
/// suspensions: the previous exception it restores is the one handled when
/// it started, even if the caller handles another one by then (design §12).
pub struct Handling<'py> {
    py: Python<'py>,
    previous: Option<Bound<'py, PyAny>>,
}

impl<'py> Handling<'py> {
    pub fn new(exception: &Bound<'py, PyAny>) -> Self {
        let py = exception.py();
        // SAFETY: attached to the interpreter. `PyErr_GetHandledException`
        // returns a new reference or NULL; `PyErr_SetHandledException` takes
        // its own reference.
        unsafe {
            let previous = Bound::from_owned_ptr_or_opt(py, ffi::PyErr_GetHandledException());
            ffi::PyErr_SetHandledException(exception.as_ptr());
            Self { py, previous }
        }
    }
}

impl Drop for Handling<'_> {
    fn drop(&mut self) {
        let previous = self
            .previous
            .as_ref()
            .map_or(std::ptr::null_mut(), Bound::as_ptr);
        let _ = self.py;
        // SAFETY: attached (we hold a `Python` token); `previous` is NULL or
        // an object we hold, and the call takes its own reference.
        unsafe { ffi::PyErr_SetHandledException(previous) };
    }
}

/// [`Frame::matches`] for an evaluated handler: whether
/// `err` is an instance of the class (or of one in a tuple of classes),
/// raising `TypeError` like CPython if it is not an exception class.
fn matches<'py>(err: &PyErr, handler: &Bound<'py, PyAny>) -> PyResult<bool> {
    let py = handler.py();
    let classes = match handler.cast::<PyTuple>() {
        Ok(tuple) => tuple.iter().collect(),
        Err(_) => vec![handler.clone()],
    };
    for class in &classes {
        let valid = class.cast::<PyType>().is_ok_and(|class| {
            class
                .is_subclass_of::<pyo3::exceptions::PyBaseException>()
                .unwrap_or(false)
        });
        if !valid {
            return Err(crate::raised(PyTypeError::new_err(
                "catching classes that do not inherit from BaseException is not allowed",
            )));
        }
    }
    Ok(err.is_instance(py, handler))
}

/// Something `raise` accepts: an exception instance or class, or a `PyErr`
/// created in Rust (`PyValueError::new_err("..")`).
pub trait IntoException<'py> {
    /// The exception instance, created like `raise` does.
    fn into_exception(self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>>;
}

impl<'py> IntoException<'py> for PyErr {
    fn into_exception(self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        Ok(self.into_value(py).into_bound(py).into_any())
    }
}

impl<'py> IntoException<'py> for Bound<'py, PyAny> {
    fn into_exception(self, _py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        instantiate(&self, "exceptions must derive from BaseException")
    }
}

impl<'py> IntoException<'py> for &Bound<'py, PyAny> {
    fn into_exception(self, _py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        instantiate(self, "exceptions must derive from BaseException")
    }
}

impl<'py> IntoException<'py> for &Caught<'py> {
    fn into_exception(self, _py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        Ok(self.exception.clone())
    }
}

/// `raise X` with a class instantiates it, an instance is used as it is.
fn instantiate<'py>(
    value: &Bound<'py, PyAny>,
    message: &'static str,
) -> PyResult<Bound<'py, PyAny>> {
    if is_exception_class(value) {
        let instance = value.call0()?;
        if !is_exception(&instance) {
            return Err(crate::raised(PyTypeError::new_err(format!(
                "calling {} should have returned an instance of BaseException, not {}",
                value.repr()?,
                instance.get_type().repr()?
            ))));
        }
        Ok(instance)
    } else if is_exception(value) {
        Ok(value.clone())
    } else {
        Err(crate::raised(PyTypeError::new_err(message)))
    }
}

fn is_exception(value: &Bound<'_, PyAny>) -> bool {
    // SAFETY: valid object pointer.
    unsafe { ffi::PyExceptionInstance_Check(value.as_ptr()) != 0 }
}

fn is_exception_class(value: &Bound<'_, PyAny>) -> bool {
    // SAFETY: valid object pointer.
    unsafe { ffi::PyExceptionClass_Check(value.as_ptr()) != 0 }
}

/// `raise .. from cause`: sets `__cause__` (and `__suppress_context__`).
fn set_cause<'py>(exception: &Bound<'py, PyAny>, cause: Option<Bound<'py, PyAny>>) -> PyResult<()> {
    let Some(cause) = cause else { return Ok(()) };
    let cause = if cause.is_none() {
        None
    } else {
        Some(instantiate(
            &cause,
            "exception causes must derive from BaseException",
        )?)
    };
    // SAFETY: an exception instance; `PyException_SetCause` steals the
    // reference to the cause (NULL for `None`) and sets `__suppress_context__`.
    unsafe {
        ffi::PyException_SetCause(
            exception.as_ptr(),
            cause.map_or(std::ptr::null_mut(), Bound::into_ptr),
        );
    }
    Ok(())
}

/// Looks up a special method such as `__enter__` on the type of `object`,
/// bound to `object`, like CPython's `_PyObject_LookupSpecial`.
fn special_method<'py>(
    object: &Bound<'py, PyAny>,
    name: &'static str,
) -> PyResult<Bound<'py, PyAny>> {
    if let Some(method) = lookup_special(object, name)? {
        return Ok(method);
    }
    let mut message = format!(
        "'{}' object does not support the context manager protocol (missed {name} method)",
        object.get_type().name()?
    );
    if lookup_special(object, "__aenter__")?.is_some()
        && lookup_special(object, "__aexit__")?.is_some()
    {
        message.push_str(
            " but it supports the asynchronous context manager protocol. \
             Did you mean to use 'async with'?",
        );
    }
    Err(crate::raised(PyTypeError::new_err(message)))
}

/// [`special_method`] for `__aenter__` and `__aexit__`.
fn async_special_method<'py>(
    object: &Bound<'py, PyAny>,
    name: &'static str,
) -> PyResult<Bound<'py, PyAny>> {
    if let Some(method) = lookup_special(object, name)? {
        return Ok(method);
    }
    let mut message = format!(
        "'{}' object does not support the asynchronous context manager protocol \
         (missed {name} method)",
        object.get_type().name()?
    );
    if lookup_special(object, "__enter__")?.is_some()
        && lookup_special(object, "__exit__")?.is_some()
    {
        message
            .push_str(" but it supports the context manager protocol. Did you mean to use 'with'?");
    }
    Err(crate::raised(PyTypeError::new_err(message)))
}

pub(crate) fn lookup_special<'py>(
    object: &Bound<'py, PyAny>,
    name: &'static str,
) -> PyResult<Option<Bound<'py, PyAny>>> {
    let py = object.py();
    let name = crate::interned(py, name);
    let class = object.get_type();
    for base in class.mro() {
        let base = base.cast_into::<PyType>()?;
        // SAFETY: a type object; `PyType_GetDict` returns a new reference to
        // the type's own dict, not the read-only proxy.
        let dict = unsafe { Bound::from_owned_ptr(py, ffi::PyType_GetDict(base.as_ptr().cast())) };
        if let Some(attr) = dict.cast_into::<PyDict>()?.get_item(name)? {
            let get = attr
                .get_type()
                .getattr_opt(crate::interned(py, "__get__"))?;
            return match get {
                Some(get) => get.call1((&attr, object, &class)).map(Some),
                None => Ok(Some(attr)),
            };
        }
    }
    Ok(None)
}

fn traceback_type(py: Python<'_>) -> PyResult<&Bound<'_, PyAny>> {
    static TRACEBACK: PyOnceLock<Py<PyAny>> = PyOnceLock::new();
    TRACEBACK
        .get_or_try_init(py, || {
            Ok(py.import("types")?.getattr("TracebackType")?.unbind())
        })
        .map(|tb| tb.bind(py))
}

/// A `def`, `lambda` or generator expression in a function body is defined
/// in the function's frame: `rt::function!(frame, g(x))` names it
/// `f.<locals>.g` after the function `f` of the frame.
impl<'py> crate::function::Definer<'py> for Frame<'_, 'py> {
    fn module(&self) -> &Module<'py> {
        self.module
    }
    fn classdict(&self) -> Option<Bound<'py, PyAny>> {
        None
    }
    fn class_qualname(&self) -> Option<&'static str> {
        None
    }
    fn enclosing_function(&self) -> Option<&'static str> {
        self.function_qualname()
    }
    fn lookup(&self, name: &'static str) -> PyResult<Bound<'py, PyAny>> {
        self.module.global(name)
    }
    fn bind(&self, name: &'static str, value: Bound<'py, PyAny>) -> PyResult<()> {
        self.module.set_global(name, value)
    }
}
