use pyo3::prelude::*;
use pyo3::pyclass::CompareOp;
use pyo3::types::IntoPyDict;
use pyo3::types::PyDict;
use pyo3::types::PyList;
use pyo3::types::PySet;
use pyo3::types::PyTuple;
use steelix_runtime as rt;

// typed_receivers.py:8
pub fn sizes<'py>(
    m: &rt::Module<'py>,
    args: &Bound<'py, PyTuple>,
    kwargs: &Bound<'py, PyDict>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "sizes", 8, |_| {
        rt::builtins::print(
            py,
            (
                args.len(),
                kwargs.len(),
                args.as_any().get_item(-1)?,
                kwargs.as_any().get_item("k")?,
            ),
        )?;
        rt::builtins::print(
            py,
            (
                kwargs.contains("k")?,
                args.contains(1)?,
                args.as_any().get_item(rt::slice(py, 1, py.None())?)?,
            ),
        )?;
        Ok(rt::none(py))
    })
}

// typed_receivers.py:13
pub fn containers<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "containers", 13, |_| {
        rt::builtins::print(
            py,
            (
                PyList::new(py, [1, 2])?.len(),
                PyTuple::new(py, [1, 2, 3])?.len(),
                PySet::new(py, [1, 2])?.len(),
                [("a", 1)].into_py_dict(py)?.len(),
                PyTuple::empty(py).len(),
            ),
        )?;
        rt::builtins::print(
            py,
            (
                PyList::new(py, [1, 2, 3])?.as_any().get_item(-1)?,
                PyList::new(py, [1, 2, 3])?
                    .as_any()
                    .get_item(rt::slice(py, 1, py.None())?)?,
                PyTuple::new(py, [1, 2])?.as_any().get_item(0)?,
                [("a", 1)].into_py_dict(py)?.as_any().get_item("a")?,
            ),
        )?;
        rt::builtins::print(
            py,
            (
                PyList::new(py, [1, 2])?.contains(2)?,
                PyTuple::new(py, [1, 2])?.contains(3)?,
                [("a", 1)].into_py_dict(py)?.contains("a")?,
                PySet::new(py, [1])?.contains(1)?,
            ),
        )?;
        rt::builtins::print(
            py,
            (
                PyList::new(py, [1, 2])?.add(PyList::new(py, [3])?)?,
                PyTuple::new(py, [1])?.mul(2)?,
                PySet::new(py, [1, 2])?.bitor(PySet::new(py, [3])?)?,
                PySet::new(py, [1, 2])?.sub(PySet::new(py, [1])?)?,
            ),
        )?;
        if PyList::new(py, [1, 2])?.is_truthy()? {
            rt::builtins::print(py, ("non-empty list",))?;
        }
        if !PyDict::new(py).is_truthy()? {
            rt::builtins::print(py, ("empty dict",))?;
        }
        let xs = PyList::new(py, [1, 2, 3])?.into_any();
        if xs.try_iter()?.as_any().eq(xs.try_iter()?)? {
            rt::builtins::print(py, ("unreachable",))?;
        }
        rt::builtins::print(
            py,
            (
                xs.try_iter()?.rich_compare(xs.try_iter()?, CompareOp::Ne)?,
                rt::int(py, 1).str()?.rich_compare("1", CompareOp::Eq)?,
                rt::int(py, 1)
                    .get_type()
                    .rich_compare(rt::builtin(py, "int")?, CompareOp::Eq)?,
            ),
        )?;
        Ok(rt::none(py))
    })
}

// typed_receivers.py:28
pub fn lookup<'py>(m: &rt::Module<'py>, key: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "lookup", 28, |_| {
        rt::builtins::print(py, ([("a", 1)].into_py_dict(py)?.as_any().get_item("a")?,))?;
        rt::builtins::print(py, ([("a", 1)].into_py_dict(py)?.as_any().get_item(key)?,))?;
        rt::builtins::print(py, ("unreachable in CPython",))?;
        Ok(rt::none(py))
    })
}

/// Executes the body of `typed_receivers.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |_| {
        m.set_global("__doc__", "Methods on displays and `*args`/`**kwargs` behave like on any object.\n\nThe concrete PyO3 types have their own `get_item`, `len`, .. with other\nsemantics; a missing dict key must raise `KeyError`, not give `None`.\n")?;
        rt::def!(m, sizes(*args, **kwargs))?;
        rt::def!(m, containers())?;
        rt::def!(m, lookup(key))?;
        sizes(
            m,
            &PyTuple::new(py, [1, 2, 3])?,
            &[("k", "v")].into_py_dict(py)?,
        )?;
        containers(m)?;
        lookup(m, &rt::str(py, "b"))?;
        Ok(())
    })
}
