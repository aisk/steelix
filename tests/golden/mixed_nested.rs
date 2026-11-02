//! Parts of `mixed_nested.py` kept as Python source and run by
//! `rt::exec_python!` in `exec_module` (steelix does not lower them yet):
//!
//! - line 13, `def describe`: function definition outside module top level
//! - line 18, `def describe`: function definition outside module top level
//! - line 27, `assign` statement: list comprehension
//! - line 32, `while` statement: match statement
//! - line 41, `for` statement: match statement
//! - line 50, `class Point`: class definition
//! - line 58, `class Local`: class definition
//! - line 62, `assign` statement: generator expression

use pyo3::prelude::*;
use pyo3::types::PyList;
use steelix_runtime as rt;

// mixed_nested.py:6
pub fn double<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "double", 6, |frame| {
        frame.line(7);
        x.mul(2)
    })
}

/// Executes the body of `mixed_nested.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        // Module level `if`, `for` and `while` statements stay native when only some
        // statements in their bodies are kept as Python (design §13).
        frame.line(3);
        m.set_global("sys", rt::import(m, "sys")?)?;
        rt::def!(m, double(x))?;
        frame.line(10);
        if m.global("sys")?.getattr("argv")?.len()? > 5 {
            frame.line(11);
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
            frame.line(16);
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
            frame.line(21);
            rt::builtins::print(py, ("native branch", m.global("mode")?))?;
        }
        frame.line(23);
        for item in rt::builtin(py, "range")?.call1((4,))?.try_iter()? {
            m.set_global("i", item?)?;
            frame.line(24);
            if m.global("i")?.eq(1)? {
                py.check_signals()?;
                frame.line(23);
                continue;
            }
            frame.line(26);
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
            frame.line(28);
            rt::builtins::print(py, ("squares", m.global("squares")?))?;
            py.check_signals()?;
            frame.line(23);
        }
        // `break` in a kept statement: the loop is kept whole.
        frame.line(31);
        m.set_global("n", 0)?;
        // mixed_nested.py:32, kept as Python: match statement at 34:5
        rt::exec_python!(
            m,
            line = 32,
            r#"
while True:
    n += 1
    match n:
        case 3:
            break
    print("tried", n)
"#
        )?;
        // A kept loop inside a native one: its `break` is its own.
        frame.line(40);
        for item2 in PyList::new(py, ["ab", "cde"])?.try_iter()? {
            m.set_global("word", item2?)?;
            // mixed_nested.py:41, kept as Python: match statement at 42:9
            rt::exec_python!(
                m,
                line = 41,
                r#"
    for ch in word:
        match ch:
            case "d":
                break
        print("char", ch)
    else:
        print("no d in", word)
"#
            )?;
            py.check_signals()?;
        }
        // mixed_nested.py:50, kept as Python: class definition at 50:1
        rt::exec_python!(
            m,
            line = 50,
            r#"
class Point:
    def __init__(self, x):
        self.x = x
"#
        )?;
        frame.line(55);
        if m.global("__name__")?.eq("__main__")? {
            frame.line(56);
            rt::builtins::print(py, ("main", m.global("mode")?))?;
            // mixed_nested.py:58, kept as Python: class definition at 58:5
            rt::exec_python!(
                m,
                line = 58,
                r#"
    class Local:
        label = "local class"
"#
            )?;
            frame.line(61);
            rt::builtins::print(
                py,
                (
                    m.global("Local")?.getattr("label")?,
                    m.global("Point")?
                        .call1((double(m, &rt::int(py, 3))?,))?
                        .getattr("x")?,
                ),
            )?;
            // mixed_nested.py:62, kept as Python: generator expression at 62:16
            rt::exec_python!(
                m,
                line = 62,
                r#"
    total = sum(i for i in range(5))
"#
            )?;
            frame.line(63);
            rt::builtins::print(py, ("total", m.global("total")?))?;
        }
        Ok(())
    })
}
