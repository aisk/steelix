use pyo3::prelude::*;
use pyo3::types::PyTuple;
use steelix_runtime as rt;

// conditional_defs.py:32
pub fn top<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "top", 32, |frame| {
        frame.line(33);
        rt::call!(m.global("helper")?, 1, scale = 3)
    })
}

// conditional_defs.py:39
pub fn f<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    Ok(rt::str(m.py(), "top"))
}

// conditional_defs.py:49
pub fn g<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "g", 49, |frame| {
        frame.line(50);
        m.global("f")?.call0()
    })
}

/// Executes the body of `conditional_defs.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        m.set_global("__doc__", "Functions defined in module level compound statements: nested fn items of\n`exec_module`, bound in the globals when the `def` runs.")?;
        frame.line(4);
        m.set_global("sys", rt::import(m, "sys")?)?;
        frame.line(6);
        m.set_global("counter", 10)?;
        frame.line(8);
        if !rt::builtin(py, "hasattr")?
            .call1((m.global("sys")?, "no_such_attribute"))?
            .is_truthy()?
        {
            // conditional_defs.py:9
            /// Doc of helper.
            fn helper<'py>(
                m: &rt::Module<'py>,
                x: &Bound<'py, PyAny>,
                scale: &Bound<'py, PyAny>,
            ) -> PyResult<Bound<'py, PyAny>> {
                rt::frame!(m, "helper", 9, |frame| {
                    frame.line(11);
                    x.mul(scale)?.add(m.global("counter")?)
                })
            }
            frame.line(9);
            m.set_global(
                "helper",
                rt::function!(m, helper(x, scale = 2), doc = "Doc of helper.")?,
            )?;
            // conditional_defs.py:13
            fn recurse<'py>(
                m: &rt::Module<'py>,
                n: &Bound<'py, PyAny>,
            ) -> PyResult<Bound<'py, PyAny>> {
                let py = m.py();
                rt::frame!(m, "recurse", 13, |frame| {
                    frame.line(14);
                    Ok(if n.eq(0)? {
                        rt::int(py, 0)
                    } else {
                        rt::int(py, 1).add(m.global("recurse")?.call1((n.sub(1)?,))?)?
                    })
                })
            }
            frame.line(13);
            m.set_global("recurse", rt::function!(m, recurse(n))?)?;
        } else {
            // conditional_defs.py:16
            fn helper_<'py>(
                m: &rt::Module<'py>,
                x: &Bound<'py, PyAny>,
                scale: &Bound<'py, PyAny>,
            ) -> PyResult<Bound<'py, PyAny>> {
                Ok(rt::int(m.py(), -1))
            }
            frame.line(16);
            m.set_global(
                "helper",
                rt::function!(m, helper_(x, scale = 2), name = "helper")?,
            )?;
        }
        frame.line(19);
        for item in rt::builtin(py, "range")?.call1((2,))?.try_iter()? {
            m.set_global("i", item?)?;
            // conditional_defs.py:20
            fn in_loop<'py>(
                m: &rt::Module<'py>,
                y: &Bound<'py, PyAny>,
            ) -> PyResult<Bound<'py, PyAny>> {
                Ok(y.clone())
            }
            frame.line(20);
            m.set_global("in_loop", rt::function!(m, in_loop(y = m.global("i")?))?)?;
            frame.line(23);
            rt::builtins::print(
                py,
                (
                    m.global("in_loop")?.call0()?,
                    m.global("helper")?.call1((m.global("i")?,))?,
                    m.global("recurse")?.call1((3,))?,
                ),
            )?;
            py.check_signals()?;
            frame.line(19);
        }
        let result = (|| -> PyResult<()> {
            // conditional_defs.py:26
            fn in_try<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
                rt::frame!(m, "in_try", 26, |frame| {
                    frame.line(27);
                    Ok(PyTuple::new(
                        m.py(),
                        [
                            &m.global("helper")?.getattr("__qualname__")?,
                            &m.global("helper")?.getattr("__doc__")?,
                            &m.global("helper")?.getattr("__module__")?,
                        ],
                    )?
                    .into_any())
                })
            }
            frame.line(26);
            m.set_global("in_try", rt::function!(m, in_try())?)?;
            Ok(())
        })();
        let finally = frame.finally(result);
        frame.line(29);
        rt::builtins::print(py, (m.global("in_try")?.call0()?,))?;
        finally.resume()?;
        rt::def!(m, top())?;
        frame.line(36);
        rt::builtins::print(py, (top(m)?,))?;
        rt::def!(m, f())?;
        frame.line(43);
        if m.global("counter")?.gt(5)? {
            // Rebinding a module function: calls go through the globals.
            // conditional_defs.py:45
            fn f_<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
                Ok(rt::str(m.py(), "conditional"))
            }
            frame.line(45);
            m.set_global("f", rt::function!(m, f_(), name = "f")?)?;
        }
        rt::def!(m, g())?;
        frame.line(53);
        rt::builtins::print(
            py,
            (
                m.global("f")?.call0()?,
                g(m)?,
                m.global("f")?.getattr("__qualname__")?,
            ),
        )?;
        Ok(())
    })
}
