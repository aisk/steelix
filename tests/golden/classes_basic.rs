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
    #[rt::frame(ns, "Vector", 6)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        ns.set("__doc__", "A 2D vector.")?;
        ns.set("scale", 2)?;
        ns.set("instances", 0)?;
        rt::def!(ns, __init__(slf as "self", x, y = 0))?;
        rt::def!(ns, __repr__(slf as "self"))?;
        rt::def!(ns, __eq__(slf as "self", other))?;
        rt::def!(ns, __hash__(slf as "self"))?;
        rt::def!(ns, __add__(slf as "self", other))?;
        rt::def!(ns, scaled(slf as "self"))?;
        rt::def!(ns, class_scaled(slf as "self"))?;
        rt::def!(ns, @property length(slf as "self"))?;
        rt::def!(ns, @property label(slf as "self"))?;
        rt::def!(ns, @label.setter label_ as "label"(slf as "self", value))?;
        rt::def!(ns, @label.deleter label__ as "label"(slf as "self"))?;
        rt::def!(ns, @classmethod zero(cls))?;
        rt::def!(ns, @staticmethod dot(a, b))?;
        rt::def!(ns, __len__(slf as "self"))?;
        rt::def!(ns, __iter__(slf as "self"))?;
        rt::def!(ns, __getitem__(slf as "self", index))?;
        rt::def!(ns, __bool__(slf as "self"))?;
        Ok(())
    }

    // classes_basic.py:12
    #[rt::frame(m, "Vector.__init__", 12)]
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        x: &Bound<'py, PyAny>,
        y: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        slf.set_attr("x", x)?;
        slf.set_attr("y", y)?;
        {
            let Vector = m.global("Vector")?;
            Vector.set_attr("instances", rt::iadd(&Vector.attr("instances")?, 1)?)?;
        }
        Ok(rt::none(m.py()))
    }

    // classes_basic.py:17
    #[rt::frame(m, "Vector.__repr__", 17)]
    pub fn __repr__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
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
    }

    // classes_basic.py:20
    #[rt::frame(m, "Vector.__eq__", 20)]
    pub fn __eq__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        other: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        if !other.is_instance(&m.global("Vector")?)? {
            return rt::builtin(py, "NotImplemented");
        }
        PyTuple::new(py, [&slf.attr("x")?, &slf.attr("y")?])?.rich_compare(
            PyTuple::new(py, [&other.attr("x")?, &other.attr("y")?])?,
            CompareOp::Eq,
        )
    }

    // classes_basic.py:25
    #[rt::frame(m, "Vector.__hash__", 25)]
    pub fn __hash__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::builtins::hash(py, (PyTuple::new(py, [&slf.attr("x")?, &slf.attr("y")?])?,))
    }

    // classes_basic.py:28
    #[rt::frame(m, "Vector.__add__", 28)]
    pub fn __add__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        other: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        m.global("Vector")?.call1((
            slf.attr("x")?.add(other.attr("x")?)?,
            slf.attr("y")?.add(other.attr("y")?)?,
        ))
    }

    // classes_basic.py:31
    #[rt::frame(m, "Vector.scaled", 31)]
    pub fn scaled<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        // The global `scale`, not the class attribute: class scope is not
        // visible in methods.
        m.global("Vector")?.call1((
            slf.attr("x")?.mul(m.global("scale")?)?,
            slf.attr("y")?.mul(m.global("scale")?)?,
        ))
    }

    // classes_basic.py:36
    #[rt::frame(m, "Vector.class_scaled", 36)]
    pub fn class_scaled<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        m.global("Vector")?.call1((
            slf.attr("x")?.mul(slf.attr("scale")?)?,
            slf.attr("y")?.mul(slf.get_type().attr("scale")?)?,
        ))
    }

    // classes_basic.py:39
    #[rt::frame(m, "Vector.length", 39)]
    pub fn length<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        slf.attr("x")?
            .pow(2, py.None())?
            .add(slf.attr("y")?.pow(2, py.None())?)?
            .pow(0.5, py.None())
    }

    // classes_basic.py:43
    #[rt::frame(m, "Vector.label", 43)]
    pub fn label<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        slf.attr("_label")
    }

    // classes_basic.py:47
    #[rt::frame(m, "Vector.label", 47)]
    pub fn label_<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        value: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::builtins::print(py, ("setting label", value))?;
        slf.set_attr("_label", value)?;
        Ok(rt::none(py))
    }

    // classes_basic.py:52
    #[rt::frame(m, "Vector.label", 52)]
    pub fn label__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::builtins::print(py, ("deleting label",))?;
        slf.del_attr("_label")?;
        Ok(rt::none(py))
    }

    // classes_basic.py:57
    #[rt::frame(m, "Vector.zero", 57)]
    pub fn zero<'py>(m: &rt::Module<'py>, cls: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        cls.call1((0, 0))
    }

    // classes_basic.py:61
    #[rt::frame(m, "Vector.dot", 61)]
    pub fn dot<'py>(
        m: &rt::Module<'py>,
        a: &Bound<'py, PyAny>,
        b: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        a.attr("x")?
            .mul(b.attr("x")?)?
            .add(a.attr("y")?.mul(b.attr("y")?)?)
    }

    // classes_basic.py:65
    pub fn __len__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(rt::int(m.py(), 2))
    }

    // classes_basic.py:68
    #[rt::frame(m, "Vector.__iter__", 68)]
    pub fn __iter__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let yield_values = PyList::new(m.py(), [&slf.attr("x")?, &slf.attr("y")?])?.into_any();
        Ok(yield_values.try_iter()?.into_any())
    }

    // classes_basic.py:72
    #[rt::frame(m, "Vector.__getitem__", 72)]
    pub fn __getitem__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        index: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        PyTuple::new(m.py(), [&slf.attr("x")?, &slf.attr("y")?])?
            .as_any()
            .get_item(index)
    }

    // classes_basic.py:75
    #[rt::frame(m, "Vector.__bool__", 75)]
    pub fn __bool__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let x = slf.attr("x")?;
        Ok(rt::bool(
            m.py(),
            (if x.is_truthy()? { x } else { slf.attr("y")? }).is_truthy()?,
        ))
    }
}

