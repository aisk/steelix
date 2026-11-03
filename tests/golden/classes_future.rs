use pyo3::prelude::*;
use pyo3::types::PyList;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

pub mod meta {
    use super::*;

    // classes_future.py:10
    /// The body of `class Meta`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Meta", 10, |_| {
            let __class__ = ns.class_cell();
            ns.set(
                "__new__",
                rt::function!(
                    m,
                    __new__[&__class__](mcls, name, bases, ns),
                    qualname = "Meta.__new__"
                )?,
            )?;
            Ok(())
        })
    }

    // classes_future.py:11
    pub fn __new__<'py>(
        m: &rt::Module<'py>,
        __class__: &rt::Cell<'py>,
        mcls: &Bound<'py, PyAny>,
        name: &Bound<'py, PyAny>,
        bases: &Bound<'py, PyAny>,
        ns: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "Meta.__new__", 11, |_| {
            let list = PyList::empty(py);
            for k in ns.try_iter()? {
                let k = k?;
                if k.ne("__module__")? {
                    list.append(&k)?;
                }
                py.check_signals()?;
            }
            rt::builtins::print(py, (name, list))?;
            rt::super_(__class__, mcls)?.method1("__new__", (mcls, name, bases, ns))
        })
    }
}

/// Settings.
pub mod config {
    use super::*;

    // classes_future.py:16
    /// The body of `class Config`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Config", 16, |_| {
            ns.setup_annotations()?;
            ns.set("__doc__", "Settings.")?;
            ns.set("name", "app")?;
            ns.annotate("name", "str")?;
            ns.annotate("size", "int")?;
            {
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
            }
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
        rt::frame!(ns, "Pair", 31, |_| {
            ns.setup_annotations()?;
            ns.annotate("left", "int")?;
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
        rt::frame!(ns, "Private", 40, |_| {
            ns.setup_annotations()?;
            // The strings keep the names as written, the keys are mangled.
            ns.set("_Private__x", 1)?;
            ns.annotate("_Private__x", "__T")?;
            ns.annotate("y", "list[__T]")?;
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
    rt::frame!(m, "<module>", 1, |_| {
        m.set_global("__doc__", "Class annotations with `from __future__ import annotations` are strings in\n`__annotations__`, created at the start of the body.")?;
        let __future__ = rt::import_names(m, "__future__", &["annotations"])?;
        m.set_global("annotations", rt::import_from(&__future__, "annotations")?)?;
        m.set_global("dataclasses", rt::import(m, "dataclasses")?)?;
        m.set_global("typing", rt::import(m, "typing")?)?;
        m.set_global(
            "Meta",
            rt::class!(m, "Meta"(rt::builtin(py, "type")?), meta::body, line = 10)?,
        )?;
        m.set_global(
            "Config",
            rt::class!(
                m,
                "Config"(metaclass = m.global("Meta")?),
                config::body,
                line = 16
            )?,
        )?;
        rt::builtins::print(
            py,
            (
                m.global("Config")?.attr("__annotations__")?,
                m.global("Config")?.attr("name")?,
                m.global("Config")?.attr("ratio")?,
                m.global("Config")?
                    .attr("scaled")?
                    .attr("__annotations__")?,
            ),
        )?;
        rt::builtins::print(
            py,
            (m.global("typing")?
                .attr("get_type_hints")?
                .call1((m.global("Config")?,))?,),
        )?;
        {
            let dataclass = m.global("dataclasses")?.attr("dataclass")?;
            let pair = rt::class!(m, "Pair"(), pair::body, line = 31)?;
            m.set_global("Pair", dataclass.call1((pair,))?)?;
        }
        rt::builtins::print(
            py,
            (
                m.global("Pair")?.call1((1,))?,
                m.global("Pair")?.attr("__annotations__")?,
            ),
        )?;
        m.set_global(
            "Private",
            rt::class!(m, "Private"(), private::body, line = 40)?,
        )?;
        rt::builtins::print(
            py,
            (
                m.global("Private")?.attr("__annotations__")?,
                m.global("Private")?.attr("f")?.attr("__annotations__")?,
            ),
        )?;
        Ok(())
    })
}
