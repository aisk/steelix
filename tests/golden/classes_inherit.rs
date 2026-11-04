use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::pyclass::CompareOp;
use pyo3::types::PyList;
use pyo3::types::PySet;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

pub mod a {
    use super::*;

    // classes_inherit.py:10
    /// The body of `class A`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "A", 10, |_| {
            rt::def!(ns, who(slf as "self"))?;
            Ok(())
        })
    }

    // classes_inherit.py:11
    pub fn who<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "A.who", 11, |_| {
            Ok(PyList::new(m.py(), ["A"])?.into_any())
        })
    }
}

pub mod b {
    use super::*;

    // classes_inherit.py:15
    /// The body of `class B`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "B", 15, |_| {
            let __class__ = ns.class_cell();
            rt::def!(ns, who[&__class__](slf as "self"))?;
            Ok(())
        })
    }

    // classes_inherit.py:16
    pub fn who<'py>(
        m: &rt::Module<'py>,
        __class__: &rt::Cell<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "B.who", 16, |_| {
            PyList::new(m.py(), ["B"])?.add(rt::super_(__class__, slf)?.method0("who")?)
        })
    }
}

pub mod c {
    use super::*;

    // classes_inherit.py:20
    /// The body of `class C`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "C", 20, |_| {
            let __class__ = ns.class_cell();
            rt::def!(ns, who[&__class__](slf as "self"))?;
            Ok(())
        })
    }

    // classes_inherit.py:21
    pub fn who<'py>(
        m: &rt::Module<'py>,
        __class__: &rt::Cell<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "C.who", 21, |_| {
            PyList::new(m.py(), ["C"])?.add(rt::super_(__class__, slf)?.method0("who")?)
        })
    }
}

pub mod d {
    use super::*;

    // classes_inherit.py:25
    /// The body of `class D`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "D", 25, |_| {
            let __class__ = ns.class_cell();
            rt::def!(ns, who[&__class__](slf as "self"))?;
            Ok(())
        })
    }

    // classes_inherit.py:26
    pub fn who<'py>(
        m: &rt::Module<'py>,
        __class__: &rt::Cell<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "D.who", 26, |_| {
            PyList::new(m.py(), ["D"])?.add(rt::super_(__class__, slf)?.method0("who")?)
        })
    }
}

pub mod color {
    use super::*;

    // classes_inherit.py:34
    /// The body of `class Color`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Color", 34, |_| {
            ns.set("RED", 1)?;
            ns.set("GREEN", 2)?;
            ns.set("BLUE", 3)?;
            rt::def!(ns, describe(slf as "self"))?;
            rt::def!(ns, @classmethod default(cls))?;
            rt::def!(ns, @property is_warm(slf as "self"))?;
            Ok(())
        })
    }

    // classes_inherit.py:39
    pub fn describe<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Color.describe", 39, |_| {
            rt::fstring(
                m.py(),
                (
                    rt::format(&slf.attr("name")?.method0("lower")?, "")?,
                    "=",
                    rt::format(&slf.attr("value")?, "")?,
                ),
            )
        })
    }

    // classes_inherit.py:43
    pub fn default<'py>(
        m: &rt::Module<'py>,
        cls: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Color.default", 42, |_| { cls.attr("GREEN") })
    }

    // classes_inherit.py:47
    pub fn is_warm<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Color.is_warm", 46, |_| {
            Ok(rt::bool(m.py(), slf.is(m.global("Color")?.attr("RED")?)))
        })
    }
}

pub mod planet {
    use super::*;

