//! Parts of `classes_build.py` kept as Python source and run in place by
//! `rt::exec_python!` (steelix does not lower them yet):
//!
//! - line 137, `def errors`: class definition in a function
//! - line 310, `def replaced`: class definition in a function

use pyo3::exceptions::PyRuntimeError;
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::pyclass::CompareOp;
use pyo3::types::IntoPyDict;
use pyo3::types::PyDict;
use pyo3::types::PyIterator;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// classes_build.py:5
#[rt::frame(m, "trace", 5)]
pub fn trace<'py>(
    m: &rt::Module<'py>,
    label: &Bound<'py, PyAny>,
    value: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::builtins::print(m.py(), ("eval", label))?;
    Ok(value.clone())
}

// classes_build.py:10
#[rt::frame(m, "deco", 10, frame)]
pub fn deco<'py>(m: &rt::Module<'py>, label: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::builtins::print(m.py(), ("deco", label))?;
    // classes_build.py:13
    #[rt::frame(m, "deco.<locals>.apply", 13)]
    fn apply<'py>(
        m: &rt::Module<'py>,
        label: &Bound<'py, PyAny>,
        cls: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::builtins::print(py, ("apply", label, cls.attr("__name__")?))?;
        cls.set_attr(
            "decorated",
            rt::builtins::getattr(py, (cls, "decorated", PyTuple::empty(py)))?
                .add(PyTuple::new(py, [label])?)?,
        )?;
        Ok(cls.clone())
    }
    let apply = rt::function!(frame, apply[label](cls))?;
    Ok(apply)
}

// classes_build.py:264
#[rt::frame(m, "fake_meta", 264, frame)]
pub fn fake_meta<'py>(
    m: &rt::Module<'py>,
    name: &Bound<'py, PyAny>,
    bases: &Bound<'py, PyAny>,
    ns: &Bound<'py, PyAny>,
    kwargs: &Bound<'py, PyDict>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    // classes_build.py:265
    #[rt::frame(m, "fake_meta.<locals>.<genexpr>", 265)]
    fn genexpr<'py>(
        m: &rt::Module<'py>,
        ns: &Bound<'py, PyIterator>,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        for k in rt::iter(ns)? {
            let k = k?;
            if !k.method1("startswith", ("__",))?.is_truthy()? {
                return Ok(Some(k));
            }
        }
        Ok(None)
    }
    rt::builtins::print(
        py,
        (
            "fake_meta",
            name,
            bases,
            rt::builtins::sorted(py, (rt::genexpr!(frame, genexpr(ns.try_iter()?))?,))?,
            kwargs,
        ),
    )?;
    Ok(rt::int(py, 42))
}

pub mod base {
    use super::*;

    // classes_build.py:21
    /// The body of `class Base`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        Ok(())
    }
}

pub mod ordered {
    use super::*;

    // classes_build.py:25
    /// The body of `class Ordered`, run in its namespace `ns`.
    #[rt::frame(ns, "Ordered", 25)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        ns.get("print")?.call1((
            "body start",
            ns.get("__name__")?,
            ns.get("__module__")?,
            ns.get("__qualname__")?,
        ))?;
        ns.set("x", ns.get("trace")?.call1(("x", 1))?)?;
        ns.set("y", ns.get("x")?.add(1)?)?;
        ns.get("print")?
            .call1(("body end", ns.get("x")?, ns.get("y")?))?;
        Ok(())
    }
}

pub mod logged {
    use super::*;

    // classes_build.py:37
    /// The body of `class Logged`, run in its namespace `ns`.
    #[rt::frame(ns, "Logged", 37)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        let __class__ = ns.class_cell();
        rt::def!(ns, __setitem__[&__class__](slf as "self", key, value))?;
        Ok(())
    }

    // classes_build.py:38
    #[rt::frame(m, "Logged.__setitem__", 38)]
    pub fn __setitem__<'py>(
        m: &rt::Module<'py>,
        __class__: &rt::Cell<'py>,
        slf: &Bound<'py, PyAny>,
        key: &Bound<'py, PyAny>,
        value: &Bound<'py, PyAny>,
    ) -> PyResult<()> {
        rt::builtins::print(m.py(), ("set", key))?;
        rt::super_(__class__, slf)?.method1("__setitem__", (key, value))?;
        Ok(())
    }
}

