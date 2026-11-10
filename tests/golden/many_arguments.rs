//! Parts of `many_arguments.py` kept as Python source and run in place by
//! `rt::exec_python!` (steelix does not lower them yet):
//!
//! - line 10, `def f`: call with more than 12 arguments
//! - line 22, `expr` statement: call with more than 12 arguments

use pyo3::prelude::*;
use pyo3::types::PyTuple;
use steelix_runtime as rt;

// many_arguments.py:6
pub fn show<'py>(m: &rt::Module<'py>, args: &Bound<'py, PyTuple>) -> PyResult<Bound<'py, PyAny>> {
    Ok(rt::int(m.py(), args.len()))
}

/// Executes the body of `many_arguments.py` in the module `m`.
#[rt::frame(m, "<module>", 1)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    m.set_global(
        "__doc__",
        "Calls with more than 12 positional arguments: PyO3's tuples stop at 12,\n\
            so the statement is kept as Python (design §13), also for the builtins that\n\
            are otherwise called directly (`print`, a raised builtin exception).",
    )?;
    rt::def!(m, show(*args))?;
    // many_arguments.py:10, kept as Python: call with more than 12 arguments at 12:5
    rt::exec_python!(
        m,
        line = 10,
        r#"
def f():
    print(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12)
    print(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13)
    print(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, sep="-")
    print(show(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14))
    try:
        raise ValueError(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13)
    except ValueError as e:
        print(len(e.args))
"#
    )?;
    m.global("f")?.call0()?;
    // many_arguments.py:22, kept as Python: call with more than 12 arguments at 22:1
    rt::exec_python!(
        m,
        line = 22,
        r#"
print(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13)
"#
    )?;
    Ok(())
}
