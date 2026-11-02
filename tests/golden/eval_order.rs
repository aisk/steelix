use pyo3::exceptions::PyAssertionError;
use pyo3::prelude::*;
use pyo3::pyclass::CompareOp;
use pyo3::types::IntoPyDict;
use pyo3::types::PyDict;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;

// eval_order.py:4
pub fn trace<'py>(
    py: Python<'py>,
    label: &Bound<'py, PyAny>,
    value: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::builtins::print(py, ("eval", label))?;
    Ok(value.clone())
}

// eval_order.py:9
pub fn make_box<'py>(py: Python<'py>, globals: &Bound<'py, PyDict>) -> PyResult<Bound<'py, PyAny>> {
    let types = rt::import(globals, "types")?;
    let r#box = types.call_method0("SimpleNamespace")?;
    r#box.setattr("x", 10)?;
    Ok(r#box)
}

// eval_order.py:16
pub fn subscripts<'py>(py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
    let data = PyList::new(py, [0, 0, 0])?.into_any();
    let value = trace(py, &rt::str(py, "value"), &rt::int(py, 5))?;
    data.set_item(trace(py, &rt::str(py, "index"), &rt::int(py, 1))?, &value)?;
    let value2 = trace(py, &rt::str(py, "value"), &rt::int(py, 7))?;
    trace(py, &rt::str(py, "target"), &data)?
        .set_item(trace(py, &rt::str(py, "index"), &rt::int(py, 2))?, &value2)?;
    rt::builtins::print(py, (&data,))?;
    rt::builtins::print(
        py,
        (trace(py, &rt::str(py, "container"), &data)?.get_item(trace(
            py,
            &rt::str(py, "index"),
            &rt::int(py, 0),
        )?)?,),
    )?;
    Ok(rt::none(py))
}

