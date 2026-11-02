use pyo3::prelude::*;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;

// builtins_special.py:4
pub fn info<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "info", 4, |frame| {
        frame.line(5);
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
        frame.line(6);
        rt::builtins::print(
            py,
            (
                x.is_instance(&rt::builtin(py, "list")?)?,
                x.is_instance(
                    PyTuple::new(py, [&rt::builtin(py, "str")?, &rt::builtin(py, "bytes")?])?
                        .as_any(),
                )?,
            ),
        )?;
        frame.line(7);
        if x.len()? > 2 {
            frame.line(8);
            rt::builtins::print(py, ("long",))?;
        }
        frame.line(9);
        if x.len()? != 0 {
            frame.line(10);
            rt::builtins::print(py, ("not empty",))?;
        }
        frame.line(11);
        if 3 >= x.len()? && x.is_instance(&rt::builtin(py, "str")?)? {
            frame.line(12);
            rt::builtins::print(py, ("short str",))?;
        }
        frame.line(13);
        if x.is_truthy()? && !x.is_instance(&rt::builtin(py, "int")?)? {
            frame.line(14);
            rt::builtins::print(py, ("truthy",))?;
        }
        frame.line(15);
        let it = x.try_iter()?.into_any();
        frame.line(16);
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
    })
}

/// Executes the body of `builtins_special.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        m.set_global(
            "__doc__",
            "Builtins with a direct PyO3 spelling, and the dynamic fallback.",
        )?;
        rt::def!(m, info(x))?;
        frame.line(19);
        info(m, PyList::new(py, [3, 1, 2])?.as_any())?;
        frame.line(20);
        info(m, &rt::str(py, "hello"))?;
        frame.line(21);
        info(m, &rt::str(py, "ab"))?;
        frame.line(22);
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
        frame.line(23);
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
    })
}
