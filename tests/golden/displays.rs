use pyo3::prelude::*;
use pyo3::types::IntoPyDict;
use pyo3::types::PyDict;
use pyo3::types::PyList;
use pyo3::types::PySet;
use pyo3::types::PyTuple;
use steelix_runtime as rt;

// displays.py:6
pub fn build<'py>(
    py: Python<'py>,
    a: &Bound<'py, PyAny>,
    b: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let items = PyList::new(py, [a, b, &a.add(b)?])?.into_any();
    let pair = PyTuple::new(py, [a, b])?.into_any();
    let single = PyTuple::new(py, [a])?.into_any();
    let empty = PyTuple::empty(py).into_any();
    let lookup = [
        (&rt::str(py, "a"), a),
        (&rt::str(py, "b"), b),
        (a, &rt::str(py, "first")),
    ]
    .into_py_dict(py)?
    .into_any();
    let unique = PySet::new(py, [a, b, a])?.into_any();
    rt::builtins::print(
        py,
        (
            &items,
            &pair,
            &single,
            &empty,
            &lookup,
            rt::builtin(py, "sorted")?.call1((&unique,))?,
            PyList::empty(py),
            PyDict::new(py),
        ),
    )?;
    rt::builtins::print(
        py,
        (
            PyList::new(
                py,
                [
                    &rt::int(py, 1),
                    &rt::str(py, "two"),
                    &rt::float(py, 3.0),
                    &rt::none(py),
                    &rt::bool(py, true),
                ],
            )?,
            PyTuple::new(py, [1, 2])?,
            [(1, "one"), (2, "two")].into_py_dict(py)?,
            PySet::new(py, ["x"])?,
        ),
    )?;
    Ok(items)
}

// displays.py:18
pub fn subscripts<'py>(py: Python<'py>, items: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::builtins::print(
        py,
        (
            items.get_item(0)?,
            items.get_item(-1)?,
            items.get_item(rt::slice(py, 1, py.None())?)?,
            items.get_item(rt::slice(py, py.None(), 2)?)?,
            items.get_item(rt::slice_step(py, py.None(), py.None(), 2)?)?,
            items.get_item(rt::slice_step(py, py.None(), py.None(), -1)?)?,
            items.get_item(rt::slice_step(py, 1, -1, 1)?)?,
        ),
    )?;
    items.set_item(0, "zero")?;
    items.set_item(rt::slice(py, 1, 2)?, PyList::new(py, ["one", "uno"])?)?;
    items.del_item(-1)?;
    items.del_item(rt::slice(py, py.None(), 1)?)?;
    rt::builtins::print(py, (items,))?;
    let table = [("k", 1)].into_py_dict(py)?.into_any();
    table.set_item("k", table.get_item("k")?.add(1)?)?;
    table.set_item("new", PyList::empty(py))?;
    table.get_item("new")?.call_method1("append", (3,))?;
    table.del_item("k")?;
    rt::builtins::print(py, (&table,))?;
    let grid = PyDict::new(py).into_any();
    grid.set_item(PyTuple::new(py, [1, 2])?, "cell")?;
    rt::builtins::print(py, (grid.get_item(PyTuple::new(py, [1, 2])?)?, &grid))?;
    Ok(rt::none(py))
}

// displays.py:36
pub fn attributes<'py>(
    py: Python<'py>,
    globals: &Bound<'py, PyDict>,
) -> PyResult<Bound<'py, PyAny>> {
    let ns = rt::global(globals, "types")?.call_method0("SimpleNamespace")?;
    ns.setattr("x", 1)?;
    ns.setattr("y", ns.getattr("x")?.add(1)?)?;
    ns.setattr("x", ns.getattr("y")?.mul(10)?)?;
    rt::builtins::print(py, (ns.getattr("x")?, ns.getattr("y")?))?;
    ns.delattr("y")?;
    rt::builtins::print(py, (rt::builtin(py, "hasattr")?.call1((&ns, "y"))?, &ns))?;
    rt::builtins::print(
        py,
        (
            rt::str(py, "a b").call_method0("split")?,
            rt::str(py, "abc")
                .call_method0("upper")?
                .call_method0("lower")?,
        ),
    )?;
    Ok(rt::none(py))
}

// displays.py:47
pub fn deletes<'py>(py: Python<'py>, flag: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let mut value = Some(rt::int(py, 1));
    rt::builtins::print(py, (rt::local(&value, "value")?,))?;
    value = None;
    if flag.is_truthy()? {
        value = Some(rt::int(py, 2));
    }
    let mut other = Some(rt::int(py, 3));
    other = None;
    Ok(rt::local(&value, "value")?.clone())
}

// displays.py:58
pub fn slicer<'py>(
    py: Python<'py>,
    seq: &Bound<'py, PyAny>,
    start: &Bound<'py, PyAny>,
    stop: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    Ok(PyTuple::new(
        py,
        [
            &seq.get_item(rt::slice(py, start, stop)?)?,
            &seq.get_item(rt::slice(py, start, py.None())?)?,
            &seq.get_item(rt::slice(py, py.None(), stop)?)?,
        ],
    )?
    .into_any())
}

/// Executes the body of `displays.py` in the module `m`.
pub fn exec_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let globals = &m.dict();
    globals.set_item(
        "__doc__",
        "Lists, tuples, dicts and sets, subscripts, slices, attributes and del.",
    )?;
    globals.set_item("types", rt::import(globals, "types")?)?;
    rt::def!(globals, build(a, b))?;
    rt::def!(globals, subscripts(items))?;
    rt::def!(globals, attributes(globals))?;
    rt::def!(globals, deletes(flag))?;
    rt::def!(globals, slicer(seq, start, stop))?;
    build(py, &rt::int(py, 1), &rt::int(py, 2))?;
    subscripts(py, PyList::new(py, [1, 2, 3, 4, 5])?.as_any())?;
    attributes(py, globals)?;
    rt::builtins::print(py, (deletes(py, &rt::bool(py, true))?,))?;
    rt::builtins::print(
        py,
        (
            slicer(py, &rt::str(py, "abcdef"), &rt::int(py, 1), &rt::int(py, 3))?,
            slicer(
                py,
                PyList::new(py, [1, 2, 3])?.as_any(),
                &rt::int(py, -2),
                &rt::none(py),
            )?,
        ),
    )?;
    globals.set_item("glob", 5)?;
    rt::del_global(globals, "glob")?;
    rt::builtins::print(py, (deletes(py, &rt::bool(py, false))?,))?;
    Ok(())
}
