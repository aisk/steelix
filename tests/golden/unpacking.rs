use pyo3::prelude::*;
use pyo3::types::IntoPyDict;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;

// unpacking.py:4
pub fn swap<'py>(
    m: &rt::Module<'py>,
    a: &Bound<'py, PyAny>,
    b: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "swap", 4, |_| {
        let mut a = a.clone();
        let mut b = b.clone();
        [a, b] = [b.clone(), a.clone()];
        Ok(PyTuple::new(m.py(), [&a, &b])?.into_any())
    })
}

// unpacking.py:9
pub fn unpack<'py>(m: &rt::Module<'py>, seq: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "unpack", 9, |_| {
        let [first, second] = rt::unpack(seq)?;
        rt::builtins::print(py, (&first, &second))?;
        let ([head], rest, []) = rt::unpack_star(PyList::new(py, [1, 2, 3, 4])?.as_any())?;
        let ([], init, [last]) = rt::unpack_star(&rt::str(py, "abc"))?;
        let ([left], middle, [right]) = rt::unpack_star(&rt::builtin(py, "range")?.call1((5,))?)?;
        rt::builtins::print(py, (&head, &rest, &init, &last, &left, &middle, &right))?;
        let [x2, items] = [rt::int(py, 1), PyList::new(py, [2, 3])?.into_any()];
        let x = x2;
        let y: Bound<'py, PyAny>;
        let z: Bound<'py, PyAny>;
        [y, z] = rt::unpack(&items)?;
        rt::builtins::print(py, (&x, &y, &z))?;
        let [p, q] = rt::unpack([("p", 1), ("q", 2)].into_py_dict(py)?.as_any())?;
        rt::builtins::print(py, (&p, &q))?;
        Ok(first)
    })
}

// unpacking.py:23
pub fn targets<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "targets", 23, |_| {
        let r#box = PyList::new(py, [0, 0, 0])?.into_any();
        let [item, item2] = [rt::str(py, "a"), rt::str(py, "c")];
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
            py.check_signals()?;
        }
        rt::builtins::print(py, (&total,))?;
        let mut key: Option<Bound<'py, PyAny>> = None;
        let mut value: Bound<'py, PyAny>;
        for item4 in [("k", "v")]
            .into_py_dict(py)?
            .call_method0("items")?
            .try_iter()?
        {
            let [key2, value2] = rt::unpack(&item4?)?;
            key = Some(key2);
            value = value2;
            rt::builtins::print(py, (rt::local(&key, "key")?, &value))?;
            py.check_signals()?;
        }
        rt::builtins::print(py, (rt::local(&key, "key")?,))?;
        Ok(rt::none(py))
    })
}

// unpacking.py:38
pub fn empty_star<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "empty_star", 38, |_| {
        let ([a], b, []) = rt::unpack_star(PyList::new(py, [1])?.as_any())?;
        let ([], c, [d]) = rt::unpack_star(PyList::new(py, [2])?.as_any())?;
        rt::builtins::print(py, (&a, &b, &c, &d))?;
        Ok(rt::none(py))
    })
}

/// Executes the body of `unpacking.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |_| {
        m.set_global(
            "__doc__",
            "Unpacking assignment, also in for loops and with starred targets.",
        )?;
        rt::def!(m, swap(a, b))?;
        rt::def!(m, unpack(seq))?;
        rt::def!(m, targets())?;
        rt::def!(m, empty_star())?;
        rt::builtins::print(py, (swap(m, &rt::int(py, 1), &rt::int(py, 2))?,))?;
        rt::builtins::print(py, (unpack(m, PyTuple::new(py, [7, 8])?.as_any())?,))?;
        targets(m)?;
        empty_star(m)?;
        let [m2, n] = [rt::int(py, 10), rt::int(py, 20)];
        m.set_global("m", m2)?;
        m.set_global("n", n)?;
        let [m3, n2] = [m.global("n")?, m.global("m")?];
        m.set_global("m", m3)?;
        m.set_global("n", n2)?;
        rt::builtins::print(py, (m.global("m")?, m.global("n")?))?;
        let ([first], others, []) = rt::unpack_star(&rt::str(py, "xyz"))?;
        m.set_global("first", first)?;
        m.set_global("others", others)?;
        rt::builtins::print(py, (m.global("first")?, m.global("others")?))?;
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
            m.set_global("k", k)?;
            m.set_global("v", v)?;
            rt::builtins::print(py, (m.global("k")?, m.global("v")?))?;
            py.check_signals()?;
        }
        Ok(())
    })
}
