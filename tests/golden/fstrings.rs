use pyo3::prelude::*;
use steelix_runtime as rt;

// fstrings.py:4
pub fn describe<'py>(
    py: Python<'py>,
    name: &Bound<'py, PyAny>,
    value: &Bound<'py, PyAny>,
    width: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
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
            rt::format(&name.call_method0("upper")?, "")?,
            "=",
            rt::format(&value.mul(2)?, "08.3f")?,
        ),
    )
}

/// Executes the body of `fstrings.py` in the module `m`.
pub fn exec_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let globals = &m.dict();
    globals.set_item(
        "__doc__",
        "f-strings with format specs, conversions and nested fields.",
    )?;
    rt::def!(globals, describe(name, value, width))?;
    rt::builtins::print(
        py,
        (describe(
            py,
            &rt::str(py, "pi"),
            &rt::float(py, 3.14159),
            &rt::int(py, 10),
        )?,),
    )?;
    rt::builtins::print(
        py,
        (describe(
            py,
            &rt::str(py, "ünïcode"),
            &rt::float(py, -1.5),
            &rt::int(py, 6),
        )?,),
    )?;
    globals.set_item("n", 255)?;
    rt::builtins::print(
        py,
        (
            rt::fstring(
                py,
                (
                    rt::format(&rt::global(globals, "n")?, "x")?,
                    " ",
                    rt::format(&rt::global(globals, "n")?, "#o")?,
                    " ",
                    rt::format(&rt::global(globals, "n")?, ",")?,
                    " ",
                    rt::format(&rt::global(globals, "n")?, "b")?,
                ),
            )?,
            rt::format(&rt::global(globals, "n")?, "")?,
        ),
    )?;
    Ok(())
}
