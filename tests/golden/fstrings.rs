use pyo3::prelude::*;
use steelix_runtime as rt;

// fstrings.py:4
pub fn describe<'py>(
    m: &rt::Module<'py>,
    name: &Bound<'py, PyAny>,
    value: &Bound<'py, PyAny>,
    width: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "describe", 4, |frame| {
        frame.line(5);
        rt::builtins::print(
            py,
            (rt::fstring(
                py,
                (rt::format(name, "")?, ": ", rt::format(value, "")?),
            )?,),
        )?;
        frame.line(6);
        rt::builtins::print(
            py,
            (rt::fstring(
                py,
                (
                    rt::format(value, ">8")?,
                    "|",
                    rt::format(value, rt::fstring(py, ("<", rt::format(width, "")?))?)?,
                    "|",
                    rt::format(
                        value,
                        rt::fstring(py, ("^", rt::format(width, "")?, ".2f"))?,
                    )?,
                ),
            )?,),
        )?;
        frame.line(7);
        rt::builtins::print(
            py,
            (rt::fstring(
                py,
                (
                    name.repr()?,
                    " ",
                    name.str()?,
                    " ",
                    rt::ascii(name)?,
                    " name=",
                    name.repr()?,
                    " value = ",
                    rt::format(value, ".1f")?,
                ),
            )?,),
        )?;
        frame.line(8);
        rt::builtins::print(
            py,
            (rt::format(
                value,
                rt::fstring(
                    py,
                    (
                        rt::format(
                            &(if value.gt(0)? {
                                rt::str(py, ">")
                            } else {
                                rt::str(py, "<")
                            }),
                            "",
                        )?,
                        rt::format(width, "")?,
                    ),
                )?,
            )?,),
        )?;
        frame.line(9);
        rt::builtins::print(
            py,
            (rt::fstring(
                py,
                ("{literal} ", rt::format(name, "")?, rt::format(name, "")?),
            )?,),
        )?;
        frame.line(10);
        rt::builtins::print(py, (rt::ascii(&rt::str(py, "é"))?, "", "plain"))?;
        frame.line(11);
        rt::fstring(
            py,
            (
                rt::format(&name.call_method0("upper")?, "")?,
                "=",
                rt::format(&value.mul(2)?, "08.3f")?,
            ),
        )
    })
}

/// Executes the body of `fstrings.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        m.set_global(
            "__doc__",
            "f-strings with format specs, conversions and nested fields.",
        )?;
        rt::def!(m, describe(name, value, width))?;
        frame.line(14);
        rt::builtins::print(
            py,
            (describe(
                m,
                &rt::str(py, "pi"),
                &rt::float(py, 3.14159),
                &rt::int(py, 10),
            )?,),
        )?;
        frame.line(15);
        rt::builtins::print(
            py,
            (describe(
                m,
                &rt::str(py, "ünïcode"),
                &rt::float(py, -1.5),
                &rt::int(py, 6),
            )?,),
        )?;
        frame.line(16);
        m.set_global("n", 255)?;
        frame.line(17);
        rt::builtins::print(
            py,
            (
                rt::fstring(
                    py,
                    (
                        rt::format(&m.global("n")?, "x")?,
                        " ",
                        rt::format(&m.global("n")?, "#o")?,
                        " ",
                        rt::format(&m.global("n")?, ",")?,
                        " ",
                        rt::format(&m.global("n")?, "b")?,
                    ),
                )?,
                rt::format(&m.global("n")?, "")?,
            ),
        )?;
        Ok(())
    })
}
