use pyo3::prelude::*;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// conditional_defs.py:32
pub fn top<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "top", 32, |_| {
        rt::call!(m.global("helper")?, 1, scale = 3)
    })
}

// conditional_defs.py:39
pub fn f<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    Ok(rt::str(m.py(), "top"))
}

// conditional_defs.py:49
pub fn g<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "g", 49, |_| { m.global("f")?.call0() })
}

/// Executes the body of `conditional_defs.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        m.set_global(
            "__doc__",
            "Functions defined in module level compound statements: nested fn items of\n\
                `exec_module`, bound in the globals when the `def` runs.",
        )?;
        m.set_global("sys", rt::import(m, "sys")?)?;
        m.set_global("counter", 10)?;
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
                rt::frame!(m, "helper", 9, |_| {
                    x.mul(scale)?.add(m.global("counter")?)
                })
            }
            rt::def!(m, helper(x, scale = 2), doc = "Doc of helper.")?;
            // conditional_defs.py:13
            fn recurse<'py>(
                m: &rt::Module<'py>,
                n: &Bound<'py, PyAny>,
            ) -> PyResult<Bound<'py, PyAny>> {
                let py = m.py();
                rt::frame!(m, "recurse", 13, |_| {
                    Ok(if n.eq(0)? {
                        rt::int(py, 0)
                    } else {
                        rt::int(py, 1).add(m.global("recurse")?.call1((n.sub(1)?,))?)?
                    })
                })
            }
            rt::def!(m, recurse(n))?;
        } else {
            // conditional_defs.py:16
            fn helper_<'py>(
                m: &rt::Module<'py>,
                x: &Bound<'py, PyAny>,
                scale: &Bound<'py, PyAny>,
            ) -> PyResult<Bound<'py, PyAny>> {
                Ok(rt::int(m.py(), -1))
            }
            rt::def!(m, helper_ as "helper"(x, scale = 2))?;
        }
        for item in rt::iter(&rt::builtin(py, "range")?.call1((2,))?)? {
            m.set_global("i", item?)?;
            // conditional_defs.py:20
            fn in_loop<'py>(
                m: &rt::Module<'py>,
                y: &Bound<'py, PyAny>,
            ) -> PyResult<Bound<'py, PyAny>> {
                Ok(y.clone())
            }
            rt::def!(m, in_loop(y = m.global("i")?))?;
            rt::builtins::print(
                py,
                (
                    m.global("in_loop")?.call0()?,
                    m.global("helper")?.call1((m.global("i")?,))?,
                    m.global("recurse")?.call1((3,))?,
                ),
            )?;
        }
        let result = (|| -> PyResult<()> {
            // conditional_defs.py:26
            fn in_try<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
                rt::frame!(m, "in_try", 26, |_| {
                    Ok(PyTuple::new(
                        m.py(),
                        [
                            &m.global("helper")?.attr("__qualname__")?,
                            &m.global("helper")?.attr("__doc__")?,
                            &m.global("helper")?.attr("__module__")?,
                        ],
                    )?
                    .into_any())
                })
            }
            rt::def!(m, in_try())?;
            Ok(())
        })();
        let finally = frame.finally(result);
        rt::builtins::print(py, (m.global("in_try")?.call0()?,))?;
        finally.resume()?;
        rt::def!(m, top())?;
        rt::builtins::print(py, (top(m)?,))?;
        rt::def!(m, f())?;
        if m.global("counter")?.gt(5)? {
            // Rebinding a module function: calls go through the globals.
            // conditional_defs.py:45
            fn f_<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
                Ok(rt::str(m.py(), "conditional"))
            }
            rt::def!(m, f_ as "f"())?;
        }
        rt::def!(m, g())?;
        rt::builtins::print(
            py,
            (
                m.global("f")?.call0()?,
                g(m)?,
                m.global("f")?.attr("__qualname__")?,
            ),
        )?;
        Ok(())
    })
}
