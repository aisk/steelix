use pyo3::prelude::*;
use pyo3::types::IntoPyDict;
use pyo3::types::PyList;
use pyo3::types::PySet;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// match_classes.py:35
pub fn r#where<'py>(m: &rt::Module<'py>, point: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "where", 35, |_| {
        if let Some([item, item2]) = rt::match_class(point, &m.global("Point")?, 2, [])?
            && item.eq(0)?
            && item2.eq(0)?
        {
            Ok(rt::str(py, "origin"))
        } else if let Some([item, y]) = rt::match_class(point, &m.global("Point")?, 2, [])?
            && item.eq(0)?
        {
            rt::fstring(py, ("on the y axis at ", rt::format(&y, "")?))
        } else if let Some([x, item]) = rt::match_class(point, &m.global("Point")?, 2, [])?
            && item.eq(0)?
        {
            rt::fstring(py, ("on the x axis at ", rt::format(&x, "")?))
        } else if let Some([x, y]) = rt::match_class(point, &m.global("Point")?, 0, ["x", "y"])?
            && x.eq(&y)?
        {
            rt::fstring(py, ("diagonal ", rt::format(&x, "")?))
        } else if let Some([x, y]) = rt::match_class(point, &m.global("Point")?, 1, ["y"])? {
            rt::fstring(
                py,
                (
                    "somewhere (",
                    rt::format(&x, "")?,
                    ", ",
                    rt::format(&y, "")?,
                    ")",
                ),
            )
        } else if rt::match_instance(point, &m.global("Point")?)? {
            Ok(rt::str(py, "a point"))
        } else {
            Ok(rt::str(py, "not a point"))
        }
    })
}

// match_classes.py:57
pub fn area<'py>(m: &rt::Module<'py>, shape: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "area", 57, |_| {
        if let Some([item, r]) = rt::match_class(shape, &m.global("Circle")?, 2, [])?
            && let Some([x, y]) = rt::match_class(&item, &m.global("Point")?, 2, [])?
            && r.gt(0)?
        {
            return rt::fstring(
                py,
                (
                    "circle at ",
                    rt::format(&x, "")?,
                    ",",
                    rt::format(&y, "")?,
                    ": ",
                    rt::format(
                        &rt::builtin(py, "round")?
                            .call1((rt::float(py, 3.14159).mul(&r)?.mul(&r)?, 2))?,
                        "",
                    )?,
                ),
            );
        } else if let Some([radius]) = rt::match_class(shape, &m.global("Circle")?, 0, ["radius"])?
            && radius.eq(0)?
        {
            return Ok(rt::str(py, "a dot"));
        } else if let Some([item, w, h]) = rt::match_class(shape, &m.global("Rect")?, 3, [])?
            && let Some([x0, y0]) = rt::match_class(&item, &m.global("Point")?, 0, ["x", "y"])?
        {
            return rt::fstring(
                py,
                (
                    "rect from ",
                    rt::format(&x0, "")?,
                    ",",
                    rt::format(&y0, "")?,
                    ": ",
                    rt::format(&w.mul(&h)?, "")?,
                ),
            );
        } else if matches!(rt::match_class(shape, &m.global("Rect")?, 0, ["width"])?, Some([width]) if width.eq(0)?)
            || matches!(rt::match_class(shape, &m.global("Rect")?, 0, ["height"])?, Some([height]) if height.eq(0)?)
            || matches!(rt::match_class(shape, &m.global("Circle")?, 0, ["radius"])?, Some([radius]) if radius.eq(-1)?)
        {
            return Ok(rt::str(py, "flat"));
        } else if let Some([x2]) = rt::match_class(shape, &m.global("Point")?, 0, ["x"])?
            && (rt::match_instance(&x2, &m.global("Point")?)?
                || rt::match_instance(&x2, &rt::builtin(py, "int")?)?)
        {
            return Ok(rt::str(py, "point with a point or int x"));
        }
        Ok(rt::str(py, "unknown"))
    })
}

// match_classes.py:78
pub fn paint<'py>(m: &rt::Module<'py>, color: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "paint", 78, |_| {
        if color.eq(m.global("Color")?.attr("RED")?)? {
            return Ok(rt::str(py, "warm"));
        } else if color.eq(m.global("Color")?.attr("GREEN")?)?
            || color.eq(m.global("Color")?.attr("BLUE")?)?
        {
            return Ok(rt::str(py, "cool"));
        } else if rt::match_instance(color, &m.global("Color")?)? {
            return Ok(rt::str(py, "another color"));
        }
        Ok(rt::str(py, "not a color"))
    })
}

