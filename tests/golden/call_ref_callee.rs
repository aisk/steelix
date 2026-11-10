use pyo3::prelude::*;
use pyo3::types::IntoPyDict;
use pyo3::types::PyTuple;
use steelix_runtime as rt;

// call_ref_callee.py:5
#[rt::frame(m, "run", 5)]
pub fn run<'py>(
    m: &rt::Module<'py>,
    flag: &Bound<'py, PyAny>,
    f: &Bound<'py, PyAny>,
    args: &Bound<'py, PyAny>,
    kwargs: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let mut action: Option<Bound<'py, PyAny>> = None;
    if flag.is_truthy()? {
        action = Some(f.clone());
    }
    rt::call!(rt::local(&action, "action")?, *args, **kwargs)?;
    Ok(rt::none(m.py()))
}

/// Executes the body of `call_ref_callee.py` in the module `m`.
#[rt::frame(m, "<module>", 1)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    m.set_global(
        "__doc__",
        "A call with `*`/`**` of a local that may be unbound: the callee is a\n\
            reference returned by the unbound-local check.",
    )?;
    rt::def!(m, run(flag, f, args, kwargs))?;
    run(
        m,
        &rt::bool(py, true),
        &rt::builtin(py, "print")?,
        PyTuple::new(py, [1])?.as_any(),
        [("sep", ", ")].into_py_dict(py)?.as_any(),
    )?;
    Ok(())
}
