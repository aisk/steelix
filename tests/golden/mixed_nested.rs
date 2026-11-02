//! Parts of `mixed_nested.py` kept as Python source and run by
//! `rt::exec_python!` in `exec_module` (steelix does not lower them yet):
//!
//! - line 13, `def describe`: function definition outside module top level
//! - line 18, `def describe`: function definition outside module top level
//! - line 27, `assign` statement: list comprehension
//! - line 32, `while` statement: try statement
//! - line 42, `for` statement: try statement
//! - line 53, `class Point`: class definition
//! - line 61, `class Local`: class definition
//! - line 65, `assign` statement: generator expression

use pyo3::prelude::*;
use pyo3::types::PyList;
use steelix_runtime as rt;

// mixed_nested.py:6
pub fn double<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    x.mul(2)
}

/// Executes the body of `mixed_nested.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    // Module level `if`, `for` and `while` statements stay native when only some
    // statements in their bodies are kept as Python (design §13).
    m.set_global("sys", rt::import(m, "sys")?)?;
    rt::def!(m, double(x))?;
    if m.global("sys")?.getattr("argv")?.len()? > 5 {
        m.set_global("mode", "many")?;
        // A conditional definition is kept as Python, the `if` is native.
        // mixed_nested.py:13, kept as Python: function definition outside module top level at 13:5
        rt::exec_python!(
            m,
            line = 13,
            r#"
    def describe(n):
        return f"{n} (many)"
"#
        )?;
    } else {
        m.set_global("mode", "few")?;
        // mixed_nested.py:18, kept as Python: function definition outside module top level at 18:5
        rt::exec_python!(
            m,
            line = 18,
            r#"
    def describe(n):
        return f"{n} ({mode})"
"#
        )?;
        rt::builtins::print(py, ("native branch", m.global("mode")?))?;
    }
    for item in rt::builtin(py, "range")?.call1((4,))?.try_iter()? {
        m.set_global("i", item?)?;
        py.check_signals()?;
        if m.global("i")?.eq(1)? {
            continue;
        }
        rt::builtins::print(
            py,
            (m.global("describe")?
                .call1((double(m, &m.global("i")?)?,))?,),
        )?;
        // mixed_nested.py:27, kept as Python: list comprehension at 27:15
        rt::exec_python!(
            m,
            line = 27,
            r#"
    squares = [j * j for j in range(i)]  # kept as Python
"#
        )?;
        rt::builtins::print(py, ("squares", m.global("squares")?))?;
    }
    // `break` in a kept statement: the loop is kept whole.
    m.set_global("n", 0)?;
    // mixed_nested.py:32, kept as Python: try statement at 34:5
    rt::exec_python!(
        m,
        line = 32,
        r#"
while True:
    n += 1
    try:
        if n == 3:
            break
    finally:
        print("tried", n)
"#
    )?;
    // A kept loop inside a native one: its `break` is its own.
    for item2 in PyList::new(py, ["ab", "cde"])?.try_iter()? {
        m.set_global("word", item2?)?;
        py.check_signals()?;
        // mixed_nested.py:42, kept as Python: try statement at 43:9
        rt::exec_python!(
            m,
            line = 42,
            r#"
    for ch in word:
        try:
            if ch == "d":
                break
        finally:
            pass
        print("char", ch)
    else:
        print("no d in", word)
"#
        )?;
    }
    // mixed_nested.py:53, kept as Python: class definition at 53:1
    rt::exec_python!(
        m,
        line = 53,
        r#"
class Point:
    def __init__(self, x):
        self.x = x
"#
    )?;
    if m.global("__name__")?.eq("__main__")? {
        rt::builtins::print(py, ("main", m.global("mode")?))?;
        // mixed_nested.py:61, kept as Python: class definition at 61:5
        rt::exec_python!(
            m,
            line = 61,
            r#"
    class Local:
        label = "local class"
"#
        )?;
        rt::builtins::print(
            py,
            (
                m.global("Local")?.getattr("label")?,
                m.global("Point")?
                    .call1((double(m, &rt::int(py, 3))?,))?
                    .getattr("x")?,
            ),
        )?;
        // mixed_nested.py:65, kept as Python: generator expression at 65:16
        rt::exec_python!(
            m,
            line = 65,
            r#"
    total = sum(i for i in range(5))
"#
        )?;
        rt::builtins::print(py, ("total", m.global("total")?))?;
    }
    Ok(())
}
