use pyo3::prelude::*;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// interrupt_for.py:7
pub fn spin<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "spin", 7, |_| {
        let mut total = rt::int(m.py(), 0);
        for i in rt::iter(&m.global("itertools")?.method0("count")?)? {
            let i = i?;
            total = rt::iadd(&total, &i)?;
        }
        Ok(total)
    })
}

/// Executes the body of `interrupt_for.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    rt::frame!(m, "<module>", 1, |_| {
        // Ctrl-C stops a native `for` loop over an iterator implemented in C, which
        // does not check for signals itself.
        // steelix-test: interrupt after the first line of output
        m.set_global("itertools", rt::import(m, "itertools")?)?;
        rt::def!(m, spin())?;
        rt::call!(rt::builtin(m.py(), "print")?, "ready", flush = true)?;
        spin(m)?;
        Ok(())
    })
}
