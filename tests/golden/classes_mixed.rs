//! Parts of `classes_mixed.py` kept as Python source and run in place by
//! `rt::exec_python!` (steelix does not lower them yet):
//!
//! - line 16, `assign` statement: generator expression with several for clauses
//! - line 22, `def kept_greet`: generator expression with several for clauses
//! - line 26, `def kept_static`: generator expression with several for clauses
//! - line 30, `def __private`: generator expression with several for clauses
//! - line 42, `assign` statement: generator expression with several for clauses
//! - line 44, `def failing`: generator expression with several for clauses
//! - line 63, `def greet`: generator expression with several for clauses
//! - line 77, `def kept`: generator expression with several for clauses
//! - line 83, `def kept_hidden`: generator expression with several for clauses
//! - line 90, `def decorate`: class definition in a function
//! - line 100, `def kept`: generator expression with several for clauses
//! - line 107, `class WholeKept`: annotation in a compound statement of a class body

use pyo3::exceptions::PyStopIteration;
use pyo3::exceptions::PyZeroDivisionError;
use pyo3::prelude::*;
use pyo3::types::PyIterator;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

pub mod base {
    use super::*;

    // classes_mixed.py:6
    /// The body of `class Base`, run in its namespace `ns`.
    #[rt::frame(ns, "Base", 6)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::def!(ns, greet(slf as "self"))?;
        rt::def!(ns, items(slf as "self"))?;
        Ok(())
    }

    // classes_mixed.py:7
    pub fn greet<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        Ok(rt::str(m.py(), "base"))
    }

    // classes_mixed.py:10
    #[rt::frame(m, "Base.items", 10)]
    pub fn items<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        Ok(PyList::new(m.py(), [1, 2, 3])?.into_any())
    }
}

pub mod mixed {
    use super::*;

    // classes_mixed.py:14
    /// The body of `class Mixed`, run in its namespace `ns`.
    #[rt::frame(ns, "Mixed", 14)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        let __class__ = ns.class_cell();
        ns.set("native_before", 1)?;
        // classes_mixed.py:16, kept as Python: generator expression with several for clauses at 16:19
        rt::exec_python!(
            ns,
            line = 16,
            r#"
    squares = list(i * j for i in range(4) for j in [i])
"#
        )?;
        ns.set("native_after", ns.get("squares")?.get_item(-1)?)?;
        rt::def!(ns, greet[&__class__](slf as "self"))?;
        // classes_mixed.py:22, kept as Python: generator expression with several for clauses at 23:22
        rt::exec_python!(
            ns,
            line = 22,
            r#"
    def kept_greet(self):
        values = list(v for v in super().items() for _ in "1")
        return f"kept:{super().greet()}:{values}:{__class__.__name__}"
"#
        )?;
        // classes_mixed.py:26, kept as Python: generator expression with several for clauses at 28:19
        rt::exec_python!(
            ns,
            line = 26,
            r#"
    @staticmethod
    def kept_static(n):
        return sum(i for i in range(n) for _ in "1")
"#
        )?;
        // classes_mixed.py:30, kept as Python: generator expression with several for clauses at 31:20
        rt::exec_python!(
            ns,
            line = 30,
            r#"
    def __private(self):
        return next(s for s in ("private",) for _ in "1")
"#
        )?;
        rt::def!(ns, call_private(slf as "self"))?;
        rt::def!(ns, async kept_async(slf as "self"))?;
        rt::def!(ns, uses_kept(slf as "self"))?;
        // classes_mixed.py:42, kept as Python: generator expression with several for clauses at 42:18
        rt::exec_python!(
            ns,
            line = 42,
            r#"
    lookup = dict(kv for kv in [("a", 1)] for _ in "1")
"#
        )?;
        // classes_mixed.py:44, kept as Python: generator expression with several for clauses at 45:20
        rt::exec_python!(
            ns,
            line = 44,
            r#"
    def failing(self):
        return list(1 / 0 for _ in range(1) for _ in "1")
"#
        )?;
        Ok(())
    }

    // classes_mixed.py:19
    #[rt::frame(m, "Mixed.greet", 19)]
    pub fn greet<'py>(
        m: &rt::Module<'py>,
        __class__: &rt::Cell<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::str(m.py(), "native:").add(rt::super_(__class__, slf)?.method0("greet")?)
    }

    // classes_mixed.py:33
    #[rt::frame(m, "Mixed.call_private", 33)]
    pub fn call_private<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        slf.method0("_Mixed__private")
    }

    // classes_mixed.py:36
    #[rt::frame(m, "Mixed.kept_async", 36)]
    pub async fn kept_async<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(rt::str(m.py(), "async"))
    }

    // classes_mixed.py:39
    #[rt::frame(m, "Mixed.uses_kept", 39)]
    pub fn uses_kept<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        slf.method0("kept_greet")?
            .add("|")?
            .add(slf.method1("kept_static", (4,))?.str()?)
    }
}

