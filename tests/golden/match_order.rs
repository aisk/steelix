use pyo3::exceptions::PyAttributeError;
use pyo3::prelude::*;
use pyo3::types::IntoPyDict;
use pyo3::types::PyDict;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// match_order.py:9
#[rt::frame(m, "log", 9)]
pub fn log<'py>(m: &rt::Module<'py>, args: &Bound<'py, PyTuple>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::call!(rt::builtin(py, "print")?, "  ", *args)?;
    Ok(rt::none(py))
}

// match_order.py:125
#[rt::frame(m, "sequences", 125)]
pub fn sequences<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::builtins::print(py, ("unpacked by iteration",))?;
    {
        let subject = m.global("Seq")?.call1((1, 2, 3))?;
        if let Some([item, _, item2]) = rt::match_seq(&subject)?
            && item.eq(1)?
            && item2.eq(4)?
        {
            rt::builtins::print(py, ("no",))?;
        } else if let Some([a, b, c]) = rt::match_seq(&subject)?
            && log(m, &PyTuple::new(py, [&rt::str(py, "guard"), &a, &b, &c])?)?.is_truthy()?
        {
            rt::builtins::print(py, ("no",))?;
        } else if let Some(([a], rest, [])) = rt::match_seq_star(&subject)? {
            rt::builtins::print(py, ("match", &a, &rest))?;
        }
    }
    rt::builtins::print(py, ("read by index around *_",))?;
    {
        let subject = m.global("Seq")?.call1((1, 2, 3, 4))?;
        if rt::match_seq_min(&subject, 2)?
            && let item = subject.get_item(0)?
            && item.eq(0)?
            && let item2 = rt::item_from_end(&subject, 1)?
            && item2.eq(4)?
        {
            rt::builtins::print(py, ("no",))?;
        } else if rt::match_seq_min(&subject, 3)?
            && let _ = subject.get_item(0)?
            && let item = rt::item_from_end(&subject, 2)?
            && let Some([item2, item3]) = rt::match_class(&item, &m.global("Eq")?, 2, [])?
            && item2.eq("third")?
            && item3.is(rt::bool(py, false))
            && let _ = rt::item_from_end(&subject, 1)?
        {
            rt::builtins::print(py, ("no",))?;
        } else if rt::match_seq_min(&subject, 3)?
            && let first = subject.get_item(0)?
            && let last = rt::item_from_end(&subject, 1)?
        {
            rt::builtins::print(py, ("match", &first, &last))?;
        }
    }
    rt::builtins::print(py, ("only wildcards",))?;
    {
        let subject = m.global("Seq")?.call1((1, 2))?;
        if rt::match_seq_len(&subject, 3)? {
            rt::builtins::print(py, ("no",))?;
        } else if rt::match_seq_min(&subject, 0)? {
            rt::builtins::print(py, ("star only: no len()",))?;
        }
    }
    {
        let subject = m.global("Seq")?.call1((1, 2))?;
        if rt::match_seq_min(&subject, 1)? {
            rt::builtins::print(py, ("at least one",))?;
        }
    }
    Ok(rt::none(py))
}

