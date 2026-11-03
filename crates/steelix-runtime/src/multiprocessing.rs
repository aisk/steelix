//! `multiprocessing` with the spawn and forkserver start methods (design §11).
//!
//! CPython starts these children as `python -c "from multiprocessing.spawn
//! import spawn_main; ..."`, and the child re-runs the parent's `__main__`
//! from its source file as `__mp_main__`, so that functions and classes
//! defined there can be unpickled. The child of a transpiled program has to
//! run the transpiled `__main__` instead, whose source file need not exist:
//! [`install_main_fixups`] replaces the functions that re-run it.
//!
//! The child is never chosen by the program's own command line, which
//! belongs to the program (`app -c "from multiprocessing.spawn import ..."`
//! must reach the script as arguments, like for `python app.py -c ..`):
//!
//! - In a distribution, `sys.executable` is the bundled interpreter
//!   `python/bin/python`, a link to the program that runs as the Python
//!   interpreter, and `multiprocessing` starts the children with it as
//!   usual. The interpreter recognizes the child command lines
//!   ([`is_child_command`]) and installs the fixups. It runs any command
//!   anyway, so this exposes nothing new.
//! - A development build has no such interpreter (`sys.executable` is the
//!   build interpreter, which does not know the transpiled code), so
//!   [`install_spawn_hook`] makes `multiprocessing` start the program itself
//!   and sets [`CHILD_MARKER`] in the environment to the program's path. The
//!   program takes the child role only for a child command line with that
//!   marker ([`is_child`]).

use std::ffi::{OsStr, OsString};

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyModule};

use crate::importer::{ExecModule, NativeImporter};

/// The commands `multiprocessing` runs with `-c` in a child process: a spawned
/// worker, the forkserver and the resource tracker (CPython 3.14).
const CHILD_COMMANDS: &[&str] = &[
    "from multiprocessing.spawn import spawn_main;",
    "import sys; from multiprocessing.forkserver import main;",
    "from multiprocessing.resource_tracker import main;",
];

/// The environment variable by which the parent process of a development
/// build marks the children it starts: the path of the program that is to
/// run as a child.
pub(crate) const CHILD_MARKER: &str = "STEELIX_MP_CHILD";

/// Whether a development build started as `program` with the command line
/// `args` and `marker` (the value of [`CHILD_MARKER`]) is a child process of
/// `multiprocessing`: a child command line, from a parent that marked it.
pub(crate) fn is_child(args: &[OsString], marker: Option<&OsStr>, program: &str) -> bool {
    marker.is_some_and(|marker| marker == OsStr::new(program)) && is_child_command(args)
}

/// Whether the command line is one `multiprocessing` starts a child with:
/// `PROGRAM [interpreter options] -c COMMAND [args]`, with one of the
/// commands above. The options are those of
/// `subprocess._args_from_interpreter_flags()`: flags, and `-X`/`-W` with
/// their value as the next argument.
pub(crate) fn is_child_command(args: &[OsString]) -> bool {
    let mut rest = args.iter().skip(1);
    while let Some(arg) = rest.next() {
        let Some(arg) = arg.to_str() else {
            return false;
        };
        if arg == "-c" {
            let Some(command) = rest.next().and_then(|command| command.to_str()) else {
                return false;
            };
            return CHILD_COMMANDS
                .iter()
                .any(|prefix| command.starts_with(prefix));
        }
        if !arg.starts_with('-') {
            return false;
        }
        if arg == "-X" || arg == "-W" {
            rest.next();
        }
    }
    false
}

/// For a development build: puts a finder on `sys.meta_path` that makes
/// `multiprocessing.spawn` start children with `program`
/// ([`set_child_executable`]) when it is first imported, while
/// `sys.executable` stays a Python interpreter. Importing `multiprocessing`
/// right away would cost every program about 12 ms of start-up time. The
/// finder removes itself once it has done that.
pub(crate) fn install_spawn_hook(py: Python<'_>, program: &str) -> PyResult<()> {
    let finder = Bound::new(
        py,
        SpawnFinder {
            program: program.to_owned(),
        },
    )?;
    let meta_path = py
        .import("sys")?
        .getattr("meta_path")?
        .cast_into::<PyList>()?;
    meta_path.insert(0, finder)?;
    Ok(())
}

/// The finder of [`install_spawn_hook`]. It finds `multiprocessing.spawn`
/// with the path based finder like the import system would, and wraps the
/// loader so that the executable is set right after the module ran.
#[pyclass(frozen, name = "SpawnFinder", module = "steelix_runtime")]
struct SpawnFinder {
    program: String,
}

#[pymethods]
impl SpawnFinder {
    #[pyo3(signature = (fullname, path = None, target = None))]
    fn find_spec<'py>(
        &self,
        py: Python<'py>,
        fullname: &str,
        path: Option<&Bound<'py, PyAny>>,
        target: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        if fullname != "multiprocessing.spawn" {
            return Ok(None);
        }
        let spec = py
            .import("_frozen_importlib_external")?
            .getattr("PathFinder")?
            .call_method1("find_spec", (fullname, path, target))?;
        if spec.is_none() {
            return Ok(None);
        }
        let loader = spec.getattr("loader")?;
        let wrapper = SpawnLoader {
            loader: loader.unbind(),
            program: self.program.clone(),
        };
        spec.setattr("loader", Bound::new(py, wrapper)?)?;
        Ok(Some(spec))
    }
}

