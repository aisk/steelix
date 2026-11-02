use pyo3::prelude::*;
use steelix_runtime as rt;

/// Executes the body of `interrupt_while.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    // Ctrl-C stops a native loop with `KeyboardInterrupt`, as in CPython.
    // steelix-test: interrupt after the first line of output
    rt::call!(rt::builtin(py, "print")?, "ready", flush = true)?;
    m.set_global("n", 0)?;
    loop {
        py.check_signals()?;
        m.set_global("n", rt::iadd(&m.global("n")?, 1)?)?;
    }
}
