//! `import` statements, with the semantics of the `IMPORT_NAME` and
//! `IMPORT_FROM` bytecodes.
//!
//! The import goes through `builtins.__import__` with the module's globals,
//! like the interpreter does, so import hooks that replace `__import__` or
//! look at the importing module keep working.

use pyo3::exceptions::{PyImportError, PyKeyError};
use pyo3::prelude::*;
use pyo3::types::{PyString, PyTuple};

use crate::Module;

/// `import a.b.c`: imports the module and returns the top-level package `a`,
/// which is the name the statement binds.
///
/// ```ignore
/// module.set_global("os", rt::import(module, "os.path")?)?;
/// ```
pub fn import<'py>(module: &Module<'py>, name: &str) -> PyResult<Bound<'py, PyAny>> {
    import_name(module, name, None)
}

/// `import a.b.c as d`: imports the module and returns `a.b.c` itself.
pub fn import_module<'py>(module: &Module<'py>, name: &str) -> PyResult<Bound<'py, PyAny>> {
    let mut imported = import_name(module, name, None)?;
    for part in name.split('.').skip(1) {
        imported = import_from(&imported, part)?;
    }
    Ok(imported)
}

/// The first half of `from module import a, b`: imports `module` (and any
/// submodules named `a` or `b`) and returns it. Each name is then read with
/// [`import_from`].
///
/// A relative import is written as in Python, with its leading dots:
/// `from . import a` is `rt::import_names(m, ".", &["a"])` and
/// `from ..util import b` is `rt::import_names(m, "..util", &["b"])`. The
/// package they are relative to comes from the module's `__package__` /
/// `__spec__`, like for the `level` argument of `__import__`.
///
/// ```ignore
/// let path = rt::import_names(module, "os.path", &["join", "sep"])?;
/// module.set_global("join", rt::import_from(&path, "join")?)?;
/// ```
pub fn import_names<'py>(
    module: &Module<'py>,
    name: &str,
    names: &[&str],
) -> PyResult<Bound<'py, PyAny>> {
    let fromlist = PyTuple::new(module.py(), names)?;
    import_name(module, name, Some(fromlist))
}

fn import_name<'py>(
    module: &Module<'py>,
    name: &str,
    fromlist: Option<Bound<'py, PyTuple>>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = module.py();
    let import = crate::builtin(py, "__import__")
        .map_err(|_| PyImportError::new_err("__import__ not found"))?;
    let relative = name.trim_start_matches('.');
    let level = name.len() - relative.len();
    import.call1((relative, module.globals(), py.None(), fromlist, level))
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
    Err(crate::raised(import_from_error(module, name, package)?))
}

/// The `ImportError` of a failed `from module import name`, with the message
/// CPython's `_PyEval_ImportFrom` picks: it mentions a circular import when
/// the module is still being initialized, and suggests renaming a module of
/// the script's directory that shadows a standard library module (or, while
/// it is being initialized, any other module).
fn import_from_error<'py>(
    module: &Bound<'py, PyAny>,
    name: &str,
    package: Option<Bound<'py, PyAny>>,
) -> PyResult<PyErr> {
    let py = module.py();
    let package_repr = match &package {
        Some(package) => package.repr()?.to_string(),
        None => "'<unknown module name>'".to_owned(),
    };
    let name_repr = PyString::new(py, name).repr()?;
    let spec = module.getattr_opt("__spec__")?;
    let mut origin: Option<Bound<'py, PyAny>> = None;
    let message = match &spec {
        None => format!("cannot import name {name_repr} from {package_repr} (unknown location)"),
        Some(spec) => {
            origin = file_origin(spec)?;
            let shadowing = match &origin {
                Some(origin) => possibly_shadowing(py, &origin.extract::<String>()?)?,
                None => false,
            };
            let shadowing_stdlib = shadowing
                && match &package {
                    Some(package) => py
                        .import("sys")?
                        .getattr("stdlib_module_names")?
                        .contains(package)?,
                    None => false,
                };
            if origin.is_none() && module.is_instance_of::<PyModule>() {
                origin = module
                    .cast::<PyModule>()?
                    .dict()
                    .get_item("__file__")?
                    .filter(|path| path.is_instance_of::<PyString>());
            }
            let initializing = !spec.is_none()
                && spec
                    .getattr_opt("_initializing")?
                    .map_or(Ok(false), |value| value.is_truthy())?;
            match &origin {
                Some(origin) if shadowing_stdlib => format!(
                    "cannot import name {name_repr} from {package_repr} (consider renaming \
                     {} since it has the same name as the standard library module named \
                     {package_repr} and prevents importing that standard library module)",
                    origin.repr()?
                ),
                Some(origin) if initializing && shadowing => format!(
                    "cannot import name {name_repr} from {package_repr} (consider renaming \
                     {} if it has the same name as a library you intended to import)",
                    origin.repr()?
                ),
                Some(origin) if initializing => format!(
                    "cannot import name {name_repr} from partially initialized module \
                     {package_repr} (most likely due to a circular import) ({origin})"
                ),
                None if initializing => format!(
                    "cannot import name {name_repr} from partially initialized module \
                     {package_repr} (most likely due to a circular import)"
                ),
                Some(origin) => {
                    format!("cannot import name {name_repr} from {package_repr} ({origin})")
                }
                None => {
                    format!("cannot import name {name_repr} from {package_repr} (unknown location)")
                }
            }
        }
    };
    let err = PyImportError::new_err(message);
    let value = err.value(py);
    value.setattr("name", package)?;
    value.setattr("path", origin)?;
    value.setattr("name_from", name)?;
    Ok(err)
}

/// `spec.origin` if the spec has a file location (`_PyModuleSpec_GetFileOrigin`).
fn file_origin<'py>(spec: &Bound<'py, PyAny>) -> PyResult<Option<Bound<'py, PyAny>>> {
    if spec.is_none() {
        return Ok(None);
    }
    let has_location = match spec.getattr_opt("has_location")? {
        Some(value) => value.is_truthy()?,
        None => false,
    };
    if !has_location {
        return Ok(None);
    }
    Ok(spec
        .getattr_opt("origin")?
        .filter(|origin| origin.is_instance_of::<PyString>()))
}

/// Whether the module at `origin` may shadow another module of the same name
/// later on the search path (`_PyModule_IsPossiblyShadowing`): it is in
/// `sys.path[0]`, and `sys.flags.safe_path` is off.
fn possibly_shadowing(py: Python<'_>, origin: &str) -> PyResult<bool> {
    let sys = py.import("sys")?;
    if sys.getattr("flags")?.getattr("safe_path")?.is_truthy()? {
        return Ok(false);
    }
    let file = origin.strip_suffix("/__init__.py").unwrap_or(origin);
    let Some((root, _)) = file.rsplit_once('/') else {
        return Ok(false);
    };
    let path = sys.getattr("path")?;
    let Ok(path) = path.cast::<pyo3::types::PyList>() else {
        return Ok(false);
    };
    let Some(first) = path.iter().next() else {
        return Ok(false);
    };
    let Ok(first) = first.extract::<String>() else {
        return Ok(false);
    };
    let first = if first.is_empty() {
        std::env::current_dir()?.to_string_lossy().into_owned()
    } else {
        first
    };
    Ok(first == root)
}
