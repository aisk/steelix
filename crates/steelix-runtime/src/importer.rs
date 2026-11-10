//! Importing the transpiled modules of a project (design §7).
//!
//! A program transpiled from several Python modules lists them in a table of
//! [`NativeModule`]s. [`install_importer`] puts a [`NativeImporter`] for that
//! table on `sys.meta_path`, so that `import pkg.sub` runs the generated
//! `exec_module` of `pkg/sub.py` instead of its Python source: lazily, at the
//! first import, with the module object, `sys.modules` entry and attributes
//! (`__name__`, `__file__`, `__spec__`, `__path__`, ..) that importing the
//! source would have created.

use std::path::{Component, MAIN_SEPARATOR, MAIN_SEPARATOR_STR, Path, PathBuf};

use pyo3::exceptions::PyImportError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyString};

use crate::Module;

/// The `exec_module` function generated for a Python module.
pub type ExecModule = fn(&Module<'_>) -> PyResult<()>;

/// A Python module of the project and the generated code that runs it.
///
/// Its source file follows from the name, the way the path finder finds it
/// in the project directory: `pkg/sub.py` for `pkg.sub`, `pkg/__init__.py`
/// for the package `pkg`.
#[derive(Clone, Copy, Debug)]
pub struct NativeModule {
    /// The full module name, `pkg.sub`.
    pub name: &'static str,
    /// Whether the module is a package (it has `__path__`).
    pub package: bool,
    pub exec: ExecModule,
}

impl NativeModule {
    /// A plain module: `NativeModule::module("pkg.sub", pkg::sub::exec_module)`.
    pub const fn module(name: &'static str, exec: ExecModule) -> Self {
        Self {
            name,
            package: false,
            exec,
        }
    }

    /// A package: `NativeModule::package("pkg", pkg::exec_module)`.
    pub const fn package(name: &'static str, exec: ExecModule) -> Self {
        Self {
            name,
            package: true,
            exec,
        }
    }

    /// The source file relative to the project directory, with the
    /// platform's separator, as the path finder spells it.
    pub fn path(&self) -> String {
        let path = self.name.replace('.', MAIN_SEPARATOR_STR);
        if self.package {
            path + MAIN_SEPARATOR_STR + "__init__.py"
        } else {
            path + ".py"
        }
    }
}

/// The finder of the project's transpiled modules, on `sys.meta_path`.
///
/// Modules it does not know (the standard library, third-party packages, and
/// project modules that were not transpiled) are left to the finders after
/// it, so they import from their Python source as before.
#[pyclass(frozen, name = "NativeImporter", module = "steelix_runtime")]
pub struct NativeImporter {
    /// The project directory: the module file names are relative to it.
    root: String,
    modules: &'static [NativeModule],
    /// Which imports the importer answers.
    scope: Scope,
}

/// Which imports a [`NativeImporter`] answers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Scope {
    /// Every import of a module in its table: the program, whose project
    /// directory is `sys.path[0]` (design §7).
    Always,
    /// Only those where the path based finder would find the module's
    /// source: the project directory is on `sys.path` (or, for a submodule,
    /// the package's directory is in the parent's `__path__`). This is the
    /// bundled interpreter (`python/bin/python`), which runs arbitrary
    /// commands and so sees the project only where CPython would.
    OnPath,
}

