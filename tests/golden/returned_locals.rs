//! Parts of `returned_locals.py` kept as Python source and run in place by
//! `rt::exec_python!` (steelix does not lower them yet):
//!
//! - line 49, `def from_with`: class definition in a function

use pyo3::exceptions::PyKeyError;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyIterator;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// returned_locals.py:19
pub fn gone<'py>(m: &rt::Module<'py>, r#ref: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "gone", 19, |_| {
        rt::builtins::print(py, ("weakref gone",))?;
        Ok(rt::none(py))
    })
}

// returned_locals.py:23
pub fn from_try<'py>(m: &rt::Module<'py>, name: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "from_try", 23, |_| {
        match (|| -> PyResult<Bound<'py, PyAny>> {
            let local = m.global("Tracked")?.call1((name,))?;
            let other = m.global("Tracked")?.call1((name.add(" other")?,))?;
            rt::builtins::print(py, ("in try", other.attr("name")?))?;
            Ok(local)
        })() {
            Ok(value) => Ok(value),
            Err(err) if err.is_instance_of::<PyValueError>(py) => Ok(rt::none(py)),
            Err(err) => Err(err),
        }
    })
}

// returned_locals.py:33
pub fn from_finally<'py>(
    m: &rt::Module<'py>,
    name: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "from_finally", 33, |frame| {
        let result = (|| -> PyResult<Bound<'py, PyAny>> {
            let kept = m.global("Tracked")?.call1((name,))?;
            Ok(kept)
        })();
        let finally = frame.finally(result);
        rt::builtins::print(m.py(), ("finally runs while", name, "is returned"))?;
        finally.resume()
    })
}

// returned_locals.py:41
pub fn from_handler<'py>(
    m: &rt::Module<'py>,
    name: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "from_handler", 41, |frame| {
        match Err(frame.raise(PyKeyError::new_err((name.clone().unbind(),)))) {
            Ok(()) => unreachable!(),
            Err(err) if err.is_instance_of::<PyKeyError>(py) => {
                let err = frame.catch(err);
                rt::builtins::print(py, ("handling", &err))?;
                Ok(err.clone())
            }
            Err(err2) => Err(err2),
        }
    })
}

// returned_locals.py:63
pub fn from_loop<'py>(
    m: &rt::Module<'py>,
    names: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "from_loop", 63, |_| {
        let mut item: Bound<'py, PyAny>;
        for name in rt::iter(names)? {
            let name = name?;
            item = m.global("Tracked")?.call1((&name,))?;
            if name.eq("b")? {
                return Ok(item);
            }
        }
        Ok(rt::none(m.py()))
    })
}

// returned_locals.py:71
pub fn outer_local<'py>(
    m: &rt::Module<'py>,
    name: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "outer_local", 71, |frame| {
        let result = m.global("Tracked")?.call1((name,))?;
        let result2 = (|| -> PyResult<Bound<'py, PyAny>> {
            rt::builtins::print(py, ("try",))?;
            Ok(result.clone())
        })();
        let finally = frame.finally(result2);
        rt::builtins::print(py, ("result still bound:", result.attr("name")?))?;
        finally.resume()
    })
}

// returned_locals.py:80
pub fn swap<'py>(
    m: &rt::Module<'py>,
    a: &Bound<'py, PyAny>,
    b: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "swap", 80, |_| {
        let mut a = a.clone();
        let mut b = b.clone();
        [a, b] = [b.clone(), a.clone()];
        rt::builtins::print(py, ("swapped", &a, &b))?;
        let [first2, second2] = [
            m.global("Tracked")?.call1(("first",))?,
            m.global("Tracked")?.call1(("second",))?,
        ];
        let mut first = Some(first2);
        let second = second2;
        rt::builtins::print(
            py,
            (
                "unpacked",
                rt::local(&first, "first")?.attr("name")?,
                second.attr("name")?,
            ),
        )?;
        first = None;
        rt::builtins::print(py, ("after del first",))?;
        Ok(PyTuple::new(py, [&a, &b])?.into_any())
    })
}

pub mod tracked {
    use super::*;

