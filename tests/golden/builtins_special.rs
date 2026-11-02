use pyo3::prelude::*;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;

// builtins_special.py:4
pub fn info<'py>(py: Python<'py>, x: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::builtins::print(
        py,
        (
            x.len()?,
            x.str()?,
            x.repr()?,
            x.is_truthy()?,
            x.get_type(),
            x.get_type().getattr("__name__")?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            x.is_instance(&rt::builtin(py, "list")?)?,
            x.is_instance(
                PyTuple::new(py, [&rt::builtin(py, "str")?, &rt::builtin(py, "bytes")?])?.as_any(),
            )?,
        ),
    )?;
    if x.len()? > 2 {
        rt::builtins::print(py, ("long",))?;
    }
    if x.len()? != 0 {
        rt::builtins::print(py, ("not empty",))?;
    }
    if 3 >= x.len()? && x.is_instance(&rt::builtin(py, "str")?)? {
        rt::builtins::print(py, ("short str",))?;
    }
    if x.is_truthy()? && !x.is_instance(&rt::builtin(py, "int")?)? {
        rt::builtins::print(py, ("truthy",))?;
    }
    let it = x.try_iter()?.into_any();
    rt::builtins::print(
        py,
        (
            rt::builtin(py, "next")?.call1((&it,))?,
            rt::int(py, x.len()?).neg()?.abs()?,
            rt::builtin(py, "sorted")?.call1((x,))?,
            rt::builtin(py, "min")?.call1((x,))?,
            rt::builtin(py, "max")?.call1((x,))?,
        ),
    )?;
    Ok(rt::none(py))
}

/// Executes the body of `builtins_special.py` in the module `m`.
pub fn exec_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let globals = &m.dict();
    globals.set_item(
        "__doc__",
        "Builtins with a direct PyO3 spelling, and the dynamic fallback.",
    )?;
    rt::def!(globals, info(x))?;
    info(py, PyList::new(py, [3, 1, 2])?.as_any())?;
    info(py, &rt::str(py, "hello"))?;
    info(py, &rt::str(py, "ab"))?;
    rt::builtins::print(
        py,
        (
            rt::str(py, "é").len()?,
            rt::float(py, 1.5).str()?,
            rt::str(py, "q").repr()?,
            rt::int(py, 0).is_truthy()?,
            rt::float(py, -2.5).abs()?,
            rt::none(py).get_type(),
        ),
    )?;
    rt::builtins::print(
        py,
        (
            rt::builtin(py, "divmod")?.call1((7, 2))?,
            rt::builtin(py, "round")?.call1((2.567, 2))?,
            rt::builtin(py, "sum")?.call1((PyList::new(py, [1, 2, 3])?,))?,
            rt::builtin(py, "list")?.call1((rt::builtin(py, "range")?.call1((3,))?,))?,
        ),
    )?;
    Ok(())
}
