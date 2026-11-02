use pyo3::exceptions::PyUnboundLocalError;
use pyo3::prelude::*;
use pyo3::pyclass::CompareOp;
use pyo3::types::PyIterator;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;

// walrus.py:5
pub fn trace<'py>(
    m: &rt::Module<'py>,
    label: &Bound<'py, PyAny>,
    value: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "trace", 5, |frame| {
        frame.line(6);
        rt::builtins::print(m.py(), (label, value))?;
        Ok(value.clone())
    })
}

// walrus.py:10
pub fn in_condition<'py>(
    m: &rt::Module<'py>,
    values: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "in_condition", 10, |frame| {
        let n: Bound<'py, PyAny>;
        frame.line(11);
        n = rt::int(py, values.len()?);
        if n.gt(2)? {
            frame.line(12);
            return rt::fstring(py, ("long (", rt::format(&n, "")?, ")"));
        }
        frame.line(13);
        rt::fstring(py, ("short (", rt::format(&n, "")?, ")"))
    })
}

// walrus.py:16
pub fn in_while<'py>(
    m: &rt::Module<'py>,
    values: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "in_while", 16, |frame| {
        let mut item: Bound<'py, PyAny>;
        let out = PyList::empty(py).into_any();
        frame.line(18);
        let it = values.try_iter()?.into_any();
        loop {
            frame.line(19);
            item = rt::builtin(py, "next")?.call1((&it, py.None()))?;
            if item.is_none() {
                break;
            }
            frame.line(20);
            out.getattr("append")?.call1((&item,))?;
            py.check_signals()?;
        }
        frame.line(21);
        Ok(PyTuple::new(py, [&out, &item])?.into_any())
    })
}

// walrus.py:24
pub fn order<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "order", 24, |frame| {
        // The read of `y` before the assignment sees the old value.
        let mut y = rt::int(py, 1);
        frame.line(27);
        let y2 = y.clone();
        y = rt::int(py, 10);
        let total = y2.add(&y)?.add(&y)?;
        frame.line(28);
        Ok(PyTuple::new(py, [&total, &y])?.into_any())
    })
}

// walrus.py:31
pub fn in_comprehension<'py>(
    m: &rt::Module<'py>,
    values: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "in_comprehension", 31, |frame| {
        let mut last: Option<Bound<'py, PyAny>> = None;
        frame.line(32);
        let found = rt::Cell::new(py, "found", py.None())?;
        frame.line(33);
        // walrus.py:33
        fn genexpr<'py>(
            m: &rt::Module<'py>,
            found: &rt::Cell<'py>,
            values: &Bound<'py, PyIterator>,
        ) -> PyResult<Option<Bound<'py, PyAny>>> {
            rt::frame!(m, "in_comprehension.<locals>.<genexpr>", 33, |_| {
                if let Some(v) = rt::next(values)? {
                    found.set(&v)?;
                    return Ok(Some(v.rich_compare(10, CompareOp::Gt)?));
                }
                Ok(None)
            })
        }
        if rt::builtin(py, "any")?
            .call1((rt::genexpr!(
                m,
                genexpr[&found](values.try_iter()?),
                qualname = "in_comprehension.<locals>.<genexpr>"
            )?,))?
            .is_truthy()?
        {
            frame.line(34);
            rt::builtins::print(py, ("found", found.get()?))?;
        }
        frame.line(35);
        let list = PyList::empty(py);
        for v in values.try_iter()? {
            let v = v?;
            let last2 = v.mul(2)?;
            last = Some(last2.clone());
            list.append(&last2)?;
            py.check_signals()?;
        }
        let total = list.into_any();
        frame.line(36);
        Ok(PyTuple::new(py, [&total, rt::local(&last, "last")?, &found.get()?])?.into_any())
    })
}

// walrus.py:39
pub fn filtered<'py>(
    m: &rt::Module<'py>,
    values: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "filtered", 39, |frame| {
        let mut y: Option<Bound<'py, PyAny>> = None;
        frame.line(40);
        let list = PyList::empty(py);
        for x in values.try_iter()? {
            let x = x?;
            let y2 = x.mul(2)?;
            y = Some(y2.clone());
            if y2.gt(2)? {
                list.append(rt::local(&y, "y")?)?;
            }
            py.check_signals()?;
        }
        Ok(PyTuple::new(py, [list.as_any(), rt::local(&y, "y")?])?.into_any())
    })
}

