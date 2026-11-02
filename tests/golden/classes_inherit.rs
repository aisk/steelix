//! Parts of `classes_inherit.py` kept as Python source and run in place by
//! `rt::exec_python!` (steelix does not lower them yet):
//!
//! - line 30, `expr` statement: list comprehension
//! - line 148, `assign` statement: list comprehension
//! - line 153, `expr` statement: list comprehension

use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::pyclass::CompareOp;
use pyo3::types::PyList;
use pyo3::types::PySet;
use pyo3::types::PyTuple;
use steelix_runtime as rt;

pub mod a {
    use super::*;

    // classes_inherit.py:10
    /// The body of `class A`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "A", 10, |frame| {
            frame.line(11);
            ns.set(
                "who",
                rt::function!(ns, who(slf as "self"), qualname = "A.who")?,
            )?;
            Ok(())
        })
    }

    // classes_inherit.py:11
    pub fn who<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "A.who", 11, |frame| {
            frame.line(12);
            Ok(PyList::new(m.py(), ["A"])?.into_any())
        })
    }
}

pub mod b {
    use super::*;

    // classes_inherit.py:15
    /// The body of `class B`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "B", 15, |frame| {
            let __class__ = ns.class_cell();
            frame.line(16);
            ns.set(
                "who",
                rt::function!(ns, who[&__class__](slf as "self"), qualname = "B.who")?,
            )?;
            Ok(())
        })
    }

    // classes_inherit.py:16
    pub fn who<'py>(
        m: &rt::Module<'py>,
        __class__: &rt::Cell<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "B.who", 16, |frame| {
            frame.line(17);
            PyList::new(m.py(), ["B"])?.add(rt::super_(__class__, slf)?.call_method0("who")?)
        })
    }
}

pub mod c {
    use super::*;

    // classes_inherit.py:20
    /// The body of `class C`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "C", 20, |frame| {
            let __class__ = ns.class_cell();
            frame.line(21);
            ns.set(
                "who",
                rt::function!(ns, who[&__class__](slf as "self"), qualname = "C.who")?,
            )?;
            Ok(())
        })
    }

    // classes_inherit.py:21
    pub fn who<'py>(
        m: &rt::Module<'py>,
        __class__: &rt::Cell<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "C.who", 21, |frame| {
            frame.line(22);
            PyList::new(m.py(), ["C"])?.add(rt::super_(__class__, slf)?.call_method0("who")?)
        })
    }
}

pub mod d {
    use super::*;

    // classes_inherit.py:25
    /// The body of `class D`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "D", 25, |frame| {
            let __class__ = ns.class_cell();
            frame.line(26);
            ns.set(
                "who",
                rt::function!(ns, who[&__class__](slf as "self"), qualname = "D.who")?,
            )?;
            Ok(())
        })
    }

    // classes_inherit.py:26
    pub fn who<'py>(
        m: &rt::Module<'py>,
        __class__: &rt::Cell<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "D.who", 26, |frame| {
            frame.line(27);
            PyList::new(m.py(), ["D"])?.add(rt::super_(__class__, slf)?.call_method0("who")?)
        })
    }
}

pub mod color {
    use super::*;

    // classes_inherit.py:34
    /// The body of `class Color`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Color", 34, |frame| {
            frame.line(35);
            ns.set("RED", 1)?;
            frame.line(36);
            ns.set("GREEN", 2)?;
            frame.line(37);
            ns.set("BLUE", 3)?;
            frame.line(39);
            ns.set(
                "describe",
                rt::function!(
                    ns,
                    describe(slf as "self"),
                    qualname = "Color.describe"
                )?,
            )?;
            frame.line(43);
            let decorator = ns.get("classmethod")?;
            let function = rt::function!(ns, default(cls), qualname = "Color.default")?;
            ns.set("default", decorator.call1((&function,))?)?;
            frame.line(47);
            let decorator2 = ns.get("property")?;
            let function2 = rt::function!(ns, is_warm(slf as "self"), qualname = "Color.is_warm")?;
            ns.set("is_warm", decorator2.call1((&function2,))?)?;
            Ok(())
        })
    }

    // classes_inherit.py:39
    pub fn describe<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Color.describe", 39, |frame| {
            frame.line(40);
            rt::fstring(
                m.py(),
                (
                    rt::format(&slf.getattr("name")?.call_method0("lower")?, "")?,
                    "=",
                    rt::format(&slf.getattr("value")?, "")?,
                ),
            )
        })
    }

    // classes_inherit.py:43
    pub fn default<'py>(
        m: &rt::Module<'py>,
        cls: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Color.default", 42, |frame| {
            frame.line(44);
            cls.getattr("GREEN")
        })
    }

    // classes_inherit.py:47
    pub fn is_warm<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Color.is_warm", 46, |frame| {
            frame.line(48);
            Ok(rt::bool(m.py(), slf.is(m.global("Color")?.getattr("RED")?)))
        })
    }
}

