use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3::types::PyIterator;
use pyo3::types::PyList;
use steelix_runtime as rt;

// temp_lifetimes.py:28
pub fn make<'py>(m: &rt::Module<'py>, name: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "make", 28, |frame| {
        frame.line(29);
        m.global("Tracked")?.call1((name,))
    })
}

// temp_lifetimes.py:32
pub fn gone<'py>(m: &rt::Module<'py>, r#ref: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "gone", 32, |frame| {
        frame.line(33);
        rt::builtins::print(py, ("weakref gone",))?;
        Ok(rt::none(py))
    })
}

// temp_lifetimes.py:64
pub fn function<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "function", 64, |frame| {
        let e = PyDict::new(py).into_any();
        frame.line(66);
        e.set_item("k", make(m, &rt::str(py, "function item"))?)?;
        frame.line(67);
        e.del_item("k")?;
        frame.line(68);
        rt::builtins::print(py, ("after function item",))?;
        frame.line(69);
        let value = {
            let value2 = make(m, &rt::str(py, "function and"))?;
            if value2.is_truthy()? {
                rt::int(py, 1)
            } else {
                value2
            }
        };
        frame.line(70);
        rt::builtins::print(py, ("after function and", &value))?;
        frame.line(71);
        let r = m
            .global("weakref")?
            .getattr("ref")?
            .call1((make(m, &rt::str(py, "function weak"))?, m.global("gone")?))?;
        frame.line(72);
        rt::builtins::print(py, ("after function weak", r.call0()?.is_none()))?;
        Ok(rt::none(py))
    })
}

// temp_lifetimes.py:78
pub fn r#loop<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "loop", 78, |frame| {
        let mut e: Bound<'py, PyAny>;
        frame.line(79);
        for i in rt::builtin(py, "range")?.call1((2,))?.try_iter()? {
            let i = i?;
            e = PyDict::new(py).into_any();
            frame.line(81);
            e.set_item(
                &i,
                make(m, &rt::fstring(py, ("loop ", rt::format(&i, "")?))?)?,
            )?;
            frame.line(82);
            e.del_item(&i)?;
            frame.line(83);
            rt::builtins::print(py, ("after loop", &i))?;
            py.check_signals()?;
            frame.line(79);
        }
        Ok(rt::none(py))
    })
}

// temp_lifetimes.py:95
pub fn augmented<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "augmented", 95, |frame| {
        frame.line(96);
        let holder = m.global("Holder")?.call0()?;
        frame.line(97);
        holder.setattr("attr", make(m, &rt::str(py, "function attr"))?)?;
        frame.line(98);
        make(m, &rt::str(py, "function aug target"))?.setattr("attr", 0)?;
        frame.line(99);
        holder.setattr("attr", rt::iadd(&holder.getattr("attr")?, 1)?)?;
        frame.line(100);
        rt::builtins::print(py, ("after function aug",))?;
        Ok(rt::none(py))
    })
}

pub mod tracked {
    use super::*;

