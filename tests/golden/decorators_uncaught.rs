use pyo3::prelude::*;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// decorators_uncaught.py:7
#[rt::frame(m, "show", 7)]
pub fn show<'py>(m: &rt::Module<'py>, label: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    m.global("print")?.call1(("evaluate", label))?;
    // decorators_uncaught.py:10
    #[rt::frame(m, "show.<locals>.decorate", 10)]
    fn decorate<'py>(
        m: &rt::Module<'py>,
        label: &Bound<'py, PyAny>,
        r#fn: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        m.global("print")?
            .call1(("apply", label, "to", r#fn.attr("__qualname__")?))?;
        Ok(r#fn.clone())
    }
    let decorate = rt::function!(
        m,
        decorate[label](r#fn),
        qualname = "show.<locals>.decorate"
    )?;
    Ok(decorate)
}

// decorators_uncaught.py:17
#[rt::frame(m, "default", 17)]
pub fn default<'py>(m: &rt::Module<'py>, label: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    m.global("print")?.call1(("default", label))?;
    Ok(label.clone())
}

// decorators_uncaught.py:22
#[rt::frame(m, "boom", 22, frame)]
pub fn boom<'py>(m: &rt::Module<'py>, r#fn: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    Err(frame.raise(
        m.global("ValueError")?
            .call1((rt::str(m.py(), "boom applied to ").add(r#fn.attr("__qualname__")?)?,))?,
    ))
}

// decorators_uncaught.py:41
#[rt::frame(m, "ordered", 38)]
pub fn ordered<'py>(
    m: &rt::Module<'py>,
    a: &Bound<'py, PyAny>,
    b: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    a.add(b)
}

// decorators_uncaught.py:121
#[rt::frame(m, "factory", 121)]
pub fn factory<'py>(
    m: &rt::Module<'py>,
    prefix: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let local = m
        .global("show")?
        .call1((rt::str(py, "local ").add(prefix)?,))?;
    // decorators_uncaught.py:126
    #[rt::frame(m, "factory.<locals>.made", 124)]
    fn made<'py>(
        m: &rt::Module<'py>,
        prefix: &Bound<'py, PyAny>,
        x: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        prefix.add(x)
    }
    let made = rt::function!(
        m,
        @(&local)
        @(m.global("functools")?.attr("wraps")?.call1((m.global("factory")?,))?)
        made[prefix](x = m.global("default")?.call1((prefix,))?),
        qualname = "factory.<locals>.made"
    )?;
    // decorators_uncaught.py:130
    #[rt::frame(m, "factory.<locals>.plain", 129)]
    fn plain<'py>(m: &rt::Module<'py>, y: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        y.attr("__qualname__")
    }
    let plain = rt::function!(
        m,
        @(&local)
        plain(y = &local),
        qualname = "factory.<locals>.plain"
    )?;
    Ok(PyTuple::new(py, [&made, &plain])?.into_any())
}

// decorators_uncaught.py:142
pub fn walrus<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    Ok(rt::str(m.py(), "walrus"))
}

// decorators_uncaught.py:173
#[rt::frame(m, "build", 173)]
pub fn build<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    // decorators_uncaught.py:177
    fn method<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        Ok(x.clone())
    }
    let method = rt::function!(
        m,
        @(m.global("show")?.call1(("outer",))?)
        @boom
        @(m.global("show")?.call1(("inner",))?)
        method(x = m.global("default")?.call1(("x",))?),
        qualname = "build.<locals>.method"
    )?;
    Ok(method)
}

pub mod tags {
    use super::*;

    // decorators_uncaught.py:26
    /// The body of `class Tags`, run in its namespace `ns`.
    #[rt::frame(ns, "Tags", 26)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::def!(ns, __init__(slf as "self"))?;
        rt::def!(ns, tag(slf as "self", r#fn))?;
        Ok(())
    }

    // decorators_uncaught.py:27
    #[rt::frame(m, "Tags.__init__", 27)]
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        slf.set_attr("seen", PyList::empty(py))?;
        Ok(rt::none(py))
    }

