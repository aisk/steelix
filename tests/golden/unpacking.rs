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
    let py = m.py();
    rt::frame!(m, "swap", 4, |frame| {
        let mut a = a.clone();
        let mut b = b.clone();
        frame.line(5);
        [a, b] = rt::unpack(PyTuple::new(py, [&b, &a])?.as_any())?;
        frame.line(6);
        Ok(PyTuple::new(py, [&a, &b])?.into_any())
    })
}

// unpacking.py:9
pub fn unpack<'py>(m: &rt::Module<'py>, seq: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "unpack", 9, |frame| {
        let x: Bound<'py, PyAny>;
        let y: Bound<'py, PyAny>;
        let z: Bound<'py, PyAny>;
        frame.line(10);
        let [first, second] = rt::unpack(seq)?;
        frame.line(11);
        rt::builtins::print(py, (&first, &second))?;
        frame.line(12);
        let ([head], rest, []) = rt::unpack_star(PyList::new(py, [1, 2, 3, 4])?.as_any())?;
        frame.line(13);
        let ([], init, [last]) = rt::unpack_star(&rt::str(py, "abc"))?;
        frame.line(14);
        let ([left], middle, [right]) = rt::unpack_star(&rt::builtin(py, "range")?.call1((5,))?)?;
        frame.line(15);
        rt::builtins::print(py, (&head, &rest, &init, &last, &left, &middle, &right))?;
        frame.line(16);
        let [x2, items] = rt::unpack(
            PyTuple::new(py, [&rt::int(py, 1), PyList::new(py, [2, 3])?.as_any()])?.as_any(),
        )?;
        x = x2;
        [y, z] = rt::unpack(&items)?;
        frame.line(17);
        rt::builtins::print(py, (&x, &y, &z))?;
        frame.line(18);
        let [p, q] = rt::unpack([("p", 1), ("q", 2)].into_py_dict(py)?.as_any())?;
        frame.line(19);
        rt::builtins::print(py, (&p, &q))?;
        Ok(first)
    })
}

// unpacking.py:23
pub fn targets<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "targets", 23, |frame| {
        let mut key: Option<Bound<'py, PyAny>> = None;
        let mut value: Bound<'py, PyAny>;
        frame.line(24);
        let r#box = PyList::new(py, [0, 0, 0])?.into_any();
        frame.line(25);
        let [item, item2] = rt::unpack(PyTuple::new(py, ["a", "c"])?.as_any())?;
        r#box.set_item(0, item)?;
        r#box.set_item(2, item2)?;
        frame.line(26);
        let value2 = PyList::new(py, [5])?.into_any();
        let a = value2.clone();
        let b = value2.clone();
        frame.line(27);
        a.call_method1("append", (6,))?;
        frame.line(28);
        rt::builtins::print(py, (&r#box, &a, &b, a.is(&b)))?;
        let mut total = rt::int(py, 0);
        frame.line(30);
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
            frame.line(31);
            total = total.add(i.mul(x.add(&y)?)?)?;
            py.check_signals()?;
            frame.line(30);
        }
        frame.line(32);
        rt::builtins::print(py, (&total,))?;
        frame.line(33);
        for item4 in [("k", "v")]
            .into_py_dict(py)?
            .call_method0("items")?
            .try_iter()?
        {
            let [key2, value2] = rt::unpack(&item4?)?;
            key = Some(key2);
            value = value2;
            frame.line(34);
            rt::builtins::print(py, (rt::local(&key, "key")?, &value))?;
            py.check_signals()?;
            frame.line(33);
        }
        frame.line(35);
        rt::builtins::print(py, (rt::local(&key, "key")?,))?;
        Ok(rt::none(py))
    })
}

// unpacking.py:38
pub fn empty_star<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "empty_star", 38, |frame| {
        frame.line(39);
        let ([a], b, []) = rt::unpack_star(PyList::new(py, [1])?.as_any())?;
        frame.line(40);
        let ([], c, [d]) = rt::unpack_star(PyList::new(py, [2])?.as_any())?;
        frame.line(41);
        rt::builtins::print(py, (&a, &b, &c, &d))?;
        Ok(rt::none(py))
    })
}

/// Executes the body of `unpacking.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        m.set_global(
            "__doc__",
            "Unpacking assignment, also in for loops and with starred targets.",
        )?;
        rt::def!(m, swap(a, b))?;
        rt::def!(m, unpack(seq))?;
        rt::def!(m, targets())?;
        rt::def!(m, empty_star())?;
        frame.line(44);
        rt::builtins::print(py, (swap(m, &rt::int(py, 1), &rt::int(py, 2))?,))?;
        frame.line(45);
        rt::builtins::print(py, (unpack(m, PyTuple::new(py, [7, 8])?.as_any())?,))?;
        frame.line(46);
        targets(m)?;
        frame.line(47);
        empty_star(m)?;
        frame.line(48);
        let [m2, n] = rt::unpack(PyTuple::new(py, [10, 20])?.as_any())?;
        m.set_global("m", m2)?;
        m.set_global("n", n)?;
        frame.line(49);
        let [m3, n2] = rt::unpack(PyTuple::new(py, [&m.global("n")?, &m.global("m")?])?.as_any())?;
        m.set_global("m", m3)?;
        m.set_global("n", n2)?;
        frame.line(50);
        rt::builtins::print(py, (m.global("m")?, m.global("n")?))?;
        frame.line(51);
        let ([first], others, []) = rt::unpack_star(&rt::str(py, "xyz"))?;
        m.set_global("first", first)?;
        m.set_global("others", others)?;
        frame.line(52);
        rt::builtins::print(py, (m.global("first")?, m.global("others")?))?;
        frame.line(53);
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
            frame.line(54);
            rt::builtins::print(py, (m.global("k")?, m.global("v")?))?;
            py.check_signals()?;
            frame.line(53);
        }
        Ok(())
    })
}
