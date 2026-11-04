//! What happens when a coroutine or generator object is deallocated or
//! collected: a suspended body is closed, a coroutine that never ran warns
//! (CPython's `_PyGen_Finalize`).

use std::ffi::c_char;
use std::sync::{Once, OnceLock};

use pyo3::exceptions::PyRuntimeWarning;
use pyo3::prelude::*;
use pyo3::{PyClass, ffi};

use super::objects::{Coroutine, Generator};
use super::state::{Core, Flavor, State};

impl Core {
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
    pub(super) fn PyErr_FormatUnraisable(format: *const c_char, ...);
}

/// Reports `err` like CPython reports an exception in a finalizer: through
/// `sys.unraisablehook`, with `message` formatted with the object.
pub(super) fn write_unraisable(
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

/// The classes whose objects close their body when they are deallocated.
pub(super) trait Finalized:
    PyClass<Frozen = pyo3::pyclass::boolean_struct::True> + Sync
{
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
pub(super) static COROUTINE_DEALLOC: OnceLock<ffi::destructor> = OnceLock::new();
pub(super) static GENERATOR_DEALLOC: OnceLock<ffi::destructor> = OnceLock::new();

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
pub(super) fn install_finalizer<T: Finalized>(
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