    // decorators_uncaught.py:30
    #[rt::frame(m, "Tags.tag", 30)]
    pub fn tag<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        r#fn: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        slf.attr("seen")?
            .attr("append")?
            .call1((r#fn.attr("__qualname__")?,))?;
        Ok(r#fn.clone())
    }
}

pub mod shape {
    use super::*;

    // decorators_uncaught.py:48
    /// The body of `class Shape`, run in its namespace `ns`.
    #[rt::frame(ns, "Shape", 48)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::def!(
            ns,
            @(ns.get("show")?.call1(("method",))?)
            area(slf as "self", scale = ns.get("default")?.call1(("scale",))?)
        )?;
        rt::def!(ns, @property label(slf as "self"))?;
        rt::def!(ns, @label.setter label_ as "label"(slf as "self", value))?;
        rt::def!(ns, @label.deleter label__ as "label"(slf as "self"))?;
        rt::def!(ns, @classmethod unit(cls))?;
        rt::def!(ns, @staticmethod double(x))?;
        rt::def!(ns, @tags.tag tagged(slf as "self"))?;
        rt::def!(
            ns,
            @(rt::call!(
                ns.get("functools")?.attr("lru_cache")?,
                maxsize = m.py().None()
            )?)
            cached(slf as "self", n)
        )?;
        rt::def!(ns, r#type(slf as "self"))?;
        rt::def!(ns, r#match(slf as "self"))?;
        ns.set(
            "Inner",
            rt::class!(
                m,
                "Inner"(),
                inner::body,
                line = 87,
                qualname = "Shape.Inner"
            )?,
        )?;
        Ok(())
    }

    // decorators_uncaught.py:50
    #[rt::frame(m, "Shape.area", 49)]
    pub fn area<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        scale: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::int(m.py(), 2).mul(scale)
    }

    // decorators_uncaught.py:54
    #[rt::frame(m, "Shape.label", 53)]
    pub fn label<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        slf.attr("_label")
    }

    // decorators_uncaught.py:58
    #[rt::frame(m, "Shape.label", 57)]
    pub fn label_<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        value: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        slf.set_attr("_label", value.method0("upper")?)?;
        Ok(rt::none(m.py()))
    }

    // decorators_uncaught.py:62
    #[rt::frame(m, "Shape.label", 61)]
    pub fn label__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        slf.set_attr("_label", "deleted")?;
        Ok(rt::none(m.py()))
    }

    // decorators_uncaught.py:66
    #[rt::frame(m, "Shape.unit", 65)]
    pub fn unit<'py>(m: &rt::Module<'py>, cls: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        cls.attr("__name__")
    }

    // decorators_uncaught.py:70
    #[rt::frame(m, "Shape.double", 69)]
    pub fn double<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        rt::int(m.py(), 2).mul(x)
    }

    // decorators_uncaught.py:74
    pub fn tagged<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(rt::str(m.py(), "tagged"))
    }

    // decorators_uncaught.py:78
    #[rt::frame(m, "Shape.cached", 77)]
    pub fn cached<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        n: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        n.mul(10)
    }

    // decorators_uncaught.py:81
    pub fn r#type<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(rt::str(m.py(), "type is a Rust keyword"))
    }

    // decorators_uncaught.py:84
    pub fn r#match<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(rt::str(m.py(), "so is match"))
    }

    pub mod inner {
        use super::*;

        // decorators_uncaught.py:87
        /// The body of `class Shape.Inner`, run in its namespace `ns`.
        #[rt::frame(ns, "Shape.Inner", 87)]
        pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
            rt::def!(ns, method(slf as "self"))?;
            ns.set(
                "Deepest",
                rt::class!(
                    m,
                    "Deepest"(),
                    deepest::body,
                    line = 91,
                    qualname = "Shape.Inner.Deepest"
                )?,
            )?;
            Ok(())
        }

        // decorators_uncaught.py:88
        pub fn method<'py>(
            m: &rt::Module<'py>,
            slf: &Bound<'py, PyAny>,
        ) -> PyResult<Bound<'py, PyAny>> {
            Ok(rt::str(m.py(), "inner"))
        }

        pub mod deepest {
            use super::*;

            // decorators_uncaught.py:91
            /// The body of `class Shape.Inner.Deepest`, run in its namespace `ns`.
            #[rt::frame(ns, "Shape.Inner.Deepest", 91)]
            pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
                rt::def!(ns, @staticmethod method())?;
                Ok(())
            }

            // decorators_uncaught.py:93
            pub fn method<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
                Ok(rt::str(m.py(), "deepest"))
            }
        }
    }
}

pub mod broken {
    use super::*;

    // decorators_uncaught.py:163
    /// The body of `class Broken`, run in its namespace `ns`.
    #[rt::frame(ns, "Broken", 163)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::def!(
            ns,
            @(ns.get("show")?.call1(("class body",))?)
            @boom
            method(slf as "self")
        )?;
        Ok(())
    }

    // decorators_uncaught.py:166
    pub fn method<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(rt::none(m.py()))
    }
}

