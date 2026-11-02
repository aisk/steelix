use pyo3::prelude::*;
use pyo3::types::IntoPyDict;
use pyo3::types::PyDict;
use pyo3::types::PyTuple;
use steelix_runtime as rt;

// `*` and `**` arguments that are local variables used again afterwards.
// star_args_locals.py:4
pub fn collect<'py>(
    m: &rt::Module<'py>,
    args: &Bound<'py, PyTuple>,
    kwargs: &Bound<'py, PyDict>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "collect", 4, |frame| {
        frame.line(5);
        Ok(PyTuple::new(m.py(), [args.as_any(), kwargs.as_any()])?.into_any())
    })
}

// star_args_locals.py:8
pub fn forward<'py>(
    m: &rt::Module<'py>,
    items: &Bound<'py, PyAny>,
    options: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "forward", 8, |frame| {
        frame.line(9);
        let extra = rt::builtin(py, "list")?.call1((items,))?;
        frame.line(10);
        let named = rt::builtin(py, "dict")?.call1((options,))?;
        frame.line(11);
        let first = rt::call!(m.global("collect")?, *extra, **named)?;
        frame.line(12);
        let second = rt::call!(m.global("collect")?, 0, *extra, flag = true, **named)?;
        frame.line(13);
        Ok(PyTuple::new(py, [&first, &second, &extra, &named])?.into_any())
    })
}

/// Executes the body of `star_args_locals.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        rt::def!(m, collect(*args, **kwargs))?;
        rt::def!(m, forward(items, options))?;
        frame.line(16);
        rt::builtins::print(
            py,
            (forward(
                m,
                PyTuple::new(py, [1, 2])?.as_any(),
                [("key", 3)].into_py_dict(py)?.as_any(),
            )?,),
        )?;
        Ok(())
    })
}
