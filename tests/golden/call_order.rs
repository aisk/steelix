use pyo3::prelude::*;
use pyo3::types::IntoPyDict;
use pyo3::types::PyDict;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;

// call_order.py:4
pub fn show<'py>(
    m: &rt::Module<'py>,
    label: &Bound<'py, PyAny>,
    value: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "show", 4, |_| {
        rt::builtins::print(m.py(), ("eval", label))?;
        Ok(value.clone())
    })
}

// call_order.py:9
pub fn target<'py>(
    m: &rt::Module<'py>,
    args: &Bound<'py, PyTuple>,
    kwargs: &Bound<'py, PyDict>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "target", 9, |_| {
        Ok(PyList::new(m.py(), [args.as_any(), kwargs.as_any()])?.into_any())
    })
}

// call_order.py:13
pub fn fixed<'py>(
    m: &rt::Module<'py>,
    a: &Bound<'py, PyAny>,
    b: &Bound<'py, PyAny>,
    c: &Bound<'py, PyAny>,
    d: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "fixed", 13, |_| {
        Ok(PyList::new(m.py(), [a, b, c, d])?.into_any())
    })
}

/// Executes the body of `call_order.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |_| {
        m.set_global(
            "__doc__",
            "Evaluation order of call arguments with keywords, `*` and `**`.",
        )?;
        rt::def!(m, show(label, value = py.None()))?;
        rt::def!(m, target(*args, **kwargs))?;
        rt::def!(m, fixed(a, b, c = 3, *, d = 4))?;
        rt::builtins::print(
            py,
            (rt::call!(
                m.global("target")?,
                show(m, &rt::str(py, "a"), &rt::int(py, 1))?,
                *show(m, &rt::str(py, "star"), PyList::new(py, [2, 3])?.as_any())?,
                show(m, &rt::str(py, "b"), &rt::int(py, 4))?,
                k = show(m, &rt::str(py, "k"), &rt::int(py, 5))?,
                **show(m, &rt::str(py, "kw"), [("z", 6)].into_py_dict(py)?.as_any())?
            )?,),
        )?;
        rt::builtins::print(
            py,
            (rt::call!(
                m.global("target")?,
                show(m, &rt::str(py, "x"), &rt::int(py, 1))?,
                *show(m, &rt::str(py, "late star"), PyList::new(py, [3])?.as_any())?,
                y = show(m, &rt::str(py, "y"), &rt::int(py, 2))?
            )?,),
        )?;
        {
            let a = show(m, &rt::str(py, "1"), &rt::int(py, 1))?;
            let c = show(m, &rt::str(py, "c"), &rt::int(py, 30))?;
            rt::builtins::print(
                py,
                (fixed(
                    m,
                    &a,
                    &show(m, &rt::str(py, "b"), &rt::int(py, 20))?,
                    &c,
                    &rt::int(py, 4),
                )?,),
            )?;
        }
        rt::builtins::print(
            py,
            (rt::call!(
                m.global("fixed")?,
                *show(m, &rt::str(py, "args"), PyList::new(py, [1, 2])?.as_any())?,
                **show(
                    m,
                    &rt::str(py, "kwargs"),
                    [("d", 40)].into_py_dict(py)?.as_any()
                )?
            )?,),
        )?;
        rt::builtins::print(
            py,
            (
                show(m, &rt::str(py, "callee"), &m.global("fixed")?)?.call1((
                    show(m, &rt::str(py, "arg"), &rt::int(py, 1))?,
                    show(m, &rt::str(py, "arg"), &rt::int(py, 2))?,
                ))?,
            ),
        )?;
        rt::builtins::print(
            py,
            (
                rt::call!(
                    m.global("target")?,
                    **[("a", 1)].into_py_dict(py)?,
                    **[("b", 2)].into_py_dict(py)?
                )?,
                rt::call!(
                    m.global("target")?,
                    *rt::str(py, "ab"),
                    *PyTuple::new(py, [1, 2])?
                )?,
            ),
        )?;
        Ok(())
    })
}
