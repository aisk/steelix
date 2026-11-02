use pyo3::exceptions::PyAssertionError;
use pyo3::prelude::*;
use pyo3::pyclass::CompareOp;
use pyo3::types::IntoPyDict;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;

// eval_order.py:4
pub fn trace<'py>(
    m: &rt::Module<'py>,
    label: &Bound<'py, PyAny>,
    value: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "trace", 4, |frame| {
        frame.line(5);
        rt::builtins::print(m.py(), ("eval", label))?;
        Ok(value.clone())
    })
}

// eval_order.py:9
pub fn make_box<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "make_box", 9, |frame| {
        frame.line(10);
        let types = rt::import(m, "types")?;
        frame.line(11);
        let r#box = types.call_method0("SimpleNamespace")?;
        frame.line(12);
        r#box.setattr("x", 10)?;
        Ok(r#box)
    })
}

// eval_order.py:16
pub fn subscripts<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "subscripts", 16, |frame| {
        frame.line(17);
        let data = PyList::new(py, [0, 0, 0])?.into_any();
        frame.line(18);
        let value = trace(m, &rt::str(py, "value"), &rt::int(py, 5))?;
        data.set_item(trace(m, &rt::str(py, "index"), &rt::int(py, 1))?, &value)?;
        frame.line(19);
        let value2 = trace(m, &rt::str(py, "value"), &rt::int(py, 7))?;
        trace(m, &rt::str(py, "target"), &data)?
            .set_item(trace(m, &rt::str(py, "index"), &rt::int(py, 2))?, &value2)?;
        frame.line(20);
        rt::builtins::print(py, (&data,))?;
        frame.line(21);
        rt::builtins::print(
            py,
            (trace(m, &rt::str(py, "container"), &data)?.get_item(trace(
                m,
                &rt::str(py, "index"),
                &rt::int(py, 0),
            )?)?,),
        )?;
        Ok(rt::none(py))
    })
}

// eval_order.py:24
pub fn attributes<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "attributes", 24, |frame| {
        frame.line(25);
        let r#box = make_box(m)?;
        frame.line(26);
        let value = trace(m, &rt::str(py, "value"), &rt::int(py, 20))?;
        trace(m, &rt::str(py, "object"), &r#box)?.setattr("x", &value)?;
        frame.line(27);
        rt::builtins::print(py, (r#box.getattr("x")?,))?;
        frame.line(28);
        let target = trace(m, &rt::str(py, "object"), &r#box)?;
        target.setattr(
            "x",
            rt::iadd(
                &target.getattr("x")?,
                trace(m, &rt::str(py, "increment"), &rt::int(py, 1))?,
            )?,
        )?;
        frame.line(29);
        rt::builtins::print(py, (r#box.getattr("x")?,))?;
        Ok(rt::none(py))
    })
}

// eval_order.py:32
pub fn augmented<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "augmented", 32, |frame| {
        frame.line(33);
        let data = PyList::new(py, [10, 20])?.into_any();
        frame.line(34);
        let target = trace(m, &rt::str(py, "target"), &data)?;
        let index = trace(m, &rt::str(py, "index"), &rt::int(py, 0))?;
        target.set_item(
            &index,
            rt::iadd(
                &target.get_item(&index)?,
                trace(m, &rt::str(py, "value"), &rt::int(py, 5))?,
            )?,
        )?;
        frame.line(35);
        rt::builtins::print(py, (&data,))?;
        frame.line(36);
        let mut total = trace(m, &rt::str(py, "start"), &rt::int(py, 1))?;
        frame.line(37);
        total = rt::iadd(&total, trace(m, &rt::str(py, "step"), &rt::int(py, 2))?)?;
        frame.line(38);
        rt::builtins::print(py, (&total,))?;
        frame.line(39);
        let mut items = PyList::new(py, [1])?.into_any();
        let alias = items.clone();
        frame.line(41);
        items = rt::iadd(&items, PyList::new(py, [2])?)?;
        frame.line(42);
        rt::builtins::print(py, (&alias, items.is(&alias)))?;
        Ok(rt::none(py))
    })
}

// eval_order.py:45
pub fn membership<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "membership", 45, |frame| {
        frame.line(46);
        let item = trace(m, &rt::str(py, "item"), &rt::int(py, 2))?;
        rt::builtins::print(
            py,
            (trace(
                m,
                &rt::str(py, "container"),
                PyList::new(py, [1, 2])?.as_any(),
            )?
            .contains(&item)?,),
        )?;
        frame.line(47);
        let item2 = trace(m, &rt::str(py, "item"), &rt::int(py, 5))?;
        rt::builtins::print(
            py,
            (!trace(
                m,
                &rt::str(py, "container"),
                PyList::new(py, [1, 2])?.as_any(),
            )?
            .contains(&item2)?,),
        )?;
        frame.line(48);
        let item3 = trace(m, &rt::str(py, "needle"), &rt::str(py, "b"))?;
        if trace(m, &rt::str(py, "haystack"), &rt::str(py, "abc"))?.contains(&item3)? {
            frame.line(49);
            rt::builtins::print(py, ("contained",))?;
        }
        Ok(rt::none(py))
    })
}

