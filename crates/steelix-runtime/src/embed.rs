//! Running a transpiled program as an executable that embeds libpython.

use std::ffi::{CString, c_char};
use std::mem::MaybeUninit;
use std::os::unix::ffi::OsStringExt;
use std::process::ExitCode;

use pyo3::ffi;
use pyo3::prelude::*;

/// The `exec_module` function generated for a Python module.
pub type ExecModule = fn(&Bound<'_, PyModule>) -> PyResult<()>;

/// Initializes Python, runs `exec_module` as the `__main__` module and shuts
/// the interpreter down again, like `python app.py` does.
///
/// `executable` is the interpreter whose installation (and virtual
/// environment) provides the standard library and `site-packages`.
///
/// The exit code follows CPython: `SystemExit` exits with its code, any other
/// uncaught exception prints a traceback and exits with 1, and a failure while
/// finalizing exits with 120.
pub fn run_main(executable: &str, exec_module: ExecModule) -> ExitCode {
    // SAFETY: called once, before anything else touches the interpreter.
    unsafe { initialize(executable) };
    let failed = Python::attach(|py| {
        let result = py.import("__main__").and_then(|main| exec_module(&main));
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

/// Initializes the interpreter with `PyConfig` instead of PyO3's
/// `auto-initialize`, so that `sys.executable`, `sys.path` and `sys.argv`
/// look like they do for `python app.py`.
unsafe fn initialize(executable: &str) {
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

        let args: Vec<CString> = std::env::args_os()
            .map(|arg| CString::new(arg.into_vec()).expect("argument contains NUL"))
            .collect();
        let mut argv: Vec<*const c_char> = args.iter().map(|arg| arg.as_ptr()).collect();
        check(ffi::PyConfig_SetBytesArgv(
            config,
            argv.len() as ffi::Py_ssize_t,
            argv.as_mut_ptr(),
        ));

        check(ffi::Py_InitializeFromConfig(config));
        ffi::PyConfig_Clear(config);
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