/// Runs `multiprocessing.spawn` with its own loader, then calls
/// [`set_child_executable`] and puts the original loader back into the
/// module's `__loader__` and `__spec__`, so that nothing of the wrapper (or
/// of the finder) remains.
#[pyclass(frozen, name = "SpawnLoader", module = "steelix_runtime")]
struct SpawnLoader {
    loader: Py<PyAny>,
    program: String,
}

#[pymethods]
impl SpawnLoader {
    fn create_module<'py>(&self, spec: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        self.loader
            .bind(spec.py())
            .call_method1("create_module", (spec,))
    }

    fn exec_module(&self, module: &Bound<'_, PyModule>) -> PyResult<()> {
        let loader = self.loader.bind(module.py());
        module.setattr("__loader__", loader)?;
        module.getattr("__spec__")?.setattr("loader", loader)?;
        loader.call_method1("exec_module", (module,))?;
        set_child_executable(module, &self.program)?;
        let py = module.py();
        let meta_path = py.import("sys")?.getattr("meta_path")?;
        let finders: Vec<_> = meta_path
            .try_iter()?
            .filter_map(Result::ok)
            .filter(|finder| finder.is_instance_of::<SpawnFinder>())
            .collect();
        for finder in finders {
            meta_path.call_method1("remove", (finder,))?;
        }
        Ok(())
    }
}

/// Makes `multiprocessing` (its module `spawn`) start children with the
/// program `program` of a development build, and marks them as children
/// through [`CHILD_MARKER`]. The marker goes into `os.environ`, which every
/// process started from here on inherits; it only matters to a program at
/// that path with a child command line.
pub(crate) fn set_child_executable(spawn: &Bound<'_, PyAny>, program: &str) -> PyResult<()> {
    spawn.call_method1("set_executable", (program,))?;
    spawn
        .py()
        .import("os")?
        .getattr("environ")?
        .set_item(CHILD_MARKER, program)?;
    Ok(())
}

