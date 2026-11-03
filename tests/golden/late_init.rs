use pyo3::exceptions::PyUnboundLocalError;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// late_init.py:18
pub fn sign<'py>(m: &rt::Module<'py>, n: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "sign", 18, |_| {
        // A branch that returns needs no value.
        let word = if n.lt(0)? {
            rt::str(py, "negative")
        } else if n.eq(0)? {
            return Ok(rt::str(py, "zero"));
        } else {
            rt::builtins::print(py, ("positive", n))?;
            rt::str(py, "positive")
        };
        Ok(word)
    })
}

// late_init.py:30
pub fn nested<'py>(
    m: &rt::Module<'py>,
    a: &Bound<'py, PyAny>,
    b: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "nested", 30, |frame| {
        // A nested `if` that assigns in every branch is a value too.
        let pair = if a.is_truthy()? {
            if b.is_truthy()? {
                rt::str(py, "both")
            } else {
                rt::str(py, "a only")
            }
        } else if b.is_truthy()? {
            rt::str(py, "b only")
        } else {
            return Err(frame.raise(PyValueError::new_err("neither")));
        };
        Ok(pair)
    })
}

// late_init.py:44
pub fn flag<'py>(m: &rt::Module<'py>, n: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "flag", 44, |_| {
        // A bool local.
        let big = if n.gt(10)? {
            true
        } else {
            PyTuple::new(py, [5, 10])?.contains(n)?
        };
        Ok(rt::bool(py, big))
    })
}

// late_init.py:53
pub fn counted<'py>(m: &rt::Module<'py>, items: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "counted", 53, |_| {
        // Assigned again later: stays `mut`.
        let mut total = if items.is_truthy()? {
            rt::int(py, items.len()?)
        } else {
            rt::int(py, 0)
        };
        total = rt::iadd(&total, 1)?;
        Ok(total)
    })
}

// late_init.py:63
pub fn lifetimes<'py>(
    m: &rt::Module<'py>,
    choose: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "lifetimes", 63, |_| {
        // Objects are created and released when CPython does: the temporary in
        // the branch right away, the local when the function returns.
        let held = if choose.is_truthy()? {
            m.global("Tracked")?.call1(("temporary",))?;
            m.global("Tracked")?.call1(("then",))?
        } else {
            m.global("Tracked")?.call1(("else",))?
        };
        rt::builtins::print(m.py(), ("holding", &held))?;
        held.attr("name")
    })
}

// late_init.py:75
pub fn parse<'py>(m: &rt::Module<'py>, text: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "parse", 75, |frame| {
        let mut number: Option<Bound<'py, PyAny>> = None;
        // `try` with `except` and `else` that both assign.
        let result = match (|| -> PyResult<()> {
            rt::builtins::print(py, ("parsing", text))?;
            number = Some(rt::builtin(py, "int")?.call1((text,))?);
            Ok(())
        })() {
            Ok(()) => rt::local(&number, "number")?.mul(2)?,
            Err(err) if err.is_instance_of::<PyValueError>(py) => {
                let _handling = rt::Handling::new(&frame.catch(err));
                rt::builtins::print(py, ("not a number:", text))?;
                rt::none(py)
            }
            Err(err) => return Err(err),
        };
        Ok(result)
    })
}

// late_init.py:88
pub fn partial<'py>(m: &rt::Module<'py>, n: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "partial", 88, |_| {
        let mut label: Option<Bound<'py, PyAny>> = None;
        // Not on every path: keeps the declaration.
        if n.gt(0)? {
            label = Some(rt::str(py, "some"));
        }
        if n.gt(5)? {
            label = Some(rt::str(py, "many"));
        }
        match (|| -> PyResult<Bound<'py, PyAny>> { Ok(rt::local(&label, "label")?.clone()) })() {
            Ok(value) => Ok(value),
            Err(err) if err.is_instance_of::<PyUnboundLocalError>(py) => Ok(rt::str(py, "unbound")),
            Err(err) => Err(err),
        }
    })
}

pub mod tracked {
    use super::*;

    // late_init.py:6
    /// The body of `class Tracked`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Tracked", 6, |_| {
            rt::def!(ns, __init__(slf as "self", name))?;
            rt::def!(ns, __del__(slf as "self"))?;
            rt::def!(ns, __repr__(slf as "self"))?;
            Ok(())
        })
    }

    // late_init.py:7
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        name: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "Tracked.__init__", 7, |_| {
            slf.set_attr("name", name)?;
            rt::builtins::print(py, ("new", name))?;
            Ok(rt::none(py))
        })
    }

    // late_init.py:11
    pub fn __del__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "Tracked.__del__", 11, |_| {
            rt::builtins::print(py, ("del", slf.attr("name")?))?;
            Ok(rt::none(py))
        })
    }

    // late_init.py:14
    pub fn __repr__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Tracked.__repr__", 14, |_| {
            rt::fstring(
                m.py(),
                ("Tracked(", rt::format(&slf.attr("name")?, "")?, ")"),
            )
        })
    }
}

/// Executes the body of `late_init.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        m.set_global(
            "__doc__",
            "Locals that every branch of an `if` chain or a `try` assigns last are\n\
                declared with the value of the chain: `let x = if .. { a } else { b };`\n\
                (design §8).",
        )?;
        m.set_global(
            "Tracked",
            rt::class!(
                m,
                "Tracked"(),
                tracked::body,
                line = 6,
                attributes = ["name"]
            )?,
        )?;
        rt::def!(m, sign(n))?;
        rt::def!(m, nested(a, b))?;
        rt::def!(m, flag(n))?;
        rt::def!(m, counted(items))?;
        rt::def!(m, lifetimes(choose))?;
        rt::def!(m, parse(text))?;
        rt::def!(m, partial(n))?;
        rt::builtins::print(
            py,
            (
                sign(m, &rt::int(py, -1))?,
                sign(m, &rt::int(py, 0))?,
                sign(m, &rt::int(py, 3))?,
            ),
        )?;
        rt::builtins::print(
            py,
            (
                nested(m, &rt::int(py, 1), &rt::int(py, 1))?,
                nested(m, &rt::int(py, 1), &rt::int(py, 0))?,
                nested(m, &rt::int(py, 0), &rt::int(py, 1))?,
            ),
        )?;
        match nested(m, &rt::int(py, 0), &rt::int(py, 0)) {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyValueError>(py) => {
                let exception = frame.catch(err);
                m.set_global("err", &exception)?;
                let _handling = rt::Handling::new(&exception);
                rt::builtins::print(py, ("ValueError:", m.global("err")?))?;
                m.unbind_global("err")?;
            }
            Err(err) => return Err(err),
        }
        rt::builtins::print(
            py,
            (
                flag(m, &rt::int(py, 11))?,
                flag(m, &rt::int(py, 10))?,
                flag(m, &rt::int(py, 1))?,
            ),
        )?;
        rt::builtins::print(
            py,
            (
                counted(m, PyList::new(py, [1, 2])?.as_any())?,
                counted(m, PyList::empty(py).as_any())?,
            ),
        )?;
        rt::builtins::print(py, (lifetimes(m, &rt::bool(py, true))?,))?;
        rt::builtins::print(py, (lifetimes(m, &rt::bool(py, false))?,))?;
        rt::builtins::print(
            py,
            (parse(m, &rt::str(py, "21"))?, parse(m, &rt::str(py, "x"))?),
        )?;
        rt::builtins::print(
            py,
            (
                partial(m, &rt::int(py, 0))?,
                partial(m, &rt::int(py, 1))?,
                partial(m, &rt::int(py, 9))?,
            ),
        )?;
        Ok(())
    })
}
