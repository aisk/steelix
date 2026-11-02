//! Running a transpiled program as an executable that embeds libpython.

use std::ffi::{CString, c_char};
use std::mem::MaybeUninit;
use std::os::unix::ffi::OsStringExt;
use std::process::ExitCode;

use pyo3::ffi;
use pyo3::prelude::*;

/// The `exec_module` function generated for a Python module.
pub type ExecModule = fn(&Bound<'_, PyModule>) -> PyResult<()>;

/// The Python script a program was transpiled from, as `python app.py` would
/// see it.
pub struct Script {
    /// `sys.argv[0]`: the script path as it was given to `steelix transpile`.
    pub argv0: &'static str,
    /// `__file__` of `__main__`: the absolute path of the script.
    pub file: &'static str,
    /// `sys.path[0]`: the directory of the script, with symlinks resolved.
    pub dir: &'static str,
}

/// Initializes Python, runs `exec_module` as the `__main__` module and shuts
/// the interpreter down again, like `python app.py` does.
///
/// `executable` is the interpreter whose installation (and virtual
/// environment) provides the standard library and `site-packages`.
/// `sys.argv` is the script path followed by the program's arguments, and the
/// real command line is in `sys.orig_argv`.
///
/// The exit code follows CPython: `SystemExit` exits with its code, any other
/// uncaught exception prints a traceback and exits with 1, and a failure while
/// finalizing exits with 120.
pub fn run_main(executable: &str, script: &Script, exec_module: ExecModule) -> ExitCode {
    // SAFETY: called once, before anything else touches the interpreter.
    unsafe { initialize(executable, script) };
    let failed = Python::attach(|py| {
        let result = py.import("__main__").and_then(|main| {
            prepare_main(&main, script)?;
            exec_module(&main)
        });
        flush_io(py);
        match result {
            Ok(()) => false,
            Err(err) => {
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
    if failed {
        ExitCode::FAILURE
    } else if finalized < 0 {
        ExitCode::from(120)
    } else {
        ExitCode::SUCCESS
    }
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
unsafe fn initialize(executable: &str, script: &Script) {
    let mut config = MaybeUninit::<ffi::PyConfig>::uninit();
    // SAFETY: `PyConfig_InitPythonConfig` fully initializes the struct.
    unsafe {
        ffi::PyConfig_InitPythonConfig(config.as_mut_ptr());
        let config = config.assume_init_mut();
        // The program's own arguments belong to the program, not to Python.
        config.parse_argv = 0;

        let executable = CString::new(executable).expect("interpreter path contains NUL");
        check(ffi::PyConfig_SetBytesString(
            config,
            &mut config.executable,
            executable.as_ptr(),
        ));

        let orig_args: Vec<CString> = std::env::args_os()
            .map(|arg| CString::new(arg.into_vec()).expect("argument contains NUL"))
            .collect();
        let script_name = CString::new(script.argv0).expect("script path contains NUL");
        // `sys.argv` as for `python app.py args...`: the script replaces the
        // executable's own path.
        let mut argv: Vec<*const c_char> = std::iter::once(script_name.as_ptr())
            .chain(orig_args.iter().skip(1).map(|arg| arg.as_ptr()))
            .collect();
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

/// Sets up `sys.path[0]` and the `__main__` globals that `python app.py`
/// provides before running the script.
fn prepare_main(main: &Bound<'_, PyModule>, script: &Script) -> PyResult<()> {
    let py = main.py();
    py.import("sys")?
        .getattr("path")?
        .call_method1("insert", (0, script.dir))?;
    let globals = main.dict();
    globals.set_item("__file__", script.file)?;
    globals.set_item("__cached__", py.None())?;
    Ok(())
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