pub mod kept_sub {
    use super::*;

    // classes_mixed.py:62
    /// The body of `class KeptSub`, run in its namespace `ns`.
    #[rt::frame(ns, "KeptSub", 62)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        // classes_mixed.py:63, kept as Python: generator expression with several for clauses at 64:20
        rt::exec_python!(
            ns,
            line = 63,
            r#"
    def greet(self):
        return next(s for s in ["sub:" + super().greet()] for _ in "1")
"#
        )?;
        Ok(())
    }
}

pub mod native_sub {
    use super::*;

    // classes_mixed.py:67
    /// The body of `class NativeSub`, run in its namespace `ns`.
    #[rt::frame(ns, "NativeSub", 67)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        let __class__ = ns.class_cell();
        rt::def!(ns, greet[&__class__](slf as "self"))?;
        Ok(())
    }

    // classes_mixed.py:68
    #[rt::frame(m, "NativeSub.greet", 68)]
    pub fn greet<'py>(
        m: &rt::Module<'py>,
        __class__: &rt::Cell<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::str(m.py(), "nsub:").add(rt::super_(__class__, slf)?.method0("greet")?)
    }
}

pub mod outer {
    use super::*;

    // classes_mixed.py:75
    /// The body of `class Outer`, run in its namespace `ns`.
    #[rt::frame(ns, "Outer", 75)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        ns.set(
            "Inner",
            rt::class!(
                m,
                "Inner"(),
                inner::body,
                line = 76,
                qualname = "Outer.Inner"
            )?,
        )?;
        Ok(())
    }

    pub mod inner {
        use super::*;

        // classes_mixed.py:76
        /// The body of `class Outer.Inner`, run in its namespace `ns`.
        #[rt::frame(ns, "Outer.Inner", 76)]
        pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
            // classes_mixed.py:77, kept as Python: generator expression with several for clauses at 78:24
            rt::exec_python!(
                ns,
                line = 77,
                r#"
        def kept(self):
            return next(s for s in [type(self).__qualname__] for _ in "1") + ":" + __class__.__qualname__
"#
            )?;
            ns.set(
                "_Inner__hidden",
                rt::function!(ns, __hidden(slf as "self"))?,
            )?;
            // classes_mixed.py:83, kept as Python: generator expression with several for clauses at 84:24
            rt::exec_python!(
                ns,
                line = 83,
                r#"
        def kept_hidden(self):
            return next(s for s in [self.__hidden()] for _ in "1")
"#
            )?;
            Ok(())
        }

        // classes_mixed.py:80
        pub fn __hidden<'py>(
            m: &rt::Module<'py>,
            slf: &Bound<'py, PyAny>,
        ) -> PyResult<Bound<'py, PyAny>> {
            Ok(rt::str(m.py(), "hidden"))
        }
    }
}

pub mod decorated {
    use super::*;

    // classes_mixed.py:98
    /// The body of `class Decorated`, run in its namespace `ns`.
    #[rt::frame(ns, "Decorated", 98)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        // classes_mixed.py:100, kept as Python: generator expression with several for clauses at 101:20
        rt::exec_python!(
            ns,
            line = 100,
            r#"
    def kept(self):
        return next(s for s in [super().__repr__()[:9]] for _ in "1") + ":" + __class__.__name__
"#
        )?;
        Ok(())
    }
}

