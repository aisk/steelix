use pyo3::prelude::*;
use pyo3::types::IntoPyDict;
use pyo3::types::PyDict;
use pyo3::types::PyList;
use pyo3::types::PySet;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// displays.py:6
#[rt::frame(m, "build", 6)]
pub fn build<'py>(
    m: &rt::Module<'py>,
    a: &Bound<'py, PyAny>,
    b: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
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
            rt::builtins::sorted(py, (&unique,))?,
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
#[rt::frame(m, "subscripts", 18)]
pub fn subscripts<'py>(
    m: &rt::Module<'py>,
    items: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
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
    table.get_item("new")?.method1("append", (3,))?;
    table.del_item("k")?;
    rt::builtins::print(py, (&table,))?;
    let grid = PyDict::new(py).into_any();
    grid.set_item(PyTuple::new(py, [1, 2])?, "cell")?;
    rt::builtins::print(py, (grid.get_item(PyTuple::new(py, [1, 2])?)?, &grid))?;
    Ok(rt::none(py))
}

// displays.py:36
#[rt::frame(m, "attributes", 36)]
pub fn attributes<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let ns = m.global("types")?.method0("SimpleNamespace")?;
    ns.set_attr("x", 1)?;
    ns.set_attr("y", ns.attr("x")?.add(1)?)?;
    ns.set_attr("x", ns.attr("y")?.mul(10)?)?;
    rt::builtins::print(py, (ns.attr("x")?, ns.attr("y")?))?;
    ns.del_attr("y")?;
    rt::builtins::print(py, (rt::builtins::hasattr(py, (&ns, "y"))?, &ns))?;
    rt::builtins::print(
        py,
        (
            rt::str(py, "a b").method0("split")?,
            rt::str(py, "abc").method0("upper")?.method0("lower")?,
        ),
    )?;
    Ok(rt::none(py))
}

// displays.py:47
#[rt::frame(m, "deletes", 47)]
pub fn deletes<'py>(m: &rt::Module<'py>, flag: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
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
#[rt::frame(m, "slicer", 58)]
pub fn slicer<'py>(
    m: &rt::Module<'py>,
    seq: &Bound<'py, PyAny>,
    start: &Bound<'py, PyAny>,
    stop: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
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
#[rt::frame(m, "<module>", 1)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    m.set_global(
        "__doc__",
        "Lists, tuples, dicts and sets, subscripts, slices, attributes and del.",
    )?;
    m.set_global("types", rt::import(m, "types")?)?;
    rt::def!(m, build(a, b))?;
    rt::def!(m, subscripts(items))?;
    rt::def!(m, attributes())?;
    rt::def!(m, deletes(flag))?;
    rt::def!(m, slicer(seq, start, stop))?;
    build(m, &rt::int(py, 1), &rt::int(py, 2))?;
    subscripts(m, PyList::new(py, [1, 2, 3, 4, 5])?.as_any())?;
    attributes(m)?;
    rt::builtins::print(py, (deletes(m, &rt::bool(py, true))?,))?;
    rt::builtins::print(
        py,
        (
            slicer(m, &rt::str(py, "abcdef"), &rt::int(py, 1), &rt::int(py, 3))?,
            slicer(
                m,
                PyList::new(py, [1, 2, 3])?.as_any(),
                &rt::int(py, -2),
                &rt::none(py),
            )?,
        ),
    )?;
    m.set_global("glob", 5)?;
    m.del_global("glob")?;
    rt::builtins::print(py, (deletes(m, &rt::bool(py, false))?,))?;
    Ok(())
}
