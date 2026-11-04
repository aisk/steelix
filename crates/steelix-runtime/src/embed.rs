//! Running a transpiled program as an executable that embeds libpython.

use std::any::Any;
use std::ffi::{CString, OsString, c_char};
use std::mem::MaybeUninit;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::process::ExitCode;

use std::panic::{self, AssertUnwindSafe};

use pyo3::exceptions::PyKeyboardInterrupt;
use pyo3::ffi;
use pyo3::panic::PanicException;
use pyo3::prelude::*;

pub use crate::importer::ExecModule;
use crate::importer::{self, NativeImporter, NativeLoader, NativeModule, Scope};
use crate::multiprocessing::{self, Main};

/// The Python script a program was transpiled from, as `python app.py`
/// would see it. Use [`script!`](crate::script) to create it.
///
/// A development build (`cargo build` of the generated crate) runs with the
/// interpreter it was built against and the project directory it was
/// transpiled from. A build for distribution (`steelix build`) has neither
/// and finds both next to the executable (design §11):
///
/// ```text
/// dist/app/app                   the executable
/// dist/app/python/               the Python installation (`sys.prefix`)
/// dist/app/python/bin/python     `sys.executable`, a link to the executable
/// dist/app/src/                  the project directory (`sys.path[0]`)
/// ```
pub struct Script {
    /// The script's path relative to the project directory: `app.py`, or
    /// `pkg/__main__.py` for `python -m pkg`.
    pub file: &'static str,
    /// The interpreter of a development build (`PYO3_PYTHON`), whose
    /// installation and virtual environment provide the standard library
    /// and `site-packages`.
    pub python: Option<&'static str>,
    /// The project directory of a development build.
    pub project: Option<&'static str>,
}

/// The [`Script`] for the file `$file` of the project, with the interpreter
/// and project directory that the crate's build script recorded for a
/// development build (`STEELIX_PYTHON`, `STEELIX_PROJECT`), or none for a
/// build for distribution.
#[macro_export]
macro_rules! script {
    ($file:literal) => {
        $crate::embed::Script {
            file: $file,
            python: ::std::option_env!("STEELIX_PYTHON"),
            project: ::std::option_env!("STEELIX_PROJECT"),
        }
    };
}

/// The Python installation of a distribution, relative to the executable's
/// directory.
pub const DIST_PYTHON: &str = "python";
/// `sys.executable` of a distribution: a link to the program's executable,
/// which runs as the Python interpreter when it is called by this name.
pub const DIST_EXECUTABLE: &str = "python/bin/python";
/// The project directory of a distribution.
pub const DIST_PROJECT: &str = "src";

/// Where a program finds Python and its project when it runs.
struct Paths {
    /// The program's own executable, which `multiprocessing` starts for the
    /// child processes of a development build.
    program: String,
    /// `sys.executable`: the interpreter of a development build, the link
    /// to the program of a distribution.
    executable: String,
    /// The Python installation of a distribution (`PyConfig.home`). A
    /// development build lets CPython find it from `executable`.
    home: Option<String>,
    /// The project directory: `sys.path[0]`, and where the project's
    /// modules have their `__file__`.
    project: String,
    /// The script's `__file__`, which is also `sys.argv[0]`.
    file: String,
}

impl Paths {
    fn locate(script: &Script) -> Self {
        let program = match std::env::current_exe() {
            // Linux's `/proc/self/exe` has the links resolved already, macOS
            // gives the path the process was started by: `python/bin/python`
            // of a distribution, a link to the program.
            Ok(program) => program.canonicalize().unwrap_or(program),
            Err(err) => fatal(&format!("can not locate the executable: {err}")),
        };
        let dist = program.parent().unwrap_or(Path::new("/"));
        let (executable, home) = match script.python {
            Some(python) => (python.to_owned(), None),
            None => {
                let home = dist.join(DIST_PYTHON);
                if !home.is_dir() {
                    fatal(&format!(
                        "no Python installation in {}: run the program from the directory \
                         `steelix build` created",
                        home.display()
                    ));
                }
                (
                    path_string(&dist.join(DIST_EXECUTABLE)),
                    Some(path_string(&home)),
                )
            }
        };
        let project = match script.project {
            Some(project) => project.to_owned(),
            None => path_string(&dist.join(DIST_PROJECT)),
        };
        let file = format!("{project}/{}", script.file);
        Self {
            program: path_string(&program),
            executable,
            home,
            project,
            file,
        }
    }
}