pub mod planet {
    use super::*;

    // classes_inherit.py:55
    /// The body of `class Planet`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        let py = m.py();
        rt::frame!(ns, "Planet", 55, |frame| {
            frame.line(56);
            ns.set("EARTH", PyTuple::new(py, [5.97e+24, 6370000.0])?)?;
            frame.line(57);
            ns.set("MARS", PyTuple::new(py, [6.42e+23, 3390000.0])?)?;
            frame.line(59);
            ns.set(
                "__init__",
                rt::function!(
                    ns,
                    __init__(slf as "self", mass, radius),
                    qualname = "Planet.__init__"
                )?,
            )?;
            frame.line(64);
            let decorator = ns.get("property")?;
            let function = rt::function!(ns, gravity(slf as "self"), qualname = "Planet.gravity")?;
            ns.set("gravity", decorator.call1((&function,))?)?;
            Ok(())
        })
    }

    // classes_inherit.py:59
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        mass: &Bound<'py, PyAny>,
        radius: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Planet.__init__", 59, |frame| {
            frame.line(60);
            slf.setattr("mass", mass)?;
            frame.line(61);
            slf.setattr("radius", radius)?;
            Ok(rt::none(m.py()))
        })
    }

    // classes_inherit.py:64
    pub fn gravity<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "Planet.gravity", 63, |frame| {
            frame.line(65);
            rt::builtin(py, "round")?.call1((
                rt::float(py, 6.67e-11)
                    .mul(slf.getattr("mass")?)?
                    .div(slf.getattr("radius")?.pow(2, py.None())?)?,
                2,
            ))
        })
    }
}

pub mod flags {
    use super::*;

    // classes_inherit.py:71
    /// The body of `class Flags`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Flags", 71, |frame| {
            frame.line(72);
            ns.set("READ", 4)?;
            frame.line(73);
            ns.set("WRITE", 2)?;
            Ok(())
        })
    }
}

pub mod shade {
    use super::*;

    // classes_inherit.py:79
    /// The body of `class Shade`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        let py = m.py();
        rt::frame!(ns, "Shade", 79, |frame| {
            frame.line(80);
            ns.set(
                "__new__",
                rt::function!(ns, __new__(cls, value, label), qualname = "Shade.__new__")?,
            )?;
            frame.line(86);
            ns.set(
                "DARK",
                PyTuple::new(py, [&rt::int(py, 1), &rt::str(py, "dark")])?,
            )?;
            frame.line(87);
            ns.set(
                "LIGHT",
                PyTuple::new(py, [&rt::int(py, 2), &rt::str(py, "light")])?,
            )?;
            Ok(())
        })
    }

    // classes_inherit.py:80
    pub fn __new__<'py>(
        m: &rt::Module<'py>,
        cls: &Bound<'py, PyAny>,
        value: &Bound<'py, PyAny>,
        label: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Shade.__new__", 80, |frame| {
            frame.line(81);
            let obj = rt::builtin(m.py(), "object")?.call_method1("__new__", (cls,))?;
            frame.line(82);
            obj.setattr("_value_", value)?;
            frame.line(83);
            obj.setattr("label", label)?;
            Ok(obj)
        })
    }
}

pub mod point {
    use super::*;

