//! Parts of `classes_mixed.py` kept as Python source and run in place by
//! `rt::exec_python!` (steelix does not lower them yet):
//!
//! - line 16, `assign` statement: unpacking in a display
//! - line 22, `def kept_greet`: unpacking in a display
//! - line 26, `def kept_static`: unpacking in a display
//! - line 30, `def __private`: unpacking in a display
//! - line 36, `def kept_async`: async function
//! - line 42, `assign` statement: unpacking in a display
//! - line 44, `def failing`: unpacking in a display
//! - line 63, `def greet`: unpacking in a display
//! - line 77, `def kept`: unpacking in a display
//! - line 83, `def kept_hidden`: unpacking in a display
//! - line 90, `def decorate`: class definition in a function
//! - line 100, `def kept`: unpacking in a display
//! - line 107, `class WholeKept`: annotation in a compound statement of a class body

use pyo3::exceptions::PyStopIteration;
use pyo3::exceptions::PyZeroDivisionError;
use pyo3::prelude::*;
use pyo3::types::PyIterator;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;

pub mod base {
    use super::*;

    // classes_mixed.py:6
    /// The body of `class Base`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Base", 6, |frame| {
            frame.line(7);
            ns.set(
                "greet",
                rt::function!(ns, greet(slf as "self"), qualname = "Base.greet")?,
            )?;
            frame.line(10);
            ns.set(
                "items",
                rt::function!(ns, items(slf as "self"), qualname = "Base.items")?,
            )?;
            Ok(())
        })
    }

    // classes_mixed.py:7
    pub fn greet<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        Ok(rt::str(m.py(), "base"))
    }

    // classes_mixed.py:10
    pub fn items<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Base.items", 10, |frame| {
            frame.line(11);
            Ok(PyList::new(m.py(), [1, 2, 3])?.into_any())
        })
    }
}

pub mod mixed {
    use super::*;

    // classes_mixed.py:14
    /// The body of `class Mixed`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Mixed", 14, |frame| {
            let __class__ = ns.class_cell();
            frame.line(15);
            ns.set("native_before", 1)?;
            // classes_mixed.py:16, kept as Python: unpacking in a display at 16:16
            rt::exec_python!(
                ns,
                line = 16,
                r#"
    squares = [*(i * i for i in range(4))]
"#
            )?;
            frame.line(17);
            ns.set("native_after", ns.get("squares")?.get_item(-1)?)?;
            frame.line(19);
            ns.set(
                "greet",
                rt::function!(
                    ns,
                    greet[&__class__](slf as "self"),
                    qualname = "Mixed.greet"
                )?,
            )?;
            // classes_mixed.py:22, kept as Python: unpacking in a display at 23:19
            rt::exec_python!(
                ns,
                line = 22,
                r#"
    def kept_greet(self):
        values = [*super().items()]
        return f"kept:{super().greet()}:{values}:{__class__.__name__}"
"#
            )?;
            // classes_mixed.py:26, kept as Python: unpacking in a display at 28:21
            rt::exec_python!(
                ns,
                line = 26,
                r#"
    @staticmethod
    def kept_static(n):
        return sum([*range(n)])
"#
            )?;
            // classes_mixed.py:30, kept as Python: unpacking in a display at 31:17
            rt::exec_python!(
                ns,
                line = 30,
                r#"
    def __private(self):
        return [*("private",)][0]
"#
            )?;
            frame.line(33);
            ns.set(
                "call_private",
                rt::function!(
                    ns,
                    call_private(slf as "self"),
                    qualname = "Mixed.call_private"
                )?,
            )?;
            // classes_mixed.py:36, kept as Python: async function at 36:5
            rt::exec_python!(
                ns,
                line = 36,
                r#"
    async def kept_async(self):
        return "async"
"#
            )?;
            frame.line(39);
            ns.set(
                "uses_kept",
                rt::function!(
                    ns,
                    uses_kept(slf as "self"),
                    qualname = "Mixed.uses_kept"
                )?,
            )?;
            // classes_mixed.py:42, kept as Python: unpacking in a display at 42:17
            rt::exec_python!(
                ns,
                line = 42,
                r#"
    lookup = {**dict([("a", 1)])}
"#
            )?;
            // classes_mixed.py:44, kept as Python: unpacking in a display at 45:17
            rt::exec_python!(
                ns,
                line = 44,
                r#"
    def failing(self):
        return [*(1 / 0 for _ in range(1))]
"#
            )?;
            Ok(())
        })
    }

    // classes_mixed.py:19
    pub fn greet<'py>(
        m: &rt::Module<'py>,
        __class__: &rt::Cell<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Mixed.greet", 19, |frame| {
            frame.line(20);
            rt::str(m.py(), "native:").add(rt::super_(__class__, slf)?.call_method0("greet")?)
        })
    }

    // classes_mixed.py:33
    pub fn call_private<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Mixed.call_private", 33, |frame| {
            frame.line(34);
            slf.call_method0("_Mixed__private")
        })
    }

    // classes_mixed.py:39
    pub fn uses_kept<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Mixed.uses_kept", 39, |frame| {
            frame.line(40);
            slf.call_method0("kept_greet")?
                .add("|")?
                .add(slf.call_method1("kept_static", (4,))?.str()?)
        })
    }
}

