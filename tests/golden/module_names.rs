use pyo3::prelude::*;
use pyo3::types::PyDict;
use steelix_runtime as rt;

// module_names.py:4
pub fn m_<'py>(py: Python<'py>, x: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    x.add(1)
}

// module_names.py:8
pub fn py_<'py>(py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
    Ok(rt::str(py, "py"))
}

// module_names.py:12
pub fn globals_<'py>(
    py: Python<'py>,
    a: &Bound<'py, PyAny>,
    b: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    a.mul(b)
}

// module_names.py:16
pub fn exec_module_<'py>(
    py: Python<'py>,
    globals: &Bound<'py, PyDict>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::global(globals, "m")?
        .call1((1,))?
        .add(rt::global(globals, "globals")?.call1((3,))?)
}

/// Executes the body of `module_names.py` in the module `m`.
pub fn exec_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let globals = &m.dict();
    globals.set_item(
        "__doc__",
        "Module level functions named like the locals of the generated module code.",
    )?;
    rt::def!(globals, m_(x), name = "m")?;
    rt::def!(globals, py_(), name = "py")?;
    rt::def!(globals, globals_(a, b = 2), name = "globals")?;
    rt::def!(globals, exec_module_(globals), name = "exec_module")?;
    rt::global(globals, "print")?.call1((
        rt::global(globals, "m")?.call1((1,))?,
        rt::global(globals, "py")?.call0()?,
        rt::global(globals, "globals")?.call1((4,))?,
        rt::global(globals, "exec_module")?.call0()?,
    ))?;
    globals.set_item("f", rt::global(globals, "m")?)?;
    rt::global(globals, "print")?.call1((
        rt::global(globals, "f")?.call1((10,))?,
        rt::global(globals, "m")?.getattr("__name__")?,
        rt::global(globals, "py")?.getattr("__name__")?,
        rt::global(globals, "exec_module")?.getattr("__name__")?,
    ))?;
    Ok(())
}