// eval_order.py:24
pub fn attributes<'py>(
    py: Python<'py>,
    globals: &Bound<'py, PyDict>,
) -> PyResult<Bound<'py, PyAny>> {
    let r#box = make_box(py, globals)?;
    let value = trace(py, &rt::str(py, "value"), &rt::int(py, 20))?;
    trace(py, &rt::str(py, "object"), &r#box)?.setattr("x", &value)?;
    rt::builtins::print(py, (r#box.getattr("x")?,))?;
    let target = trace(py, &rt::str(py, "object"), &r#box)?;
    target.setattr(
        "x",
        rt::iadd(
            &target.getattr("x")?,
            trace(py, &rt::str(py, "increment"), &rt::int(py, 1))?,
        )?,
    )?;
    rt::builtins::print(py, (r#box.getattr("x")?,))?;
    Ok(rt::none(py))
}

// eval_order.py:32
pub fn augmented<'py>(py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
    let data = PyList::new(py, [10, 20])?.into_any();
    let target = trace(py, &rt::str(py, "target"), &data)?;
    let index = trace(py, &rt::str(py, "index"), &rt::int(py, 0))?;
    target.set_item(
        &index,
        rt::iadd(
            &target.get_item(&index)?,
            trace(py, &rt::str(py, "value"), &rt::int(py, 5))?,
        )?,
    )?;
    rt::builtins::print(py, (&data,))?;
    let mut total = trace(py, &rt::str(py, "start"), &rt::int(py, 1))?;
    total = rt::iadd(&total, trace(py, &rt::str(py, "step"), &rt::int(py, 2))?)?;
    rt::builtins::print(py, (&total,))?;
    let mut items = PyList::new(py, [1])?.into_any();
    let alias = items.clone();
    items = rt::iadd(&items, PyList::new(py, [2])?)?;
    rt::builtins::print(py, (&alias, items.is(&alias)))?;
    Ok(rt::none(py))
}

// eval_order.py:45
pub fn membership<'py>(py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
    let item = trace(py, &rt::str(py, "item"), &rt::int(py, 2))?;
    rt::builtins::print(
        py,
        (trace(
            py,
            &rt::str(py, "container"),
            PyList::new(py, [1, 2])?.as_any(),
        )?
        .contains(&item)?,),
    )?;
    let item2 = trace(py, &rt::str(py, "item"), &rt::int(py, 5))?;
    rt::builtins::print(
        py,
        (!trace(
            py,
            &rt::str(py, "container"),
            PyList::new(py, [1, 2])?.as_any(),
        )?
        .contains(&item2)?,),
    )?;
    let item3 = trace(py, &rt::str(py, "needle"), &rt::str(py, "b"))?;
    if trace(py, &rt::str(py, "haystack"), &rt::str(py, "abc"))?.contains(&item3)? {
        rt::builtins::print(py, ("contained",))?;
    }
    Ok(rt::none(py))
}

// eval_order.py:52
pub fn comparisons<'py>(py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
    let lhs = trace(py, &rt::str(py, "a"), &rt::int(py, 1))?;
    let rhs = trace(py, &rt::str(py, "b"), &rt::int(py, 2))?;
    let result = lhs.rich_compare(&rhs, CompareOp::Lt)?;
    rt::builtins::print(
        py,
        (if result.is_truthy()? {
            rhs.rich_compare(
                trace(py, &rt::str(py, "c"), &rt::int(py, 3))?,
                CompareOp::Lt,
            )?
        } else {
            result
        },),
    )?;
    let lhs2 = trace(py, &rt::str(py, "a"), &rt::int(py, 3))?;
    let rhs2 = trace(py, &rt::str(py, "b"), &rt::int(py, 2))?;
    let result2 = lhs2.rich_compare(&rhs2, CompareOp::Lt)?;
    rt::builtins::print(
        py,
        (if result2.is_truthy()? {
            rhs2.rich_compare(
                trace(py, &rt::str(py, "c"), &rt::int(py, 1))?,
                CompareOp::Lt,
            )?
        } else {
            result2
        },),
    )?;
    let lhs3 = trace(py, &rt::str(py, "low"), &rt::int(py, 0))?;
    let rhs3 = trace(py, &rt::str(py, "mid"), &rt::int(py, 5))?;
    if lhs3.le(&rhs3)? && rhs3.le(trace(py, &rt::str(py, "high"), &rt::int(py, 10))?)? {
        rt::builtins::print(py, ("in range",))?;
    }
    rt::builtins::print(
        py,
        (trace(py, &rt::str(py, "x"), &rt::int(py, 1))?.rich_compare(
            trace(py, &rt::str(py, "y"), &rt::float(py, 1.0))?,
            CompareOp::Eq,
        )?,),
    )?;
    Ok(rt::none(py))
}

// eval_order.py:60
pub fn calls<'py>(py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
    let s = rt::str(py, "a,b");
    rt::builtins::print(
        py,
        (trace(py, &rt::str(py, "receiver"), &s)?
            .getattr("split")?
            .call1((trace(py, &rt::str(py, "separator"), &rt::str(py, ","))?,))?,),
    )?;
    rt::builtins::print(
        py,
        (
            trace(py, &rt::str(py, "arg"), PyList::new(py, [1, 2])?.as_any())?.len()?,
            rt::builtin(py, "max")?.call1((
                trace(py, &rt::str(py, "first"), &rt::int(py, 3))?,
                trace(py, &rt::str(py, "second"), &rt::int(py, 4))?,
            ))?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            PyList::new(
                py,
                [
                    &trace(py, &rt::str(py, "e1"), &rt::int(py, 1))?,
                    &trace(py, &rt::str(py, "e2"), &rt::int(py, 2))?,
                ],
            )?,
            [(
                &trace(py, &rt::str(py, "k"), &rt::str(py, "key"))?,
                &trace(py, &rt::str(py, "v"), &rt::str(py, "value"))?,
            )]
            .into_py_dict(py)?,
        ),
    )?;
    rt::builtins::print(
        py,
        (rt::fstring(
            py,
            (
                rt::format(&trace(py, &rt::str(py, "f1"), &rt::int(py, 1))?, "")?,
                "-",
                rt::format(
                    &trace(py, &rt::str(py, "f2"), &rt::int(py, 2))?,
                    rt::fstring(
                        py,
                        (
                            ">",
                            rt::format(&trace(py, &rt::str(py, "width"), &rt::int(py, 3))?, "")?,
                        ),
                    )?,
                )?,
            ),
        )?,),
    )?;
    Ok(rt::none(py))
}

// eval_order.py:68
pub fn boolean<'py>(py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
    let value = trace(py, &rt::str(py, "left"), &rt::int(py, 0))?;
    rt::builtins::print(
        py,
        (if value.is_truthy()? {
            value
        } else {
            trace(py, &rt::str(py, "right"), &rt::str(py, "fallback"))?
        },),
    )?;
    let value2 = trace(py, &rt::str(py, "left"), &rt::int(py, 1))?;
    rt::builtins::print(
        py,
        (if value2.is_truthy()? {
            trace(py, &rt::str(py, "right"), &rt::str(py, "second"))?
        } else {
            value2
        },),
    )?;
    let value3 = trace(py, &rt::str(py, "cond"), &rt::bool(py, true))?;
    let value4 = if value3.is_truthy()? {
        trace(py, &rt::str(py, "unused"), &rt::int(py, 0))?
    } else {
        value3
    };
    rt::builtins::print(
        py,
        (if value4.is_truthy()? {
            value4
        } else {
            trace(py, &rt::str(py, "default"), &rt::int(py, 9))?
        },),
    )?;
    rt::builtins::print(
        py,
        (
            if trace(py, &rt::str(py, "test"), &rt::int(py, 1))?.is_truthy()? {
                trace(py, &rt::str(py, "then"), &rt::str(py, "yes"))?
            } else {
                trace(py, &rt::str(py, "else"), &rt::str(py, "no"))?
            },
        ),
    )?;
    Ok(rt::none(py))
}

