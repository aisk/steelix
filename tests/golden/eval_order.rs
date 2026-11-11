use pyo3::exceptions::PyAssertionError;
use pyo3::prelude::*;
use pyo3::pyclass::CompareOp;
use pyo3::types::IntoPyDict;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// eval_order.py:4
#[rt::frame(m, "trace", 4)]
pub fn trace<'py>(
    m: &rt::Module<'py>,
    label: &Bound<'py, PyAny>,
    value: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::builtins::print(m.py(), ("eval", label))?;
    Ok(value.clone())
}

// eval_order.py:9
#[rt::frame(m, "make_box", 9)]
pub fn make_box<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let types = rt::import(m, "types")?;
    let r#box = types.method0("SimpleNamespace")?;
    r#box.set_attr("x", 10)?;
    Ok(r#box)
}

// eval_order.py:16
#[rt::frame(m, "subscripts", 16)]
pub fn subscripts<'py>(m: &rt::Module<'py>) -> PyResult<()> {
    let py = m.py();
    let data = PyList::new(py, [0, 0, 0])?.into_any();
    let value = trace(m, &rt::str(py, "value"), &rt::int(py, 5))?;
    data.set_item(trace(m, &rt::str(py, "index"), &rt::int(py, 1))?, value)?;
    let value = trace(m, &rt::str(py, "value"), &rt::int(py, 7))?;
    trace(m, &rt::str(py, "target"), &data)?
        .set_item(trace(m, &rt::str(py, "index"), &rt::int(py, 2))?, value)?;
    rt::builtins::print(py, (&data,))?;
    rt::builtins::print(
        py,
        (trace(m, &rt::str(py, "container"), &data)?.get_item(trace(
            m,
            &rt::str(py, "index"),
            &rt::int(py, 0),
        )?)?,),
    )?;
    Ok(())
}

