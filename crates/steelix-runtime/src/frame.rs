//! Traceback frames and exception handling for generated code (design §9).
//!
//! Native functions have no Python frame while they run. Like Cython, they
//! add one to the traceback of an exception on its way out instead: every
//! generated function body runs inside [`frame!`](crate::frame), which knows
//! the Python file, the function's qualified name and its `def` line, and the
//! body keeps the line of the statement it is running up to date with
//! [`Frame::line`]. Tracebacks of uncaught exceptions and
//! `traceback.extract_tb` then look like CPython's.
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
    object: PyOnceLock<Py<PyAny>>,
}

impl Code {
    /// `qualname` is the function's `__qualname__`, or `<module>` for module
    /// level code.
    pub const fn new(qualname: &'static str, first_line: u32) -> Self {
        Self {
            qualname,
            first_line,
            object: PyOnceLock::new(),
        }
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
/// body. Created by [`frame!`](crate::frame).
pub struct Frame<'py> {
    globals: Bound<'py, PyDict>,
    /// The namespace of a class body, which runs in its own frame.
    class_namespace: Option<Bound<'py, PyAny>>,
    code: &'static Code,
    line: Cell<u32>,
    /// The frame object, created when the first exception needs it and
    /// shared by all entries of this call, like a Python frame.
    object: RefCell<Option<Bound<'py, PyAny>>>,
    /// An exception that already has its entry for this frame: raised here
    /// with [`Frame::raise`], or re-raised. It gets no second entry when it
    /// reaches a handler or leaves the function.
    entered: RefCell<Option<Bound<'py, PyAny>>>,
}

/// What code runs in: a module (for the module body and functions) or the
/// namespace of a class body.
#[doc(hidden)]
pub trait Scope<'py> {
    fn globals(&self) -> &Bound<'py, PyDict>;
    fn class_namespace(&self) -> Option<&Bound<'py, PyAny>> {
        None
    }
}

impl<'py> Scope<'py> for Module<'py> {
    fn globals(&self) -> &Bound<'py, PyDict> {
        Module::globals(self)
    }
}

impl<'py> Scope<'py> for crate::Namespace<'py> {
    fn globals(&self) -> &Bound<'py, PyDict> {
        self.module().globals()
    }
    fn class_namespace(&self) -> Option<&Bound<'py, PyAny>> {
        Some(self.mapping())
    }
}

/// Runs the body of a native function (or module, or class) in a traceback
/// frame. Use it through [`frame!`](crate::frame).
pub fn run<'py, T>(
    scope: &impl Scope<'py>,
    code: &'static Code,
    body: impl FnOnce(&Frame<'py>) -> PyResult<T>,
) -> PyResult<T> {
    let frame = Frame {
        globals: scope.globals().clone(),
        class_namespace: scope.class_namespace().cloned(),
        code,
        line: Cell::new(code.first_line),
        object: RefCell::new(None),
        entered: RefCell::new(None),
    };
    body(&frame).map_err(|err| PyErr::from_value(frame.catch(err)))
}