// match_order.py:153
#[rt::frame(m, "values_and_eq", 153)]
pub fn values_and_eq<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::builtins::print(py, ("value patterns",))?;
    {
        let subject = m.global("Eq")?.call1(("subject", false))?;
        if subject.eq(1)? {
            rt::builtins::print(py, ("no",))?;
        } else if subject.eq(m.global("values")?.attr("first")?)?
            || subject.eq(m.global("values")?.attr("second")?)?
        {
            rt::builtins::print(py, ("no",))?;
        } else if let Some([item]) = rt::match_seq(&subject)?
            && item.eq(m.global("values")?.attr("never")?)?
        {
            rt::builtins::print(py, ("no",))?;
        } else if subject.is_none() {
            rt::builtins::print(py, ("no",))?;
        } else {
            rt::builtins::print(py, ("nothing matched",))?;
        }
    }
    {
        let subject = rt::int(py, 5);
        if let Some([item]) = rt::match_class(&subject, &m.global("Eq")?, 1, [])?
            && item.eq("pattern")?
        {
            rt::builtins::print(py, ("no: Eq() is a class pattern",))?;
        }
    }
    {
        let subject = PyList::new(py, [5])?.into_any();
        if matches!(rt::match_seq(&subject)?, Some([item]) if item.eq(m.global("values")?.attr("five")?)?)
            || matches!(rt::match_seq(&subject)?, Some([item2]) if item2.eq(5)?)
        {
            rt::builtins::print(py, ("matched [5]",))?;
        }
    }
    Ok(rt::none(py))
}

// match_order.py:176
#[rt::frame(m, "mappings", 176)]
pub fn mappings<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::builtins::print(py, ("mapping patterns",))?;
    {
        let subject = rt::call!(m.global("Map")?, a = 1, b = 2, c = 3)?;
        if let Some([x, y, z, w]) = rt::match_map(&subject, ["x", "y", "z", "w"])?
            && x.eq(1)?
            && y.eq(2)?
            && z.eq(3)?
            && w.eq(4)?
        {
            rt::builtins::print(py, ("no: too few items, no lookups",))?;
        } else if let Some([a, _, b2]) = rt::match_map(&subject, ["a", "missing", "b"])?
            && a.eq(1)?
            && b2.eq(2)?
        {
            rt::builtins::print(py, ("no",))?;
        } else if let Some([a]) = rt::match_map(&subject, ["a"])?
            && let Some([item, item2]) = rt::match_class(&a, &m.global("Eq")?, 2, [])?
            && item.eq("one")?
            && item2.is(rt::bool(py, false))
        {
            rt::builtins::print(py, ("no",))?;
        } else if rt::match_mapping(&subject, 2)?
            && let key = m.global("values")?.attr("a")?
            && let Some([a, b]) = rt::match_keys(&subject, (&key, "b"))?
            && a.eq(1)?
            && let rest = rt::match_rest(&subject, (&key, "b"))?
        {
            rt::builtins::print(
                py,
                ("match", &b, rt::builtin(py, "sorted")?.call1((&rest,))?),
            )?;
        }
    }
    {
        let subject = PyList::new(py, [1])?.into_any();
        if rt::match_mapping(&subject, 1)?
            && let Some([_]) =
                rt::match_keys(&subject, (m.global("values")?.attr("not_evaluated")?,))?
        {
            rt::builtins::print(py, ("no",))?;
        } else {
            rt::builtins::print(py, ("keys only after the mapping check",))?;
        }
    }
    Ok(rt::none(py))
}

// match_order.py:194
#[rt::frame(m, "classes", 194)]
pub fn classes<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::builtins::print(py, ("class patterns",))?;
    let obj = rt::call!(m.global("Obj")?, a = 1, b = 2, c = 3)?;
    if let Some([item, item2, _]) = rt::match_class(&obj, &m.global("Obj")?, 2, ["d"])?
        && item.eq(1)?
        && item2.eq(2)?
    {
        rt::builtins::print(py, ("no",))?;
    } else if let Some([item, c2]) = rt::match_class(&obj, &m.global("Obj")?, 1, ["c"])?
        && let Some([item2, item3]) = rt::match_class(&item, &m.global("Eq")?, 2, [])?
        && item2.eq("first")?
        && item3.is(rt::bool(py, false))
        && c2.eq(3)?
    {
        rt::builtins::print(py, ("no",))?;
    } else if let Some([a, b, c]) = rt::match_class(&obj, &m.global("Obj")?, 2, ["c"])?
        && log(m, &PyTuple::new(py, [&rt::str(py, "guard"), &a, &b, &c])?)?.is_truthy()?
    {
        rt::builtins::print(py, ("no",))?;
    } else if let Some([c2, a2]) = rt::match_class(&obj, &m.global("Obj")?, 0, ["c", "a"])?
        && c2.eq(3)?
        && a2.eq(1)?
    {
        rt::builtins::print(py, ("match",))?;
    }
    {
        let subject = rt::int(py, 1);
        if rt::match_instance(&subject, &m.global("Obj")?)? {
            rt::builtins::print(py, ("no",))?;
        } else if rt::match_instance(&subject, &rt::builtin(py, "int")?)? {
            rt::builtins::print(py, ("an int",))?;
        }
    }
    Ok(rt::none(py))
}