fn path_string(path: &Path) -> String {
    match path.to_str() {
        Some(text) => text.to_owned(),
        None => fatal(&format!("path is not valid UTF-8: {}", path.display())),
    }
}

/// Reports a problem with the program's installation and exits.
fn fatal(message: &str) -> ! {
    eprintln!("steelix: {message}");
    std::process::exit(1)
}

/// Initializes Python, runs `exec_module` as the `__main__` module and shuts
/// the interpreter down again, like `python app.py` does.
///
/// `sys.executable` is a Python interpreter, as for `python app.py`, so that
/// `subprocess.run([sys.executable, ...])` works (see [`Script`] for which
/// one). `sys.argv` is the script's path followed by the program's
/// arguments, and the real command line is in `sys.orig_argv`.
///
/// The exit code follows CPython: `SystemExit` exits with its code, any other
/// uncaught exception prints a traceback and exits with 1, and a failure while
/// finalizing exits with 120. An uncaught `KeyboardInterrupt` (Ctrl-C) ends
/// the process with `SIGINT` after finalizing, so that the shell sees an
/// interrupted program.
///
/// A Rust panic (a bug in hand-written code, or in steelix) is reported like
/// an uncaught Python exception, as PyO3's `PanicException`, and also exits
/// with 1 instead of Rust's usual 101.
///
/// The executable is also the Python interpreter when it is called as
/// `python` (the `sys.executable` of a distribution), which is how
/// `multiprocessing` starts its child processes (spawn and forkserver, which
/// run the transpiled code of `__main__`); a development build starts
/// itself for them instead and marks them through the environment.
///
/// The program of a distribution ignores `PYTHON*` environment variables
/// and the user's `site-packages` unless [`PYTHON_ENV`] is `1`.
pub fn run_main(script: &Script, exec_module: ExecModule) -> ExitCode {
    run(script, &[], Main::Script(exec_module))
}

/// Like [`run_main`], for a script that imports other modules of its
/// project: `modules` lists them with their generated code, which
/// `import` runs instead of their Python source (design §7). The script's
/// own module may be one of them, for `import app` from inside the program,
/// which creates a second module object like CPython does.
pub fn run_project(
    script: &Script,
    modules: &'static [NativeModule],
    exec_module: ExecModule,
) -> ExitCode {
    run(script, modules, Main::Script(exec_module))
}

/// Like `python -m name` run in the project directory: imports the parent
/// packages of the module `name` (for `python -m pkg`, `name` is
/// `pkg.__main__`), then runs the module's generated code as `__main__`
/// with the module's spec, like `runpy` does. `name` is one of `modules`,
/// and `script.file` its source file.
pub fn run_module(
    script: &Script,
    modules: &'static [NativeModule],
    name: &'static str,
) -> ExitCode {
    run(script, modules, Main::Module(name))
}

/// What the executable runs as.
#[derive(Debug, PartialEq, Eq)]
enum Role {
    /// The transpiled program. Its command line belongs to the program.
    Program,
    /// The Python interpreter: called as `python` (the `sys.executable` of
    /// a distribution), or a child process of `multiprocessing` that a
    /// development build started (design §11).
    Python,
}

/// The executable's role. It runs as Python when it is called by one of
/// Python's names, or, in a development build, as a child process that its
/// parent marked through the environment. The program's own arguments
/// never choose the role: `app -c CODE` passes `-c CODE` to the script.
fn role(args: &[OsString], paths: &Paths) -> Role {
    let name = args
        .first()
        .and_then(|arg0| Path::new(arg0).file_name())
        .and_then(|name| name.to_str())
        .unwrap_or("");
    let version = name.strip_prefix("python3").unwrap_or("x");
    let dev_child = paths.home.is_none()
        && multiprocessing::is_child(
            args,
            std::env::var_os(multiprocessing::CHILD_MARKER).as_deref(),
            &paths.program,
        );
    if name == "python" || version.is_empty() || version.starts_with('.') || dev_child {
        Role::Python
    } else {
        Role::Program
    }
}

/// The environment variable that makes the program of a distribution
/// honour `PYTHON*` environment variables and the user's `site-packages`
/// when it is `1`. By default it ignores both, so that the environment of
/// the machine it runs on does not change the program (design §11).
pub const PYTHON_ENV: &str = "STEELIX_PYTHON_ENV";