    // returned_locals.py:8
    /// The body of `class Tracked`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Tracked", 8, |_| {
            rt::def!(ns, __init__(slf as "self", name))?;
            rt::def!(ns, __del__(slf as "self"))?;
            rt::def!(ns, __repr__(slf as "self"))?;
            Ok(())
        })
    }

    // returned_locals.py:9
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        name: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Tracked.__init__", 9, |_| {
            slf.set_attr("name", name)?;
            Ok(rt::none(m.py()))
        })
    }

    // returned_locals.py:12
    pub fn __del__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "Tracked.__del__", 12, |_| {
            rt::builtins::print(py, ("del", slf.attr("name")?))?;
            Ok(rt::none(py))
        })
    }

    // returned_locals.py:15
    pub fn __repr__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Tracked.__repr__", 15, |_| {
            rt::fstring(
                m.py(),
                ("Tracked(", rt::format(&slf.attr("name")?, "")?, ")"),
            )
        })
    }
}

/// Executes the body of `returned_locals.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |_| {
        m.set_global(
            "__doc__",
            "Locals returned (or yielded) from where they are declared are moved, not\n\
                cloned, and display assignments do not build a tuple. Neither changes when\n\
                an object goes away (design §8).",
        )?;
        m.set_global("weakref", rt::import(m, "weakref")?)?;
        m.set_global(
            "Tracked",
            rt::class!(
                m,
                "Tracked"(),
                tracked::body,
                line = 8,
                attributes = ["name"]
            )?,
        )?;
        rt::def!(m, gone(r#ref))?;
        rt::def!(m, from_try(name))?;
        rt::def!(m, from_finally(name))?;
        rt::def!(m, from_handler(name))?;
        // returned_locals.py:49, kept as Python: class definition in a function at 50:5
        rt::exec_python!(
            m,
            line = 49,
            r#"
def from_with(name):
    class Manager:
        def __enter__(self):
            return self

        def __exit__(self, *exc):
            print("exit")
            return False

    with Manager():
        inside = Tracked(name)
        return inside
"#
        )?;
        rt::def!(m, from_loop(names))?;
        rt::def!(m, outer_local(name))?;
        rt::def!(m, swap(a, b))?;
        m.set_global("r", from_try(m, &rt::str(py, "t"))?)?;
        rt::builtins::print(py, ("got", m.global("r")?))?;
        m.del_global("r")?;
        rt::builtins::print(py, ("after try",))?;
        m.set_global("r", from_finally(m, &rt::str(py, "f"))?)?;
        rt::builtins::print(py, ("got", m.global("r")?))?;
        m.set_global(
            "w",
            m.global("weakref")?
                .attr("ref")?
                .call1((m.global("r")?, m.global("gone")?))?,
        )?;
        m.del_global("r")?;
        rt::builtins::print(py, ("after finally", m.global("w")?.call0()?.is_none()))?;
        rt::builtins::print(py, ("got", from_handler(m, &rt::str(py, "h"))?.repr()?))?;
        m.set_global("r", m.global("from_with")?.call1(("w",))?)?;
        rt::builtins::print(py, ("got", m.global("r")?))?;
        m.del_global("r")?;
        m.set_global(
            "r",
            from_loop(m, PyList::new(py, ["a", "b", "c"])?.as_any())?,
        )?;
        rt::builtins::print(py, ("got", m.global("r")?))?;
        m.del_global("r")?;
        m.set_global("r", outer_local(m, &rt::str(py, "o"))?)?;
        rt::builtins::print(py, ("got", m.global("r")?))?;
        m.del_global("r")?;
        rt::builtins::print(py, (swap(m, &rt::int(py, 1), &rt::int(py, 2))?,))?;
        // returned_locals.py:110
        fn genexpr<'py>(
            m: &rt::Module<'py>,
            iter: &Bound<'py, PyIterator>,
        ) -> PyResult<Option<Bound<'py, PyAny>>> {
            rt::frame!(m, "<genexpr>", 110, |_| {
                for x in rt::iter(iter)? {
                    let x = x?;
                    if x.attr("name")?.is_truthy()? {
                        return Ok(Some(x));
                    }
                }
                Ok(None)
            })
        }
        rt::builtins::print(
            py,
            (rt::builtin(py, "list")?.call1((rt::genexpr!(
                m,
                genexpr(
                    PyList::new(
                        py,
                        [
                            &m.global("Tracked")?.call1(("g1",))?,
                            &m.global("Tracked")?.call1(("g2",))?
                        ]
                    )?
                    .try_iter()?
                )
            )?,))?,),
        )?;
        rt::builtins::print(py, ("end",))?;
        Ok(())
    })
}
