use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// drop_order.py:17
#[rt::frame(m, "straight", 17)]
pub fn straight<'py>(m: &rt::Module<'py>) -> PyResult<()> {
    let b = m.global("T")?.call1(("b",))?;
    let a = m.global("T")?.call1(("a",))?;
    let c = m.global("T")?.call1(("c",))?;
    rt::builtins::print(
        m.py(),
        (
            "straight",
            a.attr("name")?,
            b.attr("name")?,
            c.attr("name")?,
        ),
    )?;
    Ok(())
}

// drop_order.py:24
#[rt::frame(m, "reassigned", 24)]
pub fn reassigned<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<()> {
    let mut x = x.clone();
    let y = m.global("T")?.call1(("y",))?;
    x = m.global("T")?.call1(("x",))?;
    rt::builtins::print(m.py(), ("reassigned", x.attr("name")?, y.attr("name")?))?;
    Ok(())
}

// drop_order.py:30
#[rt::frame(m, "branches", 30)]
pub fn branches<'py>(m: &rt::Module<'py>, n: &Bound<'py, PyAny>) -> PyResult<()> {
    let py = m.py();
    // `a` and `c` are assigned on every path of an `if` chain.
    rt::builtins::print(py, ("start",))?;
    let a = if n.gt(1)? {
        m.global("T")?.call1(("a-big",))?
    } else if n.eq(1)? {
        m.global("T")?.call1(("a-one",))?
    } else {
        return Ok(());
    };
    let b = m.global("T")?.call1(("b",))?;
    let c = if n.gt(2)? {
        m.global("T")?
            .call1((rt::str(py, "c-").add(a.attr("name")?)?,))?
    } else {
        rt::builtins::print(py, ("no c yet",))?;
        m.global("T")?.call1(("c-small",))?
    };
    rt::builtins::print(
        py,
        ("end", a.attr("name")?, b.attr("name")?, c.attr("name")?),
    )?;
    Ok(())
}

// drop_order.py:48
#[rt::frame(m, "in_loop", 48)]
pub fn in_loop<'py>(m: &rt::Module<'py>, n: &Bound<'py, PyAny>) -> PyResult<()> {
    let py = m.py();
    let first = m.global("T")?.call1(("first",))?;
    let mut last: Option<Bound<'py, PyAny>> = None;
    for i in rt::iter(&rt::builtins::range(py, (n,))?)? {
        let i = i?;
        last = Some(
            m.global("T")?
                .call1((rt::fstring(py, ("last-", rt::format(&i, "")?))?,))?,
        );
    }
    let after = m.global("T")?.call1(("after",))?;
    rt::builtins::print(
        py,
        (
            "loop",
            first.attr("name")?,
            rt::local(&last, "last")?.attr("name")?,
            after.attr("name")?,
        ),
    )?;
    Ok(())
}

// drop_order.py:56
#[rt::frame(m, "without_else", 56)]
pub fn without_else<'py>(m: &rt::Module<'py>, n: &Bound<'py, PyAny>) -> PyResult<()> {
    let head = m.global("T")?.call1(("head",))?;
    let mut maybe: Option<Bound<'py, PyAny>> = None;
    if n.is_truthy()? {
        maybe = Some(m.global("T")?.call1(("maybe",))?);
    }
    let tail = m.global("T")?.call1(("tail",))?;
    if n.is_truthy()? {
        rt::builtins::print(
            m.py(),
            (
                "without else",
                head.attr("name")?,
                rt::local(&maybe, "maybe")?.attr("name")?,
                tail.attr("name")?,
            ),
        )?;
    }
    Ok(())
}