pub mod meta {
    use super::*;

    // classes_build.py:43
    /// The body of `class Meta`, run in its namespace `ns`.
    #[rt::frame(ns, "Meta", 43)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        let __class__ = ns.class_cell();
        rt::def!(ns, @classmethod __prepare__(mcls, name, bases, **kwargs))?;
        rt::def!(ns, __new__[&__class__](mcls, name, bases, ns, **kwargs))?;
        rt::def!(ns, __init__[&__class__](cls, name, bases, ns, **kwargs))?;
        Ok(())
    }

    // classes_build.py:44
    #[rt::frame(m, "Meta.__prepare__", 44)]
    pub fn __prepare__<'py>(
        m: &rt::Module<'py>,
        mcls: &Bound<'py, PyAny>,
        name: &Bound<'py, PyAny>,
        bases: &Bound<'py, PyAny>,
        kwargs: &Bound<'py, PyDict>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        let list = PyList::empty(py);
        for b in rt::iter(bases)? {
            let b = b?;
            list.append(b.attr("__name__")?)?;
        }
        rt::builtins::print(py, ("prepare", name, list, kwargs))?;
        m.global("Logged")?.call0()
    }

    // classes_build.py:49
    #[rt::frame(m, "Meta.__new__", 49)]
    pub fn __new__<'py>(
        m: &rt::Module<'py>,
        __class__: &rt::Cell<'py>,
        mcls: &Bound<'py, PyAny>,
        name: &Bound<'py, PyAny>,
        bases: &Bound<'py, PyAny>,
        ns: &Bound<'py, PyAny>,
        kwargs: &Bound<'py, PyDict>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::builtins::print(py, ("new", name, rt::builtins::sorted(py, (kwargs,))?))?;
        rt::super_(__class__, mcls)?.attr("__new__")?.call1((
            mcls,
            name,
            bases,
            rt::builtins::dict(py, (ns,))?,
        ))
    }

    // classes_build.py:53
    #[rt::frame(m, "Meta.__init__", 53)]
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        __class__: &rt::Cell<'py>,
        cls: &Bound<'py, PyAny>,
        name: &Bound<'py, PyAny>,
        bases: &Bound<'py, PyAny>,
        ns: &Bound<'py, PyAny>,
        kwargs: &Bound<'py, PyDict>,
    ) -> PyResult<()> {
        rt::builtins::print(m.py(), ("init", name))?;
        rt::super_(__class__, cls)?.method1("__init__", (name, bases, ns))?;
        Ok(())
    }
}

/// Doc.
pub mod with_meta {
    use super::*;

    // classes_build.py:58
    /// The body of `class WithMeta`, run in its namespace `ns`.
    #[rt::frame(ns, "WithMeta", 58)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        let __class__ = ns.class_cell();
        ns.set("__doc__", "Doc.")?;
        ns.set("a", 1)?;
        rt::def!(ns, method[&__class__](slf as "self"))?;
        Ok(())
    }

    // classes_build.py:63
    #[rt::frame(m, "WithMeta.method", 63)]
    pub fn method<'py>(
        m: &rt::Module<'py>,
        __class__: &rt::Cell<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::super_(__class__, slf)?
            .method0("__repr__")?
            .get_item(rt::slice(py, py.None(), 9)?)
    }
}

pub mod plugin {
    use super::*;

    // classes_build.py:70
    /// The body of `class Plugin`, run in its namespace `ns`.
    #[rt::frame(ns, "Plugin", 70)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        let py = m.py();
        let __class__ = ns.class_cell();
        ns.set("registry", PyList::empty(py))?;
        rt::def!(
            ns,
            __init_subclass__[&__class__](cls, /, name = py.None(), **kwargs)
        )?;
        Ok(())
    }

    // classes_build.py:73
    #[rt::frame(m, "Plugin.__init_subclass__", 73)]
    pub fn __init_subclass__<'py>(
        m: &rt::Module<'py>,
        __class__: &rt::Cell<'py>,
        cls: &Bound<'py, PyAny>,
        name: &Bound<'py, PyAny>,
        kwargs: &Bound<'py, PyDict>,
    ) -> PyResult<()> {
        rt::call!(
            rt::super_(__class__, cls)?.attr("__init_subclass__")?,
            **kwargs
        )?;
        cls.set_attr(
            "plugin_name",
            if name.is_truthy()? {
                name.clone()
            } else {
                cls.attr("__name__")?.method0("lower")?
            },
        )?;
        m.global("Plugin")?
            .attr("registry")?
            .attr("append")?
            .call1((cls.attr("plugin_name")?,))?;
        Ok(())
    }
}