// eval_order.py:52
pub fn comparisons<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "comparisons", 52, |frame| {
        frame.line(53);
        let lhs = trace(m, &rt::str(py, "a"), &rt::int(py, 1))?;
        let rhs = trace(m, &rt::str(py, "b"), &rt::int(py, 2))?;
        let result = lhs.rich_compare(&rhs, CompareOp::Lt)?;
        rt::builtins::print(
            py,
            (if result.is_truthy()? {
                rhs.rich_compare(trace(m, &rt::str(py, "c"), &rt::int(py, 3))?, CompareOp::Lt)?
            } else {
                result
            },),
        )?;
        frame.line(54);
        let lhs2 = trace(m, &rt::str(py, "a"), &rt::int(py, 3))?;
        let rhs2 = trace(m, &rt::str(py, "b"), &rt::int(py, 2))?;
        let result2 = lhs2.rich_compare(&rhs2, CompareOp::Lt)?;
        rt::builtins::print(
            py,
            (if result2.is_truthy()? {
                rhs2.rich_compare(trace(m, &rt::str(py, "c"), &rt::int(py, 1))?, CompareOp::Lt)?
            } else {
                result2
            },),
        )?;
        frame.line(55);
        let lhs3 = trace(m, &rt::str(py, "low"), &rt::int(py, 0))?;
        let rhs3 = trace(m, &rt::str(py, "mid"), &rt::int(py, 5))?;
        if lhs3.le(&rhs3)? && rhs3.le(trace(m, &rt::str(py, "high"), &rt::int(py, 10))?)? {
            frame.line(56);
            rt::builtins::print(py, ("in range",))?;
        }
        frame.line(57);
        rt::builtins::print(
            py,
            (trace(m, &rt::str(py, "x"), &rt::int(py, 1))?.rich_compare(
                trace(m, &rt::str(py, "y"), &rt::float(py, 1.0))?,
                CompareOp::Eq,
            )?,),
        )?;
        Ok(rt::none(py))
    })
}