// eval_order.py:75
pub fn unpacking<'py>(py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
    let data = PyList::new(py, [0, 0])?.into_any();
    let [item, item2] = rt::unpack(&trace(
        py,
        &rt::str(py, "pair"),
        PyTuple::new(py, [1, 2])?.as_any(),
    )?)?;
    data.set_item(trace(py, &rt::str(py, "i"), &rt::int(py, 0))?, item)?;
    data.set_item(trace(py, &rt::str(py, "j"), &rt::int(py, 1))?, item2)?;
    rt::builtins::print(py, (&data,))?;
    Ok(rt::none(py))
}

// eval_order.py:81
pub fn tricky<'py>(py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
    let data = [("k", PyList::new(py, [0, 0])?.as_any())]
        .into_py_dict(py)?
        .into_any();
    let split = trace(py, &rt::str(py, "obj"), &rt::str(py, "a-b"))?.getattr("split")?;
    let item2 = trace(py, &rt::str(py, "sep"), &rt::str(py, "-"))?;
    rt::builtins::print(
        py,
        (
            split
                .call1((trace(py, &rt::str(py, "seps"), &rt::str(py, "-+"))?.contains(&item2)?,))?,
        ),
    )?;
    let value = trace(py, &rt::str(py, "first"), &rt::int(py, 1))?;
    let item3 = trace(py, &rt::str(py, "x"), &rt::int(py, 1))?;
    rt::builtins::print(
        py,
        (PyList::new(
            py,
            [
                &value,
                &rt::bool(
                    py,
                    trace(py, &rt::str(py, "xs"), PyList::new(py, [1])?.as_any())?
                        .contains(&item3)?,
                ),
            ],
        )?,),
    )?;
    let value2 = trace(py, &rt::str(py, "left"), &rt::int(py, 0))?;
    let value3 = if value2.is_truthy()? {
        value2
    } else {
        trace(py, &rt::str(py, "right"), PyList::new(py, [5, 6])?.as_any())?
    };
    data.set_item(trace(py, &rt::str(py, "key"), &rt::str(py, "k"))?, &value3)?;
    let value4 = trace(py, &rt::str(py, "value"), &rt::int(py, 7))?;
    data.get_item(trace(py, &rt::str(py, "outer"), &rt::str(py, "k"))?)?
        .set_item(trace(py, &rt::str(py, "inner"), &rt::int(py, 0))?, &value4)?;
    rt::builtins::print(py, (&data,))?;
    let lhs = trace(py, &rt::str(py, "a"), &rt::int(py, 1))?;
    let rhs = trace(py, &rt::str(py, "b"), &rt::int(py, 2))?;
    let result2 = lhs.rich_compare(&rhs, CompareOp::Lt)?;
    let result = if result2.is_truthy()? {
        rhs.rich_compare(
            trace(py, &rt::str(py, "c"), &rt::int(py, 3))?,
            CompareOp::Lt,
        )?
    } else {
        result2
    };
    let item4 = trace(py, &rt::str(py, "x"), &rt::int(py, 2))?;
    rt::builtins::print(
        py,
        (
            &result,
            if trace(py, &rt::str(py, "xs"), PyList::new(py, [2])?.as_any())?.contains(&item4)? {
                trace(py, &rt::str(py, "then"), &rt::int(py, 1))?
            } else {
                trace(py, &rt::str(py, "else"), &rt::int(py, 0))?
            },
        ),
    )?;
    let value5 = rt::format(&trace(py, &rt::str(py, "p1"), &rt::int(py, 1))?, "")?;
    let item5 = trace(py, &rt::str(py, "needle"), &rt::int(py, 2))?;
    rt::builtins::print(
        py,
        (rt::fstring(
            py,
            (
                &value5,
                rt::format(
                    &rt::bool(
                        py,
                        trace(py, &rt::str(py, "hay"), PyList::new(py, [2])?.as_any())?
                            .contains(&item5)?,
                    ),
                    "",
                )?,
            ),
        )?,),
    )?;
    let target = data.get_item(trace(py, &rt::str(py, "key"), &rt::str(py, "k"))?)?;
    let index = trace(py, &rt::str(py, "index"), &rt::int(py, 1))?;
    let item7 = target.get_item(&index)?;
    let item6 = trace(py, &rt::str(py, "x"), &rt::int(py, 1))?;
    target.set_item(
        &index,
        rt::iadd(
            &item7,
            trace(py, &rt::str(py, "xs"), PyList::new(py, [1])?.as_any())?.contains(&item6)?,
        )?,
    )?;
    rt::builtins::print(py, (&data,))?;
    let mut n = rt::int(py, 0);
    loop {
        let item8 = trace(py, &rt::str(py, "n"), &n)?;
        if !trace(
            py,
            &rt::str(py, "range"),
            PyList::new(py, [0, 1, 2])?.as_any(),
        )?
        .contains(&item8)?
        {
            break;
        }
        n = n.add(1)?;
    }
    let item9 = trace(py, &rt::str(py, "x"), &rt::int(py, 0))?;
    if trace(py, &rt::str(py, "xs"), PyList::empty(py).as_any())?.contains(&item9)? {
        rt::builtins::print(py, ("first",))?;
    } else {
        let item10 = trace(py, &rt::str(py, "y"), &rt::int(py, 1))?;
        if trace(py, &rt::str(py, "ys"), PyList::new(py, [1])?.as_any())?.contains(&item10)? {
            rt::builtins::print(py, ("second",))?;
        }
    }
    let item11 = trace(py, &rt::str(py, "x"), &rt::int(py, 1))?;
    if !trace(py, &rt::str(py, "xs"), PyList::new(py, [1])?.as_any())?.contains(&item11)? {
        return Err(PyAssertionError::new_err((trace(
            py,
            &rt::str(py, "message"),
            &rt::str(py, "never evaluated"),
        )?
        .unbind(),)));
    }
    data.get_item(trace(py, &rt::str(py, "del key"), &rt::str(py, "k"))?)?
        .del_item(trace(py, &rt::str(py, "del index"), &rt::int(py, 0))?)?;
    data.del_item(trace(py, &rt::str(py, "again"), &rt::str(py, "k"))?)?;
    rt::builtins::print(py, (&data, &n))?;
    for item in trace(
        py,
        &rt::str(py, "iterable"),
        PyList::new(py, [1, 2])?.as_any(),
    )?
    .try_iter()?
    {
        let item = item?;
        rt::builtins::print(py, ("item", &item))?;
    }
    Ok(rt::none(py))
}