// eval_order.py:24
#[rt::frame(m, "attributes", 24)]
pub fn attributes<'py>(m: &rt::Module<'py>) -> PyResult<()> {
    let py = m.py();
    let r#box = make_box(m)?;
    let value = trace(m, &rt::str(py, "value"), &rt::int(py, 20))?;
    trace(m, &rt::str(py, "object"), &r#box)?.set_attr("x", value)?;
    rt::builtins::print(py, (r#box.attr("x")?,))?;
    {
        let target = trace(m, &rt::str(py, "object"), &r#box)?;
        target.set_attr(
            "x",
            rt::iadd(
                &target.attr("x")?,
                trace(m, &rt::str(py, "increment"), &rt::int(py, 1))?,
            )?,
        )?;
    }
    rt::builtins::print(py, (r#box.attr("x")?,))?;
    Ok(())
}

// eval_order.py:32
#[rt::frame(m, "augmented", 32)]
pub fn augmented<'py>(m: &rt::Module<'py>) -> PyResult<()> {
    let py = m.py();
    let data = PyList::new(py, [10, 20])?;
    {
        let target = trace(m, &rt::str(py, "target"), &data)?;
        let index = trace(m, &rt::str(py, "index"), &rt::int(py, 0))?;
        target.set_item(
            &index,
            rt::iadd(
                &target.get_item(&index)?,
                trace(m, &rt::str(py, "value"), &rt::int(py, 5))?,
            )?,
        )?;
    }
    rt::builtins::print(py, (&data,))?;
    let mut total = trace(m, &rt::str(py, "start"), &rt::int(py, 1))?;
    total = rt::iadd(&total, trace(m, &rt::str(py, "step"), &rt::int(py, 2))?)?;
    rt::builtins::print(py, (&total,))?;
    let mut items = PyList::new(py, [1])?.into_any();
    let alias = items.clone();
    items = rt::iadd(&items, PyList::new(py, [2])?)?;
    rt::builtins::print(py, (&alias, items.is(&alias)))?;
    Ok(())
}

// eval_order.py:45
#[rt::frame(m, "membership", 45)]
pub fn membership<'py>(m: &rt::Module<'py>) -> PyResult<()> {
    let py = m.py();
    let item = trace(m, &rt::str(py, "item"), &rt::int(py, 2))?;
    rt::builtins::print(
        py,
        (trace(
            m,
            &rt::str(py, "container"),
            PyList::new(py, [1, 2])?.as_any(),
        )?
        .contains(item)?,),
    )?;
    let item = trace(m, &rt::str(py, "item"), &rt::int(py, 5))?;
    rt::builtins::print(
        py,
        (!trace(
            m,
            &rt::str(py, "container"),
            PyList::new(py, [1, 2])?.as_any(),
        )?
        .contains(item)?,),
    )?;
    let item = trace(m, &rt::str(py, "needle"), &rt::str(py, "b"))?;
    if trace(m, &rt::str(py, "haystack"), &rt::str(py, "abc"))?.contains(item)? {
        rt::builtins::print(py, ("contained",))?;
    }
    Ok(())
}

// eval_order.py:52
#[rt::frame(m, "comparisons", 52)]
pub fn comparisons<'py>(m: &rt::Module<'py>) -> PyResult<()> {
    let py = m.py();
    {
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
    }
    {
        let lhs = trace(m, &rt::str(py, "a"), &rt::int(py, 3))?;
        let rhs = trace(m, &rt::str(py, "b"), &rt::int(py, 2))?;
        let result = lhs.rich_compare(&rhs, CompareOp::Lt)?;
        rt::builtins::print(
            py,
            (if result.is_truthy()? {
                rhs.rich_compare(trace(m, &rt::str(py, "c"), &rt::int(py, 1))?, CompareOp::Lt)?
            } else {
                result
            },),
        )?;
    }
    {
        let lhs = trace(m, &rt::str(py, "low"), &rt::int(py, 0))?;
        let rhs = trace(m, &rt::str(py, "mid"), &rt::int(py, 5))?;
        if lhs.le(&rhs)? && rhs.le(trace(m, &rt::str(py, "high"), &rt::int(py, 10))?)? {
            rt::builtins::print(py, ("in range",))?;
        }
    }
    rt::builtins::print(
        py,
        (trace(m, &rt::str(py, "x"), &rt::int(py, 1))?.rich_compare(
            trace(m, &rt::str(py, "y"), &rt::float(py, 1.0))?,
            CompareOp::Eq,
        )?,),
    )?;
    Ok(())
}

// eval_order.py:60
#[rt::frame(m, "calls", 60)]
pub fn calls<'py>(m: &rt::Module<'py>) -> PyResult<()> {
    let py = m.py();
    let s = rt::str(py, "a,b");
    rt::builtins::print(
        py,
        (trace(m, &rt::str(py, "receiver"), &s)?
            .attr("split")?
            .call1((trace(m, &rt::str(py, "separator"), &rt::str(py, ","))?,))?,),
    )?;
    rt::builtins::print(
        py,
        (
            trace(m, &rt::str(py, "arg"), PyList::new(py, [1, 2])?.as_any())?.len()?,
            rt::builtins::max(
                py,
                (
                    trace(m, &rt::str(py, "first"), &rt::int(py, 3))?,
                    trace(m, &rt::str(py, "second"), &rt::int(py, 4))?,
                ),
            )?,
        ),
    )?;
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
    Ok(())
}

// eval_order.py:68
#[rt::frame(m, "boolean", 68)]
pub fn boolean<'py>(m: &rt::Module<'py>) -> PyResult<()> {
    let py = m.py();
    {
        let value = trace(m, &rt::str(py, "left"), &rt::int(py, 0))?;
        rt::builtins::print(
            py,
            (if value.is_truthy()? {
                value
            } else {
                trace(m, &rt::str(py, "right"), &rt::str(py, "fallback"))?
            },),
        )?;
    }
    {
        let value = trace(m, &rt::str(py, "left"), &rt::int(py, 1))?;
        rt::builtins::print(
            py,
            (if value.is_truthy()? {
                trace(m, &rt::str(py, "right"), &rt::str(py, "second"))?
            } else {
                value
            },),
        )?;
    }
    {
        let value = trace(m, &rt::str(py, "cond"), &rt::bool(py, true))?;
        let value2 = if value.is_truthy()? {
            trace(m, &rt::str(py, "unused"), &rt::int(py, 0))?
        } else {
            value
        };
        rt::builtins::print(
            py,
            (if value2.is_truthy()? {
                value2
            } else {
                trace(m, &rt::str(py, "default"), &rt::int(py, 9))?
            },),
        )?;
    }
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
    Ok(())
}

// eval_order.py:75
#[rt::frame(m, "unpacking", 75)]
pub fn unpacking<'py>(m: &rt::Module<'py>) -> PyResult<()> {
    let py = m.py();
    let data = PyList::new(py, [0, 0])?.into_any();
    let [item, item2] = rt::unpack(&trace(
        m,
        &rt::str(py, "pair"),
        PyTuple::new(py, [1, 2])?.as_any(),
    )?)?;
    data.set_item(trace(m, &rt::str(py, "i"), &rt::int(py, 0))?, item)?;
    data.set_item(trace(m, &rt::str(py, "j"), &rt::int(py, 1))?, item2)?;
    rt::builtins::print(py, (&data,))?;
    Ok(())
}