pub mod csv {
    use super::*;

    // classes_build.py:79
    /// The body of `class Csv`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        Ok(())
    }
}

pub mod json {
    use super::*;

    // classes_build.py:83
    /// The body of `class Json`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        Ok(())
    }
}

pub mod field {
    use super::*;

    // classes_build.py:90
    /// The body of `class Field`, run in its namespace `ns`.
    #[rt::frame(ns, "Field", 90)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::def!(ns, __set_name__(slf as "self", owner, name))?;
        rt::def!(ns, __get__(slf as "self", obj, objtype = m.py().None()))?;
        Ok(())
    }

    // classes_build.py:91
    #[rt::frame(m, "Field.__set_name__", 91)]
    pub fn __set_name__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        owner: &Bound<'py, PyAny>,
        name: &Bound<'py, PyAny>,
    ) -> PyResult<()> {
        rt::builtins::print(m.py(), ("set_name", owner.attr("__name__")?, name))?;
        slf.set_attr("name", name)?;
        Ok(())
    }

    // classes_build.py:95
    #[rt::frame(m, "Field.__get__", 95)]
    pub fn __get__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        obj: &Bound<'py, PyAny>,
        objtype: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        if obj.is_none() {
            return Ok(slf.clone());
        }
        obj.attr("__dict__")?
            .attr("get")?
            .call1((slf.attr("name")?, "unset"))
    }
}

pub mod record {
    use super::*;

    // classes_build.py:101
    /// The body of `class Record`, run in its namespace `ns`.
    #[rt::frame(ns, "Record", 101)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        ns.set("title", ns.get("Field")?.call0()?)?;
        ns.set("body", ns.get("Field")?.call0()?)?;
        Ok(())
    }
}

pub mod alias {
    use super::*;

    // classes_build.py:111
    /// The body of `class Alias`, run in its namespace `ns`.
    #[rt::frame(ns, "Alias", 111)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::def!(ns, __mro_entries__(slf as "self", bases))?;
        Ok(())
    }

    // classes_build.py:112
    #[rt::frame(m, "Alias.__mro_entries__", 112)]
    pub fn __mro_entries__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        bases: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::builtins::print(py, ("mro_entries", bases.len()?))?;
        Ok(PyTuple::new(py, [&m.global("Base")?])?.into_any())
    }
}

pub mod from_alias {
    use super::*;

    // classes_build.py:120
    /// The body of `class FromAlias`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        Ok(())
    }
}

pub mod starred {
    use super::*;

    // classes_build.py:130
    /// The body of `class Starred`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        Ok(())
    }
}

pub mod errors {
    use super::*;