pub mod kept_sub {
    use super::*;

    // classes_mixed.py:62
    /// The body of `class KeptSub`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "KeptSub", 62, |_| {
            // classes_mixed.py:63, kept as Python: unpacking in a display at 64:17
            rt::exec_python!(
                ns,
                line = 63,
                r#"
    def greet(self):
        return [*["sub:" + super().greet()]][0]
"#
            )?;
            Ok(())
        })
    }
}

pub mod native_sub {
    use super::*;

    // classes_mixed.py:67
    /// The body of `class NativeSub`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "NativeSub", 67, |frame| {
            let __class__ = ns.class_cell();
            frame.line(68);
            ns.set(
                "greet",
                rt::function!(
                    ns,
                    greet[&__class__](slf as "self"),
                    qualname = "NativeSub.greet"
                )?,
            )?;
            Ok(())
        })
    }

    // classes_mixed.py:68
    pub fn greet<'py>(
        m: &rt::Module<'py>,
        __class__: &rt::Cell<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "NativeSub.greet", 68, |frame| {
            frame.line(69);
            rt::str(m.py(), "nsub:").add(rt::super_(__class__, slf)?.call_method0("greet")?)
        })
    }
}

pub mod outer {
    use super::*;

    // classes_mixed.py:75
    /// The body of `class Outer`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Outer", 75, |frame| {
            frame.line(76);
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
        })
    }

    pub mod inner {
        use super::*;

        // classes_mixed.py:76
        /// The body of `class Outer.Inner`, run in its namespace `ns`.
        pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
            rt::frame!(ns, "Outer.Inner", 76, |frame| {
                // classes_mixed.py:77, kept as Python: unpacking in a display at 78:21
                rt::exec_python!(
                    ns,
                    line = 77,
                    r#"
        def kept(self):
            return [*[type(self).__qualname__]][0] + ":" + __class__.__qualname__
"#
                )?;
                frame.line(80);
                ns.set(
                    "_Inner__hidden",
                    rt::function!(
                        ns,
                        __hidden(slf as "self"),
                        qualname = "Outer.Inner.__hidden"
                    )?,
                )?;
                // classes_mixed.py:83, kept as Python: unpacking in a display at 84:21
                rt::exec_python!(
                    ns,
                    line = 83,
                    r#"
        def kept_hidden(self):
            return [*[self.__hidden()]][0]
"#
                )?;
                Ok(())
            })
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

    // classes_mixed.py:99
    /// The body of `class Decorated`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Decorated", 98, |_| {
            // classes_mixed.py:100, kept as Python: unpacking in a display at 101:17
            rt::exec_python!(
                ns,
                line = 100,
                r#"
    def kept(self):
        return [*[super().__repr__()[:9]]][0] + ":" + __class__.__name__
"#
            )?;
            Ok(())
        })
    }
}

