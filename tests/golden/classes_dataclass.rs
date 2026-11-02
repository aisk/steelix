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
        rt::frame!(ns, "Item", 9, |frame| {
            frame.line(12);
            ns.set("price", 0.0)?;
            frame.line(13);
            ns.set(
                "tags",
                rt::call!(ns.get("field")?, default_factory = ns.get("list")?)?,
            )?;
            frame.line(15);
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
        rt::frame!(m, "Item.total", 15, |frame| {
            frame.line(16);
            slf.getattr("price")?.mul(quantity)
        })
    }
}

pub mod version {
    use super::*;

    // classes_dataclass.py:20
    /// The body of `class Version`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Version", 19, |frame| {
            frame.line(22);
            ns.set("minor", 0)?;
            frame.line(24);
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
        rt::frame!(m, "Version.bump", 24, |frame| {
            frame.line(25);
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
        rt::frame!(ns, "Node", 40, |frame| {
            frame.line(43);
            ns.set("parent", m.py().None())?;
            frame.line(44);
            ns.set("alias", ns.get("int")?)?;
            frame.line(47);
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
        rt::frame!(ns, "Slotted", 78, |frame| {
            frame.line(81);
            ns.set("y", 2)?;
            frame.line(83);
            ns.set(
                "total",
                rt::function!(ns, total(slf as "self"), qualname = "Slotted.total")?,
            )?;
            Ok(())
        })
    }

    // classes_dataclass.py:83
    pub fn total<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Slotted.total", 83, |frame| {
            frame.line(84);
            slf.getattr("x")?.add(slf.getattr("y")?)
        })
    }
}

pub mod child {
    use super::*;

    // classes_dataclass.py:92
    /// The body of `class Child`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Child", 91, |frame| {
            let __class__ = ns.class_cell();
            frame.line(93);
            ns.set("color", "red")?;
            frame.line(95);
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
        rt::frame!(m, "Child.total", 95, |frame| {
            frame.line(96);
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
        rt::frame!(ns, "Typed", 102, |frame| {
            frame.line(103);
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
        frame.line(3);
        m.set_global("annotationlib", rt::import(m, "annotationlib")?)?;
        frame.line(4);
        m.set_global("dataclasses", rt::import(m, "dataclasses")?)?;
        frame.line(5);
        m.set_global("typing", rt::import(m, "typing")?)?;
        frame.line(6);
        let dataclasses = rt::import_names(m, "dataclasses", &["dataclass", "field"])?;
        m.set_global("dataclass", rt::import_from(&dataclasses, "dataclass")?)?;
        m.set_global("field", rt::import_from(&dataclasses, "field")?)?;
        frame.line(9);
        let dataclass = m.global("dataclass")?;
        frame.line(10);
        let item = rt::class!(
            m,
            "Item"(),
            item::body,
            line = 9,
            annotations = [("name", "str"), ("price", "float"), ("tags", "list")]
        )?;
        frame.line(9);
        let item = dataclass.call1((item,))?;
        m.set_global("Item", item)?;
        frame.line(19);
        let decorator = rt::call!(m.global("dataclass")?, frozen = true, order = true)?;
        frame.line(20);
        let version = rt::class!(
            m,
            "Version"(),
            version::body,
            line = 19,
            annotations = [("major", "int"), ("minor", "int")]
        )?;
        frame.line(19);
        let version = decorator.call1((version,))?;
        m.set_global("Version", version)?;
        frame.line(28);
        m.set_global("item", m.global("Item")?.call1(("pen", 1.5))?)?;
        frame.line(29);
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
        frame.line(30);
        m.set_global("v", m.global("Version")?.call1((1,))?)?;
        frame.line(31);
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
            frame.line(33);
            m.global("v")?.setattr("major", 3)?;
            Ok(())
        })() {
            Ok(()) => {}
            Err(err)
                if rt::matches(
                    &err,
                    &m.global("dataclasses")?.getattr("FrozenInstanceError")?,
                )? =>
            {
                let exception = frame.catch(err);
                m.set_global("err", &exception)?;
                let _handling = rt::Handling::new(&exception);
                frame.line(35);
                rt::builtins::print(py, ("FrozenInstanceError:", m.global("err")?))?;
                m.unbind_global("err")?;
            }
            Err(err) => return Err(err),
        }
        frame.line(36);
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
        rt::builtins::print(py, (&value, &list))?;
        frame.line(37);
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
        frame.line(40);
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
        frame.line(51);
        rt::builtins::print(py, (m.global("Node")?.getattr("__annotations__")?,))?;
        frame.line(52);
        rt::builtins::print(
            py,
            (m.global("typing")?
                .getattr("get_type_hints")?
                .call1((m.global("Node")?,))?,),
        )?;
        frame.line(53);
        rt::builtins::print(
            py,
            (m.global("Node")?
                .getattr("link")?
                .getattr("__annotations__")?,),
        )?;
        frame.line(54);
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
        frame.line(55);
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
        frame.line(56);
        rt::builtins::print(
            py,
            (
                m.global("Node")?
                    .getattr("__annotate__")?
                    .getattr("__qualname__")?,
                m.global("Node")?.getattr("parent")?,
            ),
        )?;
        frame.line(59);
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
        frame.line(64);
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
        frame.line(66);
        match (|| m.global("Forward")?.getattr("__annotations__"))() {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyNameError>(py) => {
                let exception = frame.catch(err);
                m.set_global("err", &exception)?;
                let _handling = rt::Handling::new(&exception);
                frame.line(68);
                rt::builtins::print(py, ("NameError:", m.global("err")?))?;
                m.unbind_global("err")?;
            }
            Err(err) => return Err(err),
        }
        frame.line(71);
        m.set_global("Later", rt::class!(m, "Later"(), later::body, line = 71)?)?;
        frame.line(75);
        rt::builtins::print(py, (m.global("Forward")?.getattr("__annotations__")?,))?;
        frame.line(78);
        let decorator2 = rt::call!(m.global("dataclass")?, slots = true)?;
        frame.line(79);
        let slotted = rt::class!(
            m,
            "Slotted"(),
            slotted::body,
            line = 78,
            annotations = [("x", "int"), ("y", "int")]
        )?;
        frame.line(78);
        let slotted = decorator2.call1((slotted,))?;
        m.set_global("Slotted", slotted)?;
        frame.line(87);
        m.set_global("s", m.global("Slotted")?.call1((1,))?)?;
        frame.line(88);
        rt::builtins::print(
            py,
            (
                m.global("s")?,
                m.global("s")?.call_method0("total")?,
                m.global("Slotted")?.getattr("__slots__")?,
                rt::builtin(py, "hasattr")?.call1((m.global("s")?, "__dict__"))?,
            ),
        )?;
        frame.line(91);
        let dataclass2 = m.global("dataclass")?;
        frame.line(92);
        let child = rt::class!(
            m,
            "Child"(m.global("Item")?),
            child::body,
            line = 91,
            annotations = [("color", "str")]
        )?;
        frame.line(91);
        let child = dataclass2.call1((child,))?;
        m.set_global("Child", child)?;
        frame.line(99);
        rt::builtins::print(
            py,
            (
                m.global("Child")?.call1(("x", 2.0))?,
                m.global("Child")?
                    .call1(("x", 2.0))?
                    .call_method1("total", (2,))?,
            ),
        )?;
        frame.line(102);
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
        frame.line(106);
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