    // classes_build.py:161
    /// The body of `class Errors`, run in its namespace `ns`.
    #[rt::frame(ns, "Errors", 161, frame)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        let py = m.py();
        match (|| -> PyResult<()> {
            ns.set(
                "Bad",
                rt::class!(
                    m,
                    "Bad"(*rt::int(py, 5)),
                    bad::body,
                    line = 164,
                    qualname = "Errors.Bad"
                )?,
            )?;
            Ok(())
        })() {
            Ok(()) => {}
            Err(err) if err.is_instance_of::<PyTypeError>(py) => {
                let exception = frame.catch(err);
                ns.set("err", &exception)?;
                ns.get("print")?.call1(("TypeError:", ns.get("err")?))?;
                ns.unbind("err")?;
            }
            Err(err) => return Err(err),
        }
        match (|| -> PyResult<()> {
            ns.set(
                "Bad2",
                rt::class!(
                    m,
                    "Bad2"(**rt::int(py, 5)),
                    bad2::body,
                    line = 171,
                    qualname = "Errors.Bad2"
                )?,
            )?;
            Ok(())
        })() {
            Ok(()) => {}
            Err(err) if err.is_instance_of::<PyTypeError>(py) => {
                let exception = frame.catch(err);
                ns.set("err", &exception)?;
                ns.get("print")?.call1(("TypeError:", ns.get("err")?))?;
                ns.unbind("err")?;
            }
            Err(err) => return Err(err),
        }
        match (|| -> PyResult<()> {
            ns.set(
                "Bad3",
                rt::class!(
                    m,
                    "Bad3"(
                        metaclass = ns.get("type")?,
                        **[("metaclass", &ns.get("type")?)].into_py_dict(py)?
                    ),
                    bad3::body,
                    line = 178,
                    qualname = "Errors.Bad3"
                )?,
            )?;
            Ok(())
        })() {
            Ok(()) => {}
            Err(err) if err.is_instance_of::<PyTypeError>(py) => {
                let exception = frame.catch(err);
                ns.set("err", &exception)?;
                ns.get("print")?.call1(("TypeError:", ns.get("err")?))?;
                ns.unbind("err")?;
            }
            Err(err) => return Err(err),
        }
        Ok(())
    }

    pub mod bad {
        use super::*;

        // classes_build.py:164
        /// The body of `class Errors.Bad`, run in its namespace `ns`.
        pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
            Ok(())
        }
    }

    pub mod bad2 {
        use super::*;

        // classes_build.py:171
        /// The body of `class Errors.Bad2`, run in its namespace `ns`.
        pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
            Ok(())
        }
    }

    pub mod bad3 {
        use super::*;

        // classes_build.py:178
        /// The body of `class Errors.Bad3`, run in its namespace `ns`.
        pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
            Ok(())
        }
    }
}

pub mod m1 {
    use super::*;

    // classes_build.py:185
    /// The body of `class M1`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        Ok(())
    }
}

pub mod m2 {
    use super::*;

    // classes_build.py:189
    /// The body of `class M2`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        Ok(())
    }
}

pub mod a1 {
    use super::*;

    // classes_build.py:193
    /// The body of `class A1`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        Ok(())
    }
}

pub mod a2 {
    use super::*;

    // classes_build.py:197
    /// The body of `class A2`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        Ok(())
    }
}

pub mod conflict {
    use super::*;

    // classes_build.py:203
    /// The body of `class Conflict`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        Ok(())
    }
}

pub mod bad_entries {
    use super::*;

    // classes_build.py:210
    /// The body of `class BadEntries`, run in its namespace `ns`.
    #[rt::frame(ns, "BadEntries", 210)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::def!(ns, __mro_entries__(slf as "self", bases))?;
        Ok(())
    }

    // classes_build.py:211
    #[rt::frame(m, "BadEntries.__mro_entries__", 211)]
    pub fn __mro_entries__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        bases: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(PyList::new(m.py(), [&m.global("Base")?])?.into_any())
    }
}

pub mod bad4 {
    use super::*;

    // classes_build.py:217
    /// The body of `class Bad4`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        Ok(())
    }
}

pub mod bad_prepare {
    use super::*;

    // classes_build.py:224
    /// The body of `class BadPrepare`, run in its namespace `ns`.
    #[rt::frame(ns, "BadPrepare", 224)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::def!(ns, @classmethod __prepare__(mcls, name, bases))?;
        Ok(())
    }

    // classes_build.py:225
    pub fn __prepare__<'py>(
        m: &rt::Module<'py>,
        mcls: &Bound<'py, PyAny>,
        name: &Bound<'py, PyAny>,
        bases: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(rt::int(m.py(), 42))
    }
}

pub mod bad5 {
    use super::*;

    // classes_build.py:232
    /// The body of `class Bad5`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        Ok(())
    }
}

pub mod drop_cell {
    use super::*;

    // classes_build.py:239
    /// The body of `class DropCell`, run in its namespace `ns`.
    #[rt::frame(ns, "DropCell", 239)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        let __class__ = ns.class_cell();
        rt::def!(ns, __new__[&__class__](mcls, name, bases, ns))?;
        Ok(())
    }

    // classes_build.py:240
    #[rt::frame(m, "DropCell.__new__", 240)]
    pub fn __new__<'py>(
        m: &rt::Module<'py>,
        __class__: &rt::Cell<'py>,
        mcls: &Bound<'py, PyAny>,
        name: &Bound<'py, PyAny>,
        bases: &Bound<'py, PyAny>,
        ns: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let mut ns = ns.clone();
        let dict = PyDict::new(m.py());
        for item in rt::iter(&ns.method0("items")?)? {
            let [k, v] = rt::unpack(&item?)?;
            if k.ne("__classcell__")? {
                dict.set_item(&k, &v)?;
            }
        }
        ns = dict.into_any();
        rt::super_(__class__, mcls)?.method1("__new__", (mcls, name, bases, &ns))
    }
}