/// The environment variable that makes the executable in the interpreter
/// role answer every import of a transpiled module with its native code
/// when it is `1`, like the program does, instead of only the imports that
/// would find the module's source on `sys.path`. `steelix test` sets it, so
/// that the project's tests run against the native code wherever the
/// project directory is (design §15).
pub const NATIVE_IMPORTS: &str = "STEELIX_NATIVE_IMPORTS";

/// How the interpreter is configured beyond the command line.
#[derive(Clone, Copy)]
struct Isolation {
    /// Read `PYTHON*` environment variables (`PyConfig.use_environment`).
    environment: bool,
    /// Add the user's `site-packages` (`PyConfig.user_site_directory`).
    user_site: bool,
}

fn run(script: &Script, modules: &'static [NativeModule], main: Main) -> ExitCode {
    let paths = Paths::locate(script);
    let args: Vec<OsString> = std::env::args_os().collect();
    // A development build behaves like the build interpreter; a
    // distribution is isolated from the machine unless asked otherwise.
    let honour = paths.home.is_none() || std::env::var_os(PYTHON_ENV).is_some_and(|v| v == "1");
    match role(&args, &paths) {
        Role::Program => {
            let isolation = Isolation {
                environment: honour,
                user_site: honour,
            };
            run_program(&paths, &args, isolation, |main_module| {
                let py = main_module.py();
                if paths.home.is_none() {
                    multiprocessing::install_spawn_hook(py, &paths.program)?;
                }
                py.import("sys")?
                    .getattr("path")?
                    .call_method1("insert", (0, &paths.project))?;
                let importer = if modules.is_empty() {
                    None
                } else {
                    Some(crate::install_importer(py, &paths.project, modules)?)
                };
                match main {
                    Main::Script(exec_module) => {
                        let globals = main_module.dict();
                        globals.set_item("__file__", &paths.file)?;
                        globals.set_item("__cached__", py.None())?;
                        exec_module(&crate::Module::new(main_module))
                    }
                    Main::Module(name) => {
                        let Some(importer) = importer else {
                            let message = format!("No module named '{name}'");
                            return Err(pyo3::exceptions::PyImportError::new_err(message));
                        };
                        init_module_as_main(main_module, &importer, name)?
                            .get()
                            .exec_native(main_module)
                    }
                }
            })
        }
        Role::Python => {
            // Like a virtual environment, the bundled interpreter leaves out
            // the user's site-packages; it is otherwise a normal interpreter.
            let isolation = Isolation {
                environment: true,
                user_site: honour,
            };
            run_python(&paths, &args, isolation, |py| {
                // The project's modules, where CPython would find their
                // sources: `python/bin/python -m pkg.cli` in the project
                // directory, and `multiprocessing` children, which get the
                // parent's `sys.path`. Or all of them, for the project's
                // tests (`steelix test`).
                let scope = if std::env::var_os(NATIVE_IMPORTS).is_some_and(|v| v == "1") {
                    Scope::Always
                } else {
                    Scope::OnPath
                };
                let importer = if modules.is_empty() {
                    None
                } else {
                    Some(importer::install(py, &paths.project, modules, scope)?)
                };
                if multiprocessing::is_child_command(&args) {
                    multiprocessing::install_main_fixups(py, main, &paths.file, importer)?;
                    if paths.home.is_none() {
                        let spawn = py.import("multiprocessing.spawn")?;
                        multiprocessing::set_child_executable(&spawn, &paths.program)?;
                    }
                }
                Ok(())
            })
        }
    }
}

/// `runpy._run_module_as_main` up to running the code: imports the parent
/// packages of `name` and gives `main` the module's spec and attributes.
/// Returns the module's loader, which runs its transpiled code.
pub(crate) fn init_module_as_main<'py>(
    main: &Bound<'py, PyModule>,
    importer: &Bound<'py, NativeImporter>,
    name: &str,
) -> PyResult<Bound<'py, NativeLoader>> {
    let py = main.py();
    if let Some((package, _)) = name.rsplit_once('.') {
        py.import(package)?;
        let existing = py
            .import("sys")?
            .getattr("modules")?
            .call_method1("get", (name,))?;
        if !existing.is_none() && !existing.hasattr("__path__")? {
            let message = format!(
                "'{name}' found in sys.modules after import of package '{package}', but prior \
                 to execution of '{name}'; this may result in unpredictable behaviour"
            );
            let warning = py
                .get_type::<pyo3::exceptions::PyRuntimeWarning>()
                .call1((message,))?;
            py.import("warnings")?.call_method1("warn", (warning,))?;
        }
    }
    let Some(spec) = importer.get().find_spec(py, name, None, None)? else {
        return Err(pyo3::exceptions::PyImportError::new_err(format!(
            "No module named '{name}'"
        )));
    };
    let globals = main.dict();
    globals.set_item("__file__", spec.getattr("origin")?)?;
    globals.set_item("__cached__", spec.getattr("cached")?)?;
    globals.set_item("__doc__", py.None())?;
    globals.set_item("__loader__", spec.getattr("loader")?)?;
    globals.set_item("__package__", spec.getattr("parent")?)?;
    globals.set_item("__spec__", &spec)?;
    Ok(spec.getattr("loader")?.cast_into::<NativeLoader>()?)
}

