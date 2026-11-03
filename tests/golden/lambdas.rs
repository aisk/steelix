use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// lambdas.py:7
pub fn make_adder<'py>(m: &rt::Module<'py>, n: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "make_adder", 7, |_| {
        // lambdas.py:8
        fn lambda<'py>(
            m: &rt::Module<'py>,
            n: &Bound<'py, PyAny>,
            x: &Bound<'py, PyAny>,
        ) -> PyResult<Bound<'py, PyAny>> {
            rt::frame!(m, "make_adder.<locals>.<lambda>", 8, |_| { x.add(n) })
        }
        rt::function!(m, lambda[n](x), qualname = "make_adder.<locals>.<lambda>")
    })
}

/// Executes the body of `lambdas.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |_| {
        m.set_global("__doc__", "Lambda expressions.")?;
        // lambdas.py:3
        fn lambda<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
            rt::frame!(m, "<lambda>", 3, |_| { x.mul(x) })
        }
        m.set_global(
            "square",
            rt::function!(m, lambda(x), qualname = "<lambda>")?,
        )?;
        // lambdas.py:4
        fn lambda2<'py>(
            m: &rt::Module<'py>,
            a: &Bound<'py, PyAny>,
            b: &Bound<'py, PyAny>,
            rest: &Bound<'py, PyTuple>,
            scale: &Bound<'py, PyAny>,
            extra: &Bound<'py, PyDict>,
        ) -> PyResult<Bound<'py, PyAny>> {
            rt::frame!(m, "<lambda>", 4, |_| {
                a.add(b)?.add(rest.len())?.add(extra.len())?.mul(scale)
            })
        }
        m.set_global(
            "add",
            rt::function!(
                m,
                lambda2(a, b = 10, *rest, scale = 1, **extra),
                qualname = "<lambda>"
            )?,
        )?;
        rt::def!(m, make_adder(n))?;
        m.set_global("words", PyList::new(py, ["banana", "Apple", "cherry"])?)?;
        rt::builtins::print(
            py,
            (
                m.global("square")?.call1((7,))?,
                m.global("add")?.call1((1,))?,
                rt::call!(m.global("add")?, 1, 2, 3, 4, scale = 2, k = 0)?,
            ),
        )?;
        // lambdas.py:13
        fn lambda3<'py>(m: &rt::Module<'py>, w: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
            rt::frame!(m, "<lambda>", 13, |_| { w.method0("lower") })
        }
        rt::builtins::print(
            py,
            (
                make_adder(m, &rt::int(py, 5))?.call1((10,))?,
                rt::call!(
                    rt::builtin(py, "sorted")?,
                    m.global("words")?,
                    key = rt::function!(m, lambda3(w), qualname = "<lambda>")?
                )?,
            ),
        )?;
        rt::builtins::print(
            py,
            (
                m.global("square")?.attr("__name__")?,
                m.global("square")?.attr("__qualname__")?,
                make_adder(m, &rt::int(py, 1))?.attr("__qualname__")?,
            ),
        )?;
        // lambdas.py:15
        fn lambda4<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
            Ok(rt::str(m.py(), "called at once"))
        }
        rt::builtins::print(
            py,
            (rt::function!(m, lambda4(), qualname = "<lambda>")?.call0()?,),
        )?;
        // lambdas.py:16
        fn lambda5<'py>(m: &rt::Module<'py>, p: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
            rt::frame!(m, "<lambda>", 16, |_| {
                p.get_item(0)?.mul(p.get_item(1)?)
            })
        }
        rt::builtins::print(
            py,
            (
                rt::builtin(py, "list")?.call1((rt::builtin(py, "map")?.call1((
                    rt::function!(m, lambda5(p), qualname = "<lambda>")?,
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
    })
}
