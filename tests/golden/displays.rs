use pyo3::prelude::*;
use pyo3::types::IntoPyDict;
use pyo3::types::PyDict;
use pyo3::types::PyList;
use pyo3::types::PySet;
use pyo3::types::PyTuple;
use steelix_runtime as rt;

// displays.py:6
pub fn build<'py>(
    m: &rt::Module<'py>,
    a: &Bound<'py, PyAny>,
    b: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "build", 6, |frame| {
        frame.line(7);
        let items = PyList::new(py, [a, b, &a.add(b)?])?.into_any();
        frame.line(8);
        let pair = PyTuple::new(py, [a, b])?.into_any();
        frame.line(9);
        let single = PyTuple::new(py, [a])?.into_any();
        let empty = PyTuple::empty(py).into_any();
        frame.line(11);
        let lookup = [
            (&rt::str(py, "a"), a),
            (&rt::str(py, "b"), b),
            (a, &rt::str(py, "first")),
        ]
        .into_py_dict(py)?
        .into_any();
        frame.line(12);
        let unique = PySet::new(py, [a, b, a])?.into_any();
        frame.line(13);
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
        frame.line(14);
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
    })
}

// displays.py:18
pub fn subscripts<'py>(
    m: &rt::Module<'py>,
    items: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "subscripts", 18, |frame| {
        frame.line(19);
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
        frame.line(20);
        items.set_item(0, "zero")?;
        frame.line(21);
        items.set_item(rt::slice(py, 1, 2)?, PyList::new(py, ["one", "uno"])?)?;
        frame.line(22);
        items.del_item(-1)?;
        frame.line(23);
        items.del_item(rt::slice(py, py.None(), 1)?)?;
        frame.line(24);
        rt::builtins::print(py, (items,))?;
        frame.line(25);
        let table = [("k", 1)].into_py_dict(py)?.into_any();
        frame.line(26);
        table.set_item("k", table.get_item("k")?.add(1)?)?;
        frame.line(27);
        table.set_item("new", PyList::empty(py))?;
        frame.line(28);
        table.get_item("new")?.call_method1("append", (3,))?;
        frame.line(29);
        table.del_item("k")?;
        frame.line(30);
        rt::builtins::print(py, (&table,))?;
        let grid = PyDict::new(py).into_any();
        frame.line(32);
        grid.set_item(PyTuple::new(py, [1, 2])?, "cell")?;
        frame.line(33);
        rt::builtins::print(py, (grid.get_item(PyTuple::new(py, [1, 2])?)?, &grid))?;
        Ok(rt::none(py))
    })
}

// displays.py:36
pub fn attributes<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "attributes", 36, |frame| {
        frame.line(37);
        let ns = m.global("types")?.call_method0("SimpleNamespace")?;
        frame.line(38);
        ns.setattr("x", 1)?;
        frame.line(39);
        ns.setattr("y", ns.getattr("x")?.add(1)?)?;
        frame.line(40);
        ns.setattr("x", ns.getattr("y")?.mul(10)?)?;
        frame.line(41);
        rt::builtins::print(py, (ns.getattr("x")?, ns.getattr("y")?))?;
        frame.line(42);
        ns.delattr("y")?;
        frame.line(43);
        rt::builtins::print(py, (rt::builtin(py, "hasattr")?.call1((&ns, "y"))?, &ns))?;
        frame.line(44);
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
    })
}

// displays.py:47
pub fn deletes<'py>(m: &rt::Module<'py>, flag: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "deletes", 47, |frame| {
        let mut value = Some(rt::int(py, 1));
        frame.line(49);
        rt::builtins::print(py, (rt::local(&value, "value")?,))?;
        value = None;
        frame.line(51);
        if flag.is_truthy()? {
            value = Some(rt::int(py, 2));
        }
        let mut other = Some(rt::int(py, 3));
        other = None;
        frame.line(55);
        Ok(rt::local(&value, "value")?.clone())
    })
}

// displays.py:58
pub fn slicer<'py>(
    m: &rt::Module<'py>,
    seq: &Bound<'py, PyAny>,
    start: &Bound<'py, PyAny>,
    stop: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "slicer", 58, |frame| {
        frame.line(59);
        Ok(PyTuple::new(
            py,
            [
                &seq.get_item(rt::slice(py, start, stop)?)?,
                &seq.get_item(rt::slice(py, start, py.None())?)?,
                &seq.get_item(rt::slice(py, py.None(), stop)?)?,
            ],
        )?
        .into_any())
    })
}

/// Executes the body of `displays.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        m.set_global(
            "__doc__",
            "Lists, tuples, dicts and sets, subscripts, slices, attributes and del.",
        )?;
        frame.line(3);
        m.set_global("types", rt::import(m, "types")?)?;
        rt::def!(m, build(a, b))?;
        rt::def!(m, subscripts(items))?;
        rt::def!(m, attributes())?;
        rt::def!(m, deletes(flag))?;
        rt::def!(m, slicer(seq, start, stop))?;
        frame.line(62);
        build(m, &rt::int(py, 1), &rt::int(py, 2))?;
        frame.line(63);
        subscripts(m, PyList::new(py, [1, 2, 3, 4, 5])?.as_any())?;
        frame.line(64);
        attributes(m)?;
        frame.line(65);
        rt::builtins::print(py, (deletes(m, &rt::bool(py, true))?,))?;
        frame.line(66);
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
        frame.line(67);
        m.set_global("glob", 5)?;
        frame.line(68);
        m.del_global("glob")?;
        frame.line(69);
        rt::builtins::print(py, (deletes(m, &rt::bool(py, false))?,))?;
        Ok(())
    })
}
