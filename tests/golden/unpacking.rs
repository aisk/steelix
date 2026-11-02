use pyo3::prelude::*;
use pyo3::types::IntoPyDict;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;

// unpacking.py:4
pub fn swap<'py>(
    py: Python<'py>,
    a: &Bound<'py, PyAny>,
    b: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let mut a = a.clone();
    let mut b = b.clone();
    [a, b] = rt::unpack(PyTuple::new(py, [&b, &a])?.as_any())?;
    Ok(PyTuple::new(py, [&a, &b])?.into_any())
}

// unpacking.py:9
pub fn unpack<'py>(py: Python<'py>, seq: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let x: Bound<'py, PyAny>;
    let y: Bound<'py, PyAny>;
    let z: Bound<'py, PyAny>;
    let [first, second] = rt::unpack(seq)?;
    rt::builtins::print(py, (&first, &second))?;
    let ([head], rest, []) = rt::unpack_star(PyList::new(py, [1, 2, 3, 4])?.as_any())?;
    let ([], init, [last]) = rt::unpack_star(&rt::str(py, "abc"))?;
    let ([left], middle, [right]) = rt::unpack_star(&rt::builtin(py, "range")?.call1((5,))?)?;
    rt::builtins::print(py, (&head, &rest, &init, &last, &left, &middle, &right))?;
    let [x2, items] = rt::unpack(
        PyTuple::new(py, [&rt::int(py, 1), PyList::new(py, [2, 3])?.as_any()])?.as_any(),
    )?;
    x = x2;
    [y, z] = rt::unpack(&items)?;
    rt::builtins::print(py, (&x, &y, &z))?;
    let [p, q] = rt::unpack([("p", 1), ("q", 2)].into_py_dict(py)?.as_any())?;
    rt::builtins::print(py, (&p, &q))?;
    Ok(first)
}

// unpacking.py:23
pub fn targets<'py>(py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
    let mut key: Option<Bound<'py, PyAny>> = None;
    let mut value: Bound<'py, PyAny>;
    let r#box = PyList::new(py, [0, 0, 0])?.into_any();
    let [item, item2] = rt::unpack(PyTuple::new(py, ["a", "c"])?.as_any())?;
    r#box.set_item(0, item)?;
    r#box.set_item(2, item2)?;
    let value2 = PyList::new(py, [5])?.into_any();
    let a = value2.clone();
    let b = value2.clone();
    a.call_method1("append", (6,))?;
    rt::builtins::print(py, (&r#box, &a, &b, a.is(&b)))?;
    let mut total = rt::int(py, 0);
    for item3 in rt::builtin(py, "enumerate")?
        .call1((PyList::new(
            py,
            [
                PyTuple::new(py, [1, 2])?.as_any(),
                PyTuple::new(py, [3, 4])?.as_any(),
            ],
        )?,))?
        .try_iter()?
    {
        let [i, items] = rt::unpack(&item3?)?;
        let [x, y] = rt::unpack(&items)?;
        total = total.add(i.mul(x.add(&y)?)?)?;
    }
    rt::builtins::print(py, (&total,))?;
    for item4 in [("k", "v")]
        .into_py_dict(py)?
        .call_method0("items")?
        .try_iter()?
    {
        let [key2, value3] = rt::unpack(&item4?)?;
        key = Some(key2);
        value = value3;
        rt::builtins::print(py, (rt::local(&key, "key")?, &value))?;
    }
    rt::builtins::print(py, (rt::local(&key, "key")?,))?;
    Ok(rt::none(py))
}

// unpacking.py:38
pub fn empty_star<'py>(py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
    let ([a], b, []) = rt::unpack_star(PyList::new(py, [1])?.as_any())?;
    let ([], c, [d]) = rt::unpack_star(PyList::new(py, [2])?.as_any())?;
    rt::builtins::print(py, (&a, &b, &c, &d))?;
    Ok(rt::none(py))
}

/// Executes the body of `unpacking.py` in the module `m`.
pub fn exec_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let globals = &m.dict();
    globals.set_item(
        "__doc__",
        "Unpacking assignment, also in for loops and with starred targets.",
    )?;
    rt::def!(globals, swap(a, b))?;
    rt::def!(globals, unpack(seq))?;
    rt::def!(globals, targets())?;
    rt::def!(globals, empty_star())?;
    rt::builtins::print(py, (swap(py, &rt::int(py, 1), &rt::int(py, 2))?,))?;
    rt::builtins::print(py, (unpack(py, PyTuple::new(py, [7, 8])?.as_any())?,))?;
    targets(py)?;
    empty_star(py)?;
    let [m, n] = rt::unpack(PyTuple::new(py, [10, 20])?.as_any())?;
    globals.set_item("m", m)?;
    globals.set_item("n", n)?;
    let [m2, n2] = rt::unpack(
        PyTuple::new(py, [&rt::global(globals, "n")?, &rt::global(globals, "m")?])?.as_any(),
    )?;
    globals.set_item("m", m2)?;
    globals.set_item("n", n2)?;
    rt::builtins::print(py, (rt::global(globals, "m")?, rt::global(globals, "n")?))?;
    let ([first], others, []) = rt::unpack_star(&rt::str(py, "xyz"))?;
    globals.set_item("first", first)?;
    globals.set_item("others", others)?;
    rt::builtins::print(
        py,
        (
            rt::global(globals, "first")?,
            rt::global(globals, "others")?,
        ),
    )?;
    for item in PyList::new(
        py,
        [
            PyTuple::new(py, [&rt::str(py, "one"), &rt::int(py, 1)])?.as_any(),
            PyTuple::new(py, [&rt::str(py, "two"), &rt::int(py, 2)])?.as_any(),
        ],
    )?
    .try_iter()?
    {
        let [k, v] = rt::unpack(&item?)?;
        globals.set_item("k", k)?;
        globals.set_item("v", v)?;
        rt::builtins::print(py, (rt::global(globals, "k")?, rt::global(globals, "v")?))?;
    }
    Ok(())
}