/// Executes the body of `classes_mixed.py` in the module `m`.
#[rt::frame(m, "<module>", 1, frame)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    m.set_global(
        "__doc__",
        "Method level mixed mode: methods and class body statements that can not\n\
            be lowered stay Python, in the class namespace, and still see `super()`,\n\
            `__class__` and the class's private names.",
    )?;
    m.set_global("Base", rt::class!(m, "Base"(), base::body, line = 6)?)?;
    m.set_global(
        "Mixed",
        rt::class!(m, "Mixed"(m.global("Base")?), mixed::body, line = 14)?,
    )?;
    m.set_global("m", m.global("Mixed")?.call0()?)?;
    rt::builtins::print(
        py,
        (
            m.global("m")?.method0("greet")?,
            m.global("m")?.method0("kept_greet")?,
            m.global("m")?.method0("uses_kept")?,
            m.global("m")?.method0("call_private")?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            m.global("Mixed")?.attr("squares")?,
            m.global("Mixed")?.attr("native_after")?,
            m.global("Mixed")?.attr("lookup")?,
            m.global("Mixed")?.method1("kept_static", (5,))?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            m.global("Mixed")?
                .attr("kept_greet")?
                .attr("__qualname__")?,
            m.global("Mixed")?
                .attr("kept_greet")?
                .attr("__code__")?
                .attr("co_firstlineno")?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            m.global("Mixed")?
                .attr("__dict__")?
                .get_item("kept_static")?
                .get_type()
                .attr("__name__")?,
            m.global("Mixed")?.attr("__static_attributes__")?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            m.global("Mixed")?.attr("__firstlineno__")?,
            m.global("Mixed")?.attr("__qualname__")?,
            m.global("Mixed")?.attr("__module__")?,
        ),
    )?;
    // classes_mixed.py:54
    #[rt::frame(m, "<genexpr>", 54)]
    fn genexpr<'py>(
        m: &rt::Module<'py>,
        __dict__: &Bound<'py, PyIterator>,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        for k in rt::iter(__dict__)? {
            let k = k?;
            if !k.method1("startswith", ("__",))?.is_truthy()? {
                return Ok(Some(k));
            }
        }
        Ok(None)
    }
    rt::builtins::print(
        py,
        (rt::builtins::sorted(
            py,
            (rt::genexpr!(
                m,
                genexpr(m.global("Mixed")?.attr("__dict__")?.try_iter()?)
            )?,),
        )?,),
    )?;
    m.set_global("coro", m.global("m")?.method0("kept_async")?)?;
    match (|| m.global("coro")?.method1("send", (py.None(),)))() {
        Ok(_) => {}
        Err(err) if err.is_instance_of::<PyStopIteration>(py) => {
            let exception = frame.catch(err);
            m.set_global("stop", &exception)?;
            rt::builtins::print(py, (m.global("stop")?.attr("value")?,))?;
            m.unbind_global("stop")?;
        }
        Err(err) => return Err(err),
    }
    m.set_global(
        "KeptSub",
        rt::class!(m, "KeptSub"(m.global("Mixed")?), kept_sub::body, line = 62)?,
    )?;
    m.set_global(
        "NativeSub",
        rt::class!(
            m,
            "NativeSub"(m.global("KeptSub")?),
            native_sub::body,
            line = 67
        )?,
    )?;
    rt::builtins::print(
        py,
        (
            m.global("NativeSub")?.call0()?.method0("greet")?,
            m.global("NativeSub")?
                .call0()?
                .is_instance(&m.global("Mixed")?)?,
            rt::builtins::issubclass(py, (m.global("KeptSub")?, m.global("Base")?))?,
        ),
    )?;
    m.set_global("Outer", rt::class!(m, "Outer"(), outer::body, line = 75)?)?;
    rt::builtins::print(
        py,
        (
            m.global("Outer")?.method0("Inner")?.method0("kept")?,
            m.global("Outer")?
                .method0("Inner")?
                .method0("kept_hidden")?,
            m.global("Outer")?
                .attr("Inner")?
                .attr("kept")?
                .attr("__qualname__")?,
        ),
    )?;
    // classes_mixed.py:90, kept as Python: class definition in a function at 91:5
    rt::exec_python!(
        m,
        line = 90,
        r#"
def decorate(cls):
    class Wrapper(cls):
        def kept(self):
            return "wrapped"

    return Wrapper
"#
    )?;
    {
        let decorate = m.global("decorate")?;
        let decorated = rt::class!(m, "Decorated"(), decorated::body, line = 98)?;
        m.set_global("Decorated", decorate.call1((decorated,))?)?;
    }
    rt::builtins::print(
        py,
        (
            m.global("Decorated")?.call0()?.method0("kept")?,
            m.global("Decorated")?
                .attr("__mro__")?
                .get_item(1)?
                .call0()?
                .method0("kept")?,
        ),
    )?;
    // classes_mixed.py:107, kept as Python: annotation in a compound statement of a class body at 109:9
    rt::exec_python!(
        m,
        line = 107,
        r#"
class WholeKept:
    if True:
        conditional: int = 1
"#
    )?;
    rt::builtins::print(
        py,
        (
            m.global("WholeKept")?.attr("conditional")?,
            m.global("WholeKept")?.attr("__annotations__")?,
        ),
    )?;
    match (|| m.global("m")?.method0("failing"))() {
        Ok(_) => {}
        Err(err) if err.is_instance_of::<PyZeroDivisionError>(py) => {
            let exception = frame.catch(err);
            m.set_global("err", &exception)?;
            m.set_global("traceback", rt::import(m, "traceback")?)?;
            m.set_global(
                "tb",
                m.global("traceback")?
                    .attr("extract_tb")?
                    .call1((m.global("err")?.attr("__traceback__")?,))?,
            )?;
            let list = PyList::empty(py);
            for f in rt::iter(&m.global("tb")?)? {
                let f = f?;
                list.append(PyTuple::new(
                    py,
                    [&f.attr("name")?, &f.attr("lineno")?, &f.attr("line")?],
                )?)?;
            }
            rt::builtins::print(py, (list,))?;
            m.unbind_global("err")?;
        }
        Err(err) => return Err(err),
    }
    Ok(())
}