/// Executes the body of `decorators_uncaught.py` in the module `m`.
#[rt::frame(m, "<module>", 1, frame)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    m.set_global(
        "__doc__",
        "Decorators of methods and functions: evaluation order, names, qualified\n\
            names, and the traceback of a decorator that raises.",
    )?;
    m.set_global("functools", rt::import(m, "functools")?)?;
    rt::def!(m, show(label))?;
    rt::def!(m, default(label))?;
    rt::def!(m, boom(r#fn))?;
    m.set_global(
        "Tags",
        rt::class!(m, "Tags"(), tags::body, line = 26, attributes = ["seen"])?,
    )?;
    m.set_global("tags", m.global("Tags")?.call0()?)?;
    rt::def!(
        m,
        @(m.global("show")?.call1(("first",))?)
        @(m.global("show")?.call1(("second",))?)
        @(m.global("show")?.call1(("third",))?)
        ordered(a = m.global("default")?.call1(("a",))?, *, b = m.global("default")?.call1(("b",))?)
    )?;
    m.global("print")?.call1((
        m.global("ordered")?.call0()?,
        m.global("ordered")?.attr("__qualname__")?,
    ))?;
    m.set_global(
        "Shape",
        rt::class!(
            m,
            "Shape"(),
            shape::body,
            line = 48,
            attributes = ["_label"]
        )?,
    )?;
    m.set_global("s", m.global("Shape")?.call0()?)?;
    m.global("s")?.set_attr("label", "box")?;
    m.global("print")?.call1((
        m.global("s")?.attr("label")?,
        m.global("Shape")?.method0("unit")?,
        m.global("Shape")?.method1("double", (4,))?,
        m.global("s")?.method0("area")?,
        m.global("s")?.method0("tagged")?,
        m.global("s")?.method1("cached", (4,))?,
    ))?;
    m.global("s")?.del_attr("label")?;
    m.global("print")?.call1((
        m.global("s")?.attr("label")?,
        m.global("s")?.method0("type")?,
        m.global("s")?.method0("match")?,
        m.global("Shape")?.method0("Inner")?.method0("method")?,
        m.global("Shape")?
            .attr("Inner")?
            .attr("Deepest")?
            .method0("method")?,
    ))?;
    m.global("print")?
        .call1((m.global("tags")?.attr("seen")?,))?;
    for item in rt::iter(
        PyList::new(
            m.py(),
            [
                &m.global("ordered")?,
                &m.global("Shape")?.attr("area")?,
                &m.global("Shape")?.attr("label")?.attr("fget")?,
                &m.global("Shape")?.attr("label")?.attr("fset")?,
                &m.global("Shape")?.attr("label")?.attr("fdel")?,
                &m.global("Shape")?.attr("unit")?,
                &m.global("Shape")?.attr("double")?,
                &m.global("Shape")?.attr("tagged")?,
                &m.global("Shape")?.attr("cached")?.attr("__wrapped__")?,
                &m.global("Shape")?.attr("type")?,
                &m.global("Shape")?.attr("match")?,
                &m.global("Shape")?.attr("Inner")?.attr("method")?,
                &m.global("Shape")?
                    .attr("Inner")?
                    .attr("Deepest")?
                    .attr("method")?,
            ],
        )?
        .as_any(),
    )? {
        m.set_global("fn", item?)?;
        m.global("print")?.call1((
            m.global("fn")?.attr("__name__")?,
            m.global("fn")?.attr("__qualname__")?,
        ))?;
    }
    rt::def!(m, factory(prefix))?;
    let [made, plain] = rt::unpack(&m.global("factory")?.call1(("p",))?)?;
    m.set_global("made", made)?;
    m.set_global("plain", plain)?;
    m.global("print")?.call1((
        m.global("made")?.call0()?,
        m.global("made")?.attr("__name__")?,
        m.global("made")?.attr("__qualname__")?,
        m.global("made")?
            .attr("__wrapped__")?
            .attr("__qualname__")?,
    ))?;
    m.global("print")?.call1((
        m.global("plain")?.call0()?,
        m.global("plain")?.attr("__name__")?,
        m.global("plain")?.attr("__qualname__")?,
    ))?;
    {
        let chosen = m.global("show")?.call1(("walrus",))?;
        m.set_global("chosen", &chosen)?;
        let function = rt::function!(m, walrus())?;
        m.set_global("walrus", chosen.call1((function,))?)?;
    }
    m.global("print")?.call1((
        m.global("walrus")?.call0()?,
        m.global("chosen")?.attr("__qualname__")?,
    ))?;
    match (|| -> PyResult<()> {
        // decorators_uncaught.py:152
        fn failing<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
            Ok(rt::none(m.py()))
        }
        rt::def!(
            m,
            @(m.global("show")?.call1(("never applied",))?)
            @boom
            failing()
        )?;
        Ok(())
    })() {
        Ok(()) => {}
        Err(err) if frame.matches(&err, || m.global("ValueError"))? => {
            let exception = frame.catch(err);
            m.set_global("e", &exception)?;
            m.global("print")?.call1(("caught", m.global("e")?))?;
            m.unbind_global("e")?;
        }
        Err(err) => return Err(err),
    }
    m.global("print")?
        .call1((m.globals().contains("failing")?,))?;
    match (|| -> PyResult<()> {
        m.set_global(
            "Broken",
            rt::class!(m, "Broken"(), broken::body, line = 163)?,
        )?;
        Ok(())
    })() {
        Ok(()) => {}
        Err(err) if frame.matches(&err, || m.global("ValueError"))? => {
            let exception = frame.catch(err);
            m.set_global("e", &exception)?;
            m.global("print")?.call1(("caught", m.global("e")?))?;
            m.unbind_global("e")?;
        }
        Err(err) => return Err(err),
    }
    rt::def!(m, build())?;
    m.global("print")?.call1(("building",))?;
    m.global("build")?.call0()?;
    Ok(())
}
