use pyo3::exceptions::PyNameError;
use pyo3::prelude::*;
use pyo3::pyclass::CompareOp;
use pyo3::types::PyList;
use steelix_runtime as rt;

pub mod item {
    use super::*;

    // classes_dataclass.py:10
    /// The body of `class Item`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Item", 9, |_| {
            ns.set("price", 0.0)?;
            ns.set(
                "tags",
                rt::call!(ns.get("field")?, default_factory = ns.get("list")?)?,
            )?;
            ns.set(
                "total",
                rt::function!(
                    ns,
                    total(slf as "self", quantity: "int") -> "float",
                    qualname = "Item.total"
                )?,
            )?;
            Ok(())
        })
    }

    // classes_dataclass.py:15
    pub fn total<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        quantity: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Item.total", 15, |_| {
            slf.getattr("price")?.mul(quantity)
        })
    }
}

pub mod version {
    use super::*;

    // classes_dataclass.py:20
    /// The body of `class Version`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Version", 19, |_| {
            ns.set("minor", 0)?;
            ns.set(
                "bump",
                rt::function!(
                    ns,
                    bump(slf as "self") -> "'Version'",
                    qualname = "Version.bump"
                )?,
            )?;
            Ok(())
        })
    }

    // classes_dataclass.py:24
    pub fn bump<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Version.bump", 24, |_| {
            m.global("Version")?
                .call1((slf.getattr("major")?, slf.getattr("minor")?.add(1)?))
        })
    }
}

pub mod node {
    use super::*;

    // classes_dataclass.py:40
    /// The body of `class Node`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Node", 40, |_| {
            ns.set("parent", m.py().None())?;
            ns.set("alias", ns.get("int")?)?;
            ns.set(
                "link",
                rt::function!(
                    ns,
                    link(slf as "self", other: "Node") -> "alias",
                    qualname = "Node.link"
                )?,
            )?;
            Ok(())
        })
    }

    // classes_dataclass.py:47
    pub fn link<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        other: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(rt::int(m.py(), 1))
    }
}

pub mod forward {
    use super::*;

    // classes_dataclass.py:59
    /// The body of `class Forward`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        Ok(())
    }
}

pub mod later {
    use super::*;

    // classes_dataclass.py:71
    /// The body of `class Later`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        Ok(())
    }
}

pub mod slotted {
    use super::*;

    // classes_dataclass.py:79
    /// The body of `class Slotted`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Slotted", 78, |_| {
            ns.set("y", 2)?;
            ns.set(
                "total",
                rt::function!(ns, total(slf as "self"), qualname = "Slotted.total")?,
            )?;
            Ok(())
        })
    }

    // classes_dataclass.py:83
    pub fn total<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Slotted.total", 83, |_| {
            slf.getattr("x")?.add(slf.getattr("y")?)
        })
    }
}

pub mod child {
    use super::*;

    // classes_dataclass.py:92
    /// The body of `class Child`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Child", 91, |_| {
            let __class__ = ns.class_cell();
            ns.set("color", "red")?;
            ns.set(
                "total",
                rt::function!(
                    ns,
                    total[&__class__](slf as "self", quantity: "int") -> "float",
                    qualname = "Child.total"
                )?,
            )?;
            Ok(())
        })
    }

    // classes_dataclass.py:95
    pub fn total<'py>(
        m: &rt::Module<'py>,
        __class__: &rt::Cell<'py>,
        slf: &Bound<'py, PyAny>,
        quantity: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Child.total", 95, |_| {
            rt::super_(__class__, slf)?
                .call_method1("total", (quantity,))?
                .add(1)
        })
    }
}

pub mod typed {
    use super::*;

    // classes_dataclass.py:102
    /// The body of `class Typed`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Typed", 102, |_| {
            ns.set("value", 3)?;
            Ok(())
        })
    }
}