// eval_order.py:60
pub fn calls<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "calls", 60, |frame| {
        let s = rt::str(py, "a,b");
        frame.line(62);
        rt::builtins::print(
            py,
            (trace(m, &rt::str(py, "receiver"), &s)?
                .getattr("split")?
                .call1((trace(m, &rt::str(py, "separator"), &rt::str(py, ","))?,))?,),
        )?;
        frame.line(63);
        rt::builtins::print(
            py,
            (
                trace(m, &rt::str(py, "arg"), PyList::new(py, [1, 2])?.as_any())?.len()?,
                rt::builtin(py, "max")?.call1((
                    trace(m, &rt::str(py, "first"), &rt::int(py, 3))?,
                    trace(m, &rt::str(py, "second"), &rt::int(py, 4))?,
                ))?,
            ),
        )?;
        frame.line(64);
        rt::builtins::print(
            py,
            (
                PyList::new(
                    py,
                    [
                        &trace(m, &rt::str(py, "e1"), &rt::int(py, 1))?,
                        &trace(m, &rt::str(py, "e2"), &rt::int(py, 2))?,
                    ],
                )?,
                [(
                    &trace(m, &rt::str(py, "k"), &rt::str(py, "key"))?,
                    &trace(m, &rt::str(py, "v"), &rt::str(py, "value"))?,
                )]
                .into_py_dict(py)?,
            ),
        )?;
        frame.line(65);
        rt::builtins::print(
            py,
            (rt::fstring(
                py,
                (
                    rt::format(&trace(m, &rt::str(py, "f1"), &rt::int(py, 1))?, "")?,
                    "-",
                    rt::format(
                        &trace(m, &rt::str(py, "f2"), &rt::int(py, 2))?,
                        rt::fstring(
                            py,
                            (
                                ">",
                                rt::format(&trace(m, &rt::str(py, "width"), &rt::int(py, 3))?, "")?,
                            ),
                        )?,
                    )?,
                ),
            )?,),
        )?;
        Ok(rt::none(py))
    })
}

// eval_order.py:68
pub fn boolean<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "boolean", 68, |frame| {
        frame.line(69);
        let value = trace(m, &rt::str(py, "left"), &rt::int(py, 0))?;
        rt::builtins::print(
            py,
            (if value.is_truthy()? {
                value
            } else {
                trace(m, &rt::str(py, "right"), &rt::str(py, "fallback"))?
            },),
        )?;
        frame.line(70);
        let value2 = trace(m, &rt::str(py, "left"), &rt::int(py, 1))?;
        rt::builtins::print(
            py,
            (if value2.is_truthy()? {
                trace(m, &rt::str(py, "right"), &rt::str(py, "second"))?
            } else {
                value2
            },),
        )?;
        frame.line(71);
        let value3 = trace(m, &rt::str(py, "cond"), &rt::bool(py, true))?;
        let value4 = if value3.is_truthy()? {
            trace(m, &rt::str(py, "unused"), &rt::int(py, 0))?
        } else {
            value3
        };
        rt::builtins::print(
            py,
            (if value4.is_truthy()? {
                value4
            } else {
                trace(m, &rt::str(py, "default"), &rt::int(py, 9))?
            },),
        )?;
        frame.line(72);
        rt::builtins::print(
            py,
            (
                if trace(m, &rt::str(py, "test"), &rt::int(py, 1))?.is_truthy()? {
                    trace(m, &rt::str(py, "then"), &rt::str(py, "yes"))?
                } else {
                    trace(m, &rt::str(py, "else"), &rt::str(py, "no"))?
                },
            ),
        )?;
        Ok(rt::none(py))
    })
}

// eval_order.py:75
pub fn unpacking<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "unpacking", 75, |frame| {
        frame.line(76);
        let data = PyList::new(py, [0, 0])?.into_any();
        frame.line(77);
        let [item, item2] = rt::unpack(&trace(
            m,
            &rt::str(py, "pair"),
            PyTuple::new(py, [1, 2])?.as_any(),
        )?)?;
        data.set_item(trace(m, &rt::str(py, "i"), &rt::int(py, 0))?, item)?;
        data.set_item(trace(m, &rt::str(py, "j"), &rt::int(py, 1))?, item2)?;
        frame.line(78);
        rt::builtins::print(py, (&data,))?;
        Ok(rt::none(py))
    })
}