pub mod seq {
    use super::*;

    // match_order.py:13
    /// The body of `class Seq`, run in its namespace `ns`.
    #[rt::frame(ns, "Seq", 13)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::def!(ns, __init__(slf as "self", *items))?;
        rt::def!(ns, __len__(slf as "self"))?;
        rt::def!(ns, __getitem__(slf as "self", index))?;
        rt::def!(ns, gen __iter__(slf as "self"))?;
        Ok(())
    }

    // match_order.py:14
    #[rt::frame(m, "Seq.__init__", 14)]
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        items: &Bound<'py, PyTuple>,
    ) -> PyResult<Bound<'py, PyAny>> {
        slf.set_attr("items", items)?;
        Ok(rt::none(m.py()))
    }

    // match_order.py:17
    #[rt::frame(m, "Seq.__len__", 17)]
    pub fn __len__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        log(m, &PyTuple::new(py, ["len"])?)?;
        Ok(rt::int(py, slf.attr("items")?.len()?))
    }

    // match_order.py:21
    #[rt::frame(m, "Seq.__getitem__", 21)]
    pub fn __getitem__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        index: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        log(m, &PyTuple::new(py, [&rt::str(py, "getitem"), index])?)?;
        slf.attr("items")?.get_item(index)
    }

    // match_order.py:25
    #[rt::frame(m, "Seq.__iter__", 25)]
    pub async fn __iter__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        log(m, &PyTuple::new(py, ["iter"])?)?;
        for item in rt::iter(&slf.attr("items")?)? {
            let item = item?;
            log(m, &PyTuple::new(py, [&rt::str(py, "next"), &item])?)?;
            rt::yield_(&item).await?;
        }
        Ok(rt::none(py))
    }
}

pub mod eq {
    use super::*;

    // match_order.py:32
    /// The body of `class Eq`, run in its namespace `ns`.
    #[rt::frame(ns, "Eq", 32)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::def!(ns, __init__(slf as "self", name, result = true))?;
        rt::def!(ns, __eq__(slf as "self", other))?;
        rt::def!(ns, __bool__(slf as "self"))?;
        rt::def!(ns, __repr__(slf as "self"))?;
        Ok(())
    }

    // match_order.py:33
    #[rt::frame(m, "Eq.__init__", 33)]
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        name: &Bound<'py, PyAny>,
        result: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        slf.set_attr("name", name)?;
        slf.set_attr("result", result)?;
        Ok(rt::none(m.py()))
    }

    // match_order.py:37
    #[rt::frame(m, "Eq.__eq__", 37)]
    pub fn __eq__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        other: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        log(
            m,
            &PyTuple::new(py, [&rt::str(py, "eq"), &slf.attr("name")?, other])?,
        )?;
        slf.attr("result")
    }

    // match_order.py:41
    #[rt::frame(m, "Eq.__bool__", 41)]
    pub fn __bool__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        log(
            m,
            &PyTuple::new(py, [&rt::str(py, "bool"), &slf.attr("name")?])?,
        )?;
        Ok(rt::bool(py, true))
    }

    // match_order.py:45
    #[rt::frame(m, "Eq.__repr__", 45)]
    pub fn __repr__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        slf.attr("name")
    }
}

