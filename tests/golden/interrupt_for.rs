use pyo3::prelude::*;
use steelix_runtime as rt;

// interrupt_for.py:7
pub fn spin<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "spin", 7, |frame| {
        let mut total = rt::int(py, 0);
        frame.line(9);
        for i in m.global("itertools")?.call_method0("count")?.try_iter()? {
            let i = i?;
            frame.line(10);
            total = rt::iadd(&total, &i)?;
            py.check_signals()?;
            frame.line(9);
        }
        Ok(total)
    })
}

/// Executes the body of `interrupt_for.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    rt::frame!(m, "<module>", 1, |frame| {
        // Ctrl-C stops a native `for` loop over an iterator implemented in C, which
        // does not check for signals itself.
        // steelix-test: interrupt after the first line of output
        frame.line(4);
        m.set_global("itertools", rt::import(m, "itertools")?)?;
        rt::def!(m, spin())?;
        frame.line(14);
        rt::call!(rt::builtin(m.py(), "print")?, "ready", flush = true)?;
        frame.line(15);
        spin(m)?;
        Ok(())
    })
}
