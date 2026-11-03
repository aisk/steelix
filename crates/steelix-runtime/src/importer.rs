//! Importing the transpiled modules of a project (design §7).
//!
//! A program transpiled from several Python modules lists them in a table of
//! [`NativeModule`]s. [`install_importer`] puts a [`NativeImporter`] for that
//! table on `sys.meta_path`, so that `import pkg.sub` runs the generated
//! `exec_module` of `pkg/sub.py` instead of its Python source: lazily, at the
//! first import, with the module object, `sys.modules` entry and attributes
//! (`__name__`, `__file__`, `__spec__`, `__path__`, ..) that importing the
//! source would have created.

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

    /// The source file relative to the project directory.
    pub fn path(&self) -> String {
        let path = self.name.replace('.', "/");
        if self.package {
            path + "/__init__.py"
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
}

impl NativeImporter {
    fn get(&self, fullname: &str) -> Option<&'static NativeModule> {
        self.modules.iter().find(|module| module.name == fullname)
    }

    /// The absolute path of a module's source file, its `__file__`.
    fn location(&self, module: &NativeModule) -> String {
        format!("{}/{}", self.root, module.path())
    }
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
        let _ = (path, target);
        let Some(module) = self.get(fullname) else {
            return Ok(None);
        };
        let location = self.location(module);
        let loader = NativeLoader {
            module,
            path: location.clone(),
        };
        let kwargs = PyDict::new(py);
        kwargs.set_item("loader", Bound::new(py, loader)?)?;
        if module.package {
            let directory = location.rsplit_once('/').map_or("", |(dir, _)| dir);
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
    let importer = Bound::new(
        py,
        NativeImporter {
            root: root.to_owned(),
            modules,
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
    let root = file.rsplit_once('/').map_or(".", |(dir, _)| dir).to_owned();
    let name: String = m.name()?.extract()?;
    let importer = install_importer(py, &root, modules)?;
    if let Some(module) = importer.get().get(&name)
        && module.package
    {
        let path = importer.get().location(module);
        let directory = path.rsplit_once('/').map_or("", |(dir, _)| dir);
        let search = PyList::new(py, [directory])?;
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