    // classes_inherit.py:93
    /// The body of `class Point`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Point", 93, |frame| {
            frame.line(95);
            ns.set("y", 0)?;
            frame.line(97);
            ns.set(
                "norm1",
                rt::function!(ns, norm1(slf as "self"), qualname = "Point.norm1")?,
            )?;
            Ok(())
        })
    }

    // classes_inherit.py:97
    pub fn norm1<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Point.norm1", 97, |frame| {
            frame.line(98);
            slf.getattr("x")?.abs()?.add(slf.getattr("y")?.abs()?)
        })
    }
}

pub mod movie {
    use super::*;

    // classes_inherit.py:105
    /// The body of `class Movie`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        Ok(())
    }
}

pub mod shape {
    use super::*;

    // classes_inherit.py:114
    /// The body of `class Shape`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Shape", 114, |frame| {
            frame.line(116);
            let decorator = ns.get("abc")?.getattr("abstractmethod")?;
            let function = rt::function!(ns, area(slf as "self"), qualname = "Shape.area")?;
            ns.set("area", decorator.call1((&function,))?)?;
            frame.line(119);
            ns.set(
                "describe",
                rt::function!(
                    ns,
                    describe(slf as "self"),
                    qualname = "Shape.describe"
                )?,
            )?;
            Ok(())
        })
    }

    // classes_inherit.py:116
    pub fn area<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        Ok(rt::none(m.py()))
    }

    // classes_inherit.py:119
    pub fn describe<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Shape.describe", 119, |frame| {
            frame.line(120);
            rt::fstring(
                m.py(),
                (
                    rt::format(&slf.get_type().getattr("__name__")?, "")?,
                    " with area ",
                    rt::format(&slf.call_method0("area")?, "")?,
                ),
            )
        })
    }
}

pub mod square {
    use super::*;

    // classes_inherit.py:123
    /// The body of `class Square`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Square", 123, |frame| {
            frame.line(124);
            ns.set(
                "__init__",
                rt::function!(
                    ns,
                    __init__(slf as "self", side),
                    qualname = "Square.__init__"
                )?,
            )?;
            frame.line(127);
            ns.set(
                "area",
                rt::function!(ns, area(slf as "self"), qualname = "Square.area")?,
            )?;
            Ok(())
        })
    }

    // classes_inherit.py:124
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        side: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Square.__init__", 124, |frame| {
            frame.line(125);
            slf.setattr("side", side)?;
            Ok(rt::none(m.py()))
        })
    }

    // classes_inherit.py:127
    pub fn area<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Square.area", 127, |frame| {
            frame.line(128);
            slf.getattr("side")?.pow(2, m.py().None())
        })
    }
}

pub mod worker {
    use super::*;

    // classes_inherit.py:138
    /// The body of `class Worker`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Worker", 138, |frame| {
            let __class__ = ns.class_cell();
            frame.line(139);
            ns.set(
                "__init__",
                rt::function!(
                    ns,
                    __init__[&__class__](slf as "self", n),
                    qualname = "Worker.__init__"
                )?,
            )?;
            frame.line(144);
            ns.set(
                "run",
                rt::function!(ns, run(slf as "self"), qualname = "Worker.run")?,
            )?;
            Ok(())
        })
    }

    // classes_inherit.py:139
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        __class__: &rt::Cell<'py>,
        slf: &Bound<'py, PyAny>,
        n: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "Worker.__init__", 139, |frame| {
            frame.line(140);
            rt::call!(
                rt::super_(__class__, slf)?.getattr("__init__")?,
                name = rt::fstring(py, ("worker-", rt::format(n, "")?))?
            )?;
            frame.line(141);
            slf.setattr("n", n)?;
            frame.line(142);
            slf.setattr("result", py.None())?;
            Ok(rt::none(py))
        })
    }

    // classes_inherit.py:144
    pub fn run<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "Worker.run", 144, |frame| {
            frame.line(145);
            slf.setattr(
                "result",
                rt::builtin(py, "sum")?
                    .call1((rt::builtin(py, "range")?.call1((slf.getattr("n")?,))?,))?,
            )?;
            Ok(rt::none(py))
        })
    }
}

pub mod upper {
    use super::*;

