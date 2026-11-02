use pyo3::prelude::*;
use pyo3::pyclass::CompareOp;
use steelix_runtime as rt;

// Ctrl-C in a native loop inside `try`: `finally` runs, then the
// `KeyboardInterrupt` traceback shows the native frames.
// steelix-test: interrupt after the first line of output
// interrupt_finally.py:6
pub fn spin<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "spin", 6, |frame| {
        let mut n = rt::int(py, 0);
        let result = (|| -> PyResult<()> {
            frame.line(9);
            rt::call!(rt::builtin(py, "print")?, "ready", flush = true)?;
            loop {
                frame.line(11);
                n = rt::iadd(&n, 1)?;
                py.check_signals()?;
            }
        })();
        let finally = frame.finally(result);
        frame.line(13);
        rt::builtins::print(py, ("cleanup", n.rich_compare(0, CompareOp::Gt)?))?;
        finally.resume()?;
        unreachable!();
    })
}

/// Executes the body of `interrupt_finally.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    rt::frame!(m, "<module>", 1, |frame| {
        rt::def!(m, spin())?;
        frame.line(16);
        spin(m)?;
        Ok(())
    })
}