// drop_order.py:65
#[rt::frame(m, "in_try", 65, frame)]
pub fn in_try<'py>(m: &rt::Module<'py>, text: &Bound<'py, PyAny>) -> PyResult<()> {
    let py = m.py();
    let number: Bound<'py, PyAny>;
    let value = match rt::builtins::int(py, (text,)) {
        Ok(value2) => {
            number = value2;
            m.global("T")?
                .call1((rt::str(py, "good ").add(number.str()?)?,))?
        }
        Err(err) if err.is_instance_of::<PyValueError>(py) => {
            let _caught = frame.catch(err);
            m.global("T")?.call1((rt::str(py, "bad ").add(text)?,))?
        }
        Err(err) => return Err(err),
    };
    let extra = m.global("T")?.call1(("extra",))?;
    rt::builtins::print(py, ("try", value.attr("name")?, extra.attr("name")?))?;
    Ok(())
}

// drop_order.py:76
#[rt::frame(m, "crossing", 76)]
pub fn crossing<'py>(m: &rt::Module<'py>, n: &Bound<'py, PyAny>) -> PyResult<()> {
    let b: Bound<'py, PyAny>;
    // `b` is declared before the chain that gives `a` its value.
    let a = if n.is_truthy()? {
        m.global("T")?.call1(("crossing a",))?
    } else {
        b = m.global("T")?.call1(("crossing b",))?;
        m.global("T")?.call1(("crossing a2",))?
    };
    rt::builtins::print(m.py(), ("crossing", a.attr("name")?))?;
    Ok(())
}

pub mod t {
    use super::*;

    // drop_order.py:9
    /// The body of `class T`, run in its namespace `ns`.
    #[rt::frame(ns, "T", 9)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::def!(ns, __init__(slf as "self", name))?;
        rt::def!(ns, __del__(slf as "self"))?;
        Ok(())
    }

    // drop_order.py:10
    #[rt::frame(m, "T.__init__", 10)]
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        name: &Bound<'py, PyAny>,
    ) -> PyResult<()> {
        slf.set_attr("name", name)?;
        Ok(())
    }

    // drop_order.py:13
    #[rt::frame(m, "T.__del__", 13)]
    pub fn __del__<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<()> {
        rt::builtins::print(m.py(), ("del", slf.attr("name")?))?;
        Ok(())
    }
}

/// Executes the body of `drop_order.py` in the module `m`.
#[rt::frame(m, "<module>", 1)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    m.set_global(
        "__doc__",
        "CPython releases the locals when the function returns in reverse order of\n\
            `co_varnames` (parameters first, then the other locals by first use), Rust\n\
            in reverse order of declaration. The generated Rust declares them by first\n\
            use, which mostly gives the same order. A declaration merged with an `if`\n\
            chain past another one (`crossing`) does not: `KNOWN_DIFFERENCES` in\n\
            tests/test_e2e.py has it (design §12).",
    )?;
    m.set_global(
        "T",
        rt::class!(m, "T"(), t::body, line = 9, attributes = ["name"])?,
    )?;
    rt::def!(m, straight())?;
    rt::def!(m, reassigned(x))?;
    rt::def!(m, branches(n))?;
    rt::def!(m, in_loop(n))?;
    rt::def!(m, without_else(n))?;
    rt::def!(m, in_try(text))?;
    rt::def!(m, crossing(n))?;
    straight(m)?;
    rt::builtins::print(py, ("--",))?;
    crossing(m, &rt::int(py, 0))?;
    rt::builtins::print(py, ("--",))?;
    reassigned(m, &rt::none(py))?;
    rt::builtins::print(py, ("--",))?;
    for item in rt::iter(PyTuple::new(py, [0, 1, 3])?.as_any())? {
        m.set_global("n", item?)?;
        branches(m, &m.global("n")?)?;
        rt::builtins::print(py, ("--",))?;
    }
    in_loop(m, &rt::int(py, 2))?;
    rt::builtins::print(py, ("--",))?;
    without_else(m, &rt::int(py, 1))?;
    rt::builtins::print(py, ("--",))?;
    without_else(m, &rt::int(py, 0))?;
    rt::builtins::print(py, ("--",))?;
    in_try(m, &rt::str(py, "4"))?;
    rt::builtins::print(py, ("--",))?;
    in_try(m, &rt::str(py, "x"))?;
    Ok(())
}