/// Executes the body of `classes_mixed.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        m.set_global("__doc__", "Method level mixed mode: methods and class body statements that can not\nbe lowered stay Python, in the class namespace, and still see `super()`,\n`__class__` and the class's private names.")?;
        frame.line(6);
        m.set_global("Base", rt::class!(m, "Base"(), base::body, line = 6)?)?;
        frame.line(14);
        m.set_global(
            "Mixed",
            rt::class!(m, "Mixed"(m.global("Base")?), mixed::body, line = 14)?,
        )?;
        frame.line(48);
        m.set_global("m", m.global("Mixed")?.call0()?)?;
        frame.line(49);
        rt::builtins::print(
            py,
            (
                m.global("m")?.call_method0("greet")?,
                m.global("m")?.call_method0("kept_greet")?,
                m.global("m")?.call_method0("uses_kept")?,
                m.global("m")?.call_method0("call_private")?,
            ),
        )?;
        frame.line(50);
        rt::builtins::print(
            py,
            (
                m.global("Mixed")?.getattr("squares")?,
                m.global("Mixed")?.getattr("native_after")?,
                m.global("Mixed")?.getattr("lookup")?,
                m.global("Mixed")?.call_method1("kept_static", (5,))?,
            ),
        )?;
        frame.line(51);
        rt::builtins::print(
            py,
            (
                m.global("Mixed")?
                    .getattr("kept_greet")?
                    .getattr("__qualname__")?,
                m.global("Mixed")?
                    .getattr("kept_greet")?
                    .getattr("__code__")?
                    .getattr("co_firstlineno")?,
            ),
        )?;
        frame.line(52);
        rt::builtins::print(
            py,
            (
                m.global("Mixed")?
                    .getattr("__dict__")?
                    .get_item("kept_static")?
                    .get_type()
                    .getattr("__name__")?,
                m.global("Mixed")?.getattr("__static_attributes__")?,
            ),
        )?;
        frame.line(53);
        rt::builtins::print(
            py,
            (
                m.global("Mixed")?.getattr("__firstlineno__")?,
                m.global("Mixed")?.getattr("__qualname__")?,
                m.global("Mixed")?.getattr("__module__")?,
            ),
        )?;
        // classes_mixed.py:54
        fn genexpr<'py>(
            m: &rt::Module<'py>,
            __dict__: &Bound<'py, PyIterator>,
        ) -> PyResult<Option<Bound<'py, PyAny>>> {
            rt::frame!(m, "<genexpr>", 54, |_| {
                for k in __dict__ {
                    let k = k?;
                    if !k.call_method1("startswith", ("__",))?.is_truthy()? {
                        return Ok(Some(k.clone()));
                    }
                    m.py().check_signals()?;
                }
                Ok(None)
            })
        }
        frame.line(54);
        rt::builtins::print(
            py,
            (rt::builtin(py, "sorted")?.call1((rt::genexpr!(
                m,
                genexpr(m.global("Mixed")?.getattr("__dict__")?.try_iter()?)
            )?,))?,),
        )?;
        frame.line(55);
        m.set_global("coro", m.global("m")?.call_method0("kept_async")?)?;
        frame.line(57);
        match (|| m.global("coro")?.call_method1("send", (py.None(),)))() {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyStopIteration>(py) => {
                let exception = frame.catch(err);
                m.set_global("stop", &exception)?;
                let _handling = rt::Handling::new(&exception);
                frame.line(59);
                rt::builtins::print(py, (m.global("stop")?.getattr("value")?,))?;
                m.unbind_global("stop")?;
            }
            Err(err) => return Err(err),
        }
        frame.line(62);
        m.set_global(
            "KeptSub",
            rt::class!(m, "KeptSub"(m.global("Mixed")?), kept_sub::body, line = 62)?,
        )?;
        frame.line(67);
        m.set_global(
            "NativeSub",
            rt::class!(
                m,
                "NativeSub"(m.global("KeptSub")?),
                native_sub::body,
                line = 67
            )?,
        )?;
        frame.line(72);
        rt::builtins::print(
            py,
            (
                m.global("NativeSub")?.call0()?.call_method0("greet")?,
                m.global("NativeSub")?
                    .call0()?
                    .is_instance(&m.global("Mixed")?)?,
                rt::builtin(py, "issubclass")?.call1((m.global("KeptSub")?, m.global("Base")?))?,
            ),
        )?;
        frame.line(75);
        m.set_global("Outer", rt::class!(m, "Outer"(), outer::body, line = 75)?)?;
        frame.line(87);
        rt::builtins::print(
            py,
            (
                m.global("Outer")?
                    .call_method0("Inner")?
                    .call_method0("kept")?,
                m.global("Outer")?
                    .call_method0("Inner")?
                    .call_method0("kept_hidden")?,
                m.global("Outer")?
                    .getattr("Inner")?
                    .getattr("kept")?
                    .getattr("__qualname__")?,
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
        frame.line(98);
        let decorate = m.global("decorate")?;
        frame.line(99);
        let decorated = rt::class!(m, "Decorated"(), decorated::body, line = 98)?;
        frame.line(98);
        let decorated = decorate.call1((decorated,))?;
        m.set_global("Decorated", decorated)?;
        frame.line(104);
        rt::builtins::print(
            py,
            (
                m.global("Decorated")?.call0()?.call_method0("kept")?,
                m.global("Decorated")?
                    .getattr("__mro__")?
                    .get_item(1)?
                    .call0()?
                    .call_method0("kept")?,
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
        frame.line(112);
        rt::builtins::print(
            py,
            (
                m.global("WholeKept")?.getattr("conditional")?,
                m.global("WholeKept")?.getattr("__annotations__")?,
            ),
        )?;
        frame.line(115);
        match (|| m.global("m")?.call_method0("failing"))() {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyZeroDivisionError>(py) => {
                let exception = frame.catch(err);
                m.set_global("err", &exception)?;
                let _handling = rt::Handling::new(&exception);
                frame.line(117);
                m.set_global("traceback", rt::import(m, "traceback")?)?;
                frame.line(119);
                m.set_global(
                    "tb",
                    m.global("traceback")?
                        .getattr("extract_tb")?
                        .call1((m.global("err")?.getattr("__traceback__")?,))?,
                )?;
                frame.line(120);
                let list = PyList::empty(py);
                for f in m.global("tb")?.try_iter()? {
                    let f = f?;
                    list.append(PyTuple::new(
                        py,
                        [
                            &f.getattr("name")?,
                            &f.getattr("lineno")?,
                            &f.getattr("line")?,
                        ],
                    )?)?;
                    py.check_signals()?;
                }
                rt::builtins::print(py, (&list,))?;
                m.unbind_global("err")?;
            }
            Err(err) => return Err(err),
        }
        Ok(())
    })
}