impl NativeImporter {
    fn get(&self, fullname: &str) -> Option<&'static NativeModule> {
        self.modules.iter().find(|module| module.name == fullname)
    }

    /// Whether `fullname` is one of the transpiled modules.
    pub(crate) fn contains(&self, fullname: &str) -> bool {
        self.get(fullname).is_some()
    }

    /// The absolute path of a module's source file, its `__file__`.
    fn location(&self, module: &NativeModule) -> String {
        format!("{}{MAIN_SEPARATOR}{}", self.root, module.path())
    }

    /// Whether [`find_spec`](Self::find_spec) answers for `module` only
    /// where the path based finder would find its source ([`Scope`]). In
    /// the program that is also the case for a module of a namespace
    /// package (a directory without `__init__.py`, not in the table): the
    /// namespace package exists only if no other `sys.path` entry has a
    /// regular package or module of that name, so its `__path__` tells.
    fn checks_path(&self, module: &NativeModule) -> bool {
        match self.scope {
            Scope::OnPath => true,
            Scope::Always => module
                .name
                .rsplit_once('.')
                .is_some_and(|(package, _)| !self.contains(package)),
        }
    }

    /// Whether the directory the path based finder would find `module` in
    /// is one of `locations`: the `path` that importlib passes to
    /// `find_spec`, which is `sys.path` for a top-level module (`None`) and
    /// the parent package's `__path__` for a submodule (also looked up when
    /// it is `None`). An empty entry is the current directory.
    ///
    /// The paths are compared with [`resolve`], which does not require them
    /// to exist: without the sources (`steelix build --no-sources`) a
    /// package's directory may not be there.
    fn on_path(
        &self,
        py: Python<'_>,
        module: &NativeModule,
        locations: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<bool> {
        let package = module.name.rsplit_once('.').map(|(package, _)| package);
        let mut parent = resolve(Path::new(&self.root));
        if let Some(package) = package {
            parent.extend(package.split('.'));
        }
        let looked_up;
        let locations = match (locations, package) {
            (Some(locations), _) => locations,
            (None, None) => {
                looked_up = py.import("sys")?.getattr("path")?;
                &looked_up
            }
            (None, Some(package)) => {
                let modules = py.import("sys")?.getattr("modules")?;
                let Some(parent) = modules.cast::<PyDict>()?.get_item(package)? else {
                    return Ok(false);
                };
                let Ok(path) = parent.getattr("__path__") else {
                    return Ok(false);
                };
                looked_up = path;
                &looked_up
            }
        };
        for entry in locations.try_iter()? {
            let entry = entry?;
            let Ok(entry) = entry.extract::<&str>() else {
                continue;
            };
            let entry = if entry.is_empty() { "." } else { entry };
            if resolve(Path::new(entry)) == parent {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

/// `path` made absolute, with links resolved as far as it exists and the
/// rest normalized lexically (`.` dropped, `..` removes the previous
/// component), so that paths that do not exist (yet) can be compared.
pub(crate) fn resolve(path: &Path) -> PathBuf {
    let absolute = std::path::absolute(path).unwrap_or_else(|_| path.to_owned());
    let mut lexical = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                lexical.pop();
            }
            other => lexical.push(other),
        }
    }
    // The longest existing prefix, with its links resolved, then the rest.
    let mut existing = lexical.as_path();
    let mut rest = Vec::new();
    loop {
        if let Ok(resolved) = std::fs::canonicalize(existing) {
            return rest
                .iter()
                .rev()
                .fold(resolved, |path, name| path.join(name));
        }
        match (existing.parent(), existing.file_name()) {
            (Some(parent), Some(name)) => {
                rest.push(name.to_owned());
                existing = parent;
            }
            _ => return lexical,
        }
    }
}

/// `path` split at its last separator (`/`, also `\` on Windows) into
/// the directory and the file name.
fn split_file_name(path: &str) -> Option<(&str, &str)> {
    let index = path.rfind(std::path::is_separator)?;
    Some((&path[..index], &path[index + 1..]))
}

/// The directory part of `path`, or `default` if it has none.
fn directory<'a>(path: &'a str, default: &'a str) -> &'a str {
    split_file_name(path).map_or(default, |(dir, _)| dir)
}

#[pymethods]
impl NativeImporter {
    /// `importlib.abc.MetaPathFinder.find_spec`: the spec importing the
    /// source file would have (`origin`, `cached`, and
    /// `submodule_search_locations` of a package), with a [`NativeLoader`].
    #[pyo3(signature = (fullname, path = None, target = None))]
    pub(crate) fn find_spec<'py>(
        &self,
        py: Python<'py>,
        fullname: &str,
        path: Option<&Bound<'py, PyAny>>,
        target: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        let _ = target;
        let Some(module) = self.get(fullname) else {
            return Ok(None);
        };
        if self.checks_path(module) && !self.on_path(py, module, path)? {
            return Ok(None);
        }
        let location = self.location(module);
        let loader = NativeLoader {
            module,
            path: location.clone(),
        };
        let kwargs = PyDict::new(py);
        kwargs.set_item("loader", Bound::new(py, loader)?)?;
        if module.package {
            let directory = directory(&location, "");
            kwargs.set_item("submodule_search_locations", PyList::new(py, [directory])?)?;
        } else {
            kwargs.set_item("submodule_search_locations", py.None())?;
        }
        let spec = py
            .import("importlib.util")?
            .getattr("spec_from_file_location")?
            .call((fullname, location), Some(&kwargs))?;
        Ok(Some(spec))
    }

    /// The names of the transpiled modules, for tools that check whether
    /// an imported module is native (`steelix test`).
    #[getter]
    fn modules(&self) -> Vec<&'static str> {
        self.modules.iter().map(|module| module.name).collect()
    }

    /// The project directory.
    #[getter]
    fn root(&self) -> &str {
        &self.root
    }

    fn __repr__(&self) -> String {
        format!(
            "<steelix_runtime.NativeImporter of {} modules in '{}'>",
            self.modules.len(),
            self.root
        )
    }
}

