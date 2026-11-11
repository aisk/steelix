use pyo3::exceptions::PyAttributeError;
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::pyclass::CompareOp;
use pyo3::types::PyIterator;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

pub mod point {
    use super::*;

    // classes_objects.py:8
    /// The body of `class Point`, run in its namespace `ns`.
    #[rt::frame(ns, "Point", 8)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::def!(ns, __init__(slf as "self", x, y))?;
        rt::def!(ns, __eq__(slf as "self", other))?;
        ns.set("__hash__", m.py().None())?;
        rt::def!(ns, __repr__(slf as "self"))?;
        Ok(())
    }

    // classes_objects.py:9
    #[rt::frame(m, "Point.__init__", 9)]
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        x: &Bound<'py, PyAny>,
        y: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        slf.set_attr("x", x)?;
        slf.set_attr("y", y)?;
        Ok(rt::none(m.py()))
    }

    // classes_objects.py:13
    #[rt::frame(m, "Point.__eq__", 13)]
    pub fn __eq__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        other: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        Ok(if other.is_instance(&m.global("Point")?)? {
            PyTuple::new(py, [&slf.attr("x")?, &slf.attr("y")?])?.rich_compare(
                PyTuple::new(py, [&other.attr("x")?, &other.attr("y")?])?,
                CompareOp::Eq,
            )?
        } else {
            rt::bool(py, false)
        })
    }

    // classes_objects.py:18
    #[rt::frame(m, "Point.__repr__", 18)]
    pub fn __repr__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::fstring(
            m.py(),
            (
                "Point(",
                slf.attr("x")?.repr()?,
                ", ",
                slf.attr("y")?.repr()?,
                ")",
            ),
        )
    }
}

pub mod slots {
    use super::*;

    // classes_objects.py:22
    /// The body of `class Slots`, run in its namespace `ns`.
    #[rt::frame(ns, "Slots", 22)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        ns.set("__slots__", PyTuple::new(m.py(), ["a", "b"])?)?;
        rt::def!(ns, __init__(slf as "self", a))?;
        rt::def!(ns, __reduce__(slf as "self"))?;
        Ok(())
    }

    // classes_objects.py:25
    #[rt::frame(m, "Slots.__init__", 25)]
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        a: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        slf.set_attr("a", a)?;
        Ok(rt::none(m.py()))
    }

    // classes_objects.py:28
    #[rt::frame(m, "Slots.__reduce__", 28)]
    pub fn __reduce__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        Ok(PyTuple::new(
            py,
            [
                &m.global("Slots")?,
                PyTuple::new(py, [&slf.attr("a")?])?.as_any(),
            ],
        )?
        .into_any())
    }
}

pub mod tree {
    use super::*;

    // classes_objects.py:32
    /// The body of `class Tree`, run in its namespace `ns`.
    #[rt::frame(ns, "Tree", 32)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::def!(ns, __init__(slf as "self", value, *children))?;
        rt::def!(ns, __deepcopy__(slf as "self", memo))?;
        rt::def!(ns, total(slf as "self"))?;
        Ok(())
    }

    // classes_objects.py:33
    #[rt::frame(m, "Tree.__init__", 33)]
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        value: &Bound<'py, PyAny>,
        children: &Bound<'py, PyTuple>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        slf.set_attr("value", value)?;
        slf.set_attr("children", rt::builtins::list(py, (children,))?)?;
        Ok(rt::none(py))
    }

    // classes_objects.py:37
    #[rt::frame(m, "Tree.__deepcopy__", 37)]
    pub fn __deepcopy__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        memo: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::builtins::print(m.py(), ("deepcopy", slf.attr("value")?))?;
        rt::call!(
            m.global("Tree")?,
            slf.attr("value")?,
            *m.global("copy")?
                .attr("deepcopy")?
                .call1((slf.attr("children")?, memo))?
        )
    }

    // classes_objects.py:41
    #[rt::frame(m, "Tree.total", 41, frame)]
    pub fn total<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        // classes_objects.py:42
        #[rt::frame(m, "Tree.total.<locals>.<genexpr>", 42)]
        fn genexpr<'py>(
            m: &rt::Module<'py>,
            children: &Bound<'py, PyIterator>,
        ) -> PyResult<Option<Bound<'py, PyAny>>> {
            if let Some(child) = rt::next(children)? {
                return Ok(Some(child.method0("total")?));
            }
            Ok(None)
        }
        Ok(if false {
            slf.attr("value")?.add(rt::builtins::sum(
                py,
                (rt::genexpr!(
                    frame,
                    genexpr(slf.attr("children")?.try_iter()?)
                )?,),
            )?)?
        } else {
            slf.attr("value")?.add(rt::builtins::sum(
                py,
                (rt::builtins::map(
                    py,
                    (m.global("Tree")?.attr("total")?, slf.attr("children")?),
                )?,),
            )?)?
        })
    }
}

