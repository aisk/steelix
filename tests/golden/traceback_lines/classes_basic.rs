use pyo3::prelude::*;
use pyo3::pyclass::CompareOp;
use pyo3::types::PyDict;
use pyo3::types::PyIterator;
use pyo3::types::PyList;
use pyo3::types::PySet;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

/// A 2D vector.
pub mod vector {
    use super::*;

    // classes_basic.py:6
    /// The body of `class Vector`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Vector", 6, |frame| {
            frame.line(7);
            ns.set("__doc__", "A 2D vector.")?;
            frame.line(9);
            ns.set("scale", 2)?;
            frame.line(10);
            ns.set("instances", 0)?;
            frame.line(12);
            ns.set(
                "__init__",
                rt::function!(
                    ns,
                    __init__(slf as "self", x, y = 0),
                    qualname = "Vector.__init__"
                )?,
            )?;
            frame.line(17);
            ns.set(
                "__repr__",
                rt::function!(
                    ns,
                    __repr__(slf as "self"),
                    qualname = "Vector.__repr__"
                )?,
            )?;
            frame.line(20);
            ns.set(
                "__eq__",
                rt::function!(
                    ns,
                    __eq__(slf as "self", other),
                    qualname = "Vector.__eq__"
                )?,
            )?;
            frame.line(25);
            ns.set(
                "__hash__",
                rt::function!(
                    ns,
                    __hash__(slf as "self"),
                    qualname = "Vector.__hash__"
                )?,
            )?;
            frame.line(28);
            ns.set(
                "__add__",
                rt::function!(
                    ns,
                    __add__(slf as "self", other),
                    qualname = "Vector.__add__"
                )?,
            )?;
            frame.line(31);
            ns.set(
                "scaled",
                rt::function!(ns, scaled(slf as "self"), qualname = "Vector.scaled")?,
            )?;
            frame.line(36);
            ns.set(
                "class_scaled",
                rt::function!(
                    ns,
                    class_scaled(slf as "self"),
                    qualname = "Vector.class_scaled"
                )?,
            )?;
            frame.line(40);
            {
                let decorator = ns.get("property")?;
                let function =
                    rt::function!(ns, length(slf as "self"), qualname = "Vector.length")?;
                ns.set("length", decorator.call1((function,))?)?;
            }
            frame.line(44);
            {
                let decorator = ns.get("property")?;
                let function = rt::function!(ns, label(slf as "self"), qualname = "Vector.label")?;
                ns.set("label", decorator.call1((function,))?)?;
            }
            frame.line(48);
            {
                let decorator = ns.get("label")?.attr("setter")?;
                let function = rt::function!(
                    ns,
                    label_(slf as "self", value),
                    qualname = "Vector.label"
                )?;
                ns.set("label", decorator.call1((function,))?)?;
            }
            frame.line(53);
            {
                let decorator = ns.get("label")?.attr("deleter")?;
                let function =
                    rt::function!(ns, label__(slf as "self"), qualname = "Vector.label")?;
                ns.set("label", decorator.call1((function,))?)?;
            }
            frame.line(58);
            {
                let decorator = ns.get("classmethod")?;
                let function = rt::function!(ns, zero(cls), qualname = "Vector.zero")?;
                ns.set("zero", decorator.call1((function,))?)?;
            }
            frame.line(62);
            {
                let decorator = ns.get("staticmethod")?;
                let function = rt::function!(ns, dot(a, b), qualname = "Vector.dot")?;
                ns.set("dot", decorator.call1((function,))?)?;
            }
            frame.line(65);
            ns.set(
                "__len__",
                rt::function!(ns, __len__(slf as "self"), qualname = "Vector.__len__")?,
            )?;
            frame.line(68);
            ns.set(
                "__iter__",
                rt::function!(
                    ns,
                    __iter__(slf as "self"),
                    qualname = "Vector.__iter__"
                )?,
            )?;
            frame.line(72);
            ns.set(
                "__getitem__",
                rt::function!(
                    ns,
                    __getitem__(slf as "self", index),
                    qualname = "Vector.__getitem__"
                )?,
            )?;
            frame.line(75);
            ns.set(
                "__bool__",
                rt::function!(
                    ns,
                    __bool__(slf as "self"),
                    qualname = "Vector.__bool__"
                )?,
            )?;
            Ok(())
        })
    }

    // classes_basic.py:12
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        x: &Bound<'py, PyAny>,
        y: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Vector.__init__", 12, |frame| {
            frame.line(13);
            slf.set_attr("x", x)?;
            frame.line(14);
            slf.set_attr("y", y)?;
            frame.line(15);
            {
                let Vector = m.global("Vector")?;
                Vector.set_attr("instances", rt::iadd(&Vector.attr("instances")?, 1)?)?;
            }
            Ok(rt::none(m.py()))
        })
    }

    // classes_basic.py:17
    pub fn __repr__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Vector.__repr__", 17, |frame| {
            frame.line(18);
            rt::fstring(
                m.py(),
                (
                    "Vector(",
                    rt::format(&slf.attr("x")?, "")?,
                    ", ",
                    rt::format(&slf.attr("y")?, "")?,
                    ")",
                ),
            )
        })
    }

    // classes_basic.py:20
    pub fn __eq__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        other: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "Vector.__eq__", 20, |frame| {
            frame.line(21);
            if !other.is_instance(&m.global("Vector")?)? {
                frame.line(22);
                return rt::builtin(py, "NotImplemented");
            }
            frame.line(23);
            PyTuple::new(py, [&slf.attr("x")?, &slf.attr("y")?])?.rich_compare(
                PyTuple::new(py, [&other.attr("x")?, &other.attr("y")?])?,
                CompareOp::Eq,
            )
        })
    }

    // classes_basic.py:25
    pub fn __hash__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "Vector.__hash__", 25, |frame| {
            frame.line(26);
            rt::builtin(py, "hash")?.call1((PyTuple::new(py, [&slf.attr("x")?, &slf.attr("y")?])?,))
        })
    }

    // classes_basic.py:28
    pub fn __add__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        other: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Vector.__add__", 28, |frame| {
            frame.line(29);
            m.global("Vector")?.call1((
                slf.attr("x")?.add(other.attr("x")?)?,
                slf.attr("y")?.add(other.attr("y")?)?,
            ))
        })
    }

    // classes_basic.py:31
    pub fn scaled<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Vector.scaled", 31, |frame| {
            // The global `scale`, not the class attribute: class scope is not
            // visible in methods.
            frame.line(34);
            m.global("Vector")?.call1((
                slf.attr("x")?.mul(m.global("scale")?)?,
                slf.attr("y")?.mul(m.global("scale")?)?,
            ))
        })
    }

    // classes_basic.py:36
    pub fn class_scaled<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Vector.class_scaled", 36, |frame| {
            frame.line(37);
            m.global("Vector")?.call1((
                slf.attr("x")?.mul(slf.attr("scale")?)?,
                slf.attr("y")?.mul(slf.get_type().attr("scale")?)?,
            ))
        })
    }

    // classes_basic.py:40
    pub fn length<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "Vector.length", 39, |frame| {
            frame.line(41);
            slf.attr("x")?
                .pow(2, py.None())?
                .add(slf.attr("y")?.pow(2, py.None())?)?
                .pow(0.5, py.None())
        })
    }

    // classes_basic.py:44
    pub fn label<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Vector.label", 43, |frame| {
            frame.line(45);
            slf.attr("_label")
        })
    }

    // classes_basic.py:48
    pub fn label_<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        value: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "Vector.label", 47, |frame| {
            frame.line(49);
            rt::builtins::print(py, ("setting label", value))?;
            frame.line(50);
            slf.set_attr("_label", value)?;
            Ok(rt::none(py))
        })
    }

    // classes_basic.py:53
    pub fn label__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "Vector.label", 52, |frame| {
            frame.line(54);
            rt::builtins::print(py, ("deleting label",))?;
            frame.line(55);
            slf.del_attr("_label")?;
            Ok(rt::none(py))
        })
    }

    // classes_basic.py:58
    pub fn zero<'py>(m: &rt::Module<'py>, cls: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Vector.zero", 57, |frame| {
            frame.line(59);
            cls.call1((0, 0))
        })
    }

    // classes_basic.py:62
    pub fn dot<'py>(
        m: &rt::Module<'py>,
        a: &Bound<'py, PyAny>,
        b: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Vector.dot", 61, |frame| {
            frame.line(63);
            a.attr("x")?
                .mul(b.attr("x")?)?
                .add(a.attr("y")?.mul(b.attr("y")?)?)
        })
    }

    // classes_basic.py:65
    pub fn __len__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(rt::int(m.py(), 2))
    }

    // classes_basic.py:68
    pub fn __iter__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Vector.__iter__", 68, |frame| {
            frame.line(69);
            let yield_values = PyList::new(m.py(), [&slf.attr("x")?, &slf.attr("y")?])?.into_any();
            frame.line(70);
            Ok(yield_values.try_iter()?.into_any())
        })
    }

    // classes_basic.py:72
    pub fn __getitem__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        index: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Vector.__getitem__", 72, |frame| {
            frame.line(73);
            PyTuple::new(m.py(), [&slf.attr("x")?, &slf.attr("y")?])?
                .as_any()
                .get_item(index)
        })
    }

    // classes_basic.py:75
    pub fn __bool__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Vector.__bool__", 75, |frame| {
            frame.line(76);
            let x = slf.attr("x")?;
            Ok(rt::bool(
                m.py(),
                (if x.is_truthy()? { x } else { slf.attr("y")? }).is_truthy()?,
            ))
        })
    }
}

