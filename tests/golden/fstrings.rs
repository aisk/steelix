use pyo3::prelude::*;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// fstrings.py:4
pub fn describe<'py>(
    m: &rt::Module<'py>,
    name: &Bound<'py, PyAny>,
    value: &Bound<'py, PyAny>,
    width: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "describe", 4, |_| {
        rt::builtins::print(
            py,
            (rt::fstring(
                py,
                (rt::format(name, "")?, ": ", rt::format(value, "")?),
            )?,),
        )?;
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
        rt::builtins::print(
            py,
            (rt::fstring(
                py,
                ("{literal} ", rt::format(name, "")?, rt::format(name, "")?),
            )?,),
        )?;
        rt::builtins::print(py, (rt::ascii(&rt::str(py, "é"))?, "", "plain"))?;
        rt::fstring(
            py,
            (
                rt::format(&name.method0("upper")?, "")?,
                "=",
                rt::format(&value.mul(2)?, "08.3f")?,
            ),
        )
    })
}

/// Executes the body of `fstrings.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |_| {
        m.set_global(
            "__doc__",
            "f-strings with format specs, conversions and nested fields.",
        )?;
        rt::def!(m, describe(name, value, width))?;
        rt::builtins::print(
            py,
            (describe(
                m,
                &rt::str(py, "pi"),
                &rt::float(py, 3.14159),
                &rt::int(py, 10),
            )?,),
        )?;
        rt::builtins::print(
            py,
            (describe(
                m,
                &rt::str(py, "ünïcode"),
                &rt::float(py, -1.5),
                &rt::int(py, 6),
            )?,),
        )?;
        m.set_global("n", 255)?;
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
