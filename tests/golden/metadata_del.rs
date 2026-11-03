use pyo3::prelude::*;
use pyo3::types::IntoPyDict;
use pyo3::types::PyDict;
use pyo3::types::PyIterator;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// metadata_del.py:21
/// Original doc.
pub fn f<'py>(
    m: &rt::Module<'py>,
    a: &Bound<'py, PyAny>,
    b: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "f", 21, |_| {
        Ok(PyTuple::new(m.py(), [a, b])?.into_any())
    })
}

// metadata_del.py:79
pub fn reader<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "reader", 79, |_| {
        m.global("started")?.method1("wait", (5,))?;
        rt::builtins::print(
            py,
            (
                "reader sees",
                m.global("f")?.attr("__qualname__")?,
                m.global("f")?.attr("__doc__")?,
            ),
        )?;
        m.global("done")?.method0("set")?;
        Ok(rt::none(py))
    })
}

pub mod nasty {
    use super::*;

    // metadata_del.py:8
    /// The body of `class Nasty`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Nasty", 8, |_| {
            rt::def!(ns, __init__(slf as "self", what))?;
            rt::def!(ns, __del__(slf as "self"))?;
            Ok(())
        })
    }

    // metadata_del.py:9
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        what: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Nasty.__init__", 9, |_| {
            slf.set_attr("what", what)?;
            Ok(rt::none(m.py()))
        })
    }

    // metadata_del.py:12
    pub fn __del__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "Nasty.__del__", 12, |_| {
            rt::builtins::print(
                py,
                (
                    "del",
                    slf.attr("what")?,
                    "reads",
                    m.global("f")?.attr("__name__")?,
                    m.global("f")?.attr("__qualname__")?,
                    m.global("f")?.attr("__defaults__")?,
                ),
            )?;
            Ok(rt::none(py))
        })
    }
}

pub mod nasty_name {
    use super::*;

    // metadata_del.py:16
    /// The body of `class NastyName`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "NastyName", 16, |_| {
            rt::def!(ns, __del__(slf as "self"))?;
            Ok(())
        })
    }

    // metadata_del.py:17
    pub fn __del__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "NastyName.__del__", 17, |_| {
            rt::builtins::print(
                py,
                (
                    "del name reads",
                    m.global("f")?.attr("__name__")?,
                    m.global("g")?.attr("__name__")?,
                ),
            )?;
            Ok(rt::none(py))
        })
    }
}

pub mod slow {
    use super::*;

    // metadata_del.py:68
    /// The body of `class Slow`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Slow", 68, |_| {
            rt::def!(ns, __del__(slf as "self"))?;
            Ok(())
        })
    }

    // metadata_del.py:69
    pub fn __del__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "Slow.__del__", 69, |_| {
            m.global("started")?.method0("set")?;
            m.global("done")?.method1("wait", (5,))?;
            rt::builtins::print(py, ("slow del done",))?;
            Ok(rt::none(py))
        })
    }
}