/// Executes the body of `classes_objects.py` in the module `m`.
#[rt::frame(m, "<module>", 1, frame)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    m.set_global(
        "__doc__",
        "Instances of transpiled classes behave like Python ones: pickling,\n\
            copying, `__slots__`, comparisons and `isinstance`.",
    )?;
    m.set_global("copy", rt::import(m, "copy")?)?;
    m.set_global("pickle", rt::import(m, "pickle")?)?;
    m.set_global(
        "Point",
        rt::class!(m, "Point"(), point::body, line = 8, attributes = ["x", "y"])?,
    )?;
    m.set_global(
        "Slots",
        rt::class!(m, "Slots"(), slots::body, line = 22, attributes = ["a"])?,
    )?;
    m.set_global(
        "Tree",
        rt::class!(
            m,
            "Tree"(),
            tree::body,
            line = 32,
            attributes = ["children", "value"]
        )?,
    )?;
    m.set_global(
        "p",
        m.global("Point")?.call1((1, PyList::new(py, [2, 3])?))?,
    )?;
    m.set_global(
        "q",
        m.global("pickle")?.attr("loads")?.call1((m
            .global("pickle")?
            .attr("dumps")?
            .call1((m.global("p")?,))?,))?,
    )?;
    rt::builtins::print(
        py,
        (
            m.global("q")?,
            m.global("q")?.rich_compare(m.global("p")?, CompareOp::Eq)?,
            m.global("q")?.is(m.global("p")?),
            m.global("q")?.get_type().is(m.global("Point")?),
        ),
    )?;
    m.set_global(
        "r",
        m.global("copy")?
            .attr("deepcopy")?
            .call1((m.global("p")?,))?,
    )?;
    m.global("r")?.attr("y")?.method1("append", (4,))?;
    rt::builtins::print(
        py,
        (
            m.global("p")?,
            m.global("r")?,
            m.global("copy")?
                .attr("copy")?
                .call1((m.global("p")?,))?
                .attr("y")?
                .is(m.global("p")?.attr("y")?),
        ),
    )?;
    match (|| rt::builtins::hash(py, (m.global("p")?,)))() {
        Ok(_) => {}
        Err(err) if err.is_instance_of::<PyTypeError>(py) => {
            let exception = frame.catch(err);
            m.set_global("err", &exception)?;
            rt::builtins::print(py, ("TypeError:", m.global("err")?))?;
            m.unbind_global("err")?;
        }
        Err(err) => return Err(err),
    }
    m.set_global("s", m.global("Slots")?.call1((5,))?)?;
    rt::builtins::print(
        py,
        (
            m.global("s")?.attr("a")?,
            rt::builtins::hasattr(py, (m.global("s")?, "__dict__"))?,
            m.global("pickle")?
                .attr("loads")?
                .call1((m
                    .global("pickle")?
                    .attr("dumps")?
                    .call1((m.global("s")?,))?,))?
                .attr("a")?,
            m.global("Slots")?.attr("__slots__")?,
        ),
    )?;
    match (|| -> PyResult<()> {
        m.global("s")?.set_attr("c", 1)?;
        Ok(())
    })() {
        Ok(()) => {}
        Err(err) if err.is_instance_of::<PyAttributeError>(py) => {
            let exception = frame.catch(err);
            m.set_global("err", &exception)?;
            rt::builtins::print(py, ("AttributeError:", m.global("err")?))?;
            m.unbind_global("err")?;
        }
        Err(err) => return Err(err),
    }
    m.set_global(
        "t",
        m.global("Tree")?.call1((
            1,
            m.global("Tree")?.call1((2,))?,
            m.global("Tree")?
                .call1((3, m.global("Tree")?.call1((4,))?))?,
        ))?,
    )?;
    m.set_global(
        "u",
        m.global("copy")?
            .attr("deepcopy")?
            .call1((m.global("t")?,))?,
    )?;
    rt::builtins::print(
        py,
        (
            m.global("u")?.method0("total")?,
            m.global("u")?
                .attr("children")?
                .get_item(1)?
                .attr("children")?
                .get_item(0)?
                .attr("value")?,
            !m.global("u")?.is(m.global("t")?),
        ),
    )?;
    rt::builtins::print(
        py,
        (m.global("pickle")?
            .attr("loads")?
            .call1((m
                .global("pickle")?
                .attr("dumps")?
                .call1((m.global("Point")?.attr("__init__")?,))?,))?
            .is(m.global("Point")?.attr("__init__")?),),
    )?;
    rt::builtins::print(
        py,
        (
            m.global("p")?.is_instance(&m.global("Point")?)?,
            m.global("s")?.is_instance(&m.global("Point")?)?,
            rt::builtins::issubclass(py, (m.global("Point")?, rt::builtin(py, "object")?))?,
            m.global("p")?.get_type().attr("__mro__")?,
        ),
    )?;
    Ok(())
}