// match_classes.py:93
/// `bool(x)`, `int(x)`, `str(x)`, ...: a single positional subpattern
/// matches the subject itself.
pub fn kind<'py>(m: &rt::Module<'py>, value: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "kind", 93, |_| {
        if let Some([b]) = rt::match_class(value, &rt::builtin(py, "bool")?, 1, [])? {
            return rt::fstring(py, ("bool ", rt::format(&b, "")?));
        } else if matches!(rt::match_class(value, &rt::builtin(py, "int")?, 1, [])?, Some([item]) if item.eq(0)?)
            || matches!(rt::match_class(value, &rt::builtin(py, "float")?, 1, [])?, Some([item2]) if item2.eq(0.0)?)
        {
            return Ok(rt::str(py, "zero"));
        } else if let Some([n]) = rt::match_class(value, &rt::builtin(py, "int")?, 1, [])?
            && n.lt(0)?
        {
            return rt::fstring(py, ("negative int ", rt::format(&n, "")?));
        } else if let Some([n]) = rt::match_class(value, &rt::builtin(py, "int")?, 1, [])? {
            return rt::fstring(py, ("int ", rt::format(&n, "")?));
        } else if let Some([f]) = rt::match_class(value, &rt::builtin(py, "float")?, 1, [])? {
            return rt::fstring(py, ("float ", rt::format(&f, "")?));
        } else if matches!(rt::match_class(value, &rt::builtin(py, "str")?, 1, [])?, Some([item]) if item.eq("")?)
            || matches!(rt::match_class(value, &rt::builtin(py, "bytes")?, 1, [])?, Some([item2]) if item2.eq(rt::bytes(py, b""))?)
        {
            return Ok(rt::str(py, "empty text"));
        } else if let Some([s]) = rt::match_class(value, &rt::builtin(py, "str")?, 1, [])? {
            return rt::fstring(py, ("str ", s.repr()?));
        } else if let Some([b]) = rt::match_class(value, &rt::builtin(py, "bytearray")?, 1, [])? {
            return rt::fstring(
                py,
                (
                    "bytearray ",
                    rt::builtin(py, "bytes")?.call1((&b,))?.repr()?,
                ),
            );
        } else if matches!(rt::match_class(value, &rt::builtin(py, "list")?, 1, [])?, Some([item]) if rt::match_seq_len(&item, 0)?)
            || matches!(rt::match_class(value, &rt::builtin(py, "tuple")?, 1, [])?, Some([item2]) if rt::match_seq_len(&item2, 0)?)
        {
            return Ok(rt::str(py, "empty sequence"));
        } else if let Some([item]) = rt::match_class(value, &rt::builtin(py, "list")?, 1, [])?
            && let Some(([first], rest, [])) = rt::match_seq_star(&item)?
        {
            return rt::fstring(
                py,
                (
                    "list starting with ",
                    rt::format(&first, "")?,
                    ", ",
                    rt::format(&rt::int(py, rest.len()?), "")?,
                    " more",
                ),
            );
        } else if let Some([t]) = rt::match_class(value, &rt::builtin(py, "tuple")?, 1, [])? {
            return rt::fstring(py, ("tuple of ", rt::format(&rt::int(py, t.len()?), "")?));
        } else if let Some([item]) = rt::match_class(value, &rt::builtin(py, "dict")?, 1, [])?
            && let Some([v]) = rt::match_map(&item, ["k"])?
        {
            return rt::fstring(py, ("dict with k=", rt::format(&v, "")?));
        } else if rt::match_instance(value, &rt::builtin(py, "set")?)?
            || rt::match_instance(value, &rt::builtin(py, "frozenset")?)?
        {
            return Ok(rt::str(py, "a set"));
        }
        Ok(rt::str(py, "something else"))
    })
}

pub mod point {
    use super::*;

    // match_classes.py:8
    /// The body of `class Point`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Point", 8, |_| {
            ns.set("__match_args__", PyTuple::new(m.py(), ["x", "y"])?)?;
            rt::def!(ns, __init__(slf as "self", x, y))?;
            Ok(())
        })
    }

    // match_classes.py:11
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        x: &Bound<'py, PyAny>,
        y: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Point.__init__", 11, |_| {
            slf.set_attr("x", x)?;
            slf.set_attr("y", y)?;
            Ok(rt::none(m.py()))
        })
    }
}

pub mod circle {
    use super::*;

    // match_classes.py:17
    /// The body of `class Circle`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        Ok(())
    }
}

pub mod rect {
    use super::*;

    // match_classes.py:23
    /// The body of `class Rect`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        Ok(())
    }
}

