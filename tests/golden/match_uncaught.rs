use pyo3::prelude::*;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// match_uncaught.py:21
#[rt::frame(m, "describe", 21)]
pub fn describe<'py>(
    m: &rt::Module<'py>,
    shape: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    if let Some([x, y]) = rt::match_class(shape, &m.global("Point")?, 2, [])?
        && x.eq(&y)?
    {
        return Ok(rt::str(py, "diagonal point"));
    } else if let Some([item, item2]) = rt::match_class(shape, &m.global("Line")?, 2, [])?
        && let Some([item3, item4]) = rt::match_class(&item, &m.global("Point")?, 2, [])?
        && item3.eq(0)?
        && item4.eq(0)?
        && let Some([x, y, item5]) = rt::match_class(&item2, &m.global("Point")?, 3, [])?
        && item5.eq(0)?
    {
        return rt::fstring(
            py,
            (
                "from the origin to ",
                rt::format(&x, "")?,
                ",",
                rt::format(&y, "")?,
            ),
        );
    }
    Ok(rt::str(py, "other"))
}

pub mod point {
    use super::*;

    // match_uncaught.py:5
    /// The body of `class Point`, run in its namespace `ns`.
    #[rt::frame(ns, "Point", 5)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        ns.set("__match_args__", PyTuple::new(m.py(), ["x", "y"])?)?;
        rt::def!(ns, __init__(slf as "self", x, y))?;
        Ok(())
    }

    // match_uncaught.py:8
    #[rt::frame(m, "Point.__init__", 8)]
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        x: &Bound<'py, PyAny>,
        y: &Bound<'py, PyAny>,
    ) -> PyResult<()> {
        slf.set_attr("x", x)?;
        slf.set_attr("y", y)?;
        Ok(())
    }
}

pub mod line {
    use super::*;

    // match_uncaught.py:13
    /// The body of `class Line`, run in its namespace `ns`.
    #[rt::frame(ns, "Line", 13)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        ns.set(
            "__match_args__",
            PyTuple::new(m.py(), ["start", "end", "width"])?,
        )?;
        rt::def!(ns, __init__(slf as "self", start, end))?;
        Ok(())
    }

    // match_uncaught.py:16
    #[rt::frame(m, "Line.__init__", 16)]
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        start: &Bound<'py, PyAny>,
        end: &Bound<'py, PyAny>,
    ) -> PyResult<()> {
        slf.set_attr("start", start)?;
        slf.set_attr("end", end)?;
        Ok(())
    }
}

/// Executes the body of `match_uncaught.py` in the module `m`.
#[rt::frame(m, "<module>", 1)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    m.set_global(
        "__doc__",
        "An error raised by a nested pattern: the traceback points at the line of\n\
            the pattern that failed, as CPython's does (design §8, §9).",
    )?;
    m.set_global(
        "Point",
        rt::class!(m, "Point"(), point::body, line = 5, attributes = ["x", "y"])?,
    )?;
    m.set_global(
        "Line",
        rt::class!(
            m,
            "Line"(),
            line::body,
            line = 13,
            attributes = ["end", "start"]
        )?,
    )?;
    rt::def!(m, describe(shape))?;
    rt::builtins::print(py, (describe(m, &m.global("Point")?.call1((1, 1))?)?,))?;
    rt::builtins::print(
        py,
        (describe(
            m,
            &m.global("Line")?.call1((
                m.global("Point")?.call1((1, 0))?,
                m.global("Point")?.call1((2, 2))?,
            ))?,
        )?,),
    )?;
    rt::builtins::print(
        py,
        (describe(
            m,
            &m.global("Line")?.call1((
                m.global("Point")?.call1((0, 0))?,
                m.global("Point")?.call1((3, 4))?,
            ))?,
        )?,),
    )?;
    Ok(())
}