// eval_order.py:107
pub fn bump<'py>(py: Python<'py>, globals: &Bound<'py, PyDict>) -> PyResult<Bound<'py, PyAny>> {
    globals.set_item("counter", 100)?;
    Ok(rt::int(py, 1))
}

/// Executes the body of `eval_order.py` in the module `m`.
pub fn exec_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let globals = &m.dict();
    globals.set_item(
        "__doc__",
        "Side effects must happen in Python's evaluation order.",
    )?;
    rt::def!(globals, trace(label, value))?;
    rt::def!(globals, make_box(globals))?;
    rt::def!(globals, subscripts())?;
    rt::def!(globals, attributes(globals))?;
    rt::def!(globals, augmented())?;
    rt::def!(globals, membership())?;
    rt::def!(globals, comparisons())?;
    rt::def!(globals, calls())?;
    rt::def!(globals, boolean())?;
    rt::def!(globals, unpacking())?;
    rt::def!(globals, tricky())?;
    rt::def!(globals, bump(globals))?;
    subscripts(py)?;
    attributes(py, globals)?;
    augmented(py)?;
    membership(py)?;
    comparisons(py)?;
    calls(py)?;
    boolean(py)?;
    unpacking(py)?;
    tricky(py)?;
    globals.set_item("counter", 5)?;
    globals.set_item(
        "counter",
        rt::iadd(&rt::global(globals, "counter")?, bump(py, globals)?)?,
    )?;
    rt::builtins::print(py, (rt::global(globals, "counter")?,))?;
    globals.set_item("counter", PyList::new(py, [0])?)?;
    let counter = rt::global(globals, "counter")?;
    let index = trace(py, &rt::str(py, "module index"), &rt::int(py, 0))?;
    counter.set_item(
        &index,
        rt::iadd(
            &counter.get_item(&index)?,
            trace(py, &rt::str(py, "module value"), &rt::int(py, 3))?,
        )?,
    )?;
    rt::builtins::print(py, (rt::global(globals, "counter")?,))?;
    Ok(())
}
