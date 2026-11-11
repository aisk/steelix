use pyo3::prelude::*;
use pyo3::pyclass::CompareOp;
use steelix_runtime as rt;

// Ctrl-C in a native loop inside `try`: `finally` runs, then the
// `KeyboardInterrupt` traceback shows the native frames.
// steelix-test: interrupt after the first line of output
// interrupt_finally.py:6
#[rt::frame(m, "spin", 6, frame)]
pub fn spin<'py>(m: &rt::Module<'py>) -> PyResult<()> {
    let py = m.py();
    let mut n = rt::int(py, 0);
    let result = (|| -> PyResult<()> {
        rt::call!(rt::builtin(py, "print")?, "ready", flush = true)?;
        loop {
            n = rt::iadd(&n, 1)?;
            py.check_signals()?;
        }
    })();
    let finally = frame.finally(result);
    rt::builtins::print(py, ("cleanup", n.rich_compare(0, CompareOp::Gt)?))?;
    finally.resume()?;
    unreachable!();
}

/// Executes the body of `interrupt_finally.py` in the module `m`.
#[rt::frame(m, "<module>", 1)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    rt::def!(m, spin())?;
    spin(m)?;
    Ok(())
}
