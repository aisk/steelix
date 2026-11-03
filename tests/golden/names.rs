use pyo3::prelude::*;
use steelix_runtime as rt;

// names.py:4
pub fn r#type<'py>(
    m: &rt::Module<'py>,
    r#match: &Bound<'py, PyAny>,
    r#loop: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "type", 4, |_| {
        let r#fn = r#match.add(r#loop)?;
        Ok(r#fn)
    })
}

// names.py:9
pub fn r#mod<'py>(
    m: &rt::Module<'py>,
    py_: &Bound<'py, PyAny>,
    globals: &Bound<'py, PyAny>,
    slf: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "mod", 9, |_| {
        let r#let = py_.mul(100)?.add(globals.mul(10)?)?.add(slf)?;
        Ok(r#let)
    })
}

// names.py:14
pub fn exec_module_<'py>(
    m: &rt::Module<'py>,
    Some_: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "exec_module", 14, |_| { Some_.add(1) })
}

// names.py:18
pub fn maybe<'py>(m: &rt::Module<'py>, flag: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "maybe", 18, |_| {
        let mut value: Option<Bound<'py, PyAny>> = None;
        if flag.is_truthy()? {
            value = Some(rt::str(m.py(), "set"));
        }
        Ok(rt::local(&value, "value")?.clone())
    })
}

/// Executes the body of `names.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |_| {
        m.set_global(
            "__doc__",
            "Python names that are Rust keywords or names the generated code uses.",
        )?;
        rt::def!(m, r#type(r#match, r#loop))?;
        rt::def!(m, r#mod(py_ as "py", globals, slf as "self"))?;
        rt::def!(m, exec_module_(Some_ as "Some"), name = "exec_module")?;
        rt::def!(m, maybe(flag))?;
        rt::builtins::print(
            py,
            (
                r#type(m, &rt::int(py, 1), &rt::int(py, 2))?,
                r#mod(m, &rt::int(py, 1), &rt::int(py, 2), &rt::int(py, 3))?,
                exec_module_(m, &rt::int(py, 41))?,
            ),
        )?;
        rt::builtins::print(py, (maybe(m, &rt::bool(py, true))?,))?;
        m.set_global("alias", m.global("type")?)?;
        rt::builtins::print(py, (m.global("alias")?.call1((3, 4))?,))?;
        Ok(())
    })
}
