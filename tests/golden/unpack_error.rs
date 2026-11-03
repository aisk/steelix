use pyo3::prelude::*;
use pyo3::types::PyList;
use steelix_runtime as rt;

/// Executes the body of `unpack_error.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |_| {
        let [first, second] = rt::unpack(PyList::new(py, [1, 2])?.as_any())?;
        m.set_global("first", first)?;
        m.set_global("second", second)?;
        rt::builtins::print(py, (m.global("first")?, m.global("second")?))?;
        let [a, b] = rt::unpack(PyList::new(py, [1, 2, 3])?.as_any())?;
        m.set_global("a", a)?;
        m.set_global("b", b)?;
        Ok(())
    })
}
