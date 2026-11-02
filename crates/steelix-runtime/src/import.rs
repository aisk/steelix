//! `import` statements, with the semantics of the `IMPORT_NAME` and
//! `IMPORT_FROM` bytecodes.
//!
//! The import goes through `builtins.__import__` with the module's globals,
//! like the interpreter does, so import hooks that replace `__import__` or
//! look at the importing module keep working.

use pyo3::exceptions::{PyImportError, PyKeyError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyString, PyTuple};

/// `import a.b.c`: imports the module and returns the top-level package `a`,
/// which is the name the statement binds.
///
/// ```ignore
/// globals.set_item("os", rt::import(globals, "os.path")?)?;
/// ```
pub fn import<'py>(globals: &Bound<'py, PyDict>, name: &str) -> PyResult<Bound<'py, PyAny>> {
    import_name(globals, name, None)
}

/// `import a.b.c as d`: imports the module and returns `a.b.c` itself.
pub fn import_module<'py>(globals: &Bound<'py, PyDict>, name: &str) -> PyResult<Bound<'py, PyAny>> {
    let mut module = import_name(globals, name, None)?;
    for part in name.split('.').skip(1) {
        module = import_from(&module, part)?;
    }
    Ok(module)
}

/// The first half of `from module import a, b`: imports `module` (and any
/// submodules named `a` or `b`) and returns it. Each name is then read with
/// [`import_from`].
///
/// ```ignore
/// let path = rt::import_names(globals, "os.path", &["join", "sep"])?;
/// globals.set_item("join", rt::import_from(&path, "join")?)?;
/// ```
pub fn import_names<'py>(
    globals: &Bound<'py, PyDict>,
    module: &str,
    names: &[&str],
) -> PyResult<Bound<'py, PyAny>> {
    let fromlist = PyTuple::new(globals.py(), names)?;
    import_name(globals, module, Some(fromlist))
}

fn import_name<'py>(
    globals: &Bound<'py, PyDict>,
    name: &str,
    fromlist: Option<Bound<'py, PyTuple>>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = globals.py();
    let import = crate::builtin(py, "__import__")
        .map_err(|_| PyImportError::new_err("__import__ not found"))?;
    import.call1((name, globals, py.None(), fromlist, 0))
}

/// `IMPORT_FROM`: reads `name` from an imported module, falling back to
/// `sys.modules` for submodules that are still being imported (circular
/// imports), and raising `ImportError` like CPython otherwise.
pub fn import_from<'py>(module: &Bound<'py, PyAny>, name: &str) -> PyResult<Bound<'py, PyAny>> {
    if let Some(value) = module.getattr_opt(name)? {
        return Ok(value);
    }
    let py = module.py();
    let package = module
        .getattr_opt("__name__")?
        .filter(|package| package.is_instance_of::<PyString>());
    if let Some(package) = &package {
        let full = format!("{package}.{name}");
        match py.import("sys")?.getattr("modules")?.get_item(&full) {
            Ok(value) => return Ok(value),
            Err(err) if !err.is_instance_of::<PyKeyError>(py) => return Err(err),
            Err(_) => {}
        }
    }
    let package_repr = match &package {
        Some(package) => package.repr()?.to_string(),
        None => "'<unknown module name>'".to_owned(),
    };
    let path = if module.is_instance_of::<PyModule>() {
        module
            .cast::<PyModule>()?
            .dict()
            .get_item("__file__")?
            .filter(|path| path.is_instance_of::<PyString>())
    } else {
        None
    };
    let location = match &path {
        Some(path) => path.to_string(),
        None => "unknown location".to_owned(),
    };
    let name_repr = PyString::new(py, name).repr()?;
    let err = PyImportError::new_err(format!(
        "cannot import name {name_repr} from {package_repr} ({location})"
    ));
    let value = err.value(py);
    value.setattr("name", package)?;
    value.setattr("path", path)?;
    value.setattr("name_from", name)?;
    Err(err)
}