/// Executes the body of `metadata_del.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |_| {
        m.set_global(
            "__doc__",
            "Replacing the metadata of a function (or a generator expression) drops the\n\
                old value, whose `__del__` may read the same metadata again, from this thread\n\
                or from another one. The runtime must not hold its lock while that runs.",
        )?;
        m.set_global("threading", rt::import(m, "threading")?)?;
        m.set_global(
            "Nasty",
            rt::class!(m, "Nasty"(), nasty::body, line = 8, attributes = ["what"])?,
        )?;
        m.set_global(
            "NastyName",
            rt::class!(
                m,
                "NastyName"(rt::builtin(py, "str")?),
                nasty_name::body,
                line = 16
            )?,
        )?;
        rt::def!(m, f(a = py.None(), *, b = py.None()), doc = "Original doc.")?;
        rt::builtins::print(py, ("start", m.global("f")?.attr("__doc__")?))?;
        let value = m.global("Nasty")?.call1(("doc",))?;
        m.global("f")?.set_attr("__doc__", value)?;
        m.global("f")?.set_attr("__doc__", py.None())?;
        rt::builtins::print(py, ("doc replaced",))?;
        let value = PyTuple::new(py, [&m.global("Nasty")?.call1(("defaults",))?])?;
        m.global("f")?.set_attr("__defaults__", value)?;
        let value = PyTuple::new(py, [1])?;
        m.global("f")?.set_attr("__defaults__", value)?;
        rt::builtins::print(
            py,
            (
                "defaults replaced",
                f(m, &m.default("f", "a")?, &m.default("f", "b")?)?,
            ),
        )?;
        let value = [("b", &m.global("Nasty")?.call1(("kwdefaults",))?)].into_py_dict(py)?;
        m.global("f")?.set_attr("__kwdefaults__", value)?;
        m.global("f")?.set_attr("__kwdefaults__", py.None())?;
        rt::builtins::print(py, ("kwdefaults replaced",))?;
        let value = m.global("Nasty")?.call1(("module",))?;
        m.global("f")?.set_attr("__module__", value)?;
        m.global("f")?.set_attr("__module__", "elsewhere")?;
        rt::builtins::print(py, ("module replaced",))?;
        let value = [("a", &m.global("Nasty")?.call1(("annotations",))?)].into_py_dict(py)?;
        m.global("f")?.set_attr("__annotations__", value)?;
        let value = PyDict::new(py);
        m.global("f")?.set_attr("__annotations__", value)?;
        rt::builtins::print(py, ("annotations replaced",))?;
        let value = [(
            "a",
            &m.global("Nasty")?.call1(("annotations before annotate",))?,
        )]
        .into_py_dict(py)?;
        m.global("f")?.set_attr("__annotations__", value)?;
        m.global("f")?.set_attr("__annotate__", py.None())?;
        rt::builtins::print(
            py,
            (
                "annotate cleared",
                rt::builtin(py, "list")?.call1((m.global("f")?.attr("__annotations__")?,))?,
            ),
        )?;
        // metadata_del.py:45
        fn lambda<'py>(
            m: &rt::Module<'py>,
            format: &Bound<'py, PyAny>,
        ) -> PyResult<Bound<'py, PyAny>> {
            let py = m.py();
            rt::frame!(m, "<lambda>", 45, |_| {
                Ok([("a", &rt::builtin(py, "int")?)]
                    .into_py_dict(py)?
                    .into_any())
            })
        }
        let value = rt::function!(m, lambda(format), qualname = "<lambda>")?;
        m.global("f")?.set_attr("__annotate__", value)?;
        rt::builtins::print(
            py,
            ("annotate replaced", m.global("f")?.attr("__annotations__")?),
        )?;
        let value = [(
            "a",
            &m.global("Nasty")?.call1(("annotations before annotate",))?,
        )]
        .into_py_dict(py)?;
        m.global("f")?.set_attr("__annotations__", value)?;
        m.global("f")?.set_attr("__annotate__", py.None())?;
        let value = PyDict::new(py);
        m.global("f")?.set_attr("__annotations__", value)?;
        let value = PyTuple::new(py, [&m.global("Nasty")?.call1(("type params",))?])?;
        m.global("f")?.set_attr("__type_params__", value)?;
        let value = PyTuple::empty(py);
        m.global("f")?.set_attr("__type_params__", value)?;
        rt::builtins::print(py, ("type params replaced",))?;
        // metadata_del.py:53
        fn genexpr<'py>(
            m: &rt::Module<'py>,
            iter: &Bound<'py, PyIterator>,
        ) -> PyResult<Option<Bound<'py, PyAny>>> {
            rt::frame!(m, "<genexpr>", 53, |_| {
                if let Some(x) = rt::next(iter)? {
                    return Ok(Some(x));
                }
                Ok(None)
            })
        }
        m.set_global(
            "g",
            rt::genexpr!(
                m,
                genexpr(rt::builtin(py, "range")?.call1((3,))?.try_iter()?)
            )?,
        )?;
        let value = m.global("NastyName")?.call1(("renamed",))?;
        m.global("f")?.set_attr("__name__", value)?;
        m.global("f")?.set_attr("__name__", "f")?;
        rt::builtins::print(py, ("name replaced",))?;
        let value = m.global("NastyName")?.call1(("requalified",))?;
        m.global("f")?.set_attr("__qualname__", value)?;
        m.global("f")?.set_attr("__qualname__", "f")?;
        rt::builtins::print(py, ("qualname replaced",))?;
        let value = m.global("NastyName")?.call1(("genexpr name",))?;
        m.global("g")?.set_attr("__name__", value)?;
        m.global("g")?.set_attr("__name__", "plain")?;
        rt::builtins::print(
            py,
            (
                "generator name replaced",
                m.global("g")?.attr("__name__")?,
                rt::builtin(py, "list")?.call1((m.global("g")?,))?,
            ),
        )?;
        // Another thread reads the metadata while a `__del__` runs in this thread and
        // waits for it.
        m.set_global("Slow", rt::class!(m, "Slow"(), slow::body, line = 68)?)?;
        m.set_global("started", m.global("threading")?.method0("Event")?)?;
        m.set_global("done", m.global("threading")?.method0("Event")?)?;
        rt::def!(m, reader())?;
        m.set_global(
            "thread",
            rt::call!(
                m.global("threading")?.attr("Thread")?,
                target = m.global("reader")?
            )?,
        )?;
        m.global("thread")?.method0("start")?;
        let value = m.global("Slow")?.call0()?;
        m.global("f")?.set_attr("__doc__", value)?;
        m.global("f")?.set_attr("__doc__", "fresh")?;
        m.global("thread")?.method1("join", (10,))?;
        rt::builtins::print(
            py,
            (
                "end",
                m.global("thread")?.method0("is_alive")?,
                m.global("f")?.attr("__doc__")?,
            ),
        )?;
        Ok(())
    })
}