/// What the parent process runs as `__main__`.
#[derive(Clone, Copy)]
pub(crate) enum Main {
    /// A script (`python app.py`), with its generated code.
    Script(ExecModule),
    /// A module of the project (`python -m pkg.mod`), by name.
    Module(&'static str),
}

/// In a child process: replaces `_fixup_main_from_path` and
/// `_fixup_main_from_name` of `multiprocessing.spawn`, which re-run the
/// parent's `__main__` from its source as `__mp_main__`, with versions that
/// run its transpiled code instead. `file` is the script's `__file__` in the
/// parent. Anything else (a `__main__` that is not this program's) goes to
/// the original functions.
pub(crate) fn install_main_fixups(
    py: Python<'_>,
    main: Main,
    file: &str,
    importer: Option<Bound<'_, NativeImporter>>,
) -> PyResult<()> {
    let spawn = py.import("multiprocessing.spawn")?;
    let from_path = MainFromPath {
        original: spawn.getattr("_fixup_main_from_path")?.unbind(),
        exec: match main {
            Main::Script(exec) => Some(exec),
            Main::Module(_) => None,
        },
        file: file.to_owned(),
    };
    spawn.setattr("_fixup_main_from_path", Bound::new(py, from_path)?)?;
    let from_name = MainFromName {
        original: spawn.getattr("_fixup_main_from_name")?.unbind(),
        importer: importer.map(Bound::unbind),
    };
    spawn.setattr("_fixup_main_from_name", Bound::new(py, from_name)?)?;
    Ok(())
}

/// `multiprocessing.spawn._fixup_main_from_path` for a transpiled script.
#[pyclass(frozen, name = "MainFromPath", module = "steelix_runtime")]
struct MainFromPath {
    original: Py<PyAny>,
    exec: Option<ExecModule>,
    file: String,
}

#[pymethods]
impl MainFromPath {
    fn __call__(&self, py: Python<'_>, main_path: &str) -> PyResult<()> {
        let Some(exec) = self.exec.filter(|_| main_path == self.file) else {
            self.original.call1(py, (main_path,))?;
            return Ok(());
        };
        let current = current_main(py)?;
        if let Ok(file) = current.getattr("__file__")
            && file.extract::<&str>().is_ok_and(|file| file == main_path)
        {
            return Ok(());
        }
        let main = new_main(py, &current)?;
        let globals = main.dict();
        globals.set_item("__file__", main_path)?;
        globals.set_item("__cached__", py.None())?;
        globals.set_item("__loader__", py.None())?;
        globals.set_item("__package__", "")?;
        run_as_main(&main, |main| exec(&crate::Module::new(main)))
    }
}

/// `multiprocessing.spawn._fixup_main_from_name` for a transpiled module
/// run with `-m`.
#[pyclass(frozen, name = "MainFromName", module = "steelix_runtime")]
struct MainFromName {
    original: Py<PyAny>,
    importer: Option<Py<NativeImporter>>,
}

#[pymethods]
impl MainFromName {
    fn __call__(&self, py: Python<'_>, mod_name: &str) -> PyResult<()> {
        let importer = self.importer.as_ref().map(|importer| importer.bind(py));
        let native = importer.filter(|importer| importer.get().contains(mod_name));
        let Some(importer) = native.filter(|_| !mod_name.ends_with(".__main__")) else {
            self.original.call1(py, (mod_name,))?;
            return Ok(());
        };
        let current = current_main(py)?;
        let spec_name = current.getattr("__spec__")?.getattr("name").ok();
        if spec_name.is_some_and(|name| name.extract::<&str>().is_ok_and(|name| name == mod_name)) {
            return Ok(());
        }
        let main = new_main(py, &current)?;
        run_as_main(&main, |main| {
            crate::embed::init_module_as_main(main, importer, mod_name)?
                .get()
                .exec_native(main)
        })
    }
}

fn sys_modules(py: Python<'_>) -> PyResult<Bound<'_, PyDict>> {
    Ok(py
        .import("sys")?
        .getattr("modules")?
        .cast_into::<PyDict>()?)
}

fn current_main(py: Python<'_>) -> PyResult<Bound<'_, PyAny>> {
    sys_modules(py)?
        .get_item("__main__")?
        .ok_or_else(|| pyo3::exceptions::PyKeyError::new_err("__main__"))
}

/// A new `__mp_main__` module. The old `__main__` is kept alive in
/// `old_main_modules`, like `multiprocessing.spawn` does.
fn new_main<'py>(py: Python<'py>, current: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyModule>> {
    py.import("multiprocessing.spawn")?
        .getattr("old_main_modules")?
        .call_method1("append", (current,))?;
    PyModule::new(py, "__mp_main__")
}

/// Runs `body` in `main` as `__mp_main__`, then makes it `__main__` too.
/// If `body` fails, `sys.modules["__mp_main__"]` is restored, as
/// `runpy._TempModule` does for the source.
fn run_as_main(
    main: &Bound<'_, PyModule>,
    body: impl FnOnce(&Bound<'_, PyModule>) -> PyResult<()>,
) -> PyResult<()> {
    let modules = sys_modules(main.py())?;
    let saved = modules.get_item("__mp_main__")?;
    modules.set_item("__mp_main__", main)?;
    if let Err(err) = body(main) {
        match saved {
            Some(saved) => modules.set_item("__mp_main__", saved)?,
            None => {
                if modules.contains("__mp_main__")? {
                    modules.del_item("__mp_main__")?;
                }
            }
        }
        return Err(err);
    }
    modules.set_item("__main__", main)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(items: &[&str]) -> Vec<OsString> {
        items.iter().map(OsString::from).collect()
    }

    #[test]
    fn recognizes_multiprocessing_children() {
        let spawn =
            "from multiprocessing.spawn import spawn_main; spawn_main(tracker_fd=5, pipe_handle=7)";
        assert!(is_child_command(&args(&[
            "app",
            "-c",
            spawn,
            "--multiprocessing-fork"
        ])));
        assert!(is_child_command(&args(&[
            "app",
            "-B",
            "-X",
            "faulthandler",
            "-W",
            "error",
            "-c",
            spawn,
            "--multiprocessing-fork",
        ])));
        let tracker = "from multiprocessing.resource_tracker import main;main(4)";
        assert!(is_child_command(&args(&["app", "-c", tracker])));
        let forkserver = "import sys; from multiprocessing.forkserver import main; main(3, 4, ['__main__'], sys_argv=sys.argv[1:], **{})";
        assert!(is_child_command(&args(&["app", "-c", forkserver, "x"])));
    }

    #[test]
    fn children_of_development_builds_need_the_marker() {
        let spawn = "from multiprocessing.spawn import spawn_main; spawn_main()";
        let child = args(&["/bin/app", "-c", spawn, "--multiprocessing-fork"]);
        let marker = OsStr::new("/bin/app");
        assert!(is_child(&child, Some(marker), "/bin/app"));
        assert!(!is_child(&child, None, "/bin/app"));
        assert!(!is_child(
            &child,
            Some(OsStr::new("/bin/other")),
            "/bin/app"
        ));
        assert!(!is_child(
            &args(&["/bin/app", "x"]),
            Some(marker),
            "/bin/app"
        ));
    }

    #[test]
    fn leaves_other_command_lines_to_the_program() {
        assert!(!is_child_command(&args(&["app"])));
        assert!(!is_child_command(&args(&["app", "-c", "print(1)"])));
        assert!(!is_child_command(&args(&["app", "-c"])));
        let spawn = "from multiprocessing.spawn import spawn_main; spawn_main()";
        assert!(!is_child_command(&args(&["app", "input.txt", "-c", spawn])));
    }
}