pub mod bad6 {
    use super::*;

    // classes_build.py:247
    /// The body of `class Bad6`, run in its namespace `ns`.
    #[rt::frame(ns, "Bad6", 247)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        let __class__ = ns.class_cell();
        rt::def!(ns, method[&__class__](slf as "self"))?;
        Ok(())
    }

    // classes_build.py:248
    #[rt::frame(m, "Bad6.method", 248)]
    pub fn method<'py>(
        m: &rt::Module<'py>,
        __class__: &rt::Cell<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        __class__.get()
    }
}

pub mod bad7 {
    use super::*;

    // classes_build.py:257
    /// The body of `class Bad7`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        Ok(())
    }
}

pub mod not_a_class {
    use super::*;

    // classes_build.py:269
    /// The body of `class NotAClass`, run in its namespace `ns`.
    #[rt::frame(ns, "NotAClass", 269)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        ns.set("value", 1)?;
        Ok(())
    }
}

pub mod conditional {
    use super::*;

    // classes_build.py:278
    /// The body of `class Conditional`, run in its namespace `ns`.
    #[rt::frame(ns, "Conditional", 278)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        ns.set("branch", "then")?;
        Ok(())
    }
}

pub mod conditional_ {
    use super::*;

    // classes_build.py:283
    /// The body of `class Conditional`, run in its namespace `ns`.
    #[rt::frame(ns, "Conditional", 283)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        ns.set("branch", "else")?;
        Ok(())
    }
}

pub mod outer {
    use super::*;

    // classes_build.py:290
    /// The body of `class Outer`, run in its namespace `ns`.
    #[rt::frame(ns, "Outer", 290)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        ns.set(
            "Inner",
            rt::class!(
                m,
                "Inner"(),
                inner::body,
                line = 291,
                qualname = "Outer.Inner"
            )?,
        )?;
        rt::def!(ns, make(slf as "self"))?;
        Ok(())
    }

    pub mod inner {
        use super::*;

        // classes_build.py:291
        /// The body of `class Outer.Inner`, run in its namespace `ns`.
        #[rt::frame(ns, "Outer.Inner", 291)]
        pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
            ns.set(
                "Deepest",
                rt::class!(
                    m,
                    "Deepest"(),
                    deepest::body,
                    line = 292,
                    qualname = "Outer.Inner.Deepest"
                )?,
            )?;
            Ok(())
        }

        pub mod deepest {
            use super::*;

            // classes_build.py:292
            /// The body of `class Outer.Inner.Deepest`, run in its namespace `ns`.
            #[rt::frame(ns, "Outer.Inner.Deepest", 292)]
            pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
                rt::def!(ns, r#where(slf as "self"))?;
                Ok(())
            }

            // classes_build.py:293
            #[rt::frame(m, "Outer.Inner.Deepest.where", 293)]
            pub fn r#where<'py>(
                m: &rt::Module<'py>,
                slf: &Bound<'py, PyAny>,
            ) -> PyResult<Bound<'py, PyAny>> {
                slf.get_type().attr("__qualname__")
            }
        }
    }

    // classes_build.py:296
    #[rt::frame(m, "Outer.make", 296)]
    pub fn make<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        slf.attr("Inner")?.method0("Deepest")
    }
}

pub mod in_loop {
    use super::*;

    // classes_build.py:304
    /// The body of `class InLoop`, run in its namespace `ns`.
    #[rt::frame(ns, "InLoop", 304)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        ns.set("index", ns.get("i")?)?;
        Ok(())
    }
}

pub mod original {
    use super::*;