pub mod counter {
    use super::*;

    // classes_basic.py:98
    /// The body of `class Counter`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Counter", 98, |frame| {
            frame.line(99);
            ns.set("count", 0)?;
            frame.line(101);
            ns.set(
                "increment",
                rt::function!(
                    ns,
                    increment(slf as "self", by = 1),
                    qualname = "Counter.increment"
                )?,
            )?;
            frame.line(105);
            ns.set(
                "__call__",
                rt::function!(
                    ns,
                    __call__(slf as "self", *args, **kwargs),
                    qualname = "Counter.__call__"
                )?,
            )?;
            frame.line(108);
            ns.set(
                "__str__",
                rt::function!(
                    ns,
                    __str__(slf as "self"),
                    qualname = "Counter.__str__"
                )?,
            )?;
            Ok(())
        })
    }

    // classes_basic.py:101
    pub fn increment<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        by: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Counter.increment", 101, |frame| {
            frame.line(102);
            slf.set_attr("count", rt::iadd(&slf.attr("count")?, by)?)?;
            Ok(slf.clone())
        })
    }

    // classes_basic.py:105
    pub fn __call__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        args: &Bound<'py, PyTuple>,
        kwargs: &Bound<'py, PyDict>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Counter.__call__", 105, |frame| {
            frame.line(106);
            Ok(PyTuple::new(m.py(), [args.as_any(), kwargs.as_any()])?.into_any())
        })
    }

    // classes_basic.py:108
    pub fn __str__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Counter.__str__", 108, |frame| {
            frame.line(109);
            rt::str(m.py(), "Counter(%d)").rem(slf.attr("count")?)
        })
    }
}