/// The `__loader__` of a transpiled module, like the `SourceFileLoader` of a
/// module imported from source: one per module, with its `name` and `path`.
#[pyclass(frozen, name = "NativeLoader", module = "steelix_runtime")]
pub struct NativeLoader {
    module: &'static NativeModule,
    /// The source file, `__file__`.
    #[pyo3(get)]
    path: String,
}

impl NativeLoader {
    fn check(&self, fullname: Option<&str>) -> PyResult<()> {
        match fullname {
            Some(name) if name != self.module.name => {
                let err = PyImportError::new_err(format!(
                    "loader for {} cannot handle {name}",
                    self.module.name
                ));
                Err(crate::raised(err))
            }
            _ => Ok(()),
        }
    }

    /// Runs the module's generated code in `module`.
    pub(crate) fn exec_native(&self, module: &Bound<'_, PyModule>) -> PyResult<()> {
        (self.module.exec)(&Module::new(module))
    }
}

#[pymethods]
impl NativeLoader {
    /// The full module name.
    #[getter]
    fn name(&self) -> &'static str {
        self.module.name
    }

    /// The default module creation.
    fn create_module(&self, spec: &Bound<'_, PyAny>) -> Option<()> {
        let _ = spec;
        None
    }

    /// Runs the generated code of the module. It goes through importlib's
    /// `_call_with_frames_removed`, like the source loader does, so that a
    /// traceback leaving the module body does not show importlib's frames.
    fn exec_module(slf: &Bound<'_, Self>, module: &Bound<'_, PyModule>) -> PyResult<()> {
        let py = slf.py();
        py.import("_frozen_importlib")?
            .call_method1("_call_with_frames_removed", (slf.getattr("_exec")?, module))?;
        Ok(())
    }

    fn _exec(&self, module: &Bound<'_, PyModule>) -> PyResult<()> {
        self.exec_native(module)
    }

    /// `importlib.abc.InspectLoader.is_package`.
    #[pyo3(signature = (fullname = None))]
    fn is_package(&self, fullname: Option<&str>) -> PyResult<bool> {
        self.check(fullname)?;
        Ok(self.module.package)
    }

    /// `importlib.abc.ExecutionLoader.get_filename`: the source file.
    #[pyo3(signature = (fullname = None))]
    fn get_filename(&self, fullname: Option<&str>) -> PyResult<String> {
        self.check(fullname)?;
        Ok(self.path.clone())
    }

    /// `importlib.abc.InspectLoader.get_code`, for `runpy` (`python -m
    /// pkg.mod`, `runpy.run_module`): a code object that runs the module's
    /// generated code in the namespace it is executed in. `runpy` puts the
    /// loader into that namespace as `__loader__`.
    #[pyo3(signature = (fullname = None))]
    fn get_code<'py>(
        &self,
        py: Python<'py>,
        fullname: Option<&str>,
    ) -> PyResult<Bound<'py, PyAny>> {
        self.check(fullname)?;
        let compile = py.import("builtins")?.getattr("compile")?;
        let kwargs = PyDict::new(py);
        kwargs.set_item("dont_inherit", true)?;
        compile.call(
            ("__loader__._exec_in(globals())", &self.path, "exec"),
            Some(&kwargs),
        )
    }

    /// Runs the generated code with `globals` as the module's namespace,
    /// for the code object of [`get_code`](Self::get_code).
    fn _exec_in(&self, globals: Bound<'_, PyDict>) -> PyResult<()> {
        (self.module.exec)(&Module::from_globals(globals))
    }

    /// `importlib.abc.ResourceLoader.get_data`: the contents of a file,
    /// for `pkgutil.get_data` (data files next to the module's source).
    fn get_data<'py>(
        &self,
        py: Python<'py>,
        path: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let file = py.import("_io")?.getattr("FileIO")?.call1((path, "r"))?;
        let data = file.call_method0("readall");
        let closed = file.call_method0("close");
        let data = data?;
        closed?;
        Ok(data)
    }

    /// `importlib.resources` support, like the source loader's: a
    /// `FileReader` over the directory of the module's source file (the
    /// package directory for a package), so that
    /// `importlib.resources.files(pkg)` finds the package's data files.
    fn get_resource_reader<'py>(
        slf: &Bound<'py, Self>,
        fullname: &str,
    ) -> PyResult<Bound<'py, PyAny>> {
        slf.get().check(Some(fullname))?;
        slf.py()
            .import("importlib.readers")?
            .getattr("FileReader")?
            .call1((slf,))
    }

    /// `importlib.abc.InspectLoader.get_source`: the Python source the
    /// module was transpiled from, or `None` if the file is not there
    /// (any more).
    #[pyo3(signature = (fullname = None))]
    fn get_source<'py>(
        &self,
        py: Python<'py>,
        fullname: Option<&str>,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        self.check(fullname)?;
        let Ok(data) = std::fs::read(&self.path) else {
            return Ok(None);
        };
        let decode = py.import("importlib.util")?.getattr("decode_source")?;
        decode
            .call1((pyo3::types::PyBytes::new(py, &data),))
            .map(Some)
    }

    fn __repr__(&self) -> String {
        format!("<steelix_runtime.NativeLoader '{}'>", self.module.name)
    }
}