pub mod color {
    use super::*;

    // match_classes.py:29
    /// The body of `class Color`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Color", 29, |_| {
            ns.set("RED", 1)?;
            ns.set("GREEN", 2)?;
            ns.set("BLUE", 3)?;
            Ok(())
        })
    }
}

/// A subclass of a builtin without `__match_args__` matches itself too.
pub mod sub {
    use super::*;

    // match_classes.py:132
    /// The body of `class Sub`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Sub", 132, |_| {
            ns.set(
                "__doc__",
                "A subclass of a builtin without `__match_args__` matches itself too.",
            )?;
            Ok(())
        })
    }
}

pub mod named {
    use super::*;

    // match_classes.py:136
    /// The body of `class Named`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Named", 136, |_| {
            ns.set("__match_args__", PyTuple::new(m.py(), ["real", "imag"])?)?;
            Ok(())
        })
    }
}

pub mod missing {
    use super::*;

    // match_classes.py:149
    /// The body of `class Missing`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Missing", 149, |_| {
            ns.set("__match_args__", PyTuple::new(m.py(), ["a", "b"])?)?;
            rt::def!(ns, __init__(slf as "self"))?;
            Ok(())
        })
    }

    // match_classes.py:152
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Missing.__init__", 152, |_| {
            slf.set_attr("a", 1)?;
            Ok(rt::none(m.py()))
        })
    }
}

