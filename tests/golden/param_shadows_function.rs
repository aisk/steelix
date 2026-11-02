//! Parts of `param_shadows_function.py` kept as Python source and run in place by
//! `rt::exec_python!` (steelix does not lower them yet):
//!
//! - line 16, `expr` statement: list comprehension

use pyo3::prelude::*;
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
        // param_shadows_function.py:16, kept as Python: list comprehension at 16:7
        rt::exec_python!(
            m,
            line = 16,
            r#"
print([repeat("x", n) for n in range(3)])
"#
        )?;
        Ok(())
    })
}
