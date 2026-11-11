use pyo3::prelude::*;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// builtins_special.py:4
#[rt::frame(m, "info", 4)]
pub fn info<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<()> {
    let py = m.py();
    rt::builtins::print(
        py,
        (
            x.len()?,
            x.str()?,
            x.repr()?,
            x.is_truthy()?,
            x.get_type(),
            x.get_type().attr("__name__")?,
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
            rt::builtins::next(py, (&it,))?,
            rt::int(py, x.len()?).neg()?.abs()?,
            rt::builtins::sorted(py, (x,))?,
            rt::builtins::min(py, (x,))?,
            rt::builtins::max(py, (x,))?,
        ),
    )?;
    Ok(())
}

/// Executes the body of `builtins_special.py` in the module `m`.
#[rt::frame(m, "<module>", 1)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    m.set_global(
        "__doc__",
        "Builtins with a direct PyO3 spelling, and the dynamic fallback.",
    )?;
    rt::def!(m, info(x))?;
    info(m, PyList::new(py, [3, 1, 2])?.as_any())?;
    info(m, &rt::str(py, "hello"))?;
    info(m, &rt::str(py, "ab"))?;
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
            rt::builtins::divmod(py, (7, 2))?,
            rt::builtins::round(py, (2.567, 2))?,
            rt::builtins::sum(py, (PyList::new(py, [1, 2, 3])?,))?,
            rt::builtins::list(py, (rt::builtins::range(py, (3,))?,))?,
        ),
    )?;
    Ok(())
}
