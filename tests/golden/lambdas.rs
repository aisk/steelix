use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;

// lambdas.py:7
pub fn make_adder<'py>(
    py: Python<'py>,
    globals: &Bound<'py, PyDict>,
    n: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    // lambdas.py:8
    fn lambda<'py>(
        py: Python<'py>,
        n: &Bound<'py, PyAny>,
        x: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        x.add(n)
    }
    rt::function!(
        globals,
        lambda[n](x),
        qualname = "make_adder.<locals>.<lambda>"
    )
}

/// Executes the body of `lambdas.py` in the module `m`.
pub fn exec_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let globals = &m.dict();
    globals.set_item("__doc__", "Lambda expressions.")?;
    // lambdas.py:3
    fn lambda<'py>(py: Python<'py>, x: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        x.mul(x)
    }
    globals.set_item(
        "square",
        rt::function!(globals, lambda(x), qualname = "<lambda>")?,
    )?;
    // lambdas.py:4
    fn lambda2<'py>(
        py: Python<'py>,
        a: &Bound<'py, PyAny>,
        b: &Bound<'py, PyAny>,
        rest: &Bound<'py, PyTuple>,
        scale: &Bound<'py, PyAny>,
        extra: &Bound<'py, PyDict>,
    ) -> PyResult<Bound<'py, PyAny>> {
        a.add(b)?
            .add(rest.as_any().len()?)?
            .add(extra.as_any().len()?)?
            .mul(scale)
    }
    globals.set_item(
        "add",
        rt::function!(
            globals,
            lambda2(a, b = 10, *rest, scale = 1, **extra),
            qualname = "<lambda>"
        )?,
    )?;
    rt::def!(globals, make_adder(globals, n))?;
    globals.set_item("words", PyList::new(py, ["banana", "Apple", "cherry"])?)?;
    rt::builtins::print(
        py,
        (
            rt::global(globals, "square")?.call1((7,))?,
            rt::global(globals, "add")?.call1((1,))?,
            rt::call!(rt::global(globals, "add")?, 1, 2, 3, 4, scale = 2, k = 0)?,
        ),
    )?;
    // lambdas.py:13
    fn lambda3<'py>(py: Python<'py>, w: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        w.call_method0("lower")
    }
    rt::builtins::print(
        py,
        (
            make_adder(py, globals, &rt::int(py, 5))?.call1((10,))?,
            rt::call!(
                rt::builtin(py, "sorted")?,
                rt::global(globals, "words")?,
                key = rt::function!(globals, lambda3(w), qualname = "<lambda>")?
            )?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            rt::global(globals, "square")?.getattr("__name__")?,
            rt::global(globals, "square")?.getattr("__qualname__")?,
            make_adder(py, globals, &rt::int(py, 1))?.getattr("__qualname__")?,
        ),
    )?;
    // lambdas.py:15
    fn lambda4<'py>(py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        Ok(rt::str(py, "called at once"))
    }
    rt::builtins::print(
        py,
        (rt::function!(globals, lambda4(), qualname = "<lambda>")?.call0()?,),
    )?;
    // lambdas.py:16
    fn lambda5<'py>(py: Python<'py>, p: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        p.get_item(0)?.mul(p.get_item(1)?)
    }
    rt::builtins::print(
        py,
        (
            rt::builtin(py, "list")?.call1((rt::builtin(py, "map")?.call1((
                rt::function!(globals, lambda5(p), qualname = "<lambda>")?,
                PyList::new(
                    py,
                    [
                        PyTuple::new(py, [1, 2])?.as_any(),
                        PyTuple::new(py, [3, 4])?.as_any(),
                    ],
                )?,
            ))?,))?,
        ),
    )?;
    Ok(())
}