    // classes_inherit.py:55
    /// The body of `class Planet`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        let py = m.py();
        rt::frame!(ns, "Planet", 55, |_| {
            ns.set("EARTH", PyTuple::new(py, [5.97e+24, 6370000.0])?)?;
            ns.set("MARS", PyTuple::new(py, [6.42e+23, 3390000.0])?)?;
            rt::def!(ns, __init__(slf as "self", mass, radius))?;
            rt::def!(ns, @property gravity(slf as "self"))?;
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
        rt::frame!(m, "Planet.__init__", 59, |_| {
            slf.set_attr("mass", mass)?;
            slf.set_attr("radius", radius)?;
            Ok(rt::none(m.py()))
        })
    }

    // classes_inherit.py:64
    pub fn gravity<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "Planet.gravity", 63, |_| {
            rt::builtin(py, "round")?.call1((
                rt::float(py, 6.67e-11)
                    .mul(slf.attr("mass")?)?
                    .div(slf.attr("radius")?.pow(2, py.None())?)?,
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
        rt::frame!(ns, "Flags", 71, |_| {
            ns.set("READ", 4)?;
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
        rt::frame!(ns, "Shade", 79, |_| {
            rt::def!(ns, __new__(cls, value, label))?;
            ns.set(
                "DARK",
                PyTuple::new(py, [&rt::int(py, 1), &rt::str(py, "dark")])?,
            )?;
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
        rt::frame!(m, "Shade.__new__", 80, |_| {
            let obj = rt::builtin(m.py(), "object")?.method1("__new__", (cls,))?;
            obj.set_attr("_value_", value)?;
            obj.set_attr("label", label)?;
            Ok(obj)
        })
    }
}

pub mod point {
    use super::*;

    // classes_inherit.py:93
    /// The body of `class Point`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Point", 93, |_| {
            ns.set("y", 0)?;
            rt::def!(ns, norm1(slf as "self"))?;
            Ok(())
        })
    }

    // classes_inherit.py:97
    pub fn norm1<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Point.norm1", 97, |_| {
            slf.attr("x")?.abs()?.add(slf.attr("y")?.abs()?)
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
        rt::frame!(ns, "Shape", 114, |_| {
            rt::def!(ns, @abc.abstractmethod area(slf as "self"))?;
            rt::def!(ns, describe(slf as "self"))?;
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
        rt::frame!(m, "Shape.describe", 119, |_| {
            rt::fstring(
                m.py(),
                (
                    rt::format(&slf.get_type().attr("__name__")?, "")?,
                    " with area ",
                    rt::format(&slf.method0("area")?, "")?,
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
        rt::frame!(ns, "Square", 123, |_| {
            rt::def!(ns, __init__(slf as "self", side))?;
            rt::def!(ns, area(slf as "self"))?;
            Ok(())
        })
    }

    // classes_inherit.py:124
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        side: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Square.__init__", 124, |_| {
            slf.set_attr("side", side)?;
            Ok(rt::none(m.py()))
        })
    }

    // classes_inherit.py:127
    pub fn area<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Square.area", 127, |_| {
            slf.attr("side")?.pow(2, m.py().None())
        })
    }
}

pub mod worker {
    use super::*;

    // classes_inherit.py:138
    /// The body of `class Worker`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Worker", 138, |_| {
            let __class__ = ns.class_cell();
            rt::def!(ns, __init__[&__class__](slf as "self", n))?;
            rt::def!(ns, run(slf as "self"))?;
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
        rt::frame!(m, "Worker.__init__", 139, |_| {
            rt::call!(
                rt::super_(__class__, slf)?.attr("__init__")?,
                name = rt::fstring(py, ("worker-", rt::format(n, "")?))?
            )?;
            slf.set_attr("n", n)?;
            slf.set_attr("result", py.None())?;
            Ok(rt::none(py))
        })
    }

    // classes_inherit.py:144
    pub fn run<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "Worker.run", 144, |_| {
            slf.set_attr(
                "result",
                rt::builtin(py, "sum")?
                    .call1((rt::builtin(py, "range")?.call1((slf.attr("n")?,))?,))?,
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
        rt::frame!(ns, "Upper", 156, |_| {
            rt::def!(
                ns,
                __call__(slf as "self", parser, namespace, values, option_string = m.py().None())
            )?;
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
        rt::frame!(m, "Upper.__call__", 157, |_| {
            rt::builtin(py, "setattr")?.call1((
                namespace,
                slf.attr("dest")?,
                values.method0("upper")?,
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
        rt::frame!(ns, "Reversed", 166, |_| {
            let __class__ = ns.class_cell();
            rt::def!(ns, __init__[&__class__](slf as "self", items))?;
            rt::def!(ns, append[&__class__](slf as "self", item))?;
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
        rt::frame!(m, "Reversed.__init__", 167, |_| {
            rt::super_(__class__, slf)?
                .attr("__init__")?
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
        rt::frame!(m, "Reversed.append", 170, |_| {
            rt::super_(__class__, slf)?.method1("insert", (0, item))?;
            Ok(rt::none(m.py()))
        })
    }
}

pub mod default {
    use super::*;

    // classes_inherit.py:179
    /// The body of `class Default`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Default", 179, |_| {
            rt::def!(ns, __missing__(slf as "self", key))?;
            Ok(())
        })
    }

    // classes_inherit.py:180
    pub fn __missing__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        key: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Default.__missing__", 180, |_| { key.mul(2) })
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
        m.set_global("abc", rt::import(m, "abc")?)?;
        m.set_global("argparse", rt::import(m, "argparse")?)?;
        m.set_global("enum", rt::import(m, "enum")?)?;
        m.set_global("threading", rt::import(m, "threading")?)?;
        let typing = rt::import_names(m, "typing", &["NamedTuple", "TypedDict"])?;
        m.set_global("NamedTuple", rt::import_from(&typing, "NamedTuple")?)?;
        m.set_global("TypedDict", rt::import_from(&typing, "TypedDict")?)?;
        m.set_global("A", rt::class!(m, "A"(), a::body, line = 10)?)?;
        m.set_global("B", rt::class!(m, "B"(m.global("A")?), b::body, line = 15)?)?;
        m.set_global("C", rt::class!(m, "C"(m.global("A")?), c::body, line = 20)?)?;
        m.set_global(
            "D",
            rt::class!(m, "D"(m.global("B")?, m.global("C")?), d::body, line = 25)?,
        )?;
        let value = m.global("D")?.call0()?.method0("who")?;
        let list = PyList::empty(py);
        for k in rt::iter(&m.global("D")?.attr("__mro__")?)? {
            let k = k?;
            list.append(k.attr("__name__")?)?;
        }
        rt::builtins::print(py, (value, list))?;
        rt::builtins::print(
            py,
            (
                m.global("D")?.call0()?.is_instance(&m.global("A")?)?,
                rt::builtin(py, "issubclass")?.call1((m.global("D")?, m.global("C")?))?,
                m.global("B")?.call0()?.is_instance(&m.global("C")?)?,
            ),
        )?;
        m.set_global(
            "Color",
            rt::class!(
                m,
                "Color"(m.global("enum")?.attr("Enum")?),
                color::body,
                line = 34
            )?,
        )?;
        rt::builtins::print(
            py,
            (
                m.global("Color")?.attr("RED")?,
                m.global("Color")?.call1((2,))?,
                m.global("Color")?.attr("BLUE")?.method0("describe")?,
                m.global("Color")?.method0("default")?,
                m.global("Color")?.attr("RED")?.attr("is_warm")?,
            ),
        )?;
        rt::builtins::print(
            py,
            (
                rt::builtin(py, "list")?.call1((m.global("Color")?,))?,
                m.global("Color")?.get_item("BLUE")?.attr("value")?,
                m.global("Color")?.len()?,
            ),
        )?;
        m.set_global(
            "Planet",
            rt::class!(
                m,
                "Planet"(m.global("enum")?.attr("Enum")?),
                planet::body,
                line = 55,
                attributes = ["mass", "radius"]
            )?,
        )?;
        rt::builtins::print(
            py,
            (
                m.global("Planet")?.attr("EARTH")?.attr("gravity")?,
                m.global("Planet")?.attr("MARS")?.attr("gravity")?,
            ),
        )?;
        m.set_global(
            "Flags",
            rt::class!(
                m,
                "Flags"(m.global("enum")?.attr("IntFlag")?),
                flags::body,
                line = 71
            )?,
        )?;
        rt::builtins::print(
            py,
            (
                m.global("Flags")?
                    .attr("READ")?
                    .bitor(m.global("Flags")?.attr("WRITE")?)?,
                rt::builtin(py, "int")?.call1((m.global("Flags")?.attr("READ")?,))?,
            ),
        )?;
        m.set_global(
            "Shade",
            rt::class!(
                m,
                "Shade"(m.global("enum")?.attr("Enum")?),
                shade::body,
                line = 79
            )?,
        )?;
        rt::builtins::print(
            py,
            (
                m.global("Shade")?.call1((1,))?.attr("label")?,
                m.global("Shade")?.attr("LIGHT")?.attr("value")?,
            ),
        )?;
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
        m.set_global("p", m.global("Point")?.call1((3, -4))?)?;
        rt::builtins::print(
            py,
            (
                m.global("p")?,
                m.global("p")?.method0("norm1")?,
                m.global("Point")?.attr("_fields")?,
                m.global("Point")?.call1((1,))?,
                rt::call!(m.global("p")?.attr("_replace")?, y = 1)?,
            ),
        )?;
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
        m.set_global(
            "m",
            rt::call!(m.global("Movie")?, title = "Brazil", year = 1985)?,
        )?;
        rt::builtins::print(
            py,
            (
                m.global("m")?,
                m.global("Movie")?
                    .attr("__required_keys__")?
                    .rich_compare(PySet::new(py, ["title", "year"])?, CompareOp::Eq)?,
                m.global("Movie")?.attr("__annotations__")?,
            ),
        )?;
        m.set_global(
            "Shape",
            rt::class!(
                m,
                "Shape"(m.global("abc")?.attr("ABC")?),
                shape::body,
                line = 114
            )?,
        )?;
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
        match (|| m.global("Shape")?.call0())() {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyTypeError>(py) => {
                let exception = frame.catch(err);
                m.set_global("err", &exception)?;
                rt::builtins::print(py, ("TypeError:", m.global("err")?))?;
                m.unbind_global("err")?;
            }
            Err(err) => return Err(err),
        }
        rt::builtins::print(
            py,
            (
                m.global("Square")?.call1((3,))?.method0("describe")?,
                m.global("Shape")?.attr("__abstractmethods__")?,
            ),
        )?;
        m.set_global(
            "Worker",
            rt::class!(
                m,
                "Worker"(m.global("threading")?.attr("Thread")?),
                worker::body,
                line = 138,
                attributes = ["n", "result"]
            )?,
        )?;
        m.set_global(
            "workers",
            if false {
                let list = PyList::empty(py);
                for n in rt::iter(PyTuple::new(py, [10, 100])?.as_any())? {
                    let n = n?;
                    list.append(m.global("Worker")?.call1((&n,))?)?;
                }
                list.into_any()
            } else {
                PyList::new(
                    py,
                    [
                        &m.global("Worker")?.call1((10,))?,
                        &m.global("Worker")?.call1((100,))?,
                    ],
                )?
                .into_any()
            },
        )?;
        for item in rt::iter(&m.global("workers")?)? {
            m.set_global("w", item?)?;
            m.global("w")?.method0("start")?;
        }
        for item2 in rt::iter(&m.global("workers")?)? {
            m.set_global("w", item2?)?;
            m.global("w")?.method0("join")?;
        }
        let list2 = PyList::empty(py);
        for w in rt::iter(&m.global("workers")?)? {
            let w = w?;
            list2.append(w.attr("result")?)?;
        }
        let list3 = PyList::empty(py);
        for w in rt::iter(&m.global("workers")?)? {
            let w = w?;
            list3.append(w.attr("name")?)?;
        }
        rt::builtins::print(py, (list2, list3))?;
        m.set_global(
            "Upper",
            rt::class!(
                m,
                "Upper"(m.global("argparse")?.attr("Action")?),
                upper::body,
                line = 156
            )?,
        )?;
        m.set_global(
            "parser",
            rt::call!(m.global("argparse")?.attr("ArgumentParser")?, prog = "demo")?,
        )?;
        rt::call!(
            m.global("parser")?.attr("add_argument")?,
            "--name",
            action = m.global("Upper")?
        )?;
        rt::builtins::print(
            py,
            (m.global("parser")?
                .attr("parse_args")?
                .call1((PyList::new(py, ["--name", "bob"])?,))?,),
        )?;
        m.set_global(
            "Reversed",
            rt::class!(
                m,
                "Reversed"(rt::builtin(py, "list")?),
                reversed::body,
                line = 166
            )?,
        )?;
        m.set_global(
            "r",
            m.global("Reversed")?
                .call1((PyList::new(py, [1, 2, 3])?,))?,
        )?;
        m.global("r")?.method1("append", (0,))?;
        rt::builtins::print(
            py,
            (
                m.global("r")?,
                m.global("r")?.len()?,
                m.global("r")?.is_instance(&rt::builtin(py, "list")?)?,
            ),
        )?;
        m.set_global(
            "Default",
            rt::class!(
                m,
                "Default"(rt::builtin(py, "dict")?),
                default::body,
                line = 179
            )?,
        )?;
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
