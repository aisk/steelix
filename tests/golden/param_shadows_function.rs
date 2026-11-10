use pyo3::prelude::*;
use pyo3::types::PyList;
use steelix_runtime as rt;

// A parameter named like its own function, as in `timeit.repeat(repeat=...)`.
// param_shadows_function.py:4
#[rt::frame(m, "repeat", 4)]
pub fn repeat<'py>(
    m: &rt::Module<'py>,
    text: &Bound<'py, PyAny>,
    repeat: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    text.mul(repeat)
}

// param_shadows_function.py:8
#[rt::frame(m, "outer", 8)]
pub fn outer<'py>(m: &rt::Module<'py>, inner: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let mut inner = inner.clone();
    // param_shadows_function.py:9
    #[rt::frame(m, "outer.<locals>.inner", 9)]
    fn inner_<'py>(m: &rt::Module<'py>, inner: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        inner.add(1)
    }
    inner = rt::function!(m, inner_(inner), qualname = "outer.<locals>.inner")?;
    inner.call1((1,))
}

/// Executes the body of `param_shadows_function.py` in the module `m`.
#[rt::frame(m, "<module>", 1)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::def!(m, repeat(text, repeat = 2))?;
    rt::def!(m, outer(inner))?;
    rt::builtins::print(
        py,
        (
            repeat(m, &rt::str(py, "ab"), &rt::int(py, 2))?,
            repeat(m, &rt::str(py, "ab"), &rt::int(py, 3))?,
            outer(m, &rt::int(py, 0))?,
        ),
    )?;
    let list = PyList::empty(py);
    for n in rt::iter(&rt::builtin(py, "range")?.call1((3,))?)? {
        let n = n?;
        list.append(repeat(m, &rt::str(py, "x"), &n)?)?;
    }
    rt::builtins::print(py, (list,))?;
    Ok(())
}