/// Puts a [`NativeImporter`] for `modules` on `sys.meta_path`. `root` is the
/// project directory, which `sys.path[0]` is for `python app.py`.
///
/// It goes right before the path based finder, so builtin and frozen modules
/// keep their precedence, and the project's modules come before anything
/// else on `sys.path`, as they would from `sys.path[0]`.
pub fn install_importer<'py>(
    py: Python<'py>,
    root: &str,
    modules: &'static [NativeModule],
) -> PyResult<Bound<'py, NativeImporter>> {
    install(py, root, modules, Scope::Always)
}

/// [`install_importer`] answering the imports of `scope`.
pub(crate) fn install<'py>(
    py: Python<'py>,
    root: &str,
    modules: &'static [NativeModule],
    scope: Scope,
) -> PyResult<Bound<'py, NativeImporter>> {
    let importer = Bound::new(
        py,
        NativeImporter {
            root: root.to_owned(),
            modules,
            scope,
        },
    )?;
    let meta_path = py
        .import("sys")?
        .getattr("meta_path")?
        .cast_into::<PyList>()?;
    let path_finder = py
        .import("_frozen_importlib_external")?
        .getattr("PathFinder")?;
    let mut index = meta_path.len();
    for (i, finder) in meta_path.iter().enumerate() {
        if finder.is(&path_finder) {
            index = i;
            break;
        }
    }
    meta_path.insert(index, &importer)?;
    Ok(importer)
}

/// Extension module mode: installs the importer for the project's other
/// modules, which are next to the extension module `m` (the project
/// directory is the one of its `__file__`).
///
/// If `m` is a package of the project, it also becomes a package, with
/// `__path__` and `__package__` as for its `__init__.py`, so that its
/// submodules and relative imports work.
pub fn init_extension(m: &Bound<'_, PyModule>, modules: &'static [NativeModule]) -> PyResult<()> {
    let py = m.py();
    let globals = m.dict();
    let file = match globals.get_item("__file__")? {
        Some(file) if file.is_instance_of::<PyString>() => file.extract::<String>()?,
        _ => return Err(PyImportError::new_err("extension module without __file__")),
    };
    let name: String = m.name()?.extract()?;
    let mut root = directory(&file, ".");
    // Installed as the package's `__init__` (`pkg/__init__.so`), the
    // extension module is a package from the start, which `python -m pkg`
    // needs: the project directory is above the package's directory.
    let file_name = split_file_name(&file).map_or(file.as_str(), |(_, name)| name);
    if file_name.starts_with("__init__.") {
        for _ in name.split('.') {
            root = directory(root, ".");
        }
    }
    let root = root.to_owned();
    let importer = install_importer(py, &root, modules)?;
    if let Some(module) = importer.get().get(&name)
        && module.package
    {
        let path = importer.get().location(module);
        let search = PyList::new(py, [directory(&path, "")])?;
        globals.set_item("__path__", &search)?;
        globals.set_item("__package__", &name)?;
        if let Some(spec) = globals.get_item("__spec__")?
            && !spec.is_none()
        {
            spec.setattr("submodule_search_locations", search)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_paths_that_do_not_exist() {
        let dir = std::env::temp_dir().join(format!("steelix-resolve-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("root")).unwrap();
        let root = std::fs::canonicalize(dir.join("root")).unwrap();
        // A package directory that is not there (no sources), through `.`
        // and `..`, and on Unix through a link (Windows needs a privilege
        // to create one).
        assert_eq!(resolve(&dir.join("root/pkg")), root.join("pkg"));
        assert_eq!(resolve(&dir.join("root/./x/../pkg")), root.join("pkg"));
        assert_ne!(resolve(&dir.join("root/other")), root.join("pkg"));
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(dir.join("root"), dir.join("link")).unwrap();
            assert_eq!(resolve(&dir.join("link/pkg/sub")), root.join("pkg/sub"));
            assert_eq!(resolve(&dir.join("link/./x/../pkg")), root.join("pkg"));
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
