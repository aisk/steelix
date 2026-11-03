use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3::types::PyIterator;
use pyo3::types::PyList;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// temp_lifetimes.py:28
pub fn make<'py>(m: &rt::Module<'py>, name: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "make", 28, |_| { m.global("Tracked")?.call1((name,)) })
}

// temp_lifetimes.py:32
pub fn gone<'py>(m: &rt::Module<'py>, r#ref: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "gone", 32, |_| {
        rt::builtins::print(py, ("weakref gone",))?;
        Ok(rt::none(py))
    })
}

// temp_lifetimes.py:64
pub fn function<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "function", 64, |_| {
        let e = PyDict::new(py).into_any();
        e.set_item("k", make(m, &rt::str(py, "function item"))?)?;
        e.del_item("k")?;
        rt::builtins::print(py, ("after function item",))?;
        let value = {
            let value2 = make(m, &rt::str(py, "function and"))?;
            if value2.is_truthy()? {
                rt::int(py, 1)
            } else {
                value2
            }
        };
        rt::builtins::print(py, ("after function and", &value))?;
        let r = m
            .global("weakref")?
            .attr("ref")?
            .call1((make(m, &rt::str(py, "function weak"))?, m.global("gone")?))?;
        rt::builtins::print(py, ("after function weak", r.call0()?.is_none()))?;
        Ok(rt::none(py))
    })
}

// temp_lifetimes.py:78
pub fn r#loop<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "loop", 78, |_| {
        let mut e: Bound<'py, PyAny>;
        for i in rt::builtin(py, "range")?.call1((2,))?.try_iter()? {
            let i = i?;
            e = PyDict::new(py).into_any();
            e.set_item(
                &i,
                make(m, &rt::fstring(py, ("loop ", rt::format(&i, "")?))?)?,
            )?;
            e.del_item(&i)?;
            rt::builtins::print(py, ("after loop", &i))?;
            py.check_signals()?;
        }
        Ok(rt::none(py))
    })
}

// temp_lifetimes.py:95
pub fn augmented<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "augmented", 95, |_| {
        let holder = m.global("Holder")?.call0()?;
        holder.set_attr("attr", make(m, &rt::str(py, "function attr"))?)?;
        make(m, &rt::str(py, "function aug target"))?.set_attr("attr", 0)?;
        holder.set_attr("attr", rt::iadd(&holder.attr("attr")?, 1)?)?;
        rt::builtins::print(py, ("after function aug",))?;
        Ok(rt::none(py))
    })
}

pub mod tracked {
    use super::*;

    // temp_lifetimes.py:10
    /// The body of `class Tracked`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Tracked", 10, |_| {
            rt::def!(ns, __init__(slf as "self", name))?;
            rt::def!(ns, __del__(slf as "self"))?;
            rt::def!(ns, __iadd__(slf as "self", other))?;
            rt::def!(ns, __bool__(slf as "self"))?;
            Ok(())
        })
    }

    // temp_lifetimes.py:11
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        name: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Tracked.__init__", 11, |_| {
            slf.set_attr("name", name)?;
            Ok(rt::none(m.py()))
        })
    }

    // temp_lifetimes.py:14
    pub fn __del__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "Tracked.__del__", 14, |_| {
            rt::builtins::print(py, ("del", slf.attr("name")?))?;
            Ok(rt::none(py))
        })
    }

    // temp_lifetimes.py:17
    pub fn __iadd__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        other: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(slf.clone())
    }

    // temp_lifetimes.py:20
    pub fn __bool__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(rt::bool(m.py(), true))
    }
}

pub mod holder {
    use super::*;

    // temp_lifetimes.py:24
    /// The body of `class Holder`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        Ok(())
    }
}

