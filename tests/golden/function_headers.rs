use pyo3::prelude::*;
use pyo3::pyclass::CompareOp;
use pyo3::types::IntoPyDict;
use steelix_runtime as rt;

// function_headers.py:6
pub fn report<'py>(m: &rt::Module<'py>, data: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    // function_headers.py:7
    fn lambda<'py>(
        m: &rt::Module<'py>,
        data: &Bound<'py, PyAny>,
        k: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        data.get_item(k)
    }
    for name in rt::call!(
        rt::builtin(py, "sorted")?,
        data,
        key = rt::function!(m, lambda[data](k), qualname = "report.<locals>.<lambda>")?
    )?
    .try_iter()?
    {
        let name = name?;
        rt::builtins::print(py, (&name, data.get_item(&name)?))?;
    }
    let n = rt::Cell::new(py, "n", 0)?;
    // function_headers.py:10
    fn lambda2<'py>(m: &rt::Module<'py>, n: &rt::Cell<'py>) -> PyResult<Bound<'py, PyAny>> {
        n.get()
    }
    while rt::function!(m, lambda2[&n](), qualname = "report.<locals>.<lambda>")?
        .call0()?
        .lt(2)?
    {
        n.set(rt::iadd(&n.get()?, 1)?)?;
    }
    // function_headers.py:12
    fn lambda3<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        x.rich_compare(3, CompareOp::Gt)
    }
    if rt::function!(m, lambda3(x), qualname = "report.<locals>.<lambda>")?
        .call1((data.get_item("a")?,))?
        .is_truthy()?
    {
        rt::builtins::print(py, ("big a",))?;
    } else {
        // function_headers.py:14
        fn lambda4<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
            Ok(x.clone())
        }
        if rt::function!(m, lambda4(x), qualname = "report.<locals>.<lambda>")?
            .call1((0,))?
            .is_truthy()?
        {
            rt::builtins::print(py, ("unreachable",))?;
        }
    }
    // function_headers.py:16
    fn lambda5<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
        Ok(rt::bool(m.py(), false))
    }
    if rt::function!(m, lambda5(), qualname = "report.<locals>.<lambda>")?
        .call0()?
        .is_truthy()?
    {
        rt::builtins::print(py, ("unreachable",))?;
    } else {
        // function_headers.py:18
        fn lambda6<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
            x.rich_compare(3, CompareOp::Lt)
        }
        if rt::function!(m, lambda6(x), qualname = "report.<locals>.<lambda>")?
            .call1((data.get_item("c")?,))?
            .is_truthy()?
        {
            rt::builtins::print(py, ("small c",))?;
        }
    }
    Ok(rt::none(py))
}

/// Executes the body of `function_headers.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    m.set_global(
        "__doc__",
        "Lambdas in the headers of `for`, `while`, `if` and `elif`.",
    )?;
    m.set_global("totals", [("b", 3), ("a", 5), ("c", 1)].into_py_dict(py)?)?;
    rt::def!(m, report(data))?;
    // function_headers.py:22
    fn lambda<'py>(m: &rt::Module<'py>, k: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        m.global("totals")?.get_item(k)?.neg()
    }
    for item in rt::call!(
        rt::builtin(py, "sorted")?,
        m.global("totals")?,
        key = rt::function!(m, lambda(k), qualname = "<lambda>")?
    )?
    .try_iter()?
    {
        m.set_global("name", item?)?;
        rt::builtins::print(py, (m.global("name")?,))?;
    }
    report(m, &m.global("totals")?)?;
    Ok(())
}