// eval_order.py:81
pub fn tricky<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "tricky", 81, |frame| {
        frame.line(82);
        let data = [("k", PyList::new(py, [0, 0])?.as_any())]
            .into_py_dict(py)?
            .into_any();
        frame.line(83);
        let split = trace(m, &rt::str(py, "obj"), &rt::str(py, "a-b"))?.getattr("split")?;
        let item2 = trace(m, &rt::str(py, "sep"), &rt::str(py, "-"))?;
        rt::builtins::print(
            py,
            (split
                .call1((trace(m, &rt::str(py, "seps"), &rt::str(py, "-+"))?.contains(&item2)?,))?,),
        )?;
        frame.line(84);
        let value = trace(m, &rt::str(py, "first"), &rt::int(py, 1))?;
        let item3 = trace(m, &rt::str(py, "x"), &rt::int(py, 1))?;
        rt::builtins::print(
            py,
            (PyList::new(
                py,
                [
                    &value,
                    &rt::bool(
                        py,
                        trace(m, &rt::str(py, "xs"), PyList::new(py, [1])?.as_any())?
                            .contains(&item3)?,
                    ),
                ],
            )?,),
        )?;
        frame.line(85);
        let value2 = trace(m, &rt::str(py, "left"), &rt::int(py, 0))?;
        let value3 = if value2.is_truthy()? {
            value2
        } else {
            trace(m, &rt::str(py, "right"), PyList::new(py, [5, 6])?.as_any())?
        };
        data.set_item(trace(m, &rt::str(py, "key"), &rt::str(py, "k"))?, &value3)?;
        frame.line(86);
        let value4 = trace(m, &rt::str(py, "value"), &rt::int(py, 7))?;
        data.get_item(trace(m, &rt::str(py, "outer"), &rt::str(py, "k"))?)?
            .set_item(trace(m, &rt::str(py, "inner"), &rt::int(py, 0))?, &value4)?;
        frame.line(87);
        rt::builtins::print(py, (&data,))?;
        frame.line(88);
        let lhs = trace(m, &rt::str(py, "a"), &rt::int(py, 1))?;
        let rhs = trace(m, &rt::str(py, "b"), &rt::int(py, 2))?;
        let result2 = lhs.rich_compare(&rhs, CompareOp::Lt)?;
        let result = if result2.is_truthy()? {
            rhs.rich_compare(trace(m, &rt::str(py, "c"), &rt::int(py, 3))?, CompareOp::Lt)?
        } else {
            result2
        };
        frame.line(89);
        let item4 = trace(m, &rt::str(py, "x"), &rt::int(py, 2))?;
        rt::builtins::print(
            py,
            (
                &result,
                if trace(m, &rt::str(py, "xs"), PyList::new(py, [2])?.as_any())?.contains(&item4)? {
                    trace(m, &rt::str(py, "then"), &rt::int(py, 1))?
                } else {
                    trace(m, &rt::str(py, "else"), &rt::int(py, 0))?
                },
            ),
        )?;
        frame.line(90);
        let value5 = rt::format(&trace(m, &rt::str(py, "p1"), &rt::int(py, 1))?, "")?;
        let item5 = trace(m, &rt::str(py, "needle"), &rt::int(py, 2))?;
        rt::builtins::print(
            py,
            (rt::fstring(
                py,
                (
                    &value5,
                    rt::format(
                        &rt::bool(
                            py,
                            trace(m, &rt::str(py, "hay"), PyList::new(py, [2])?.as_any())?
                                .contains(&item5)?,
                        ),
                        "",
                    )?,
                ),
            )?,),
        )?;
        frame.line(91);
        let target = data.get_item(trace(m, &rt::str(py, "key"), &rt::str(py, "k"))?)?;
        let index = trace(m, &rt::str(py, "index"), &rt::int(py, 1))?;
        let item7 = target.get_item(&index)?;
        let item6 = trace(m, &rt::str(py, "x"), &rt::int(py, 1))?;
        target.set_item(
            &index,
            rt::iadd(
                &item7,
                trace(m, &rt::str(py, "xs"), PyList::new(py, [1])?.as_any())?.contains(&item6)?,
            )?,
        )?;
        frame.line(92);
        rt::builtins::print(py, (&data,))?;
        let mut n = rt::int(py, 0);
        loop {
            frame.line(94);
            let item8 = trace(m, &rt::str(py, "n"), &n)?;
            if !trace(
                m,
                &rt::str(py, "range"),
                PyList::new(py, [0, 1, 2])?.as_any(),
            )?
            .contains(&item8)?
            {
                break;
            }
            frame.line(95);
            n = n.add(1)?;
            py.check_signals()?;
        }
        frame.line(96);
        let item9 = trace(m, &rt::str(py, "x"), &rt::int(py, 0))?;
        if trace(m, &rt::str(py, "xs"), PyList::empty(py).as_any())?.contains(&item9)? {
            frame.line(97);
            rt::builtins::print(py, ("first",))?;
        } else {
            frame.line(98);
            let item10 = trace(m, &rt::str(py, "y"), &rt::int(py, 1))?;
            if trace(m, &rt::str(py, "ys"), PyList::new(py, [1])?.as_any())?.contains(&item10)? {
                frame.line(99);
                rt::builtins::print(py, ("second",))?;
            }
        }
        frame.line(100);
        let item11 = trace(m, &rt::str(py, "x"), &rt::int(py, 1))?;
        if !trace(m, &rt::str(py, "xs"), PyList::new(py, [1])?.as_any())?.contains(&item11)? {
            return Err(frame.raise(PyAssertionError::new_err((trace(
                m,
                &rt::str(py, "message"),
                &rt::str(py, "never evaluated"),
            )?
            .unbind(),))));
        }
        frame.line(101);
        data.get_item(trace(m, &rt::str(py, "del key"), &rt::str(py, "k"))?)?
            .del_item(trace(m, &rt::str(py, "del index"), &rt::int(py, 0))?)?;
        data.del_item(trace(m, &rt::str(py, "again"), &rt::str(py, "k"))?)?;
        frame.line(102);
        rt::builtins::print(py, (&data, &n))?;
        frame.line(103);
        for item in trace(
            m,
            &rt::str(py, "iterable"),
            PyList::new(py, [1, 2])?.as_any(),
        )?
        .try_iter()?
        {
            let item = item?;
            frame.line(104);
            rt::builtins::print(py, ("item", &item))?;
            py.check_signals()?;
            frame.line(103);
        }
        Ok(rt::none(py))
    })
}