// walrus.py:43
pub fn captured<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "captured", 43, |frame| {
        let value = rt::Cell::empty(m.py(), "value")?;
        // walrus.py:44
        fn get<'py>(m: &rt::Module<'py>, value: &rt::Cell<'py>) -> PyResult<Bound<'py, PyAny>> {
            rt::frame!(m, "captured.<locals>.get", 44, |frame| {
                frame.line(45);
                value.get()
            })
        }
        let get = rt::function!(m, get[&value](), qualname = "captured.<locals>.get")?;
        frame.line(47);
        value.set(0)?;
        frame.line(48);
        value.set(5)?;
        frame.line(49);
        get.call0()
    })
}

// walrus.py:52
pub fn maybe_unbound<'py>(
    m: &rt::Module<'py>,
    flag: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "maybe_unbound", 52, |frame| {
        let mut result: Option<Bound<'py, PyAny>> = None;
        frame.line(53);
        if flag.is_truthy()? {
            result = Some(rt::str(py, "set"));
        }
        frame.line(56);
        match (|| -> PyResult<Bound<'py, PyAny>> { Ok(rt::local(&result, "result")?.clone()) })() {
            Ok(value) => return Ok(value),
            Err(err) if err.is_instance_of::<PyUnboundLocalError>(py) => {
                let exc = frame.catch(err);
                let _handling = rt::Handling::new(&exc);
                frame.line(58);
                return exc.get_type().getattr("__name__");
            }
            Err(err) => return Err(err),
        }
    })
}

// walrus.py:64
pub fn global_walrus<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "global_walrus", 64, |frame| {
        frame.line(66);
        let counter = m.global("counter")?.add(1)?;
        m.set_global("counter", &counter)?;
        Ok(PyTuple::new(m.py(), [&counter, &m.global("counter")?])?.into_any())
    })
}

/// Executes the body of `walrus.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        m.set_global("__doc__", "Assignment expressions (`:=`), in comprehensions too, where they bind\nin the enclosing function or module (PEP 572).")?;
        rt::def!(m, trace(label, value))?;
        rt::def!(m, in_condition(values))?;
        rt::def!(m, in_while(values))?;
        rt::def!(m, order())?;
        rt::def!(m, in_comprehension(values))?;
        rt::def!(m, filtered(values))?;
        rt::def!(m, captured())?;
        rt::def!(m, maybe_unbound(flag))?;
        frame.line(61);
        m.set_global("counter", 0)?;
        rt::def!(m, global_walrus())?;
        frame.line(69);
        let list = PyList::empty(py);
        for v in PyTuple::new(py, [3, 7])?.try_iter()? {
            let v = v?;
            m.set_global("min", &v)?;
            list.append(&v)?;
            py.check_signals()?;
        }
        frame.line(70);
        rt::builtins::print(
            py,
            (
                in_condition(m, PyList::new(py, [1, 2, 3])?.as_any())?,
                in_condition(m, PyList::empty(py).as_any())?,
                in_while(m, PyList::new(py, [1, 2])?.as_any())?,
                order(m)?,
            ),
        )?;
        frame.line(71);
        rt::builtins::print(
            py,
            (
                in_comprehension(m, PyList::new(py, [1, 20, 3])?.as_any())?,
                filtered(m, PyList::new(py, [1, 2, 3])?.as_any())?,
                captured(m)?,
            ),
        )?;
        frame.line(72);
        rt::builtins::print(
            py,
            (
                maybe_unbound(m, &rt::bool(py, true))?,
                maybe_unbound(m, &rt::bool(py, false))?,
                global_walrus(m)?,
                global_walrus(m)?,
            ),
        )?;
        frame.line(73);
        let min = m.global("min")?;
        let z = trace(m, &rt::str(py, "z"), &rt::int(py, 3))?;
        m.set_global("z", &z)?;
        rt::builtins::print(py, (&min, z.add(m.global("z")?)?, m.global("z")?))?;
        frame.line(74);
        m.del_global("min")?;
        Ok(())
    })
}