    // classes_build.py:318
    /// The body of `class Original`, run in its namespace `ns`.
    #[rt::frame(ns, "Original", 318)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        let __class__ = ns.class_cell();
        rt::def!(ns, hello[&__class__](slf as "self"))?;
        rt::def!(ns, via_super[&__class__](slf as "self"))?;
        Ok(())
    }

    // classes_build.py:320
    #[rt::frame(m, "Original.hello", 320)]
    pub fn hello<'py>(
        m: &rt::Module<'py>,
        __class__: &rt::Cell<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::str(m.py(), "original:").add(__class__.get()?.attr("__name__")?)
    }

    // classes_build.py:323
    #[rt::frame(m, "Original.via_super", 323)]
    pub fn via_super<'py>(
        m: &rt::Module<'py>,
        __class__: &rt::Cell<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::super_(__class__, slf)?
            .method0("__repr__")?
            .get_item(rt::slice(py, py.None(), 10)?)
    }
}

/// Executes the body of `classes_build.py` in the module `m`.
#[rt::frame(m, "<module>", 1, frame)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    m.set_global(
        "__doc__",
        "How a class statement runs: order of evaluation, metaclasses,\n\
            `__prepare__`, `__mro_entries__`, `__init_subclass__`, `__set_name__`.",
    )?;
    rt::def!(m, trace(label, value = py.None()))?;
    rt::def!(m, deco(label))?;
    m.set_global("Base", rt::class!(m, "Base"(), base::body, line = 21)?)?;
    {
        let decorator = deco(m, &rt::str(py, "outer"))?;
        let decorator2 = deco(m, &rt::str(py, "inner"))?;
        let ordered = rt::class!(
            m,
            "Ordered"(
                trace(m, &rt::str(py, "base"), &m.global("Base")?)?,
                metaclass = trace(m, &rt::str(py, "meta"), &rt::builtin(py, "type")?)?
            ),
            ordered::body,
            line = 25
        )?;
        let ordered = decorator2.call1((ordered,))?;
        m.set_global("Ordered", decorator.call1((ordered,))?)?;
    }
    rt::builtins::print(
        py,
        (
            m.global("Ordered")?.attr("decorated")?,
            m.global("Ordered")?.attr("x")?,
            m.global("Ordered")?.attr("y")?,
            m.global("Ordered")?.attr("__firstlineno__")?,
        ),
    )?;
    m.set_global(
        "Logged",
        rt::class!(
            m,
            "Logged"(rt::builtin(py, "dict")?),
            logged::body,
            line = 37
        )?,
    )?;
    m.set_global(
        "Meta",
        rt::class!(m, "Meta"(rt::builtin(py, "type")?), meta::body, line = 43)?,
    )?;
    m.set_global(
        "WithMeta",
        rt::class!(
            m,
            "WithMeta"(metaclass = m.global("Meta")?, flag = true),
            with_meta::body,
            line = 58
        )?,
    )?;
    rt::builtins::print(
        py,
        (
            m.global("WithMeta")?.get_type().attr("__name__")?,
            m.global("WithMeta")?.attr("a")?,
            m.global("WithMeta")?.call0()?.method0("method")?,
        ),
    )?;
    m.set_global(
        "Plugin",
        rt::class!(m, "Plugin"(), plugin::body, line = 70)?,
    )?;
    m.set_global(
        "Csv",
        rt::class!(
            m,
            "Csv"(m.global("Plugin")?, name = "csv"),
            csv::body,
            line = 79
        )?,
    )?;
    m.set_global(
        "Json",
        rt::class!(m, "Json"(m.global("Plugin")?), json::body, line = 83)?,
    )?;
    rt::builtins::print(
        py,
        (
            m.global("Plugin")?.attr("registry")?,
            m.global("Csv")?.attr("plugin_name")?,
            m.global("Json")?.attr("plugin_name")?,
        ),
    )?;
    m.set_global(
        "Field",
        rt::class!(m, "Field"(), field::body, line = 90, attributes = ["name"])?,
    )?;
    m.set_global(
        "Record",
        rt::class!(m, "Record"(), record::body, line = 101)?,
    )?;
    m.set_global("r", m.global("Record")?.call0()?)?;
    m.global("r")?
        .attr("__dict__")?
        .set_item("title", "hello")?;
    rt::builtins::print(
        py,
        (
            m.global("r")?.attr("title")?,
            m.global("r")?.attr("body")?,
            m.global("Record")?.attr("title")?.attr("name")?,
        ),
    )?;
    m.set_global("Alias", rt::class!(m, "Alias"(), alias::body, line = 111)?)?;
    m.set_global("alias", m.global("Alias")?.call0()?)?;
    m.set_global(
        "FromAlias",
        rt::class!(
            m,
            "FromAlias"(m.global("alias")?),
            from_alias::body,
            line = 120
        )?,
    )?;
    rt::builtins::print(
        py,
        (
            m.global("FromAlias")?
                .attr("__bases__")?
                .rich_compare(PyTuple::new(py, [&m.global("Base")?])?, CompareOp::Eq)?,
            m.global("FromAlias")?
                .attr("__orig_bases__")?
                .rich_compare(PyTuple::new(py, [&m.global("alias")?])?, CompareOp::Eq)?,
        ),
    )?;
    m.set_global("bases", PyTuple::new(py, [&m.global("Base")?])?)?;
    m.set_global(
        "extra",
        [("metaclass", &rt::builtin(py, "type")?)].into_py_dict(py)?,
    )?;
    m.set_global(
        "Starred",
        rt::class!(
            m,
            "Starred"(*m.global("bases")?, **m.global("extra")?),
            starred::body,
            line = 130
        )?,
    )?;
    rt::builtins::print(
        py,
        (m.global("Starred")?
            .attr("__bases__")?
            .rich_compare(PyTuple::new(py, [&m.global("Base")?])?, CompareOp::Eq)?,),
    )?;
    // classes_build.py:137, kept as Python: class definition in a function at 140:9
    rt::exec_python!(
        m,
        line = 137,
        r#"
def errors():
    try:

        class Bad(*5):
            pass

    except TypeError as err:
        print("TypeError:", err)
    try:

        class Bad2(**5):
            pass

    except TypeError as err:
        print("TypeError:", err)
    try:

        class Bad3(metaclass=type, **{"metaclass": type}):
            pass

    except TypeError as err:
        print("TypeError:", err)
"#
    )?;
    m.set_global(
        "Errors",
        rt::class!(m, "Errors"(), errors::body, line = 161)?,
    )?;
    m.set_global(
        "M1",
        rt::class!(m, "M1"(rt::builtin(py, "type")?), m1::body, line = 185)?,
    )?;
    m.set_global(
        "M2",
        rt::class!(m, "M2"(rt::builtin(py, "type")?), m2::body, line = 189)?,
    )?;
    m.set_global(
        "A1",
        rt::class!(m, "A1"(metaclass = m.global("M1")?), a1::body, line = 193)?,
    )?;
    m.set_global(
        "A2",
        rt::class!(m, "A2"(metaclass = m.global("M2")?), a2::body, line = 197)?,
    )?;
    match (|| -> PyResult<()> {
        m.set_global(
            "Conflict",
            rt::class!(
                m,
                "Conflict"(m.global("A1")?, m.global("A2")?),
                conflict::body,
                line = 203
            )?,
        )?;
        Ok(())
    })() {
        Ok(()) => {}
        Err(err) if err.is_instance_of::<PyTypeError>(py) => {
            let exception = frame.catch(err);
            m.set_global("err", &exception)?;
            rt::builtins::print(py, ("TypeError:", m.global("err")?))?;
            m.unbind_global("err")?;
        }
        Err(err) => return Err(err),
    }
    m.set_global(
        "BadEntries",
        rt::class!(m, "BadEntries"(), bad_entries::body, line = 210)?,
    )?;
    match (|| -> PyResult<()> {
        m.set_global(
            "Bad4",
            rt::class!(
                m,
                "Bad4"(m.global("BadEntries")?.call0()?),
                bad4::body,
                line = 217
            )?,
        )?;
        Ok(())
    })() {
        Ok(()) => {}
        Err(err) if err.is_instance_of::<PyTypeError>(py) => {
            let exception = frame.catch(err);
            m.set_global("err", &exception)?;
            rt::builtins::print(py, ("TypeError:", m.global("err")?))?;
            m.unbind_global("err")?;
        }
        Err(err) => return Err(err),
    }
    m.set_global(
        "BadPrepare",
        rt::class!(
            m,
            "BadPrepare"(rt::builtin(py, "type")?),
            bad_prepare::body,
            line = 224
        )?,
    )?;
    match (|| -> PyResult<()> {
        m.set_global(
            "Bad5",
            rt::class!(
                m,
                "Bad5"(metaclass = m.global("BadPrepare")?),
                bad5::body,
                line = 232
            )?,
        )?;
        Ok(())
    })() {
        Ok(()) => {}
        Err(err) if err.is_instance_of::<PyTypeError>(py) => {
            let exception = frame.catch(err);
            m.set_global("err", &exception)?;
            rt::builtins::print(py, ("TypeError:", m.global("err")?))?;
            m.unbind_global("err")?;
        }
        Err(err) => return Err(err),
    }
    m.set_global(
        "DropCell",
        rt::class!(
            m,
            "DropCell"(rt::builtin(py, "type")?),
            drop_cell::body,
            line = 239
        )?,
    )?;
    match (|| -> PyResult<()> {
        m.set_global(
            "Bad6",
            rt::class!(
                m,
                "Bad6"(metaclass = m.global("DropCell")?),
                bad6::body,
                line = 247
            )?,
        )?;
        Ok(())
    })() {
        Ok(()) => {}
        Err(err) if err.is_instance_of::<PyRuntimeError>(py) => {
            let exception = frame.catch(err);
            m.set_global("err", &exception)?;
            rt::builtins::print(py, ("RuntimeError:", m.global("err")?))?;
            m.unbind_global("err")?;
        }
        Err(err) => return Err(err),
    }
    match (|| -> PyResult<()> {
        m.set_global(
            "Bad7",
            rt::class!(m, "Bad7"(metaclass = 5), bad7::body, line = 257)?,
        )?;
        Ok(())
    })() {
        Ok(()) => {}
        Err(err) if err.is_instance_of::<PyTypeError>(py) => {
            let exception = frame.catch(err);
            m.set_global("err", &exception)?;
            rt::builtins::print(py, ("TypeError:", m.global("err")?))?;
            m.unbind_global("err")?;
        }
        Err(err) => return Err(err),
    }
    rt::def!(m, fake_meta(name, bases, ns, **kwargs))?;
    m.set_global(
        "NotAClass",
        rt::class!(
            m,
            "NotAClass"(metaclass = m.global("fake_meta")?, option = 1),
            not_a_class::body,
            line = 269
        )?,
    )?;
    rt::builtins::print(py, (m.global("NotAClass")?,))?;
    if m.global("Ordered")?.attr("x")?.is_truthy()? {
        m.set_global(
            "Conditional",
            rt::class!(m, "Conditional"(), conditional::body, line = 278)?,
        )?;
    } else {
        m.set_global(
            "Conditional",
            rt::class!(m, "Conditional"(), conditional_::body, line = 283)?,
        )?;
    }
    rt::builtins::print(py, (m.global("Conditional")?.attr("branch")?,))?;
    m.set_global("Outer", rt::class!(m, "Outer"(), outer::body, line = 290)?)?;
    rt::builtins::print(
        py,
        (
            m.global("Outer")?.attr("Inner")?.attr("__qualname__")?,
            m.global("Outer")?
                .call0()?
                .method0("make")?
                .method0("where")?,
            m.global("Outer")?
                .attr("Inner")?
                .attr("Deepest")?
                .attr("where")?
                .attr("__qualname__")?,
        ),
    )?;
    for item in rt::iter(&rt::builtins::range(py, (2,))?)? {
        m.set_global("i", item?)?;
        m.set_global(
            "InLoop",
            rt::class!(m, "InLoop"(), in_loop::body, line = 304)?,
        )?;
        rt::builtins::print(
            py,
            (
                m.global("InLoop")?.attr("index")?,
                m.global("InLoop")?.attr("__qualname__")?,
            ),
        )?;
    }
    // classes_build.py:310, kept as Python: class definition in a function at 311:5
    rt::exec_python!(
        m,
        line = 310,
        r#"
def replaced(cls):
    class Replacement(cls):
        def hello(self):
            return "replacement+" + super().hello()

    return Replacement
"#
    )?;
    {
        let replaced = m.global("replaced")?;
        let original = rt::class!(m, "Original"(), original::body, line = 318)?;
        m.set_global("Original", replaced.call1((original,))?)?;
    }
    rt::builtins::print(
        py,
        (
            m.global("Original")?.call0()?.method0("hello")?,
            m.global("Original")?.attr("__name__")?,
            m.global("Original")?.call0()?.method0("via_super")?,
        ),
    )?;
    Ok(())
}