/// Executes the body of `classes_dataclass.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        m.set_global(
            "__doc__",
            "Class annotations (PEP 649) as dataclasses and `typing` see them.",
        )?;
        m.set_global("annotationlib", rt::import(m, "annotationlib")?)?;
        m.set_global("dataclasses", rt::import(m, "dataclasses")?)?;
        m.set_global("typing", rt::import(m, "typing")?)?;
        let dataclasses = rt::import_names(m, "dataclasses", &["dataclass", "field"])?;
        m.set_global("dataclass", rt::import_from(&dataclasses, "dataclass")?)?;
        m.set_global("field", rt::import_from(&dataclasses, "field")?)?;
        {
            let dataclass = m.global("dataclass")?;
            let item = rt::class!(
                m,
                "Item"(),
                item::body,
                line = 9,
                annotations = [("name", "str"), ("price", "float"), ("tags", "list")]
            )?;
            m.set_global("Item", dataclass.call1((item,))?)?;
        }
        {
            let decorator = rt::call!(m.global("dataclass")?, frozen = true, order = true)?;
            let version = rt::class!(
                m,
                "Version"(),
                version::body,
                line = 19,
                annotations = [("major", "int"), ("minor", "int")]
            )?;
            m.set_global("Version", decorator.call1((version,))?)?;
        }
        m.set_global("item", m.global("Item")?.call1(("pen", 1.5))?)?;
        rt::builtins::print(
            py,
            (
                m.global("item")?,
                m.global("item")?.call_method1("total", (3,))?,
                m.global("Item")?.getattr("__annotations__")?,
                m.global("Item")?
                    .call1(("a",))?
                    .rich_compare(m.global("Item")?.call1(("a",))?, CompareOp::Eq)?,
            ),
        )?;
        m.set_global("v", m.global("Version")?.call1((1,))?)?;
        rt::builtins::print(
            py,
            (
                m.global("v")?,
                m.global("v")?.call_method0("bump")?,
                rt::builtin(py, "sorted")?.call1((PyList::new(
                    py,
                    [
                        &m.global("Version")?.call1((2,))?,
                        &m.global("Version")?.call1((1, 5))?,
                        &m.global("v")?,
                    ],
                )?,))?,
                m.global("v")?
                    .rich_compare(m.global("v")?.call_method0("bump")?, CompareOp::Lt)?,
            ),
        )?;
        match (|| -> PyResult<()> {
            m.global("v")?.setattr("major", 3)?;
            Ok(())
        })() {
            Ok(()) => {}
            Err(err)
                if frame.matches(&err, 1, || {
                    m.global("dataclasses")?.getattr("FrozenInstanceError")
                })? =>
            {
                let exception = frame.catch(err);
                m.set_global("err", &exception)?;
                let _handling = rt::Handling::new(&exception);
                rt::builtins::print(py, ("FrozenInstanceError:", m.global("err")?))?;
                m.unbind_global("err")?;
            }
            Err(err) => return Err(err),
        }
        let value = rt::builtin(py, "hash")?
            .call1((m.global("v")?,))?
            .rich_compare(
                rt::builtin(py, "hash")?.call1((m.global("Version")?.call1((1,))?,))?,
                CompareOp::Eq,
            )?;
        let list = PyList::empty(py);
        for f in m
            .global("dataclasses")?
            .getattr("fields")?
            .call1((m.global("Version")?,))?
            .try_iter()?
        {
            let f = f?;
            list.append(f.getattr("name")?)?;
            py.check_signals()?;
        }
        rt::builtins::print(py, (value, list))?;
        rt::builtins::print(
            py,
            (
                m.global("typing")?
                    .getattr("get_type_hints")?
                    .call1((m.global("Item")?.getattr("total")?,))?,
                m.global("typing")?
                    .getattr("get_type_hints")?
                    .call1((m.global("Version")?.getattr("bump")?,))?,
            ),
        )?;
        m.set_global(
            "Node",
            rt::class!(
                m,
                "Node"(),
                node::body,
                line = 40,
                annotations = [
                    ("value", "int"),
                    ("children", "'list[Node]'"),
                    ("parent", "Node | None"),
                    ("with_alias", "alias")
                ]
            )?,
        )?;
        rt::builtins::print(py, (m.global("Node")?.getattr("__annotations__")?,))?;
        rt::builtins::print(
            py,
            (m.global("typing")?
                .getattr("get_type_hints")?
                .call1((m.global("Node")?,))?,),
        )?;
        rt::builtins::print(
            py,
            (m.global("Node")?
                .getattr("link")?
                .getattr("__annotations__")?,),
        )?;
        rt::builtins::print(
            py,
            (rt::call!(
                m.global("annotationlib")?.getattr("get_annotations")?,
                m.global("Node")?,
                format = m
                    .global("annotationlib")?
                    .getattr("Format")?
                    .getattr("STRING")?
            )?,),
        )?;
        rt::builtins::print(
            py,
            (m.global("Node")?
                .getattr("__annotate__")?
                .call1((m
                    .global("annotationlib")?
                    .getattr("Format")?
                    .getattr("VALUE")?,))?
                .rich_compare(m.global("Node")?.getattr("__annotations__")?, CompareOp::Eq)?,),
        )?;
        rt::builtins::print(
            py,
            (
                m.global("Node")?
                    .getattr("__annotate__")?
                    .getattr("__qualname__")?,
                m.global("Node")?.getattr("parent")?,
            ),
        )?;
        m.set_global(
            "Forward",
            rt::class!(
                m,
                "Forward"(),
                forward::body,
                line = 59,
                annotations = [("later", "Later"), ("count", "int")]
            )?,
        )?;
        rt::builtins::print(
            py,
            (rt::call!(
                m.global("annotationlib")?.getattr("get_annotations")?,
                m.global("Forward")?,
                format = m
                    .global("annotationlib")?
                    .getattr("Format")?
                    .getattr("FORWARDREF")?
            )?,),
        )?;
        match (|| m.global("Forward")?.getattr("__annotations__"))() {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyNameError>(py) => {
                let exception = frame.catch(err);
                m.set_global("err", &exception)?;
                let _handling = rt::Handling::new(&exception);
                rt::builtins::print(py, ("NameError:", m.global("err")?))?;
                m.unbind_global("err")?;
            }
            Err(err) => return Err(err),
        }
        m.set_global("Later", rt::class!(m, "Later"(), later::body, line = 71)?)?;
        rt::builtins::print(py, (m.global("Forward")?.getattr("__annotations__")?,))?;
        {
            let decorator = rt::call!(m.global("dataclass")?, slots = true)?;
            let slotted = rt::class!(
                m,
                "Slotted"(),
                slotted::body,
                line = 78,
                annotations = [("x", "int"), ("y", "int")]
            )?;
            m.set_global("Slotted", decorator.call1((slotted,))?)?;
        }
        m.set_global("s", m.global("Slotted")?.call1((1,))?)?;
        rt::builtins::print(
            py,
            (
                m.global("s")?,
                m.global("s")?.call_method0("total")?,
                m.global("Slotted")?.getattr("__slots__")?,
                rt::builtin(py, "hasattr")?.call1((m.global("s")?, "__dict__"))?,
            ),
        )?;
        {
            let dataclass = m.global("dataclass")?;
            let child = rt::class!(
                m,
                "Child"(m.global("Item")?),
                child::body,
                line = 91,
                annotations = [("color", "str")]
            )?;
            m.set_global("Child", dataclass.call1((child,))?)?;
        }
        rt::builtins::print(
            py,
            (
                m.global("Child")?.call1(("x", 2.0))?,
                m.global("Child")?
                    .call1(("x", 2.0))?
                    .call_method1("total", (2,))?,
            ),
        )?;
        m.set_global(
            "Typed",
            rt::class!(
                m,
                "Typed"(
                    m.global("typing")?
                        .getattr("Generic")?
                        .get_item(m.global("typing")?.call_method1("TypeVar", ("T",))?)?
                ),
                typed::body,
                line = 102,
                annotations = [("value", "int")]
            )?,
        )?;
        rt::builtins::print(
            py,
            (
                m.global("Typed")?.get_item(rt::builtin(py, "int")?)?,
                m.global("Typed")?.getattr("__annotations__")?,
                m.global("Typed")?.call0()?.getattr("value")?,
            ),
        )?;
        Ok(())
    })
}