pub mod empty {
    use super::*;

    // classes_basic.py:116
    /// The body of `class Empty`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        Ok(())
    }
}

/// Executes the body of `classes_basic.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        m.set_global(
            "__doc__",
            "Methods, properties, class and static methods, special methods.",
        )?;
        frame.line(3);
        m.set_global("scale", 10)?;
        frame.line(6);
        m.set_global(
            "Vector",
            rt::class!(
                m,
                "Vector"(),
                vector::body,
                line = 6,
                attributes = ["_label", "x", "y"]
            )?,
        )?;
        frame.line(79);
        m.set_global("v", m.global("Vector")?.call1((3, 4))?)?;
        frame.line(80);
        m.set_global("w", m.global("Vector")?.call1((1,))?)?;
        frame.line(81);
        rt::builtins::print(
            py,
            (
                m.global("v")?,
                m.global("w")?,
                m.global("v")?.add(m.global("w")?)?,
                m.global("v")?
                    .rich_compare(m.global("Vector")?.call1((3, 4))?, CompareOp::Eq)?,
                m.global("v")?.rich_compare(m.global("w")?, CompareOp::Ne)?,
                m.global("v")?.rich_compare(3, CompareOp::Eq)?,
            ),
        )?;
        frame.line(82);
        rt::builtins::print(
            py,
            (
                rt::builtin(py, "hash")?
                    .call1((m.global("v")?,))?
                    .rich_compare(
                        rt::builtin(py, "hash")?.call1((m.global("Vector")?.call1((3, 4))?,))?,
                        CompareOp::Eq,
                    )?,
                PySet::new(
                    py,
                    [
                        &m.global("v")?,
                        &m.global("Vector")?.call1((3, 4))?,
                        &m.global("w")?,
                    ],
                )?
                .len(),
            ),
        )?;
        frame.line(83);
        rt::builtins::print(
            py,
            (
                m.global("v")?.attr("length")?,
                m.global("v")?.method0("scaled")?,
                m.global("v")?.method0("class_scaled")?,
                m.global("Vector")?.method0("zero")?,
                m.global("Vector")?
                    .attr("dot")?
                    .call1((m.global("v")?, m.global("w")?))?,
            ),
        )?;
        frame.line(84);
        m.global("v")?.set_attr("label", "a")?;
        frame.line(85);
        rt::builtins::print(py, (m.global("v")?.attr("label")?,))?;
        frame.line(86);
        m.global("v")?.del_attr("label")?;
        frame.line(87);
        rt::builtins::print(
            py,
            (
                rt::builtin(py, "hasattr")?.call1((m.global("v")?, "_label"))?,
                m.global("v")?.len()?,
                rt::builtin(py, "list")?.call1((m.global("v")?,))?,
                m.global("v")?.get_item(1)?,
                m.global("v")?.is_truthy()?,
                m.global("Vector")?.method0("zero")?.is_truthy()?,
            ),
        )?;
        frame.line(88);
        rt::builtins::print(
            py,
            (
                m.global("Vector")?.attr("instances")?,
                m.global("Vector")?.attr("__doc__")?,
                m.global("Vector")?.attr("__name__")?,
                m.global("Vector")?.attr("__qualname__")?,
                m.global("Vector")?.attr("__module__")?,
            ),
        )?;
        frame.line(89);
        rt::builtins::print(
            py,
            (
                m.global("Vector")?.attr("__init__")?.attr("__qualname__")?,
                m.global("Vector")?.attr("__init__")?.attr("__name__")?,
                m.global("Vector")?.attr("__init__")?.attr("__defaults__")?,
            ),
        )?;
        frame.line(90);
        rt::builtins::print(
            py,
            (
                m.global("Vector")?
                    .attr("__dict__")?
                    .get_item("zero")?
                    .get_type()
                    .attr("__name__")?,
                m.global("Vector")?
                    .attr("__dict__")?
                    .get_item("dot")?
                    .get_type()
                    .attr("__name__")?,
            ),
        )?;
        frame.line(91);
        rt::builtins::print(
            py,
            (
                m.global("Vector")?.attr("__static_attributes__")?,
                m.global("Vector")?.attr("__firstlineno__")?,
            ),
        )?;
        // classes_basic.py:92
        fn genexpr<'py>(
            m: &rt::Module<'py>,
            __dict__: &Bound<'py, PyIterator>,
        ) -> PyResult<Option<Bound<'py, PyAny>>> {
            rt::frame!(m, "<genexpr>", 92, |_| {
                for k in __dict__ {
                    let k = k?;
                    if !k.method1("startswith", ("__",))?.is_truthy()? {
                        return Ok(Some(k));
                    }
                    m.py().check_signals()?;
                }
                Ok(None)
            })
        }
        frame.line(92);
        rt::builtins::print(
            py,
            (rt::builtin(py, "sorted")?.call1((rt::genexpr!(
                m,
                genexpr(m.global("Vector")?.attr("__dict__")?.try_iter()?)
            )?,))?,),
        )?;
        frame.line(93);
        rt::builtins::print(
            py,
            (
                m.global("v")?.attr("__dict__")?,
                rt::builtin(py, "vars")?.call1((m.global("w")?,))?,
            ),
        )?;
        frame.line(94);
        m.set_global("bound", m.global("v")?.attr("scaled")?)?;
        frame.line(95);
        rt::builtins::print(
            py,
            (
                m.global("bound")?.attr("__self__")?.is(m.global("v")?),
                m.global("bound")?
                    .attr("__func__")?
                    .is(m.global("Vector")?.attr("scaled")?),
                m.global("bound")?.call0()?,
            ),
        )?;
        frame.line(98);
        m.set_global(
            "Counter",
            rt::class!(m, "Counter"(), counter::body, line = 98)?,
        )?;
        frame.line(112);
        m.set_global(
            "c",
            m.global("Counter")?
                .call0()?
                .method0("increment")?
                .method1("increment", (5,))?,
        )?;
        frame.line(113);
        rt::builtins::print(
            py,
            (
                m.global("c")?,
                m.global("Counter")?.attr("count")?,
                m.global("c")?.attr("count")?,
                rt::call!(m.global("c")?, 1, 2, k = 3)?,
                m.global("c")?.str()?,
                m.global("c")?
                    .repr()?
                    .get_item(rt::slice(py, py.None(), 8)?)?,
            ),
        )?;
        frame.line(116);
        m.set_global("Empty", rt::class!(m, "Empty"(), empty::body, line = 116)?)?;
        frame.line(120);
        m.set_global("e", m.global("Empty")?.call0()?)?;
        frame.line(121);
        m.global("e")?.set_attr("attr", 1)?;
        frame.line(122);
        rt::builtins::print(
            py,
            (
                m.global("e")?.get_type().attr("__name__")?,
                m.global("e")?.attr("attr")?,
                m.global("Empty")?.attr("__doc__")?,
                m.global("Empty")?.attr("__bases__")?,
                m.global("Empty")?.attr("__static_attributes__")?,
            ),
        )?;
        Ok(())
    })
}
