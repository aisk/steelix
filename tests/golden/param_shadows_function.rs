use pyo3::prelude::*;
use pyo3::types::PyList;
use steelix_runtime as rt;

// A parameter named like its own function, as in `timeit.repeat(repeat=...)`.
// param_shadows_function.py:4
pub fn repeat<'py>(
    m: &rt::Module<'py>,
    text: &Bound<'py, PyAny>,
    repeat: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "repeat", 4, |frame| {
        frame.line(5);
        text.mul(repeat)
    })
}

// param_shadows_function.py:8
pub fn outer<'py>(m: &rt::Module<'py>, inner: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "outer", 8, |frame| {
        let mut inner = inner.clone();
        // param_shadows_function.py:9
        fn inner_<'py>(
            m: &rt::Module<'py>,
            inner: &Bound<'py, PyAny>,
        ) -> PyResult<Bound<'py, PyAny>> {
            rt::frame!(m, "outer.<locals>.inner", 9, |frame| {
                frame.line(10);
                inner.add(1)
            })
        }
        inner = rt::function!(m, inner_(inner), qualname = "outer.<locals>.inner")?;
        frame.line(12);
        inner.call1((1,))
    })
}

/// Executes the body of `param_shadows_function.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        rt::def!(m, repeat(text, repeat = 2))?;
        rt::def!(m, outer(inner))?;
        frame.line(15);
        rt::builtins::print(
            py,
            (
                repeat(m, &rt::str(py, "ab"), &rt::int(py, 2))?,
                repeat(m, &rt::str(py, "ab"), &rt::int(py, 3))?,
                outer(m, &rt::int(py, 0))?,
            ),
        )?;
        frame.line(16);
        let list = PyList::empty(py);
        for n in rt::builtin(py, "range")?.call1((3,))?.try_iter()? {
            let n = n?;
            list.append(repeat(m, &rt::str(py, "x"), &n)?)?;
            py.check_signals()?;
        }
        rt::builtins::print(py, (&list,))?;
        Ok(())
    })
}