    // temp_lifetimes.py:10
    /// The body of `class Tracked`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Tracked", 10, |frame| {
            frame.line(11);
            ns.set(
                "__init__",
                rt::function!(
                    ns,
                    __init__(slf as "self", name),
                    qualname = "Tracked.__init__"
                )?,
            )?;
            frame.line(14);
            ns.set(
                "__del__",
                rt::function!(
                    ns,
                    __del__(slf as "self"),
                    qualname = "Tracked.__del__"
                )?,
            )?;
            frame.line(17);
            ns.set(
                "__iadd__",
                rt::function!(
                    ns,
                    __iadd__(slf as "self", other),
                    qualname = "Tracked.__iadd__"
                )?,
            )?;
            frame.line(20);
            ns.set(
                "__bool__",
                rt::function!(
                    ns,
                    __bool__(slf as "self"),
                    qualname = "Tracked.__bool__"
                )?,
            )?;
            Ok(())
        })
    }

    // temp_lifetimes.py:11
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        name: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Tracked.__init__", 11, |frame| {
            frame.line(12);
            slf.setattr("name", name)?;
            Ok(rt::none(m.py()))
        })
    }

    // temp_lifetimes.py:14
    pub fn __del__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "Tracked.__del__", 14, |frame| {
            frame.line(15);
            rt::builtins::print(py, ("del", slf.getattr("name")?))?;
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
    rt::frame!(m, "<module>", 1, |frame| {
        m.set_global("__doc__", "Values computed for a statement go away when the statement is done.\n\nOnly the statement is the unit: CPython may drop a value earlier, in the\nmiddle of the statement, so each value here is the last thing a statement\nuses.")?;
        frame.line(7);
        m.set_global("weakref", rt::import(m, "weakref")?)?;
        frame.line(10);
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
        frame.line(24);
        m.set_global(
            "Holder",
            rt::class!(m, "Holder"(), holder::body, line = 24)?,
        )?;
        rt::def!(m, make(name))?;
        rt::def!(m, gone(r#ref))?;
        // Module level: a subscript target value, a value used twice, and an
        // augmented assignment target object.
        frame.line(38);
        m.set_global("d", PyDict::new(py))?;
        frame.line(39);
        let value = make(m, &rt::str(py, "module item"))?;
        m.global("d")?.set_item("k", value)?;
        frame.line(40);
        m.global("d")?.del_item("k")?;
        frame.line(41);
        rt::builtins::print(py, ("after module item",))?;
        frame.line(43);
        {
            let value = make(m, &rt::str(py, "module pair"))?;
            m.set_global("x", &value)?;
            m.set_global("y", &value)?;
        }
        frame.line(44);
        m.del_global("x")?;
        frame.line(45);
        m.del_global("y")?;
        frame.line(46);
        rt::builtins::print(py, ("after module pair",))?;
        frame.line(48);
        m.set_global("h", m.global("Holder")?.call0()?)?;
        frame.line(49);
        let value = make(m, &rt::str(py, "module attr"))?;
        m.global("h")?.setattr("attr", value)?;
        frame.line(50);
        make(m, &rt::str(py, "aug target"))?.setattr("attr", 0)?;
        frame.line(51);
        rt::builtins::print(py, ("after aug target",))?;
        frame.line(52);
        {
            let h = m.global("h")?;
            h.setattr("attr", rt::iadd(&h.getattr("attr")?, 1)?)?;
        }
        frame.line(53);
        rt::builtins::print(py, ("after aug",))?;
        // A weakref callback fires when the last reference goes.
        frame.line(56);
        m.set_global(
            "w",
            m.global("weakref")?
                .getattr("ref")?
                .call1((make(m, &rt::str(py, "weak"))?, m.global("gone")?))?,
        )?;
        frame.line(57);
        rt::builtins::print(py, ("after weak", m.global("w")?.call0()?.is_none()))?;
        // A temporary only borrowed (the operand of `and`) must not linger.
        frame.line(60);
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
        frame.line(61);
        rt::builtins::print(py, ("after and", m.global("result")?))?;
        rt::def!(m, function())?;
        frame.line(75);
        function(m)?;
        rt::def!(m, r#loop())?;
        frame.line(86);
        r#loop(m)?;
        // A comprehension's result is dropped once the statement is done with it.
        frame.line(89);
        {
            let list = PyList::empty(py);
            for i in rt::builtin(py, "range")?.call1((2,))?.try_iter()? {
                let i = i?;
                list.append(make(m, &rt::fstring(py, ("comp ", rt::format(&i, "")?))?)?)?;
                py.check_signals()?;
            }
            m.set_global("n", list.len())?;
        }
        frame.line(90);
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
        frame.line(91);
        m.set_global(
            "n",
            rt::builtin(py, "sum")?.call1((rt::genexpr!(
                m,
                genexpr(PyList::new(py, [&make(m, &rt::str(py, "genexpr"))?])?.try_iter()?)
            )?,))?,
        )?;
        frame.line(92);
        rt::builtins::print(py, ("after genexpr", m.global("n")?))?;
        rt::def!(m, augmented())?;
        frame.line(103);
        augmented(m)?;
        frame.line(104);
        rt::builtins::print(py, ("done",))?;
        Ok(())
    })
}