pub mod map {
    use super::*;

    // match_order.py:49
    /// The body of `class Map`, run in its namespace `ns`.
    #[rt::frame(ns, "Map", 49)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::def!(ns, __init__(slf as "self", **items))?;
        rt::def!(ns, __len__(slf as "self"))?;
        rt::def!(ns, __getitem__(slf as "self", key))?;
        rt::def!(ns, __iter__(slf as "self"))?;
        rt::def!(ns, get(slf as "self", key, default = m.py().None()))?;
        rt::def!(ns, keys(slf as "self"))?;
        Ok(())
    }

    // match_order.py:50
    #[rt::frame(m, "Map.__init__", 50)]
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        items: &Bound<'py, PyDict>,
    ) -> PyResult<Bound<'py, PyAny>> {
        slf.set_attr("items", items)?;
        Ok(rt::none(m.py()))
    }

    // match_order.py:53
    #[rt::frame(m, "Map.__len__", 53)]
    pub fn __len__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        log(m, &PyTuple::new(py, ["len"])?)?;
        Ok(rt::int(py, slf.attr("items")?.len()?))
    }

    // match_order.py:57
    #[rt::frame(m, "Map.__getitem__", 57)]
    pub fn __getitem__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        key: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        log(m, &PyTuple::new(py, [&rt::str(py, "getitem"), key])?)?;
        slf.attr("items")?.get_item(key)
    }

    // match_order.py:61
    #[rt::frame(m, "Map.__iter__", 61)]
    pub fn __iter__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        log(m, &PyTuple::new(m.py(), ["iter"])?)?;
        Ok(slf.attr("items")?.try_iter()?.into_any())
    }

    // match_order.py:65
    #[rt::frame(m, "Map.get", 65)]
    pub fn get<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        key: &Bound<'py, PyAny>,
        default: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        log(m, &PyTuple::new(py, [&rt::str(py, "get"), key])?)?;
        slf.attr("items")?.method1("get", (key, default))
    }

    // match_order.py:69
    #[rt::frame(m, "Map.keys", 69)]
    pub fn keys<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        log(m, &PyTuple::new(m.py(), ["keys"])?)?;
        slf.attr("items")?.method0("keys")
    }
}

/// Value patterns are evaluated when the pattern gets to them.
pub mod values {
    use super::*;

    // match_order.py:74
    /// The body of `class Values`, run in its namespace `ns`.
    #[rt::frame(ns, "Values", 74)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        ns.set(
            "__doc__",
            "Value patterns are evaluated when the pattern gets to them.",
        )?;
        rt::def!(ns, __getattr__(slf as "self", name))?;
        Ok(())
    }

    // match_order.py:77
    #[rt::frame(m, "Values.__getattr__", 77)]
    pub fn __getattr__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        name: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        log(m, &PyTuple::new(py, [&rt::str(py, "value"), name])?)?;
        Ok(name.clone())
    }
}

pub mod meta {
    use super::*;

    // match_order.py:85
    /// The body of `class Meta`, run in its namespace `ns`.
    #[rt::frame(ns, "Meta", 85)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        let __class__ = ns.class_cell();
        rt::def!(ns, __instancecheck__[&__class__](cls, instance))?;
        rt::def!(ns, @property __match_args__(cls))?;
        Ok(())
    }

    // match_order.py:86
    #[rt::frame(m, "Meta.__instancecheck__", 86)]
    pub fn __instancecheck__<'py>(
        m: &rt::Module<'py>,
        __class__: &rt::Cell<'py>,
        cls: &Bound<'py, PyAny>,
        instance: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        log(
            m,
            &PyTuple::new(py, [&rt::str(py, "isinstance"), &cls.attr("__name__")?])?,
        )?;
        rt::super_(__class__, cls)?.method1("__instancecheck__", (instance,))
    }

    // match_order.py:91
    #[rt::frame(m, "Meta.__match_args__", 90)]
    pub fn __match_args__<'py>(
        m: &rt::Module<'py>,
        cls: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        log(m, &PyTuple::new(py, ["__match_args__"])?)?;
        Ok(PyTuple::new(py, ["a", "b"])?.into_any())
    }
}