/// Executes the body of `temp_lifetimes.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |_| {
        m.set_global(
            "__doc__",
            "Values computed for a statement go away when the statement is done.\n\
                \n\
                Only the statement is the unit: CPython may drop a value earlier, in the\n\
                middle of the statement, so each value here is the last thing a statement\n\
                uses.",
        )?;
        m.set_global("weakref", rt::import(m, "weakref")?)?;
        m.set_global(
            "Tracked",
            rt::class!(
                m,
                "Tracked"(),
                tracked::body,
                line = 10,
                attributes = ["name"]
            )?,
        )?;
        m.set_global(
            "Holder",
            rt::class!(m, "Holder"(), holder::body, line = 24)?,
        )?;
        rt::def!(m, make(name))?;
        rt::def!(m, gone(r#ref))?;
        // Module level: a subscript target value, a value used twice, and an
        // augmented assignment target object.
        m.set_global("d", PyDict::new(py))?;
        let value = make(m, &rt::str(py, "module item"))?;
        m.global("d")?.set_item("k", value)?;
        m.global("d")?.del_item("k")?;
        rt::builtins::print(py, ("after module item",))?;
        {
            let value = make(m, &rt::str(py, "module pair"))?;
            m.set_global("x", &value)?;
            m.set_global("y", &value)?;
        }
        m.del_global("x")?;
        m.del_global("y")?;
        rt::builtins::print(py, ("after module pair",))?;
        m.set_global("h", m.global("Holder")?.call0()?)?;
        let value = make(m, &rt::str(py, "module attr"))?;
        m.global("h")?.set_attr("attr", value)?;
        make(m, &rt::str(py, "aug target"))?.set_attr("attr", 0)?;
        rt::builtins::print(py, ("after aug target",))?;
        {
            let h = m.global("h")?;
            h.set_attr("attr", rt::iadd(&h.attr("attr")?, 1)?)?;
        }
        rt::builtins::print(py, ("after aug",))?;
        // A weakref callback fires when the last reference goes.
        m.set_global(
            "w",
            m.global("weakref")?
                .attr("ref")?
                .call1((make(m, &rt::str(py, "weak"))?, m.global("gone")?))?,
        )?;
        rt::builtins::print(py, ("after weak", m.global("w")?.call0()?.is_none()))?;
        // A temporary only borrowed (the operand of `and`) must not linger.
        {
            let value = make(m, &rt::str(py, "left"))?;
            m.set_global(
                "result",
                if value.is_truthy()? {
                    rt::str(py, "right")
                } else {
                    value
                },
            )?;
        }
        rt::builtins::print(py, ("after and", m.global("result")?))?;
        rt::def!(m, function())?;
        function(m)?;
        rt::def!(m, r#loop())?;
        r#loop(m)?;
        // A comprehension's result is dropped once the statement is done with it.
        {
            let list = PyList::empty(py);
            for i in rt::builtin(py, "range")?.call1((2,))?.try_iter()? {
                let i = i?;
                list.append(make(m, &rt::fstring(py, ("comp ", rt::format(&i, "")?))?)?)?;
                py.check_signals()?;
            }
            m.set_global("n", list.len())?;
        }
        rt::builtins::print(py, ("after comp", m.global("n")?))?;
        // temp_lifetimes.py:91
        fn genexpr<'py>(
            m: &rt::Module<'py>,
            iter: &Bound<'py, PyIterator>,
        ) -> PyResult<Option<Bound<'py, PyAny>>> {
            rt::frame!(m, "<genexpr>", 91, |_| {
                if let Some(underscore) = rt::next(iter)? {
                    return Ok(Some(rt::int(m.py(), 1)));
                }
                Ok(None)
            })
        }
        m.set_global(
            "n",
            rt::builtin(py, "sum")?.call1((rt::genexpr!(
                m,
                genexpr(PyList::new(py, [&make(m, &rt::str(py, "genexpr"))?])?.try_iter()?)
            )?,))?,
        )?;
        rt::builtins::print(py, ("after genexpr", m.global("n")?))?;
        rt::def!(m, augmented())?;
        augmented(m)?;
        rt::builtins::print(py, ("done",))?;
        Ok(())
    })
}
