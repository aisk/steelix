use pyo3::prelude::*;
use pyo3::pyclass::CompareOp;
use pyo3::types::PyList;
use steelix_runtime as rt;

// operators.py:4
#[rt::frame(m, "unary", 4)]
pub fn unary<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::builtins::print(py, (x.neg()?, x.pos()?, x.bitnot()?, !x.is_truthy()?))?;
    Ok(rt::none(py))
}

// operators.py:8
#[rt::frame(m, "logic", 8)]
pub fn logic<'py>(
    m: &rt::Module<'py>,
    a: &Bound<'py, PyAny>,
    b: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::builtins::print(
        py,
        (
            if a.is_truthy()? { b.clone() } else { a.clone() },
            if a.is_truthy()? { a.clone() } else { b.clone() },
            if !a.is_truthy()? {
                b.clone()
            } else {
                rt::bool(py, false)
            },
            if a.is_truthy()? {
                a.clone()
            } else if b.is_truthy()? {
                a.clone()
            } else {
                b.clone()
            },
        ),
    )?;
    if a.is_truthy()? && !b.is_truthy()? {
        rt::builtins::print(py, ("a only",))?;
    } else if a.is_truthy()? || b.is_truthy()? {
        rt::builtins::print(py, ("some",))?;
    } else {
        rt::builtins::print(py, ("none",))?;
    }
    Ok(rt::none(py))
}

// operators.py:18
#[rt::frame(m, "choose", 18)]
pub fn choose<'py>(
    m: &rt::Module<'py>,
    flag: &Bound<'py, PyAny>,
    x: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let label = if x.gt(10)? {
        rt::str(py, "big")
    } else {
        rt::str(py, "small")
    };
    rt::builtins::print(
        py,
        (
            &label,
            if flag.is_truthy()? {
                x.clone()
            } else {
                x.neg()?
            },
        ),
    )?;
    Ok(if flag.is_truthy()? && x.is_truthy()? {
        rt::str(py, "yes")
    } else {
        rt::str(py, "no")
    })
}

// operators.py:24
#[rt::frame(m, "identity", 24)]
pub fn identity<'py>(
    m: &rt::Module<'py>,
    x: &Bound<'py, PyAny>,
    y: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::builtins::print(
        py,
        (x.is(y), !x.is(y), x.is_none(), !x.is_none(), x.is_none()),
    )?;
    if x.is_none() {
        return Ok(rt::str(py, "nothing"));
    }
    Ok(rt::str(py, "something"))
}

// operators.py:31
#[rt::frame(m, "membership", 31)]
pub fn membership<'py>(
    m: &rt::Module<'py>,
    item: &Bound<'py, PyAny>,
    container: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    if container.contains(item)? && !rt::str(py, "xyz").contains(item)? {
        return Ok(rt::bool(py, true));
    }
    Ok(rt::bool(py, container.contains(item)?))
}

// operators.py:37
#[rt::frame(m, "values", 37)]
pub fn values<'py>(
    m: &rt::Module<'py>,
    a: &Bound<'py, PyAny>,
    b: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let lt = a.rich_compare(b, CompareOp::Lt)?;
    rt::builtins::print(
        py,
        (
            &lt,
            a.rich_compare(b, CompareOp::Le)?,
            a.rich_compare(b, CompareOp::Eq)?,
            a.rich_compare(b, CompareOp::Ne)?,
            a.rich_compare(b, CompareOp::Gt)?,
            a.rich_compare(b, CompareOp::Ge)?,
        ),
    )?;
    {
        let result = rt::int(py, 1).rich_compare(a, CompareOp::Lt)?;
        let value = if result.is_truthy()? {
            a.rich_compare(5, CompareOp::Lt)?
        } else {
            result
        };
        let result2 = rt::int(py, 1).rich_compare(a, CompareOp::Lt)?;
        let value2 = if result2.is_truthy()? {
            let result3 = a.rich_compare(b, CompareOp::Lt)?;
            if result3.is_truthy()? {
                b.rich_compare(10, CompareOp::Lt)?
            } else {
                result3
            }
        } else {
            result2
        };
        let result4 = a.rich_compare(b, CompareOp::Eq)?;
        rt::builtins::print(
            py,
            (
                value,
                value2,
                if result4.is_truthy()? {
                    b.rich_compare(3, CompareOp::Eq)?
                } else {
                    result4
                },
            ),
        )?;
    }
    if rt::int(py, 0).le(a)? && a.lt(10)? {
        rt::builtins::print(py, ("digit",))?;
    }
    if !(rt::int(py, 0).le(b)? && b.lt(10)?) {
        rt::builtins::print(py, ("not a digit",))?;
    }
    Ok(rt::none(py))
}

/// Executes the body of `operators.py` in the module `m`.
#[rt::frame(m, "<module>", 1)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    m.set_global(
        "__doc__",
        "Unary operators, and/or, conditional expressions and comparisons.",
    )?;
    rt::def!(m, unary(x))?;
    rt::def!(m, logic(a, b))?;
    rt::def!(m, choose(flag, x))?;
    rt::def!(m, identity(x, y))?;
    rt::def!(m, membership(item, container))?;
    rt::def!(m, values(a, b))?;
    unary(m, &rt::int(py, 5))?;
    unary(m, &rt::int(py, -3))?;
    logic(m, &rt::int(py, 0), &rt::int(py, 3))?;
    logic(m, &rt::int(py, 2), &rt::int(py, 0))?;
    logic(m, &rt::int(py, 0), &rt::int(py, 0))?;
    logic(m, PyList::empty(py).as_any(), &rt::str(py, "s"))?;
    rt::builtins::print(
        py,
        (
            choose(m, &rt::bool(py, true), &rt::int(py, 20))?,
            choose(m, &rt::bool(py, false), &rt::int(py, 0))?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            identity(m, &rt::none(py), &rt::none(py))?,
            identity(m, &rt::int(py, 1), &rt::none(py))?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            membership(m, &rt::str(py, "a"), &rt::str(py, "abc"))?,
            membership(m, &rt::str(py, "x"), &rt::str(py, "xyz"))?,
            membership(m, &rt::int(py, 4), PyList::new(py, [1, 2])?.as_any())?,
        ),
    )?;
    values(m, &rt::int(py, 3), &rt::int(py, 7))?;
    values(m, &rt::int(py, 12), &rt::int(py, 3))?;
    values(m, &rt::int(py, 3), &rt::int(py, 3))?;
    m.set_global("nothing", py.None())?;
    {
        let nothing = m.global("nothing")?;
        let value = if nothing.is_truthy()? {
            nothing
        } else {
            rt::str(py, "default")
        };
        let nothing2 = m.global("nothing")?;
        rt::builtins::print(
            py,
            (
                value,
                if nothing2.is_truthy()? {
                    rt::int(py, 1)
                } else {
                    nothing2
                },
                "last",
            ),
        )?;
    }
    rt::builtins::print(
        py,
        (
            py.Ellipsis(),
            m.global("nothing")?.is(py.Ellipsis().into_bound(py)),
            PyList::new(py, [&py.Ellipsis().into_bound(py)])?,
        ),
    )?;
    Ok(())
}
