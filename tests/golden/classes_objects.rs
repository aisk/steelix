use pyo3::exceptions::PyAttributeError;
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::pyclass::CompareOp;
use pyo3::types::PyIterator;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;

pub mod point {
    use super::*;

    // classes_objects.py:8
    /// The body of `class Point`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Point", 8, |frame| {
            frame.line(9);
            ns.set(
                "__init__",
                rt::function!(
                    ns,
                    __init__(slf as "self", x, y),
                    qualname = "Point.__init__"
                )?,
            )?;
            frame.line(13);
            ns.set(
                "__eq__",
                rt::function!(
                    ns,
                    __eq__(slf as "self", other),
                    qualname = "Point.__eq__"
                )?,
            )?;
            frame.line(16);
            ns.set("__hash__", m.py().None())?;
            frame.line(18);
            ns.set(
                "__repr__",
                rt::function!(
                    ns,
                    __repr__(slf as "self"),
                    qualname = "Point.__repr__"
                )?,
            )?;
            Ok(())
        })
    }

    // classes_objects.py:9
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        x: &Bound<'py, PyAny>,
        y: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Point.__init__", 9, |frame| {
            frame.line(10);
            slf.setattr("x", x)?;
            frame.line(11);
            slf.setattr("y", y)?;
            Ok(rt::none(m.py()))
        })
    }

    // classes_objects.py:13
    pub fn __eq__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        other: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "Point.__eq__", 13, |frame| {
            frame.line(14);
            Ok(if other.is_instance(&m.global("Point")?)? {
                PyTuple::new(py, [&slf.getattr("x")?, &slf.getattr("y")?])?.rich_compare(
                    PyTuple::new(py, [&other.getattr("x")?, &other.getattr("y")?])?,
                    CompareOp::Eq,
                )?
            } else {
                rt::bool(py, false)
            })
        })
    }

    // classes_objects.py:18
    pub fn __repr__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Point.__repr__", 18, |frame| {
            frame.line(19);
            rt::fstring(
                m.py(),
                (
                    "Point(",
                    slf.getattr("x")?.repr()?,
                    ", ",
                    slf.getattr("y")?.repr()?,
                    ")",
                ),
            )
        })
    }
}

pub mod slots {
    use super::*;

    // classes_objects.py:22
    /// The body of `class Slots`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Slots", 22, |frame| {
            frame.line(23);
            ns.set("__slots__", PyTuple::new(m.py(), ["a", "b"])?)?;
            frame.line(25);
            ns.set(
                "__init__",
                rt::function!(
                    ns,
                    __init__(slf as "self", a),
                    qualname = "Slots.__init__"
                )?,
            )?;
            frame.line(28);
            ns.set(
                "__reduce__",
                rt::function!(
                    ns,
                    __reduce__(slf as "self"),
                    qualname = "Slots.__reduce__"
                )?,
            )?;
            Ok(())
        })
    }

    // classes_objects.py:25
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        a: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Slots.__init__", 25, |frame| {
            frame.line(26);
            slf.setattr("a", a)?;
            Ok(rt::none(m.py()))
        })
    }

    // classes_objects.py:28
    pub fn __reduce__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "Slots.__reduce__", 28, |frame| {
            frame.line(29);
            Ok(PyTuple::new(
                py,
                [
                    &m.global("Slots")?,
                    PyTuple::new(py, [&slf.getattr("a")?])?.as_any(),
                ],
            )?
            .into_any())
        })
    }
}

pub mod tree {
    use super::*;

    // classes_objects.py:32
    /// The body of `class Tree`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Tree", 32, |frame| {
            frame.line(33);
            ns.set(
                "__init__",
                rt::function!(
                    ns,
                    __init__(slf as "self", value, *children),
                    qualname = "Tree.__init__"
                )?,
            )?;
            frame.line(37);
            ns.set(
                "__deepcopy__",
                rt::function!(
                    ns,
                    __deepcopy__(slf as "self", memo),
                    qualname = "Tree.__deepcopy__"
                )?,
            )?;
            frame.line(41);
            ns.set(
                "total",
                rt::function!(ns, total(slf as "self"), qualname = "Tree.total")?,
            )?;
            Ok(())
        })
    }

    // classes_objects.py:33
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        value: &Bound<'py, PyAny>,
        children: &Bound<'py, PyTuple>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "Tree.__init__", 33, |frame| {
            frame.line(34);
            slf.setattr("value", value)?;
            frame.line(35);
            slf.setattr("children", rt::builtin(py, "list")?.call1((children,))?)?;
            Ok(rt::none(py))
        })
    }

    // classes_objects.py:37
    pub fn __deepcopy__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        memo: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Tree.__deepcopy__", 37, |frame| {
            frame.line(38);
            rt::builtins::print(m.py(), ("deepcopy", slf.getattr("value")?))?;
            frame.line(39);
            rt::call!(
                m.global("Tree")?,
                slf.getattr("value")?,
                *m.global("copy")?
                    .getattr("deepcopy")?
                    .call1((slf.getattr("children")?, memo))?
            )
        })
    }

    // classes_objects.py:41
    pub fn total<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "Tree.total", 41, |frame| {
            // classes_objects.py:42
            fn genexpr<'py>(
                m: &rt::Module<'py>,
                children: &Bound<'py, PyIterator>,
            ) -> PyResult<Option<Bound<'py, PyAny>>> {
                rt::frame!(m, "Tree.total.<locals>.<genexpr>", 42, |_| {
                    if let Some(child) = rt::next(children)? {
                        return Ok(Some(child.call_method0("total")?));
                    }
                    Ok(None)
                })
            }
            frame.line(42);
            Ok(if false {
                slf.getattr("value")?
                    .add(rt::builtin(py, "sum")?.call1((rt::genexpr!(
                        m,
                        genexpr(slf.getattr("children")?.try_iter()?),
                        qualname = "Tree.total.<locals>.<genexpr>"
                    )?,))?)?
            } else {
                slf.getattr("value")?.add(rt::builtin(py, "sum")?.call1((
                    rt::builtin(py, "map")?.call1((
                        m.global("Tree")?.getattr("total")?,
                        slf.getattr("children")?,
                    ))?,
                ))?)?
            })
        })
    }
}