/// Executes the body of `match_classes.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |_| {
        // Class patterns (design §8): `__match_args__`, keyword attributes,
        // dataclasses, enums as value patterns and the builtins that match themselves.
        m.set_global("enum", rt::import(m, "enum")?)?;
        let dataclasses = rt::import_names(m, "dataclasses", &["dataclass"])?;
        m.set_global("dataclass", rt::import_from(&dataclasses, "dataclass")?)?;
        m.set_global(
            "Point",
            rt::class!(m, "Point"(), point::body, line = 8, attributes = ["x", "y"])?,
        )?;
        {
            let dataclass = m.global("dataclass")?;
            let circle = rt::class!(
                m,
                "Circle"(),
                circle::body,
                line = 16,
                annotations = [("center", "Point"), ("radius", "float")]
            )?;
            m.set_global("Circle", dataclass.call1((circle,))?)?;
        }
        {
            let dataclass = m.global("dataclass")?;
            let rect = rt::class!(
                m,
                "Rect"(),
                rect::body,
                line = 22,
                annotations = [("corner", "Point"), ("width", "float"), ("height", "float")]
            )?;
            m.set_global("Rect", dataclass.call1((rect,))?)?;
        }
        m.set_global(
            "Color",
            rt::class!(
                m,
                "Color"(m.global("enum")?.attr("Enum")?),
                color::body,
                line = 29
            )?,
        )?;
        rt::def!(m, r#where(point))?;
        for item in PyList::new(
            py,
            [
                &m.global("Point")?.call1((0, 0))?,
                &m.global("Point")?.call1((0, 5))?,
                &m.global("Point")?.call1((3, 0))?,
                &m.global("Point")?.call1((2, 2))?,
                &m.global("Point")?.call1((1, 7))?,
                &rt::str(py, "p"),
            ],
        )?
        .try_iter()?
        {
            m.set_global("p", item?)?;
            rt::builtins::print(py, (r#where(m, &m.global("p")?)?,))?;
            py.check_signals()?;
        }
        rt::def!(m, area(shape))?;
        m.set_global(
            "shapes",
            PyList::new(
                py,
                [
                    &m.global("Circle")?
                        .call1((m.global("Point")?.call1((1, 2))?, 2))?,
                    &m.global("Circle")?
                        .call1((m.global("Point")?.call1((0, 0))?, 0))?,
                    &m.global("Rect")?
                        .call1((m.global("Point")?.call1((1, 1))?, 2, 3))?,
                ],
            )?,
        )?;
        m.set_global(
            "shapes",
            rt::iadd(
                &m.global("shapes")?,
                PyList::new(
                    py,
                    [
                        &m.global("Rect")?
                            .call1((m.global("Point")?.call1((1, 1))?, 0, 3))?,
                        &m.global("Point")?.call1((4, 4))?,
                        &m.global("Point")?
                            .call1((m.global("Point")?.call1((0, 0))?, 1))?,
                        &m.global("Point")?.call1(("x", 1))?,
                    ],
                )?,
            )?,
        )?;
        for item2 in m.global("shapes")?.try_iter()? {
            m.set_global("shape", item2?)?;
            rt::builtins::print(py, (area(m, &m.global("shape")?)?,))?;
            py.check_signals()?;
        }
        rt::def!(m, paint(color))?;
        for item3 in PyList::new(
            py,
            [
                &m.global("Color")?.attr("RED")?,
                &m.global("Color")?.attr("BLUE")?,
                &rt::int(py, 1),
                &rt::str(py, "red"),
            ],
        )?
        .try_iter()?
        {
            m.set_global("color", item3?)?;
            rt::builtins::print(py, (paint(m, &m.global("color")?)?,))?;
            py.check_signals()?;
        }
        rt::def!(
            m,
            kind(value),
            doc = "`bool(x)`, `int(x)`, `str(x)`, ...: a single positional subpattern\n\
                matches the subject itself."
        )?;
        m.set_global(
            "values",
            PyList::new(
                py,
                [
                    &rt::bool(py, true),
                    &rt::int(py, 0),
                    &rt::float(py, 0.0),
                    &rt::int(py, -4),
                    &rt::int(py, 7),
                    &rt::float(py, 2.5),
                    &rt::str(py, ""),
                    &rt::bytes(py, b""),
                    &rt::str(py, "hi"),
                    &rt::builtin(py, "bytearray")?.call1((rt::bytes(py, b"ab"),))?,
                    PyList::new(py, [1, 2, 3])?.as_any(),
                    PyTuple::new(py, [1, 2])?.as_any(),
                    PyList::empty(py).as_any(),
                ],
            )?,
        )?;
        m.set_global(
            "values",
            rt::iadd(
                &m.global("values")?,
                PyList::new(
                    py,
                    [
                        PyTuple::empty(py).as_any(),
                        [("k", 9)].into_py_dict(py)?.as_any(),
                        PySet::new(py, [1])?.as_any(),
                        &rt::builtin(py, "frozenset")?.call0()?,
                        &rt::none(py),
                    ],
                )?,
            )?,
        )?;
        for item4 in m.global("values")?.try_iter()? {
            m.set_global("value", item4?)?;
            rt::builtins::print(
                py,
                (
                    m.global("value")?.repr()?,
                    "->",
                    kind(m, &m.global("value")?)?,
                ),
            )?;
            py.check_signals()?;
        }
        m.set_global(
            "Sub",
            rt::class!(m, "Sub"(rt::builtin(py, "int")?), sub::body, line = 132)?,
        )?;
        m.set_global(
            "Named",
            rt::class!(m, "Named"(rt::builtin(py, "int")?), named::body, line = 136)?,
        )?;
        {
            let subject = m.global("Sub")?.call1((5,))?;
            if let Some([n]) = rt::match_class(&subject, &rt::builtin(py, "int")?, 1, [])? {
                m.set_global("n", &n)?;
                rt::builtins::print(
                    py,
                    (
                        "Sub matched as int",
                        m.global("n")?,
                        m.global("n")?.get_type().attr("__name__")?,
                    ),
                )?;
            }
        }
        {
            let subject = m.global("Named")?.call1((5,))?;
            if let Some([real, imag]) = rt::match_class(&subject, &m.global("Named")?, 2, [])? {
                m.set_global("real", &real)?;
                m.set_global("imag", &imag)?;
                rt::builtins::print(
                    py,
                    (
                        "Named by __match_args__",
                        m.global("real")?,
                        m.global("imag")?,
                    ),
                )?;
            }
        }
        m.set_global(
            "Missing",
            rt::class!(
                m,
                "Missing"(),
                missing::body,
                line = 149,
                attributes = ["a"]
            )?,
        )?;
        {
            let subject = m.global("Missing")?.call0()?;
            if let Some([a, b]) = rt::match_class(&subject, &m.global("Missing")?, 2, [])? {
                m.set_global("a", &a)?;
                m.set_global("b", &b)?;
                rt::builtins::print(py, ("both",))?;
            } else if let Some([a]) = rt::match_class(&subject, &m.global("Missing")?, 1, [])? {
                m.set_global("a", &a)?;
                rt::builtins::print(py, ("only a", m.global("a")?))?;
            }
        }
        {
            let subject = m.global("Missing")?.call0()?;
            if let Some([c]) = rt::match_class(&subject, &m.global("Missing")?, 0, ["c"])?
                && c.eq(1)?
            {
                rt::builtins::print(py, ("c",))?;
            } else if let Some([a]) = rt::match_class(&subject, &m.global("Missing")?, 0, ["a"])?
                && a.eq(1)?
            {
                rt::builtins::print(py, ("a is 1",))?;
            }
        }
        Ok(())
    })
}
