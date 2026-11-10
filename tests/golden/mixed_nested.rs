//! Parts of `mixed_nested.py` kept as Python source and run in place by
//! `rt::exec_python!` (steelix does not lower them yet):
//!
//! - line 12, `assign` statement: generator expression with several for clauses
//! - line 28, `assign` statement: generator expression with several for clauses
//! - line 33, `while` statement: try/except*
//! - line 44, `for` statement: try/except*
//! - line 67, `assign` statement: generator expression with several for clauses

use pyo3::prelude::*;
use pyo3::types::PyList;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// mixed_nested.py:6
#[rt::frame(m, "double", 6)]
pub fn double<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    x.mul(2)
}

pub mod point {
    use super::*;

    // mixed_nested.py:55
    /// The body of `class Point`, run in its namespace `ns`.
    #[rt::frame(ns, "Point", 55)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::def!(ns, __init__(slf as "self", x))?;
        Ok(())
    }

    // mixed_nested.py:56
    #[rt::frame(m, "Point.__init__", 56)]
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        x: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        slf.set_attr("x", x)?;
        Ok(rt::none(m.py()))
    }
}

pub mod local {
    use super::*;

    // mixed_nested.py:63
    /// The body of `class Local`, run in its namespace `ns`.
    #[rt::frame(ns, "Local", 63)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        ns.set("label", "local class")?;
        Ok(())
    }
}

/// Executes the body of `mixed_nested.py` in the module `m`.
#[rt::frame(m, "<module>", 1)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    // Module level `if`, `for` and `while` statements stay native when only some
    // statements in their bodies are kept as Python (design §13).
    m.set_global("sys", rt::import(m, "sys")?)?;
    rt::def!(m, double(x))?;
    if m.global("sys")?.attr("argv")?.len()? > 5 {
        m.set_global("mode", "many")?;
        // mixed_nested.py:12, kept as Python: generator expression with several for clauses at 12:18
        rt::exec_python!(
            m,
            line = 12,
            r#"
    labels = list(c for c in "many" for _ in "1")  # kept as Python, the `if` is native
"#
        )?;
        // mixed_nested.py:14
        #[rt::frame(m, "describe", 14)]
        fn describe<'py>(
            m: &rt::Module<'py>,
            n: &Bound<'py, PyAny>,
        ) -> PyResult<Bound<'py, PyAny>> {
            rt::fstring(m.py(), (rt::format(n, "")?, " (many)"))
        }
        rt::def!(m, describe(n))?;
    } else {
        m.set_global("mode", "few")?;
        // mixed_nested.py:19
        #[rt::frame(m, "describe", 19)]
        fn describe_<'py>(
            m: &rt::Module<'py>,
            n: &Bound<'py, PyAny>,
        ) -> PyResult<Bound<'py, PyAny>> {
            rt::fstring(
                m.py(),
                (
                    rt::format(n, "")?,
                    " (",
                    rt::format(&m.global("mode")?, "")?,
                    ")",
                ),
            )
        }
        rt::def!(m, describe_ as "describe"(n))?;
        rt::builtins::print(py, ("native branch", m.global("mode")?))?;
    }
    for item in rt::iter(&rt::builtin(py, "range")?.call1((4,))?)? {
        m.set_global("i", item?)?;
        if m.global("i")?.eq(1)? {
            continue;
        }
        rt::builtins::print(
            py,
            (m.global("describe")?
                .call1((double(m, &m.global("i")?)?,))?,),
        )?;
        // mixed_nested.py:28, kept as Python: generator expression with several for clauses at 28:19
        rt::exec_python!(
            m,
            line = 28,
            r#"
    squares = list(j for j in range(i) for _ in "1")  # kept as Python
"#
        )?;
        rt::builtins::print(py, ("squares", m.global("squares")?))?;
    }
    // `break` in a kept statement: the loop is kept whole.
    m.set_global("n", 0)?;
    // mixed_nested.py:33, kept as Python: try/except* at 35:5
    rt::exec_python!(
        m,
        line = 33,
        r#"
while True:
    n += 1
    try:
        if n == 3:
            break
    except* ValueError:
        pass
    print("tried", n)
"#
    )?;
    // A kept loop inside a native one: its `break` is its own.
    for item2 in rt::iter(PyList::new(py, ["ab", "cde"])?.as_any())? {
        m.set_global("word", item2?)?;
        // mixed_nested.py:44, kept as Python: try/except* at 45:9
        rt::exec_python!(
            m,
            line = 44,
            r#"
    for ch in word:
        try:
            if ch == "d":
                break
        except* ValueError:
            pass
        print("char", ch)
    else:
        print("no d in", word)
"#
        )?;
    }
    m.set_global(
        "Point",
        rt::class!(m, "Point"(), point::body, line = 55, attributes = ["x"])?,
    )?;
    if m.global("__name__")?.eq("__main__")? {
        rt::builtins::print(py, ("main", m.global("mode")?))?;
        m.set_global("Local", rt::class!(m, "Local"(), local::body, line = 63)?)?;
        rt::builtins::print(
            py,
            (
                m.global("Local")?.attr("label")?,
                m.global("Point")?
                    .call1((double(m, &rt::int(py, 3))?,))?
                    .attr("x")?,
            ),
        )?;
        // mixed_nested.py:67, kept as Python: generator expression with several for clauses at 67:16
        rt::exec_python!(
            m,
            line = 67,
            r#"
    total = sum(j for j in range(5) for _ in "1")
"#
        )?;
        rt::builtins::print(py, ("total", m.global("total")?))?;
    }
    Ok(())
}