// eval_order.py:81
#[rt::frame(m, "tricky", 81, frame)]
pub fn tricky<'py>(m: &rt::Module<'py>) -> PyResult<()> {
    let py = m.py();
    let data = [("k", PyList::new(py, [0, 0])?.as_any())]
        .into_py_dict(py)?
        .into_any();
    {
        let split = trace(m, &rt::str(py, "obj"), &rt::str(py, "a-b"))?.attr("split")?;
        let item2 = trace(m, &rt::str(py, "sep"), &rt::str(py, "-"))?;
        rt::builtins::print(
            py,
            (split
                .call1((trace(m, &rt::str(py, "seps"), &rt::str(py, "-+"))?.contains(item2)?,))?,),
        )?;
    }
    {
        let value = trace(m, &rt::str(py, "first"), &rt::int(py, 1))?;
        let item2 = trace(m, &rt::str(py, "x"), &rt::int(py, 1))?;
        rt::builtins::print(
            py,
            (PyList::new(
                py,
                [
                    &value,
                    &rt::bool(
                        py,
                        trace(m, &rt::str(py, "xs"), PyList::new(py, [1])?.as_any())?
                            .contains(item2)?,
                    ),
                ],
            )?,),
        )?;
    }
    {
        let value = trace(m, &rt::str(py, "left"), &rt::int(py, 0))?;
        let value2 = if value.is_truthy()? {
            value
        } else {
            trace(m, &rt::str(py, "right"), PyList::new(py, [5, 6])?.as_any())?
        };
        data.set_item(trace(m, &rt::str(py, "key"), &rt::str(py, "k"))?, value2)?;
    }
    let value = trace(m, &rt::str(py, "value"), &rt::int(py, 7))?;
    data.get_item(trace(m, &rt::str(py, "outer"), &rt::str(py, "k"))?)?
        .set_item(trace(m, &rt::str(py, "inner"), &rt::int(py, 0))?, value)?;
    rt::builtins::print(py, (&data,))?;
    let result = {
        let lhs = trace(m, &rt::str(py, "a"), &rt::int(py, 1))?;
        let rhs = trace(m, &rt::str(py, "b"), &rt::int(py, 2))?;
        let result2 = lhs.rich_compare(&rhs, CompareOp::Lt)?;
        if result2.is_truthy()? {
            rhs.rich_compare(trace(m, &rt::str(py, "c"), &rt::int(py, 3))?, CompareOp::Lt)?
        } else {
            result2
        }
    };
    let item2 = trace(m, &rt::str(py, "x"), &rt::int(py, 2))?;
    rt::builtins::print(
        py,
        (
            &result,
            if trace(m, &rt::str(py, "xs"), PyList::new(py, [2])?.as_any())?.contains(item2)? {
                trace(m, &rt::str(py, "then"), &rt::int(py, 1))?
            } else {
                trace(m, &rt::str(py, "else"), &rt::int(py, 0))?
            },
        ),
    )?;
    let value = rt::format(&trace(m, &rt::str(py, "p1"), &rt::int(py, 1))?, "")?;
    let item2 = trace(m, &rt::str(py, "needle"), &rt::int(py, 2))?;
    rt::builtins::print(
        py,
        (rt::fstring(
            py,
            (
                value,
                rt::format(
                    &rt::bool(
                        py,
                        trace(m, &rt::str(py, "hay"), PyList::new(py, [2])?.as_any())?
                            .contains(item2)?,
                    ),
                    "",
                )?,
            ),
        )?,),
    )?;
    {
        let target = data.get_item(trace(m, &rt::str(py, "key"), &rt::str(py, "k"))?)?;
        let index = trace(m, &rt::str(py, "index"), &rt::int(py, 1))?;
        let item3 = target.get_item(&index)?;
        let item2 = trace(m, &rt::str(py, "x"), &rt::int(py, 1))?;
        target.set_item(
            index,
            rt::iadd(
                &item3,
                trace(m, &rt::str(py, "xs"), PyList::new(py, [1])?.as_any())?.contains(item2)?,
            )?,
        )?;
    }
    rt::builtins::print(py, (&data,))?;
    let mut n = rt::int(py, 0);
    loop {
        let item2 = trace(m, &rt::str(py, "n"), &n)?;
        if !trace(
            m,
            &rt::str(py, "range"),
            PyList::new(py, [0, 1, 2])?.as_any(),
        )?
        .contains(&item2)?
        {
            break;
        }
        n = n.add(1)?;
        py.check_signals()?;
    }
    let item3 = trace(m, &rt::str(py, "x"), &rt::int(py, 0))?;
    if trace(m, &rt::str(py, "xs"), PyList::empty(py).as_any())?.contains(item3)? {
        rt::builtins::print(py, ("first",))?;
    } else {
        let item4 = trace(m, &rt::str(py, "y"), &rt::int(py, 1))?;
        if trace(m, &rt::str(py, "ys"), PyList::new(py, [1])?.as_any())?.contains(item4)? {
            rt::builtins::print(py, ("second",))?;
        }
    }
    let item3 = trace(m, &rt::str(py, "x"), &rt::int(py, 1))?;
    if !trace(m, &rt::str(py, "xs"), PyList::new(py, [1])?.as_any())?.contains(item3)? {
        return Err(frame.raise(PyAssertionError::new_err((trace(
            m,
            &rt::str(py, "message"),
            &rt::str(py, "never evaluated"),
        )?
        .unbind(),))));
    }
    data.get_item(trace(m, &rt::str(py, "del key"), &rt::str(py, "k"))?)?
        .del_item(trace(m, &rt::str(py, "del index"), &rt::int(py, 0))?)?;
    data.del_item(trace(m, &rt::str(py, "again"), &rt::str(py, "k"))?)?;
    rt::builtins::print(py, (&data, &n))?;
    for item in rt::iter(&trace(
        m,
        &rt::str(py, "iterable"),
        PyList::new(py, [1, 2])?.as_any(),
    )?)? {
        let item = item?;
        rt::builtins::print(py, ("item", &item))?;
    }
    Ok(())
}

// eval_order.py:107
#[rt::frame(m, "bump", 107)]
pub fn bump<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    m.set_global("counter", 100)?;
    Ok(rt::int(m.py(), 1))
}

/// Executes the body of `eval_order.py` in the module `m`.
#[rt::frame(m, "<module>", 1)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
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
    subscripts(m)?;
    attributes(m)?;
    augmented(m)?;
    membership(m)?;
    comparisons(m)?;
    calls(m)?;
    boolean(m)?;
    unpacking(m)?;
    tricky(m)?;
    m.set_global("counter", 5)?;
    m.set_global("counter", rt::iadd(&m.global("counter")?, bump(m)?)?)?;
    rt::builtins::print(py, (m.global("counter")?,))?;
    m.set_global("counter", PyList::new(py, [0])?)?;
    {
        let counter = m.global("counter")?;
        let index = trace(m, &rt::str(py, "module index"), &rt::int(py, 0))?;
        counter.set_item(
            &index,
            rt::iadd(
                &counter.get_item(&index)?,
                trace(m, &rt::str(py, "module value"), &rt::int(py, 3))?,
            )?,
        )?;
    }
    rt::builtins::print(py, (m.global("counter")?,))?;
    Ok(())
}
