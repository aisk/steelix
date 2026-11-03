//! Variables shared between a function and the functions nested in it.

use pyo3::exceptions::{PyNameError, PyUnboundLocalError};
use pyo3::ffi;
use pyo3::prelude::*;
use pyo3::{IntoPyObject, IntoPyObjectExt};

/// A variable that nested functions capture and may rebind (`nonlocal`).
///
/// It wraps a Python cell object, the same object CPython uses for closures,
/// so `__closure__` sees it and every function sharing the variable sees
/// every assignment. The function that owns the variable creates the cell;
/// nested functions receive it as a `&rt::Cell` parameter.
///
/// ```ignore
/// let count = rt::Cell::new(py, "count", 0)?;
/// count.set(rt::iadd(&count.get()?, 1)?)?;
/// ```
pub struct Cell<'py> {
    cell: Bound<'py, PyAny>,
    name: &'static str,
    /// Captured from an enclosing function rather than owned, which only
    /// changes the error message when it is read while unassigned.
    free: bool,
}

impl<'py> Cell<'py> {
    /// A variable of the current function holding `value`.
    pub fn new(
        py: Python<'py>,
        name: &'static str,
        value: impl IntoPyObject<'py>,
    ) -> PyResult<Self> {
        let value = value.into_bound_py_any(py)?;
        let cell = crate::function::capture(&value)?;
        Ok(Self {
            cell,
            name,
            free: false,
        })
    }

    /// A variable of the current function that is not assigned yet.
    pub fn empty(py: Python<'py>, name: &'static str) -> PyResult<Self> {
        // SAFETY: `PyCell_New` accepts NULL and returns a new reference.
        let cell =
            unsafe { Bound::from_owned_ptr_or_err(py, ffi::PyCell_New(std::ptr::null_mut()))? };
        Ok(Self {
            cell,
            name,
            free: false,
        })
    }

    /// A variable captured from an enclosing function.
    #[doc(hidden)]
    pub fn free(cell: &Bound<'py, PyAny>, name: &'static str) -> Self {
        Self {
            cell: cell.clone(),
            name,
            free: true,
        }
    }

    /// The cell object itself, for the closure of a nested function.
    #[doc(hidden)]
    pub fn object(&self) -> Bound<'py, PyAny> {
        self.cell.clone()
    }

    /// The cell object, borrowed (for the GC's `__traverse__`).
    pub(crate) fn as_object(&self) -> &Bound<'py, PyAny> {
        &self.cell
    }

    /// Reads the variable, raising `UnboundLocalError` (or `NameError` for a
    /// captured variable) like CPython if it is not assigned.
    pub fn get(&self) -> PyResult<Bound<'py, PyAny>> {
        let py = self.cell.py();
        // SAFETY: `cell` is a cell object; `PyCell_Get` returns a new
        // reference, or NULL without an exception when the cell is empty.
        let value =
            unsafe { Bound::from_owned_ptr_or_opt(py, ffi::PyCell_Get(self.cell.as_ptr())) };
        value.ok_or_else(|| self.unbound())
    }

    /// Assigns the variable.
    pub fn set(&self, value: impl IntoPyObject<'py>) -> PyResult<()> {
        let value = value.into_bound_py_any(self.cell.py())?;
        self.store(value.as_ptr())
    }

    /// `del x`: unassigns the variable, which must be assigned.
    pub fn delete(&self) -> PyResult<()> {
        self.get()?;
        self.store(std::ptr::null_mut())
    }

    fn store(&self, value: *mut ffi::PyObject) -> PyResult<()> {
        // SAFETY: `cell` is a cell object and `value` a valid pointer or NULL;
        // `PyCell_Set` takes its own reference.
        if unsafe { ffi::PyCell_Set(self.cell.as_ptr(), value) } < 0 {
            return Err(PyErr::fetch(self.cell.py()));
        }
        Ok(())
    }

    fn unbound(&self) -> PyErr {
        let name = crate::function::python_name(self.name);
        if self.free {
            crate::raised(PyNameError::new_err(format!(
                "cannot access free variable '{name}' where it is not associated with a value in enclosing scope"
            )))
        } else {
            crate::raised(PyUnboundLocalError::new_err(format!(
                "cannot access local variable '{name}' where it is not associated with a value"
            )))
        }
    }
}