pub mod obj {
    use super::*;

    // match_order.py:96
    /// The body of `class Obj`, run in its namespace `ns`.
    #[rt::frame(ns, "Obj", 96)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        let __class__ = ns.class_cell();
        rt::def!(ns, __init__(slf as "self", **attrs))?;
        rt::def!(ns, __getattr__(slf as "self", name))?;
        rt::def!(ns, __getattribute__[&__class__](slf as "self", name))?;
        Ok(())
    }

    // match_order.py:97
    #[rt::frame(m, "Obj.__init__", 97)]
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        attrs: &Bound<'py, PyDict>,
    ) -> PyResult<Bound<'py, PyAny>> {
        slf.attr("__dict__")?.method1("update", (attrs,))?;
        Ok(rt::none(m.py()))
    }

    // match_order.py:100
    #[rt::frame(m, "Obj.__getattr__", 100, frame)]
    pub fn __getattr__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        name: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        log(
            m,
            &PyTuple::new(
                py,
                [&rt::str(py, "getattr"), name, &rt::str(py, "(missing)")],
            )?,
        )?;
        Err(frame.raise(PyAttributeError::new_err((name.clone().unbind(),))))
    }

    // match_order.py:104
    #[rt::frame(m, "Obj.__getattribute__", 104)]
    pub fn __getattribute__<'py>(
        m: &rt::Module<'py>,
        __class__: &rt::Cell<'py>,
        slf: &Bound<'py, PyAny>,
        name: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        if !name.method1("startswith", ("__",))?.is_truthy()? {
            log(m, &PyTuple::new(py, [&rt::str(py, "getattribute"), name])?)?;
        }
        rt::super_(__class__, slf)?.method1("__getattribute__", (name,))
    }
}

/// A class namespace that shows when names are stored.
pub mod stored {
    use super::*;

    // match_order.py:110
    /// The body of `class Stored`, run in its namespace `ns`.
    #[rt::frame(ns, "Stored", 110)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        let __class__ = ns.class_cell();
        ns.set(
            "__doc__",
            "A class namespace that shows when names are stored.",
        )?;
        rt::def!(ns, __setitem__[&__class__](slf as "self", key, value))?;
        Ok(())
    }

    // match_order.py:113
    #[rt::frame(m, "Stored.__setitem__", 113)]
    pub fn __setitem__<'py>(
        m: &rt::Module<'py>,
        __class__: &rt::Cell<'py>,
        slf: &Bound<'py, PyAny>,
        key: &Bound<'py, PyAny>,
        value: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        if !key.method1("startswith", ("__",))?.is_truthy()? {
            log(m, &PyTuple::new(py, [&rt::str(py, "store"), key, value])?)?;
        }
        rt::super_(__class__, slf)?.method1("__setitem__", (key, value))?;
        Ok(rt::none(py))
    }
}

pub mod store_meta {
    use super::*;

    // match_order.py:119
    /// The body of `class StoreMeta`, run in its namespace `ns`.
    #[rt::frame(ns, "StoreMeta", 119)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::def!(ns, @classmethod __prepare__(mcs, name, bases))?;
        Ok(())
    }

    // match_order.py:121
    #[rt::frame(m, "StoreMeta.__prepare__", 120)]
    pub fn __prepare__<'py>(
        m: &rt::Module<'py>,
        mcs: &Bound<'py, PyAny>,
        name: &Bound<'py, PyAny>,
        bases: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        m.global("Stored")?.call0()
    }
}

pub mod body_ {
    use super::*;

