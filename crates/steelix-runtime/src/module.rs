//! `Module`: the context every generated function receives.

use pyo3::exceptions::{PyNameError, PyTypeError};
use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::Function;

/// The Python module a transpiled function belongs to.
///
/// Every generated function takes `m: &rt::Module<'py>` as its first
/// parameter, and `exec_module` receives the module it executes in. It gives
/// access to the interpreter (`m.py()`) and to the module's globals, which is
/// where Python looks up names that are not local:
///
/// ```ignore
/// let py = m.py();
/// let limit = m.global("LIMIT")?;     // LOAD_GLOBAL
/// m.set_global("LIMIT", 20)?;         // STORE_GLOBAL
/// ```
///
/// A function object keeps the globals of the module that defined it, so a
/// call through Python passes the same module as a direct call from Rust.
#[derive(Clone)]
pub struct Module<'py> {
    globals: Bound<'py, PyDict>,
}

impl<'py> Module<'py> {
    /// The context for running code in `module`.
    pub fn new(module: &Bound<'py, PyModule>) -> Self {
        Self::from_globals(module.dict())
    }

    /// The context for code whose globals are `globals`.
    pub fn from_globals(globals: Bound<'py, PyDict>) -> Self {
        Self { globals }
    }

    pub fn py(&self) -> Python<'py> {
        self.globals.py()
    }

    /// The module's namespace, what `globals()` returns.
    pub fn globals(&self) -> &Bound<'py, PyDict> {
        &self.globals
    }

    /// Reads a global the way `LOAD_GLOBAL` does: the module's globals first,
    /// then builtins, raising `NameError` if neither has it.
    ///
    /// The name is a literal, which lets the lookup reuse an interned Python
    /// string. Look names computed at run time up in [`Module::globals`].
    pub fn global(&self, name: &'static str) -> PyResult<Bound<'py, PyAny>> {
        if let Some(value) = self.globals.get_item(crate::interned(self.py(), name))? {
            return Ok(value);
        }
        crate::builtin(self.py(), name)
    }

    /// Binds a global, like an assignment at module level.
    pub fn set_global(&self, name: &str, value: impl IntoPyObject<'py>) -> PyResult<()> {
        self.globals.set_item(name, value)
    }

    /// Deletes a global like `del name` does, raising `NameError` if it is
    /// not bound.
    pub fn del_global(&self, name: &str) -> PyResult<()> {
        if self.globals.contains(name)? {
            self.globals.del_item(name)
        } else {
            Err(crate::raised(PyNameError::new_err(format!(
                "name '{name}' is not defined"
            ))))
        }
    }

    /// Unbinds a global if it is bound: the implicit `del name` at the end
    /// of `except .. as name` at module level, which CPython compiles as
    /// `name = None; del name`.
    pub fn unbind_global(&self, name: &str) -> PyResult<()> {
        if self.globals.contains(name)? {
            self.globals.del_item(name)?;
        }
        Ok(())
    }

    /// The value a direct call of the module function `function` passes for
    /// the omitted parameter `param`.
    ///
    /// Like a call through Python, it reads `__defaults__` / `__kwdefaults__`
    /// of the function object at call time, so assigning to them is honoured.
    ///
    /// A direct call is bound to the `def` at transpile time. If the global
    /// now holds another object (a monkeypatch from another module), there is
    /// no default to read and this raises `TypeError` (design §12).
    pub fn default(&self, function: &str, param: &str) -> PyResult<Bound<'py, PyAny>> {
        let Some(object) = self.globals.get_item(function)? else {
            return Err(crate::raised(PyNameError::new_err(format!(
                "name '{function}' is not defined"
            ))));
        };
        match object.cast::<Function>() {
            Ok(found) if found.get().is_def(function, &self.globals) => {
                found.get().default(self.py(), param)
            }
            _ => Err(crate::raised(PyTypeError::new_err(format!(
                "{function}() was replaced by {}, but transpiled code calls the original \
                 directly and can not take the default of its parameter '{param}' from the \
                 replacement",
                object.repr()?
            )))),
        }
    }

    /// `dir()` at module level: the sorted names of the module's globals.
    pub fn dir(&self) -> PyResult<Bound<'py, PyAny>> {
        let names = self.globals.keys();
        names.sort()?;
        Ok(names.into_any())
    }
}