impl<'py> Frame<'py> {
    pub fn py(&self) -> Python<'py> {
        self.globals.py()
    }

    /// Records that the function is now running the statement on `line`
    /// of the Python file.
    pub fn line(&self, line: u32) {
        self.line.set(line);
    }

    /// An exception that reached a handler (`except`, `finally`, the end of
    /// a `with` block) or the end of the function: its traceback gets this
    /// frame at the current line, unless it already has it.
    pub fn catch(&self, err: PyErr) -> Bound<'py, PyAny> {
        let py = self.py();
        let exception = err.into_value(py).into_bound(py).into_any();
        self.enter(&exception);
        self.entered.borrow_mut().take();
        exception
    }

    /// Gives `exception` this frame's traceback entry at the current line,
    /// unless it already has it, and remembers that it has it.
    fn enter(&self, exception: &Bound<'py, PyAny>) {
        let entered = self.entered.borrow_mut().take();
        let here = entered.is_some_and(|entered| entered.is(exception))
            || self.raised_by_kept_code(exception);
        if !here {
            // Adding the entry can only fail when Python itself is in
            // trouble (no memory, no stack left); the traceback then just
            // lacks this frame.
            let _ = self.add_entry(exception);
        }
        *self.entered.borrow_mut() = Some(exception.clone());
    }

    /// `except handler:` on `line`, where `handler` is not a builtin
    /// exception class: evaluates `handler` and tells whether `err` is an
    /// instance of it (or of a class in a tuple of classes).
    ///
    /// Like CPython, the handler expression runs with `err` as the exception
    /// being handled, on the line of the `except`: an exception it raises,
    /// or the `TypeError` for a handler that is not an exception class, has
    /// `err` as its `__context__` and is reported on that line.
    ///
    /// ```ignore
    /// Err(err) if frame.matches(&err, 7, || m.global("ParseError"))? => { .. }
    /// ```
    pub fn matches(
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
        let _ = self.add_entry(&exception);
        *self.entered.borrow_mut() = Some(exception.clone());
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
        *self.entered.borrow_mut() = Some(exception.clone());
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
                let exception = self.catch(err);
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
    /// `__exit__` runs on the line of the `with` statement, the current line
    /// when this is called. The body returns what the statement does next
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
                let exception = self.catch(err);
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

    fn add_entry(&self, exception: &Bound<'py, PyAny>) -> PyResult<()> {
        let py = self.py();
        let frame = self.object()?;
        let next = exception.getattr("__traceback__")?;
        let entry = traceback_type(py)?.call1((next, frame, -1, self.line.get()))?;
        exception.setattr("__traceback__", entry)
    }

    fn object(&self) -> PyResult<Bound<'py, PyAny>> {
        if let Some(frame) = self.object.borrow().as_ref() {
            return Ok(frame.clone());
        }
        let py = self.py();
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
        *self.object.borrow_mut() = Some(frame.clone());
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
            Ok(frame.getattr("f_globals")?.is(&self.globals) && same_code)
        };
        head().unwrap_or(false)
    }
}

/// The rest of a `try` statement after its `finally` block: see
/// [`Frame::finally`].
pub struct Finally<'a, 'py, T> {
    frame: &'a Frame<'py>,
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

/// The exception being handled while an `except` (or `finally`) block runs:
/// `sys.exception()` returns it, and exceptions raised meanwhile get it as
/// `__context__`. Restores the previous one when dropped, on every way out
/// of the block (`?`, `return`, `break`, a panic).
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
        // SAFETY: attached (we hold `Bound`s); NULL clears it.
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
fn special_method<'py>(object: &Bound<'py, PyAny>, name: &str) -> PyResult<Bound<'py, PyAny>> {
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

fn lookup_special<'py>(
    object: &Bound<'py, PyAny>,
    name: &str,
) -> PyResult<Option<Bound<'py, PyAny>>> {
    let class = object.get_type();
    for base in class.mro() {
        let base = base.cast_into::<PyType>()?;
        // SAFETY: a type object; `PyType_GetDict` returns a new reference to
        // the type's own dict, not the read-only proxy.
        let dict = unsafe {
            Bound::from_owned_ptr(object.py(), ffi::PyType_GetDict(base.as_ptr().cast()))
        };
        if let Some(attr) = dict.cast_into::<PyDict>()?.get_item(name)? {
            let get = attr.get_type().getattr_opt("__get__")?;
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

/// Runs the body of a generated function in a traceback [`Frame`]:
///
/// ```ignore
/// pub fn parse<'py>(m: &rt::Module<'py>, text: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
///     rt::frame!(m, "parse", 10, |frame| {
///         frame.line(11);
///         let value = text.call_method0("strip")?;
///         ..
///     })
/// }
/// ```
///
/// The arguments are the module, the function's `__qualname__` (`<module>`
/// for the module body) and the line of its `def`. A class body passes its
/// namespace instead of the module, its `__qualname__` and the line of its
/// first decorator (or `class`).
#[macro_export]
macro_rules! frame {
    ($module:expr, $qualname:literal, $line:literal, $body:expr $(,)?) => {{
        static CODE: $crate::frame::Code = $crate::frame::Code::new($qualname, $line);
        $crate::frame::run($module, &CODE, $body)
    }};
}
