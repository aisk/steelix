//! Parts of `match_kept.py` kept as Python source and run in place by
//! `rt::exec_python!` (steelix does not lower them yet):
//!
//! - line 14, `def kept`: OR pattern with captures
//! - line 28, `match` statement: OR pattern with captures

use pyo3::prelude::*;
use pyo3::types::IntoPyDict;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;

// match_kept.py:5
#[rt::frame(m, "native", 5)]
pub fn native<'py>(m: &rt::Module<'py>, value: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    if let Some([x, y]) = rt::match_seq(value)? {
        return rt::fstring(py, ("pair ", rt::format(&x, "")?, " ", rt::format(&y, "")?));
    } else if value.eq(0)? || value.eq(1)? {
        return Ok(rt::str(py, "bit"));
    }
    Ok(rt::str(py, "other"))
}

/// Executes the body of `match_kept.py` in the module `m`.
#[rt::frame(m, "<module>", 1)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    m.set_global(
        "__doc__",
        "OR patterns with captures stay Python (design §13): the function or the\n\
            module level statement containing one is kept, other matches are native.",
    )?;
    rt::def!(m, native(value))?;
    // match_kept.py:14, kept as Python: OR pattern with captures at 16:14
    rt::exec_python!(
        m,
        line = 14,
        r#"
def kept(value):
    match value:
        case [x, _] | {"x": x}:
            return f"x is {x}"
        case (0 | 1) as bit:
            return f"bit {bit}"
    return "other"
"#
    )?;
    for item in rt::iter(
        PyList::new(
            py,
            [
                PyList::new(py, [1, 2])?.as_any(),
                [("x", 3)].into_py_dict(py)?.as_any(),
                &rt::int(py, 0),
                &rt::int(py, 5),
            ],
        )?
        .as_any(),
    )? {
        m.set_global("value", item?)?;
        rt::builtins::print(
            py,
            (
                native(m, &m.global("value")?)?,
                "/",
                m.global("kept")?.call1((m.global("value")?,))?,
            ),
        )?;
    }
    // A module level loop keeps only the `match` that contains the OR pattern.
    for item2 in rt::iter(
        PyList::new(
            py,
            [
                PyTuple::new(py, [&rt::str(py, "add"), &rt::int(py, 1)])?.as_any(),
                PyTuple::new(py, [&rt::str(py, "sub"), &rt::int(py, 2)])?.as_any(),
                PyTuple::new(py, [&rt::str(py, "mul"), &rt::int(py, 3)])?.as_any(),
            ],
        )?
        .as_any(),
    )? {
        m.set_global("value", item2?)?;
        // match_kept.py:28, kept as Python: OR pattern with captures at 29:14
        rt::exec_python!(
            m,
            line = 28,
            r#"
    match value:
        case ("add", n) | ("sub", n):
            print("additive", n)
        case _:
            print("other", value)
"#
        )?;
        rt::builtins::print(py, ("native", m.global("value")?))?;
    }
    rt::builtins::print(py, (m.global("n")?,))?;
    Ok(())
}