/// Executes the body of `classes_objects.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        m.set_global("__doc__", "Instances of transpiled classes behave like Python ones: pickling,\ncopying, `__slots__`, comparisons and `isinstance`.")?;
        frame.line(4);
        m.set_global("copy", rt::import(m, "copy")?)?;
        frame.line(5);
        m.set_global("pickle", rt::import(m, "pickle")?)?;
        frame.line(8);
        m.set_global(
            "Point",
            rt::class!(m, "Point"(), point::body, line = 8, attributes = ["x", "y"])?,
        )?;
        frame.line(22);
        m.set_global(
            "Slots",
            rt::class!(m, "Slots"(), slots::body, line = 22, attributes = ["a"])?,
        )?;
        frame.line(32);
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
        frame.line(47);
        m.set_global(
            "p",
            m.global("Point")?.call1((1, PyList::new(py, [2, 3])?))?,
        )?;
        frame.line(48);
        m.set_global(
            "q",
            m.global("pickle")?.getattr("loads")?.call1((m
                .global("pickle")?
                .getattr("dumps")?
                .call1((m.global("p")?,))?,))?,
        )?;
        frame.line(49);
        rt::builtins::print(
            py,
            (
                m.global("q")?,
                m.global("q")?.rich_compare(m.global("p")?, CompareOp::Eq)?,
                m.global("q")?.is(m.global("p")?),
                m.global("q")?.get_type().is(m.global("Point")?),
            ),
        )?;
        frame.line(50);
        m.set_global(
            "r",
            m.global("copy")?
                .getattr("deepcopy")?
                .call1((m.global("p")?,))?,
        )?;
        frame.line(51);
        m.global("r")?.getattr("y")?.call_method1("append", (4,))?;
        frame.line(52);
        rt::builtins::print(
            py,
            (
                m.global("p")?,
                m.global("r")?,
                m.global("copy")?
                    .getattr("copy")?
                    .call1((m.global("p")?,))?
                    .getattr("y")?
                    .is(m.global("p")?.getattr("y")?),
            ),
        )?;
        frame.line(54);
        match (|| rt::builtin(py, "hash")?.call1((m.global("p")?,)))() {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyTypeError>(py) => {
                let exception = frame.catch(err);
                m.set_global("err", &exception)?;
                let _handling = rt::Handling::new(&exception);
                frame.line(56);
                rt::builtins::print(py, ("TypeError:", m.global("err")?))?;
                m.unbind_global("err")?;
            }
            Err(err) => return Err(err),
        }
        frame.line(57);
        m.set_global("s", m.global("Slots")?.call1((5,))?)?;
        frame.line(58);
        rt::builtins::print(
            py,
            (
                m.global("s")?.getattr("a")?,
                rt::builtin(py, "hasattr")?.call1((m.global("s")?, "__dict__"))?,
                m.global("pickle")?
                    .getattr("loads")?
                    .call1((m
                        .global("pickle")?
                        .getattr("dumps")?
                        .call1((m.global("s")?,))?,))?
                    .getattr("a")?,
                m.global("Slots")?.getattr("__slots__")?,
            ),
        )?;
        match (|| -> PyResult<()> {
            frame.line(60);
            m.global("s")?.setattr("c", 1)?;
            Ok(())
        })() {
            Ok(()) => {}
            Err(err) if err.is_instance_of::<PyAttributeError>(py) => {
                let exception = frame.catch(err);
                m.set_global("err", &exception)?;
                let _handling = rt::Handling::new(&exception);
                frame.line(62);
                rt::builtins::print(py, ("AttributeError:", m.global("err")?))?;
                m.unbind_global("err")?;
            }
            Err(err) => return Err(err),
        }
        frame.line(63);
        m.set_global(
            "t",
            m.global("Tree")?.call1((
                1,
                m.global("Tree")?.call1((2,))?,
                m.global("Tree")?
                    .call1((3, m.global("Tree")?.call1((4,))?))?,
            ))?,
        )?;
        frame.line(64);
        m.set_global(
            "u",
            m.global("copy")?
                .getattr("deepcopy")?
                .call1((m.global("t")?,))?,
        )?;
        frame.line(65);
        rt::builtins::print(
            py,
            (
                m.global("u")?.call_method0("total")?,
                m.global("u")?
                    .getattr("children")?
                    .get_item(1)?
                    .getattr("children")?
                    .get_item(0)?
                    .getattr("value")?,
                !m.global("u")?.is(m.global("t")?),
            ),
        )?;
        frame.line(66);
        rt::builtins::print(
            py,
            (m.global("pickle")?
                .getattr("loads")?
                .call1((m
                    .global("pickle")?
                    .getattr("dumps")?
                    .call1((m.global("Point")?.getattr("__init__")?,))?,))?
                .is(m.global("Point")?.getattr("__init__")?),),
        )?;
        frame.line(67);
        rt::builtins::print(
            py,
            (
                m.global("p")?.is_instance(&m.global("Point")?)?,
                m.global("s")?.is_instance(&m.global("Point")?)?,
                rt::builtin(py, "issubclass")?
                    .call1((m.global("Point")?, rt::builtin(py, "object")?))?,
                m.global("p")?.get_type().getattr("__mro__")?,
            ),
        )?;
        Ok(())
    })
}
