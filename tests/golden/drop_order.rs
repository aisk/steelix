use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// drop_order.py:14
pub fn straight<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "straight", 14, |_| {
        let b = m.global("T")?.call1(("b",))?;
        let a = m.global("T")?.call1(("a",))?;
        let c = m.global("T")?.call1(("c",))?;
        rt::builtins::print(
            py,
            (
                "straight",
                a.attr("name")?,
                b.attr("name")?,
                c.attr("name")?,
            ),
        )?;
        Ok(rt::none(py))
    })
}

// drop_order.py:21
pub fn reassigned<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "reassigned", 21, |_| {
        let mut x = x.clone();
        let y = m.global("T")?.call1(("y",))?;
        x = m.global("T")?.call1(("x",))?;
        rt::builtins::print(py, ("reassigned", x.attr("name")?, y.attr("name")?))?;
        Ok(rt::none(py))
    })
}

// drop_order.py:27
pub fn branches<'py>(m: &rt::Module<'py>, n: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "branches", 27, |_| {
        // `a` and `c` are assigned on every path of an `if` chain.
        rt::builtins::print(py, ("start",))?;
        let a = if n.gt(1)? {
            m.global("T")?.call1(("a-big",))?
        } else if n.eq(1)? {
            m.global("T")?.call1(("a-one",))?
        } else {
            return Ok(rt::none(py));
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
        Ok(rt::none(py))
    })
}

// drop_order.py:45
pub fn in_loop<'py>(m: &rt::Module<'py>, n: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "in_loop", 45, |_| {
        let first = m.global("T")?.call1(("first",))?;
        let mut last: Option<Bound<'py, PyAny>> = None;
        for i in rt::builtin(py, "range")?.call1((n,))?.try_iter()? {
            let i = i?;
            last = Some(
                m.global("T")?
                    .call1((rt::fstring(py, ("last-", rt::format(&i, "")?))?,))?,
            );
            py.check_signals()?;
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
        Ok(rt::none(py))
    })
}

// drop_order.py:53
pub fn without_else<'py>(
    m: &rt::Module<'py>,
    n: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "without_else", 53, |_| {
        let head = m.global("T")?.call1(("head",))?;
        let mut maybe: Option<Bound<'py, PyAny>> = None;
        if n.is_truthy()? {
            maybe = Some(m.global("T")?.call1(("maybe",))?);
        }
        let tail = m.global("T")?.call1(("tail",))?;
        if n.is_truthy()? {
            rt::builtins::print(
                py,
                (
                    "without else",
                    head.attr("name")?,
                    rt::local(&maybe, "maybe")?.attr("name")?,
                    tail.attr("name")?,
                ),
            )?;
        }
        Ok(rt::none(py))
    })
}

// drop_order.py:62
pub fn in_try<'py>(m: &rt::Module<'py>, text: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "in_try", 62, |frame| {
        let int = rt::builtin(py, "int")?;
        let number: Bound<'py, PyAny>;
        let value = match int.call1((text,)) {
            Ok(value2) => {
                number = value2;
                m.global("T")?
                    .call1((rt::str(py, "good ").add(number.str()?)?,))?
            }
            Err(err) if err.is_instance_of::<PyValueError>(py) => {
                let _handling = rt::Handling::new(&frame.catch(err));
                m.global("T")?.call1((rt::str(py, "bad ").add(text)?,))?
            }
            Err(err) => return Err(err),
        };
        let extra = m.global("T")?.call1(("extra",))?;
        rt::builtins::print(py, ("try", value.attr("name")?, extra.attr("name")?))?;
        Ok(rt::none(py))
    })
}

pub mod t {
    use super::*;

    // drop_order.py:6
    /// The body of `class T`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "T", 6, |_| {
            rt::def!(ns, __init__(slf as "self", name))?;
            rt::def!(ns, __del__(slf as "self"))?;
            Ok(())
        })
    }

    // drop_order.py:7
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        name: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "T.__init__", 7, |_| {
            slf.set_attr("name", name)?;
            Ok(rt::none(m.py()))
        })
    }

    // drop_order.py:10
    pub fn __del__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "T.__del__", 10, |_| {
            rt::builtins::print(py, ("del", slf.attr("name")?))?;
            Ok(rt::none(py))
        })
    }
}

/// Executes the body of `drop_order.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |_| {
        m.set_global(
            "__doc__",
            "Locals are released when the function returns in reverse order of\n\
                `co_varnames` (parameters first, then the other locals by first use), and\n\
                the generated Rust declares them in that order (design §5).",
        )?;
        m.set_global(
            "T",
            rt::class!(m, "T"(), t::body, line = 6, attributes = ["name"])?,
        )?;
        rt::def!(m, straight())?;
        rt::def!(m, reassigned(x))?;
        rt::def!(m, branches(n))?;
        rt::def!(m, in_loop(n))?;
        rt::def!(m, without_else(n))?;
        rt::def!(m, in_try(text))?;
        straight(m)?;
        rt::builtins::print(py, ("--",))?;
        reassigned(m, &rt::none(py))?;
        rt::builtins::print(py, ("--",))?;
        for item in PyTuple::new(py, [0, 1, 3])?.try_iter()? {
            m.set_global("n", item?)?;
            branches(m, &m.global("n")?)?;
            rt::builtins::print(py, ("--",))?;
            py.check_signals()?;
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
    })
}
