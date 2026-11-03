use pyo3::prelude::*;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

pub mod base {
    use super::*;

    // classes_dataclass_slots.py:10
    /// The body of `class Base`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Base", 10, |_| {
            ns.set(
                "hello",
                rt::function!(ns, hello(slf as "self"), qualname = "Base.hello")?,
            )?;
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
        rt::frame!(ns, "P", 19, |_| {
            let __class__ = ns.class_cell();
            {
                let decorator = ns.get("property")?;
                let function = rt::function!(ns, name(slf as "self"), qualname = "P.name")?;
                ns.set("name", decorator.call1((function,))?)?;
            }
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
        rt::frame!(m, "P.hello", 27, |_| {
            rt::str(m.py(), "p+").add(rt::super_(__class__, slf)?.method0("hello")?)
        })
    }
}

/// Executes the body of `classes_dataclass_slots.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |_| {
        m.set_global("__doc__", "`@dataclass(slots=True)` creates a new class and updates the `__class__`\ncell its methods share. It only looks at Python functions and properties, so\nwhen no property uses `super()`, the transpiled methods still refer to the\nold class (design §12).")?;
        // steelix-test: expect-error TypeError: super(type, obj): obj (instance of P) is not an instance or subtype of type (P).
        let dataclasses = rt::import_names(m, "dataclasses", &["dataclass"])?;
        m.set_global("dataclass", rt::import_from(&dataclasses, "dataclass")?)?;
        m.set_global("Base", rt::class!(m, "Base"(), base::body, line = 10)?)?;
        {
            let decorator = rt::call!(m.global("dataclass")?, slots = true)?;
            let p = rt::class!(
                m,
                "P"(m.global("Base")?),
                p::body,
                line = 19,
                annotations = [("x", "int")]
            )?;
            m.set_global("P", decorator.call1((p,))?)?;
        }
        rt::builtins::print(
            py,
            (
                m.global("P")?.call1((2,))?.attr("x")?,
                m.global("P")?.call1((2,))?.attr("name")?,
            ),
        )?;
        rt::builtins::print(py, (m.global("P")?.call1((1,))?.method0("hello")?,))?;
        Ok(())
    })
}