// eval_order.py:107
pub fn bump<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "bump", 107, |frame| {
        frame.line(109);
        m.set_global("counter", 100)?;
        Ok(rt::int(m.py(), 1))
    })
}

/// Executes the body of `eval_order.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        m.set_global(
            "__doc__",
            "Side effects must happen in Python's evaluation order.",
        )?;
        rt::def!(m, trace(label, value))?;
        rt::def!(m, make_box())?;
        rt::def!(m, subscripts())?;
        rt::def!(m, attributes())?;
        rt::def!(m, augmented())?;
        rt::def!(m, membership())?;
        rt::def!(m, comparisons())?;
        rt::def!(m, calls())?;
        rt::def!(m, boolean())?;
        rt::def!(m, unpacking())?;
        rt::def!(m, tricky())?;
        rt::def!(m, bump())?;
        frame.line(113);
        subscripts(m)?;
        frame.line(114);
        attributes(m)?;
        frame.line(115);
        augmented(m)?;
        frame.line(116);
        membership(m)?;
        frame.line(117);
        comparisons(m)?;
        frame.line(118);
        calls(m)?;
        frame.line(119);
        boolean(m)?;
        frame.line(120);
        unpacking(m)?;
        frame.line(121);
        tricky(m)?;
        frame.line(122);
        m.set_global("counter", 5)?;
        frame.line(123);
        m.set_global("counter", rt::iadd(&m.global("counter")?, bump(m)?)?)?;
        frame.line(124);
        rt::builtins::print(py, (m.global("counter")?,))?;
        frame.line(125);
        m.set_global("counter", PyList::new(py, [0])?)?;
        frame.line(126);
        let counter = m.global("counter")?;
        let index = trace(m, &rt::str(py, "module index"), &rt::int(py, 0))?;
        counter.set_item(
            &index,
            rt::iadd(
                &counter.get_item(&index)?,
                trace(m, &rt::str(py, "module value"), &rt::int(py, 3))?,
            )?,
        )?;
        frame.line(127);
        rt::builtins::print(py, (m.global("counter")?,))?;
        Ok(())
    })
}