pub mod counter {
    use super::*;

    // classes_basic.py:98
    /// The body of `class Counter`, run in its namespace `ns`.
    #[rt::frame(ns, "Counter", 98)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        ns.set("count", 0)?;
        rt::def!(ns, increment(slf as "self", by = 1))?;
        rt::def!(ns, __call__(slf as "self", *args, **kwargs))?;
        rt::def!(ns, __str__(slf as "self"))?;
        Ok(())
    }

    // classes_basic.py:101
    #[rt::frame(m, "Counter.increment", 101)]
    pub fn increment<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        by: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        slf.set_attr("count", rt::iadd(&slf.attr("count")?, by)?)?;
        Ok(slf.clone())
    }

    // classes_basic.py:105
    #[rt::frame(m, "Counter.__call__", 105)]
    pub fn __call__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        args: &Bound<'py, PyTuple>,
        kwargs: &Bound<'py, PyDict>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(PyTuple::new(m.py(), [args.as_any(), kwargs.as_any()])?.into_any())
    }

    // classes_basic.py:108
    #[rt::frame(m, "Counter.__str__", 108)]
    pub fn __str__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::str(m.py(), "Counter(%d)").rem(slf.attr("count")?)
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
#[rt::frame(m, "<module>", 1)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
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
            rt::builtins::hash(py, (m.global("v")?,))?.rich_compare(
                rt::builtins::hash(py, (m.global("Vector")?.call1((3, 4))?,))?,
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
            rt::builtins::hasattr(py, (m.global("v")?, "_label"))?,
            m.global("v")?.len()?,
            rt::builtins::list(py, (m.global("v")?,))?,
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
    #[rt::frame(m, "<genexpr>", 92)]
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
                genexpr(m.global("Vector")?.attr("__dict__")?.try_iter()?)
            )?,),
        )?,),
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
}