/// Initializes Python with the program's own command line as a Python
/// command line (`python [options] -c command ...`), runs `setup` and then
/// the command, like the `python` executable does. Returns its exit code.
fn run_python(
    paths: &Paths,
    args: &[OsString],
    isolation: Isolation,
    setup: impl FnOnce(Python<'_>) -> PyResult<()>,
) -> ExitCode {
    // SAFETY: called once, before anything else touches the interpreter.
    unsafe { initialize(paths, args, true, isolation) };
    let ready = Python::attach(|py| match setup(py) {
        Ok(()) => true,
        Err(err) => {
            err.print(py);
            false
        }
    });
    if !ready {
        // SAFETY: the main thread still holds the GIL taken by initialization.
        unsafe { ffi::Py_FinalizeEx() };
        return ExitCode::FAILURE;
    }
    // SAFETY: the interpreter is initialized from a configuration with the
    // command line, which is what `Py_RunMain` runs; it also finalizes.
    let code = unsafe { ffi::Py_RunMain() };
    ExitCode::from(u8::try_from(code).unwrap_or(1))
}

/// Initializes Python, runs `body` with the `__main__` module and shuts the
/// interpreter down again, with the exit code CPython would use.
fn run_program(
    paths: &Paths,
    args: &[OsString],
    isolation: Isolation,
    body: impl FnOnce(&Bound<'_, PyModule>) -> PyResult<()>,
) -> ExitCode {
    // SAFETY: called once, before anything else touches the interpreter.
    unsafe { initialize(paths, args, false, isolation) };
    let mut interrupted = false;
    let failed = Python::attach(|py| {
        let run = || body(&py.import("__main__")?);
        let result = panic::catch_unwind(AssertUnwindSafe(run))
            .unwrap_or_else(|payload| Err(panic_error(payload)));
        flush_io(py);
        match result {
            Ok(()) => false,
            Err(err) => {
                interrupted = err.is_instance_of::<PyKeyboardInterrupt>(py);
                // For `SystemExit` this does not return: CPython finalizes and
                // exits with the requested code itself.
                err.restore(py);
                unsafe { ffi::PyErr_Print() };
                true
            }
        }
    });
    // SAFETY: the main thread still holds the GIL taken by initialization.
    let finalized = unsafe { ffi::Py_FinalizeEx() };
    if interrupted {
        exit_sigint();
    }
    if failed {
        ExitCode::FAILURE
    } else if finalized < 0 {
        ExitCode::from(120)
    } else {
        ExitCode::SUCCESS
    }
}

/// Ends the process with `SIGINT`, like CPython does after an uncaught
/// `KeyboardInterrupt`. Returns only if that fails.
fn exit_sigint() {
    // SAFETY: plain libc calls; the interpreter is finalized, nothing else
    // depends on the `SIGINT` handler any more.
    unsafe {
        if libc::signal(libc::SIGINT, libc::SIG_DFL) != libc::SIG_ERR {
            libc::kill(libc::getpid(), libc::SIGINT);
        }
    }
}

/// The `PanicException` for a caught panic, carrying its message.
fn panic_error(payload: Box<dyn Any + Send>) -> PyErr {
    let message = if let Some(text) = payload.downcast_ref::<&str>() {
        (*text).to_owned()
    } else if let Some(text) = payload.downcast_ref::<String>() {
        text.clone()
    } else {
        "panic with a non-string payload".to_owned()
    };
    PanicException::new_err(message)
}

/// Flushes `sys.stderr` and `sys.stdout`, ignoring errors, like CPython's
/// `flush_io` does after running the script and before printing an uncaught
/// exception. Without it, output still buffered in `sys.stdout` would come
/// after the traceback when both go to the same file.
fn flush_io(py: Python<'_>) {
    let Ok(sys) = py.import("sys") else { return };
    for name in ["stderr", "stdout"] {
        if let Ok(stream) = sys.getattr(name)
            && !stream.is_none()
        {
            let _ = stream.call_method0("flush");
        }
    }
}

/// Initializes the interpreter with `PyConfig` instead of PyO3's
/// `auto-initialize`, so that `sys.executable`, `sys.path` and `sys.argv`
/// look like they do for `python app.py`.
///
/// With `python_args` the command line is parsed like the `python`
/// executable's (options, `-c`, `-m`, a script), for [`run_python`].
/// Otherwise the arguments belong to the program, and `sys.argv` is the
/// script's path followed by them, as for `python app.py args...`.
/// `isolation` comes before the command line, so `-E` and `-s` still apply.
unsafe fn initialize(paths: &Paths, args: &[OsString], python_args: bool, isolation: Isolation) {
    let orig_args: Vec<CString> = args.iter().map(|arg| c_string(arg.as_bytes())).collect();
    let script_name = c_string(paths.file.as_bytes());
    let mut argv: Vec<*const c_char> = if python_args {
        orig_args.iter().map(|arg| arg.as_ptr()).collect()
    } else {
        std::iter::once(script_name.as_ptr())
            .chain(orig_args.iter().skip(1).map(|arg| arg.as_ptr()))
            .collect()
    };
    let mut preconfig = MaybeUninit::<ffi::PyPreConfig>::uninit();
    let mut config = MaybeUninit::<ffi::PyConfig>::uninit();
    // SAFETY: `PyPreConfig_InitPythonConfig` and `PyConfig_InitPythonConfig`
    // fully initialize the structs.
    unsafe {
        // Pre-initialize from the command line first: the `PyConfig_Set*`
        // calls below would otherwise pre-initialize without it, and the
        // options that apply then (`-E`, `-I`, `-X dev`, `-X utf8`) would be
        // lost.
        ffi::PyPreConfig_InitPythonConfig(preconfig.as_mut_ptr());
        let preconfig = preconfig.assume_init_mut();
        preconfig.parse_argv = python_args.into();
        preconfig.use_environment = isolation.environment.into();
        check(ffi::Py_PreInitializeFromBytesArgs(
            preconfig,
            argv.len() as ffi::Py_ssize_t,
            argv.as_mut_ptr().cast(),
        ));

        ffi::PyConfig_InitPythonConfig(config.as_mut_ptr());
        let config = config.assume_init_mut();
        config.parse_argv = python_args.into();
        config.use_environment = isolation.environment.into();
        config.user_site_directory = isolation.user_site.into();

        let executable = c_string(paths.executable.as_bytes());
        check(ffi::PyConfig_SetBytesString(
            config,
            &mut config.executable,
            executable.as_ptr(),
        ));
        if let Some(home) = &paths.home {
            let home = c_string(home.as_bytes());
            check(ffi::PyConfig_SetBytesString(
                config,
                &mut config.home,
                home.as_ptr(),
            ));
        }

        check(ffi::PyConfig_SetBytesArgv(
            config,
            argv.len() as ffi::Py_ssize_t,
            argv.as_mut_ptr(),
        ));
        let mut orig_argv = Vec::with_capacity(orig_args.len());
        for arg in &orig_args {
            let wide = ffi::Py_DecodeLocale(arg.as_ptr(), std::ptr::null_mut());
            assert!(!wide.is_null(), "can not decode command line argument");
            orig_argv.push(wide);
        }
        check(ffi::PyConfig_SetWideStringList(
            config,
            &mut config.orig_argv,
            orig_argv.len() as ffi::Py_ssize_t,
            orig_argv.as_mut_ptr(),
        ));
        for wide in orig_argv {
            ffi::PyMem_RawFree(wide.cast());
        }

        check(ffi::Py_InitializeFromConfig(config));
        ffi::PyConfig_Clear(config);
    }
}

fn c_string(bytes: &[u8]) -> CString {
    match CString::new(bytes) {
        Ok(text) => text,
        Err(_) => fatal("a path or argument contains a NUL character"),
    }
}

/// Exits the process like CPython does when initialization fails.
unsafe fn check(status: ffi::PyStatus) {
    // SAFETY: `status` comes straight from a CPython initialization API.
    unsafe {
        if ffi::PyStatus_Exception(status) != 0 {
            ffi::Py_ExitStatusException(status);
        }
    }
}
