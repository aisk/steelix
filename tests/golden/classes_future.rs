//! Parts of `classes_future.py` kept as Python source and run in place by
//! `rt::exec_python!` (steelix does not lower them yet):
//!
//! - line 11, `def __new__`: list comprehension

use pyo3::prelude::*;
use steelix_runtime as rt;

pub mod meta {
    use super::*;

    // classes_future.py:10
    /// The body of `class Meta`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Meta", 10, |_| {
            // classes_future.py:11, kept as Python: list comprehension at 12:21
            rt::exec_python!(
                ns,
                line = 11,
                future_annotations,
                r#"
    def __new__(mcls, name, bases, ns):
        print(name, [k for k in ns if k != "__module__"])
        return super().__new__(mcls, name, bases, ns)
"#
            )?;
            Ok(())
        })
    }
}

/// Settings.
pub mod config {
    use super::*;

    // classes_future.py:16
    /// The body of `class Config`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Config", 16, |frame| {
            ns.setup_annotations()?;
            frame.line(17);
            ns.set("__doc__", "Settings.")?;
            frame.line(19);
            ns.set("name", "app")?;
            ns.annotate("name", "str")?;
            frame.line(20);
            ns.annotate("size", "int")?;
            frame.line(21);
            let name = ns.get("name")?;
            ns.set(
                "ratio",
                if name.is_truthy()? {
                    rt::float(m.py(), 0.5)
                } else {
                    name
                },
            )?;
            ns.annotate("ratio", "float")?;
            frame.line(23);
            ns.set(
                "scaled",
                rt::function!(
                    m,
                    scaled(slf as "self", by: "'int'") -> "'Config'",
                    qualname = "Config.scaled"
                )?,
            )?;
            Ok(())
        })
    }

    // classes_future.py:23
    pub fn scaled<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        by: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(slf.clone())
    }
}

pub mod pair {
    use super::*;

    // classes_future.py:32
    /// The body of `class Pair`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Pair", 31, |frame| {
            ns.setup_annotations()?;
            frame.line(33);
            ns.annotate("left", "int")?;
            frame.line(34);
            ns.set("right", 0)?;
            ns.annotate("right", "int")?;
            Ok(())
        })
    }
}

pub mod private {
    use super::*;

    // classes_future.py:40
    /// The body of `class Private`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Private", 40, |frame| {
            ns.setup_annotations()?;
            // The strings keep the names as written, the keys are mangled.
            frame.line(42);
            ns.set("_Private__x", 1)?;
            ns.annotate("_Private__x", "__T")?;
            frame.line(43);
            ns.annotate("y", "list[__T]")?;
            frame.line(45);
            ns.set(
                "f",
                rt::function!(
                    m,
                    f(slf as "self", _Private__a: "'__T'") -> "'__T'",
                    qualname = "Private.f"
                )?,
            )?;
            Ok(())
        })
    }

    // classes_future.py:45
    pub fn f<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        _Private__a: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(_Private__a.clone())
    }
}

/// Executes the body of `classes_future.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        m.set_global("__doc__", "Class annotations with `from __future__ import annotations` are strings in\n`__annotations__`, created at the start of the body.")?;
        frame.line(4);
        let __future__ = rt::import_names(m, "__future__", &["annotations"])?;
        m.set_global("annotations", rt::import_from(&__future__, "annotations")?)?;
        frame.line(6);
        m.set_global("dataclasses", rt::import(m, "dataclasses")?)?;
        frame.line(7);
        m.set_global("typing", rt::import(m, "typing")?)?;
        frame.line(10);
        m.set_global(
            "Meta",
            rt::class!(m, "Meta"(rt::builtin(py, "type")?), meta::body, line = 10)?,
        )?;
        frame.line(16);
        m.set_global(
            "Config",
            rt::class!(
                m,
                "Config"(metaclass = m.global("Meta")?),
                config::body,
                line = 16
            )?,
        )?;
        frame.line(27);
        rt::builtins::print(
            py,
            (
                m.global("Config")?.getattr("__annotations__")?,
                m.global("Config")?.getattr("name")?,
                m.global("Config")?.getattr("ratio")?,
                m.global("Config")?
                    .getattr("scaled")?
                    .getattr("__annotations__")?,
            ),
        )?;
        frame.line(28);
        rt::builtins::print(
            py,
            (m.global("typing")?
                .getattr("get_type_hints")?
                .call1((m.global("Config")?,))?,),
        )?;
        frame.line(31);
        let dataclass = m.global("dataclasses")?.getattr("dataclass")?;
        frame.line(32);
        let pair = rt::class!(m, "Pair"(), pair::body, line = 31)?;
        frame.line(31);
        let pair = dataclass.call1((pair,))?;
        m.set_global("Pair", pair)?;
        frame.line(37);
        rt::builtins::print(
            py,
            (
                m.global("Pair")?.call1((1,))?,
                m.global("Pair")?.getattr("__annotations__")?,
            ),
        )?;
        frame.line(40);
        m.set_global(
            "Private",
            rt::class!(m, "Private"(), private::body, line = 40)?,
        )?;
        frame.line(49);
        rt::builtins::print(
            py,
            (
                m.global("Private")?.getattr("__annotations__")?,
                m.global("Private")?
                    .getattr("f")?
                    .getattr("__annotations__")?,
            ),
        )?;
        Ok(())
    })
}
