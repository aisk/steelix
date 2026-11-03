use pyo3::prelude::*;
use steelix_runtime as rt;

pub mod base {
    use super::*;

    // classes_dataclass_slots.py:10
    /// The body of `class Base`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Base", 10, |frame| {
            frame.line(11);
            ns.set(
                "hello",
                rt::function!(ns, hello(slf as "self"), qualname = "Base.hello")?,
            )?;
            frame.line(15);
            {
                let decorator = ns.get("property")?;
                let function = rt::function!(ns, name(slf as "self"), qualname = "Base.name")?;
                ns.set("name", decorator.call1((function,))?)?;
            }
            Ok(())
        })
    }

    // classes_dataclass_slots.py:11
    pub fn hello<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        Ok(rt::str(m.py(), "base"))
    }

    // classes_dataclass_slots.py:15
    pub fn name<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        Ok(rt::str(m.py(), "base"))
    }
}

pub mod p {
    use super::*;

    // classes_dataclass_slots.py:20
    /// The body of `class P`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "P", 19, |frame| {
            let __class__ = ns.class_cell();
            frame.line(24);
            {
                let decorator = ns.get("property")?;
                let function = rt::function!(ns, name(slf as "self"), qualname = "P.name")?;
                ns.set("name", decorator.call1((function,))?)?;
            }
            frame.line(27);
            ns.set(
                "hello",
                rt::function!(
                    ns,
                    hello[&__class__](slf as "self"),
                    qualname = "P.hello"
                )?,
            )?;
            Ok(())
        })
    }

    // classes_dataclass_slots.py:24
    pub fn name<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        Ok(rt::str(m.py(), "p"))
    }

    // classes_dataclass_slots.py:27
    pub fn hello<'py>(
        m: &rt::Module<'py>,
        __class__: &rt::Cell<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "P.hello", 27, |frame| {
            frame.line(28);
            rt::str(m.py(), "p+").add(rt::super_(__class__, slf)?.call_method0("hello")?)
        })
    }
}

/// Executes the body of `classes_dataclass_slots.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        m.set_global("__doc__", "`@dataclass(slots=True)` creates a new class and updates the `__class__`\ncell its methods share. It only looks at Python functions and properties, so\nwhen no property uses `super()`, the transpiled methods still refer to the\nold class (design §12).")?;
        // steelix-test: expect-error TypeError: super(type, obj): obj (instance of P) is not an instance or subtype of type (P).
        frame.line(7);
        let dataclasses = rt::import_names(m, "dataclasses", &["dataclass"])?;
        m.set_global("dataclass", rt::import_from(&dataclasses, "dataclass")?)?;
        frame.line(10);
        m.set_global("Base", rt::class!(m, "Base"(), base::body, line = 10)?)?;
        {
            frame.line(19);
            let decorator = rt::call!(m.global("dataclass")?, slots = true)?;
            frame.line(20);
            let p = rt::class!(
                m,
                "P"(m.global("Base")?),
                p::body,
                line = 19,
                annotations = [("x", "int")]
            )?;
            frame.line(19);
            let p = decorator.call1((p,))?;
            m.set_global("P", p)?;
        }
        frame.line(31);
        rt::builtins::print(
            py,
            (
                m.global("P")?.call1((2,))?.getattr("x")?,
                m.global("P")?.call1((2,))?.getattr("name")?,
            ),
        )?;
        frame.line(32);
        rt::builtins::print(py, (m.global("P")?.call1((1,))?.call_method0("hello")?,))?;
        Ok(())
    })
}