    // classes_inherit.py:156
    /// The body of `class Upper`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Upper", 156, |frame| {
            frame.line(157);
            ns.set("__call__", rt::function!(
    ns,
    __call__(slf as "self", parser, namespace, values, option_string = m.py().None()),
    qualname = "Upper.__call__"
)?)?;
            Ok(())
        })
    }

    // classes_inherit.py:157
    pub fn __call__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        parser: &Bound<'py, PyAny>,
        namespace: &Bound<'py, PyAny>,
        values: &Bound<'py, PyAny>,
        option_string: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "Upper.__call__", 157, |frame| {
            frame.line(158);
            rt::builtin(py, "setattr")?.call1((
                namespace,
                slf.getattr("dest")?,
                values.call_method0("upper")?,
            ))?;
            Ok(rt::none(py))
        })
    }
}

pub mod reversed {
    use super::*;

    // classes_inherit.py:166
    /// The body of `class Reversed`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Reversed", 166, |frame| {
            let __class__ = ns.class_cell();
            frame.line(167);
            ns.set(
                "__init__",
                rt::function!(
                    ns,
                    __init__[&__class__](slf as "self", items),
                    qualname = "Reversed.__init__"
                )?,
            )?;
            frame.line(170);
            ns.set(
                "append",
                rt::function!(
                    ns,
                    append[&__class__](slf as "self", item),
                    qualname = "Reversed.append"
                )?,
            )?;
            Ok(())
        })
    }

    // classes_inherit.py:167
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        __class__: &rt::Cell<'py>,
        slf: &Bound<'py, PyAny>,
        items: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "Reversed.__init__", 167, |frame| {
            frame.line(168);
            rt::super_(__class__, slf)?
                .getattr("__init__")?
                .call1((rt::builtin(py, "reversed")?.call1((items,))?,))?;
            Ok(rt::none(py))
        })
    }

    // classes_inherit.py:170
    pub fn append<'py>(
        m: &rt::Module<'py>,
        __class__: &rt::Cell<'py>,
        slf: &Bound<'py, PyAny>,
        item: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Reversed.append", 170, |frame| {
            frame.line(171);
            rt::super_(__class__, slf)?.call_method1("insert", (0, item))?;
            Ok(rt::none(m.py()))
        })
    }
}

pub mod default {
    use super::*;

    // classes_inherit.py:179
    /// The body of `class Default`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Default", 179, |frame| {
            frame.line(180);
            ns.set(
                "__missing__",
                rt::function!(
                    ns,
                    __missing__(slf as "self", key),
                    qualname = "Default.__missing__"
                )?,
            )?;
            Ok(())
        })
    }

    // classes_inherit.py:180
    pub fn __missing__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        key: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Default.__missing__", 180, |frame| {
            frame.line(181);
            key.mul(2)
        })
    }
}