    // match_order.py:220
    /// The body of `class Body`, run in its namespace `ns`.
    #[rt::frame(ns, "Body", 220)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        let py = m.py();
        'match_: {
            let subject = PyList::new(
                py,
                [
                    &rt::int(py, 1),
                    PyTuple::new(py, [2, 3])?.as_any(),
                    [("k", 4), ("j", 5)].into_py_dict(py)?.as_any(),
                ],
            )?
            .into_any();
            if let Some([a, pair, item]) = rt::match_seq(&subject)?
                && let Some([b, c]) = rt::match_seq(&pair)?
                && let Some([d]) = rt::match_map(&item, ["k"])?
                && let rest = rt::match_rest(&item, ["k"])?
            {
                ns.set("a", &a)?;
                ns.set("b", &b)?;
                ns.set("c", &c)?;
                ns.set("pair", &pair)?;
                ns.set("d", &d)?;
                ns.set("rest", &rest)?;
                ns.set("whole", &subject)?;
                if ns.get("log")?.call1(("guard",))?.is_truthy()? {
                    break 'match_;
                }
            }
            if let Some(([a], more, [])) = rt::match_seq_star(&subject)? {
                ns.set("a", &a)?;
                ns.set("more", &more)?;
                ns.get("log")?.call1(("body",))?;
            }
        }
        Ok(())
    }
}

/// Executes the body of `match_order.py` in the module `m`.
#[rt::frame(m, "<module>", 1)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    // The order of everything a `match` does (design §8), observed by objects
    // that print each call: `len()`, iteration and indexing of sequences, `.get()`
    // of mappings, `__eq__` of value patterns, `isinstance` and attributes of
    // class patterns, and when names are stored.
    m.set_global("collections", rt::import(m, "collections.abc")?)?;
    rt::def!(m, log(*args))?;
    m.set_global(
        "Seq",
        rt::class!(
            m,
            "Seq"(m.global("collections")?.attr("abc")?.attr("Sequence")?),
            seq::body,
            line = 13,
            attributes = ["items"]
        )?,
    )?;
    m.set_global(
        "Eq",
        rt::class!(
            m,
            "Eq"(),
            eq::body,
            line = 32,
            attributes = ["name", "result"]
        )?,
    )?;
    m.set_global(
        "Map",
        rt::class!(
            m,
            "Map"(m.global("collections")?.attr("abc")?.attr("Mapping")?),
            map::body,
            line = 49,
            attributes = ["items"]
        )?,
    )?;
    m.set_global(
        "Values",
        rt::class!(m, "Values"(), values::body, line = 74)?,
    )?;
    m.set_global("values", m.global("Values")?.call0()?)?;
    m.set_global(
        "Meta",
        rt::class!(m, "Meta"(rt::builtin(py, "type")?), meta::body, line = 85)?,
    )?;
    m.set_global(
        "Obj",
        rt::class!(
            m,
            "Obj"(metaclass = m.global("Meta")?),
            obj::body,
            line = 96
        )?,
    )?;
    m.set_global(
        "Stored",
        rt::class!(
            m,
            "Stored"(rt::builtin(py, "dict")?),
            stored::body,
            line = 110
        )?,
    )?;
    m.set_global(
        "StoreMeta",
        rt::class!(
            m,
            "StoreMeta"(rt::builtin(py, "type")?),
            store_meta::body,
            line = 119
        )?,
    )?;
    rt::def!(m, sequences())?;
    rt::def!(m, values_and_eq())?;
    rt::def!(m, mappings())?;
    rt::def!(m, classes())?;
    sequences(m)?;
    values_and_eq(m)?;
    mappings(m)?;
    classes(m)?;
    rt::builtins::print(
        py,
        ("captures are stored after the whole pattern, in order",),
    )?;
    m.set_global(
        "Body",
        rt::class!(
            m,
            "Body"(metaclass = m.global("StoreMeta")?),
            body_::body,
            line = 220
        )?,
    )?;
    Ok(())
}
