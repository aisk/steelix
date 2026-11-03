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
        rt::frame!(ns, "Vector", 6, |_| {
            ns.set("__doc__", "A 2D vector.")?;
            ns.set("scale", 2)?;
            ns.set("instances", 0)?;
            ns.set(
                "__init__",
                rt::function!(
                    ns,
                    __init__(slf as "self", x, y = 0),
                    qualname = "Vector.__init__"
                )?,
            )?;
            ns.set(
                "__repr__",
                rt::function!(
                    ns,
                    __repr__(slf as "self"),
                    qualname = "Vector.__repr__"
                )?,
            )?;
            ns.set(
                "__eq__",
                rt::function!(
                    ns,
                    __eq__(slf as "self", other),
                    qualname = "Vector.__eq__"
                )?,
            )?;
            ns.set(
                "__hash__",
                rt::function!(
                    ns,
                    __hash__(slf as "self"),
                    qualname = "Vector.__hash__"
                )?,
            )?;
            ns.set(
                "__add__",
                rt::function!(
                    ns,
                    __add__(slf as "self", other),
                    qualname = "Vector.__add__"
                )?,
            )?;
            ns.set(
                "scaled",
                rt::function!(ns, scaled(slf as "self"), qualname = "Vector.scaled")?,
            )?;
            ns.set(
                "class_scaled",
                rt::function!(
                    ns,
                    class_scaled(slf as "self"),
                    qualname = "Vector.class_scaled"
                )?,
            )?;
            {
                let decorator = ns.get("property")?;
                let function =
                    rt::function!(ns, length(slf as "self"), qualname = "Vector.length")?;
                ns.set("length", decorator.call1((function,))?)?;
            }
            {
                let decorator = ns.get("property")?;
                let function = rt::function!(ns, label(slf as "self"), qualname = "Vector.label")?;
                ns.set("label", decorator.call1((function,))?)?;
            }
            {
                let decorator = ns.get("label")?.attr("setter")?;
                let function = rt::function!(
                    ns,
                    label_(slf as "self", value),
                    qualname = "Vector.label"
                )?;
                ns.set("label", decorator.call1((function,))?)?;
            }
            {
                let decorator = ns.get("label")?.attr("deleter")?;
                let function =
                    rt::function!(ns, label__(slf as "self"), qualname = "Vector.label")?;
                ns.set("label", decorator.call1((function,))?)?;
            }
            {
                let decorator = ns.get("classmethod")?;
                let function = rt::function!(ns, zero(cls), qualname = "Vector.zero")?;
                ns.set("zero", decorator.call1((function,))?)?;
            }
            {
                let decorator = ns.get("staticmethod")?;
                let function = rt::function!(ns, dot(a, b), qualname = "Vector.dot")?;
                ns.set("dot", decorator.call1((function,))?)?;
            }
            ns.set(
                "__len__",
                rt::function!(ns, __len__(slf as "self"), qualname = "Vector.__len__")?,
            )?;
            ns.set(
                "__iter__",
                rt::function!(
                    ns,
                    __iter__(slf as "self"),
                    qualname = "Vector.__iter__"
                )?,
            )?;
            ns.set(
                "__getitem__",
                rt::function!(
                    ns,
                    __getitem__(slf as "self", index),
                    qualname = "Vector.__getitem__"
                )?,
            )?;
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
        rt::frame!(m, "Vector.__init__", 12, |_| {
            slf.set_attr("x", x)?;
            slf.set_attr("y", y)?;
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
        rt::frame!(m, "Vector.__repr__", 17, |_| {
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
        rt::frame!(m, "Vector.__eq__", 20, |_| {
            if !other.is_instance(&m.global("Vector")?)? {
                return rt::builtin(py, "NotImplemented");
            }
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
        rt::frame!(m, "Vector.__hash__", 25, |_| {
            rt::builtin(py, "hash")?.call1((PyTuple::new(py, [&slf.attr("x")?, &slf.attr("y")?])?,))
        })
    }

    // classes_basic.py:28
    pub fn __add__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        other: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Vector.__add__", 28, |_| {
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
        rt::frame!(m, "Vector.scaled", 31, |_| {
            // The global `scale`, not the class attribute: class scope is not
            // visible in methods.
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
        rt::frame!(m, "Vector.class_scaled", 36, |_| {
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
        rt::frame!(m, "Vector.length", 39, |_| {
            slf.attr("x")?
                .pow(2, py.None())?
                .add(slf.attr("y")?.pow(2, py.None())?)?
                .pow(0.5, py.None())
        })
    }

    // classes_basic.py:44
    pub fn label<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Vector.label", 43, |_| { slf.attr("_label") })
    }

    // classes_basic.py:48
    pub fn label_<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        value: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "Vector.label", 47, |_| {
            rt::builtins::print(py, ("setting label", value))?;
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
        rt::frame!(m, "Vector.label", 52, |_| {
            rt::builtins::print(py, ("deleting label",))?;
            slf.del_attr("_label")?;
            Ok(rt::none(py))
        })
    }

    // classes_basic.py:58
    pub fn zero<'py>(m: &rt::Module<'py>, cls: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Vector.zero", 57, |_| { cls.call1((0, 0)) })
    }

    // classes_basic.py:62
    pub fn dot<'py>(
        m: &rt::Module<'py>,
        a: &Bound<'py, PyAny>,
        b: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Vector.dot", 61, |_| {
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
        rt::frame!(m, "Vector.__iter__", 68, |_| {
            let yield_values = PyList::new(m.py(), [&slf.attr("x")?, &slf.attr("y")?])?.into_any();
            Ok(yield_values.try_iter()?.into_any())
        })
    }

    // classes_basic.py:72
    pub fn __getitem__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        index: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Vector.__getitem__", 72, |_| {
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
        rt::frame!(m, "Vector.__bool__", 75, |_| {
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
        rt::frame!(ns, "Counter", 98, |_| {
            ns.set("count", 0)?;
            ns.set(
                "increment",
                rt::function!(
                    ns,
                    increment(slf as "self", by = 1),
                    qualname = "Counter.increment"
                )?,
            )?;
            ns.set(
                "__call__",
                rt::function!(
                    ns,
                    __call__(slf as "self", *args, **kwargs),
                    qualname = "Counter.__call__"
                )?,
            )?;
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
        rt::frame!(m, "Counter.increment", 101, |_| {
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
        rt::frame!(m, "Counter.__call__", 105, |_| {
            Ok(PyTuple::new(m.py(), [args.as_any(), kwargs.as_any()])?.into_any())
        })
    }

    // classes_basic.py:108
    pub fn __str__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Counter.__str__", 108, |_| {
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
    rt::frame!(m, "<module>", 1, |_| {
        m.set_global(
            "__doc__",
            "Methods, properties, class and static methods, special methods.",
        )?;
        m.set_global("scale", 10)?;
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
        m.set_global("v", m.global("Vector")?.call1((3, 4))?)?;
        m.set_global("w", m.global("Vector")?.call1((1,))?)?;
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
        m.global("v")?.set_attr("label", "a")?;
        rt::builtins::print(py, (m.global("v")?.attr("label")?,))?;
        m.global("v")?.del_attr("label")?;
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
        rt::builtins::print(
            py,
            (
                m.global("Vector")?.attr("__init__")?.attr("__qualname__")?,
                m.global("Vector")?.attr("__init__")?.attr("__name__")?,
                m.global("Vector")?.attr("__init__")?.attr("__defaults__")?,
            ),
        )?;
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
        rt::builtins::print(
            py,
            (rt::builtin(py, "sorted")?.call1((rt::genexpr!(
                m,
                genexpr(m.global("Vector")?.attr("__dict__")?.try_iter()?)
            )?,))?,),
        )?;
        rt::builtins::print(
            py,
            (
                m.global("v")?.attr("__dict__")?,
                rt::builtin(py, "vars")?.call1((m.global("w")?,))?,
            ),
        )?;
        m.set_global("bound", m.global("v")?.attr("scaled")?)?;
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
        m.set_global(
            "Counter",
            rt::class!(m, "Counter"(), counter::body, line = 98)?,
        )?;
        m.set_global(
            "c",
            m.global("Counter")?
                .call0()?
                .method0("increment")?
                .method1("increment", (5,))?,
        )?;
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
        m.set_global("Empty", rt::class!(m, "Empty"(), empty::body, line = 116)?)?;
        m.set_global("e", m.global("Empty")?.call0()?)?;
        m.global("e")?.set_attr("attr", 1)?;
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