/// Executes the body of `classes_inherit.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        m.set_global(
            "__doc__",
            "Inheritance from Python-defined and standard library classes.",
        )?;
        frame.line(3);
        m.set_global("abc", rt::import(m, "abc")?)?;
        frame.line(4);
        m.set_global("argparse", rt::import(m, "argparse")?)?;
        frame.line(5);
        m.set_global("enum", rt::import(m, "enum")?)?;
        frame.line(6);
        m.set_global("threading", rt::import(m, "threading")?)?;
        frame.line(7);
        let typing = rt::import_names(m, "typing", &["NamedTuple", "TypedDict"])?;
        m.set_global("NamedTuple", rt::import_from(&typing, "NamedTuple")?)?;
        m.set_global("TypedDict", rt::import_from(&typing, "TypedDict")?)?;
        frame.line(10);
        m.set_global("A", rt::class!(m, "A"(), a::body, line = 10)?)?;
        frame.line(15);
        m.set_global("B", rt::class!(m, "B"(m.global("A")?), b::body, line = 15)?)?;
        frame.line(20);
        m.set_global("C", rt::class!(m, "C"(m.global("A")?), c::body, line = 20)?)?;
        frame.line(25);
        m.set_global(
            "D",
            rt::class!(m, "D"(m.global("B")?, m.global("C")?), d::body, line = 25)?,
        )?;
        // classes_inherit.py:30, kept as Python: list comprehension at 30:18
        rt::exec_python!(
            m,
            line = 30,
            r#"
print(D().who(), [k.__name__ for k in D.__mro__])
"#
        )?;
        frame.line(31);
        rt::builtins::print(
            py,
            (
                m.global("D")?.call0()?.is_instance(&m.global("A")?)?,
                rt::builtin(py, "issubclass")?.call1((m.global("D")?, m.global("C")?))?,
                m.global("B")?.call0()?.is_instance(&m.global("C")?)?,
            ),
        )?;
        frame.line(34);
        m.set_global(
            "Color",
            rt::class!(
                m,
                "Color"(m.global("enum")?.getattr("Enum")?),
                color::body,
                line = 34
            )?,
        )?;
        frame.line(51);
        rt::builtins::print(
            py,
            (
                m.global("Color")?.getattr("RED")?,
                m.global("Color")?.call1((2,))?,
                m.global("Color")?
                    .getattr("BLUE")?
                    .call_method0("describe")?,
                m.global("Color")?.call_method0("default")?,
                m.global("Color")?.getattr("RED")?.getattr("is_warm")?,
            ),
        )?;
        frame.line(52);
        rt::builtins::print(
            py,
            (
                rt::builtin(py, "list")?.call1((m.global("Color")?,))?,
                m.global("Color")?.get_item("BLUE")?.getattr("value")?,
                m.global("Color")?.len()?,
            ),
        )?;
        frame.line(55);
        m.set_global(
            "Planet",
            rt::class!(
                m,
                "Planet"(m.global("enum")?.getattr("Enum")?),
                planet::body,
                line = 55,
                attributes = ["mass", "radius"]
            )?,
        )?;
        frame.line(68);
        rt::builtins::print(
            py,
            (
                m.global("Planet")?.getattr("EARTH")?.getattr("gravity")?,
                m.global("Planet")?.getattr("MARS")?.getattr("gravity")?,
            ),
        )?;
        frame.line(71);
        m.set_global(
            "Flags",
            rt::class!(
                m,
                "Flags"(m.global("enum")?.getattr("IntFlag")?),
                flags::body,
                line = 71
            )?,
        )?;
        frame.line(76);
        rt::builtins::print(
            py,
            (
                m.global("Flags")?
                    .getattr("READ")?
                    .bitor(m.global("Flags")?.getattr("WRITE")?)?,
                rt::builtin(py, "int")?.call1((m.global("Flags")?.getattr("READ")?,))?,
            ),
        )?;
        frame.line(79);
        m.set_global(
            "Shade",
            rt::class!(
                m,
                "Shade"(m.global("enum")?.getattr("Enum")?),
                shade::body,
                line = 79
            )?,
        )?;
        frame.line(90);
        rt::builtins::print(
            py,
            (
                m.global("Shade")?.call1((1,))?.getattr("label")?,
                m.global("Shade")?.getattr("LIGHT")?.getattr("value")?,
            ),
        )?;
        frame.line(93);
        m.set_global(
            "Point",
            rt::class!(
                m,
                "Point"(m.global("NamedTuple")?),
                point::body,
                line = 93,
                annotations = [("x", "int"), ("y", "int")]
            )?,
        )?;
        frame.line(101);
        m.set_global("p", m.global("Point")?.call1((3, -4))?)?;
        frame.line(102);
        rt::builtins::print(
            py,
            (
                m.global("p")?,
                m.global("p")?.call_method0("norm1")?,
                m.global("Point")?.getattr("_fields")?,
                m.global("Point")?.call1((1,))?,
                rt::call!(m.global("p")?.getattr("_replace")?, y = 1)?,
            ),
        )?;
        frame.line(105);
        m.set_global(
            "Movie",
            rt::class!(
                m,
                "Movie"(m.global("TypedDict")?),
                movie::body,
                line = 105,
                annotations = [("title", "str"), ("year", "int")]
            )?,
        )?;
        frame.line(110);
        m.set_global(
            "m",
            rt::call!(m.global("Movie")?, title = "Brazil", year = 1985)?,
        )?;
        frame.line(111);
        rt::builtins::print(
            py,
            (
                m.global("m")?,
                m.global("Movie")?
                    .getattr("__required_keys__")?
                    .rich_compare(PySet::new(py, ["title", "year"])?, CompareOp::Eq)?,
                m.global("Movie")?.getattr("__annotations__")?,
            ),
        )?;
        frame.line(114);
        m.set_global(
            "Shape",
            rt::class!(
                m,
                "Shape"(m.global("abc")?.getattr("ABC")?),
                shape::body,
                line = 114
            )?,
        )?;
        frame.line(123);
        m.set_global(
            "Square",
            rt::class!(
                m,
                "Square"(m.global("Shape")?),
                square::body,
                line = 123,
                attributes = ["side"]
            )?,
        )?;
        frame.line(132);
        match (|| m.global("Shape")?.call0())() {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyTypeError>(py) => {
                let exception = frame.catch(err);
                m.set_global("err", &exception)?;
                let _handling = rt::Handling::new(&exception);
                frame.line(134);
                rt::builtins::print(py, ("TypeError:", m.global("err")?))?;
                m.unbind_global("err")?;
            }
            Err(err) => return Err(err),
        }
        frame.line(135);
        rt::builtins::print(
            py,
            (
                m.global("Square")?.call1((3,))?.call_method0("describe")?,
                m.global("Shape")?.getattr("__abstractmethods__")?,
            ),
        )?;
        frame.line(138);
        m.set_global(
            "Worker",
            rt::class!(
                m,
                "Worker"(m.global("threading")?.getattr("Thread")?),
                worker::body,
                line = 138,
                attributes = ["n", "result"]
            )?,
        )?;
        // classes_inherit.py:148, kept as Python: list comprehension at 148:11
        rt::exec_python!(
            m,
            line = 148,
            r#"
workers = [Worker(n) for n in (10, 100)] if False else [Worker(10), Worker(100)]
"#
        )?;
        frame.line(149);
        for item in m.global("workers")?.try_iter()? {
            m.set_global("w", item?)?;
            frame.line(150);
            m.global("w")?.call_method0("start")?;
            py.check_signals()?;
            frame.line(149);
        }
        frame.line(151);
        for item2 in m.global("workers")?.try_iter()? {
            m.set_global("w", item2?)?;
            frame.line(152);
            m.global("w")?.call_method0("join")?;
            py.check_signals()?;
            frame.line(151);
        }
        // classes_inherit.py:153, kept as Python: list comprehension at 153:7
        rt::exec_python!(
            m,
            line = 153,
            r#"
print([w.result for w in workers], [w.name for w in workers])
"#
        )?;
        frame.line(156);
        m.set_global(
            "Upper",
            rt::class!(
                m,
                "Upper"(m.global("argparse")?.getattr("Action")?),
                upper::body,
                line = 156
            )?,
        )?;
        frame.line(161);
        m.set_global(
            "parser",
            rt::call!(
                m.global("argparse")?.getattr("ArgumentParser")?,
                prog = "demo"
            )?,
        )?;
        frame.line(162);
        rt::call!(
            m.global("parser")?.getattr("add_argument")?,
            "--name",
            action = m.global("Upper")?
        )?;
        frame.line(163);
        rt::builtins::print(
            py,
            (m.global("parser")?
                .getattr("parse_args")?
                .call1((PyList::new(py, ["--name", "bob"])?,))?,),
        )?;
        frame.line(166);
        m.set_global(
            "Reversed",
            rt::class!(
                m,
                "Reversed"(rt::builtin(py, "list")?),
                reversed::body,
                line = 166
            )?,
        )?;
        frame.line(174);
        m.set_global(
            "r",
            m.global("Reversed")?
                .call1((PyList::new(py, [1, 2, 3])?,))?,
        )?;
        frame.line(175);
        m.global("r")?.call_method1("append", (0,))?;
        frame.line(176);
        rt::builtins::print(
            py,
            (
                m.global("r")?,
                m.global("r")?.len()?,
                m.global("r")?.is_instance(&rt::builtin(py, "list")?)?,
            ),
        )?;
        frame.line(179);
        m.set_global(
            "Default",
            rt::class!(
                m,
                "Default"(rt::builtin(py, "dict")?),
                default::body,
                line = 179
            )?,
        )?;
        frame.line(184);
        rt::builtins::print(
            py,
            (
                rt::call!(m.global("Default")?, a = 1)?.get_item("a")?,
                m.global("Default")?.call0()?.get_item("xy")?,
            ),
        )?;
        Ok(())
    })
}
