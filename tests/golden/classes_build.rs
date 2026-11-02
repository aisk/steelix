//! Parts of `classes_build.py` kept as Python source and run in place by
//! `rt::exec_python!` (steelix does not lower them yet):
//!
//! - line 44, `def __prepare__`: list comprehension
//! - line 137, `def errors`: class definition in a function
//! - line 240, `def __new__`: dict comprehension
//! - line 264, `def fake_meta`: generator expression
//! - line 310, `def replaced`: class definition in a function

use pyo3::exceptions::PyRuntimeError;
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::pyclass::CompareOp;
use pyo3::types::IntoPyDict;
use pyo3::types::PyDict;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;

// classes_build.py:5
pub fn trace<'py>(
    m: &rt::Module<'py>,
    label: &Bound<'py, PyAny>,
    value: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "trace", 5, |frame| {
        frame.line(6);
        rt::builtins::print(m.py(), ("eval", label))?;
        Ok(value.clone())
    })
}

// classes_build.py:10
pub fn deco<'py>(m: &rt::Module<'py>, label: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "deco", 10, |frame| {
        frame.line(11);
        rt::builtins::print(m.py(), ("deco", label))?;
        // classes_build.py:13
        fn apply<'py>(
            m: &rt::Module<'py>,
            label: &Bound<'py, PyAny>,
            cls: &Bound<'py, PyAny>,
        ) -> PyResult<Bound<'py, PyAny>> {
            let py = m.py();
            rt::frame!(m, "deco.<locals>.apply", 13, |frame| {
                frame.line(14);
                rt::builtins::print(py, ("apply", label, cls.getattr("__name__")?))?;
                frame.line(15);
                cls.setattr(
                    "decorated",
                    rt::builtin(py, "getattr")?
                        .call1((cls, "decorated", PyTuple::empty(py)))?
                        .add(PyTuple::new(py, [label])?)?,
                )?;
                Ok(cls.clone())
            })
        }
        let apply = rt::function!(m, apply[label](cls), qualname = "deco.<locals>.apply")?;
        Ok(apply)
    })
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

    // classes_build.py:27
    /// The body of `class Ordered`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Ordered", 25, |frame| {
            frame.line(28);
            ns.get("print")?.call1((
                "body start",
                ns.get("__name__")?,
                ns.get("__module__")?,
                ns.get("__qualname__")?,
            ))?;
            frame.line(29);
            ns.set("x", ns.get("trace")?.call1(("x", 1))?)?;
            frame.line(30);
            ns.set("y", ns.get("x")?.add(1)?)?;
            frame.line(31);
            ns.get("print")?
                .call1(("body end", ns.get("x")?, ns.get("y")?))?;
            Ok(())
        })
    }
}

pub mod logged {
    use super::*;

    // classes_build.py:37
    /// The body of `class Logged`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Logged", 37, |frame| {
            let __class__ = ns.class_cell();
            frame.line(38);
            ns.set(
                "__setitem__",
                rt::function!(
                    ns,
                    __setitem__[&__class__](slf as "self", key, value),
                    qualname = "Logged.__setitem__"
                )?,
            )?;
            Ok(())
        })
    }

    // classes_build.py:38
    pub fn __setitem__<'py>(
        m: &rt::Module<'py>,
        __class__: &rt::Cell<'py>,
        slf: &Bound<'py, PyAny>,
        key: &Bound<'py, PyAny>,
        value: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "Logged.__setitem__", 38, |frame| {
            frame.line(39);
            rt::builtins::print(py, ("set", key))?;
            frame.line(40);
            rt::super_(__class__, slf)?.call_method1("__setitem__", (key, value))?;
            Ok(rt::none(py))
        })
    }
}

pub mod meta {
    use super::*;

    // classes_build.py:43
    /// The body of `class Meta`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Meta", 43, |frame| {
            let __class__ = ns.class_cell();
            // classes_build.py:44, kept as Python: list comprehension at 46:32
            rt::exec_python!(
                ns,
                line = 44,
                r#"
    @classmethod
    def __prepare__(mcls, name, bases, **kwargs):
        print("prepare", name, [b.__name__ for b in bases], kwargs)
        return Logged()
"#
            )?;
            frame.line(49);
            ns.set(
                "__new__",
                rt::function!(
                    ns,
                    __new__[&__class__](mcls, name, bases, ns, **kwargs),
                    qualname = "Meta.__new__"
                )?,
            )?;
            frame.line(53);
            ns.set(
                "__init__",
                rt::function!(
                    ns,
                    __init__[&__class__](cls, name, bases, ns, **kwargs),
                    qualname = "Meta.__init__"
                )?,
            )?;
            Ok(())
        })
    }

    // classes_build.py:49
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
        rt::frame!(m, "Meta.__new__", 49, |frame| {
            frame.line(50);
            rt::builtins::print(
                py,
                ("new", name, rt::builtin(py, "sorted")?.call1((kwargs,))?),
            )?;
            frame.line(51);
            rt::super_(__class__, mcls)?.getattr("__new__")?.call1((
                mcls,
                name,
                bases,
                rt::builtin(py, "dict")?.call1((ns,))?,
            ))
        })
    }

    // classes_build.py:53
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        __class__: &rt::Cell<'py>,
        cls: &Bound<'py, PyAny>,
        name: &Bound<'py, PyAny>,
        bases: &Bound<'py, PyAny>,
        ns: &Bound<'py, PyAny>,
        kwargs: &Bound<'py, PyDict>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "Meta.__init__", 53, |frame| {
            frame.line(54);
            rt::builtins::print(py, ("init", name))?;
            frame.line(55);
            rt::super_(__class__, cls)?.call_method1("__init__", (name, bases, ns))?;
            Ok(rt::none(py))
        })
    }
}

/// Doc.
pub mod with_meta {
    use super::*;

    // classes_build.py:58
    /// The body of `class WithMeta`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "WithMeta", 58, |frame| {
            let __class__ = ns.class_cell();
            frame.line(59);
            ns.set("__doc__", "Doc.")?;
            frame.line(61);
            ns.set("a", 1)?;
            frame.line(63);
            ns.set(
                "method",
                rt::function!(
                    ns,
                    method[&__class__](slf as "self"),
                    qualname = "WithMeta.method"
                )?,
            )?;
            Ok(())
        })
    }

    // classes_build.py:63
    pub fn method<'py>(
        m: &rt::Module<'py>,
        __class__: &rt::Cell<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "WithMeta.method", 63, |frame| {
            frame.line(64);
            rt::super_(__class__, slf)?
                .call_method0("__repr__")?
                .get_item(rt::slice(py, py.None(), 9)?)
        })
    }
}

pub mod plugin {
    use super::*;

    // classes_build.py:70
    /// The body of `class Plugin`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        let py = m.py();
        rt::frame!(ns, "Plugin", 70, |frame| {
            let __class__ = ns.class_cell();
            frame.line(71);
            ns.set("registry", PyList::empty(py))?;
            frame.line(73);
            ns.set(
                "__init_subclass__",
                rt::function!(
                    ns,
                    __init_subclass__[&__class__](cls, /, name = py.None(), **kwargs),
                    qualname = "Plugin.__init_subclass__"
                )?,
            )?;
            Ok(())
        })
    }

    // classes_build.py:73
    pub fn __init_subclass__<'py>(
        m: &rt::Module<'py>,
        __class__: &rt::Cell<'py>,
        cls: &Bound<'py, PyAny>,
        name: &Bound<'py, PyAny>,
        kwargs: &Bound<'py, PyDict>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Plugin.__init_subclass__", 73, |frame| {
            frame.line(74);
            rt::call!(
                rt::super_(__class__, cls)?.getattr("__init_subclass__")?,
                **kwargs
            )?;
            frame.line(75);
            cls.setattr(
                "plugin_name",
                if name.is_truthy()? {
                    name.clone()
                } else {
                    cls.getattr("__name__")?.call_method0("lower")?
                },
            )?;
            frame.line(76);
            m.global("Plugin")?
                .getattr("registry")?
                .getattr("append")?
                .call1((cls.getattr("plugin_name")?,))?;
            Ok(rt::none(m.py()))
        })
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
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Field", 90, |frame| {
            frame.line(91);
            ns.set(
                "__set_name__",
                rt::function!(
                    ns,
                    __set_name__(slf as "self", owner, name),
                    qualname = "Field.__set_name__"
                )?,
            )?;
            frame.line(95);
            ns.set(
                "__get__",
                rt::function!(
                    ns,
                    __get__(slf as "self", obj, objtype = m.py().None()),
                    qualname = "Field.__get__"
                )?,
            )?;
            Ok(())
        })
    }

    // classes_build.py:91
    pub fn __set_name__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        owner: &Bound<'py, PyAny>,
        name: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "Field.__set_name__", 91, |frame| {
            frame.line(92);
            rt::builtins::print(py, ("set_name", owner.getattr("__name__")?, name))?;
            frame.line(93);
            slf.setattr("name", name)?;
            Ok(rt::none(py))
        })
    }

    // classes_build.py:95
    pub fn __get__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        obj: &Bound<'py, PyAny>,
        objtype: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Field.__get__", 95, |frame| {
            if obj.is_none() {
                return Ok(slf.clone());
            }
            frame.line(98);
            obj.getattr("__dict__")?
                .getattr("get")?
                .call1((slf.getattr("name")?, "unset"))
        })
    }
}

pub mod record {
    use super::*;

    // classes_build.py:101
    /// The body of `class Record`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Record", 101, |frame| {
            frame.line(102);
            ns.set("title", ns.get("Field")?.call0()?)?;
            frame.line(103);
            ns.set("body", ns.get("Field")?.call0()?)?;
            Ok(())
        })
    }
}

pub mod alias {
    use super::*;

    // classes_build.py:111
    /// The body of `class Alias`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Alias", 111, |frame| {
            frame.line(112);
            ns.set(
                "__mro_entries__",
                rt::function!(
                    ns,
                    __mro_entries__(slf as "self", bases),
                    qualname = "Alias.__mro_entries__"
                )?,
            )?;
            Ok(())
        })
    }

    // classes_build.py:112
    pub fn __mro_entries__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        bases: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "Alias.__mro_entries__", 112, |frame| {
            frame.line(113);
            rt::builtins::print(py, ("mro_entries", bases.len()?))?;
            frame.line(114);
            Ok(PyTuple::new(py, [&m.global("Base")?])?.into_any())
        })
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
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        let py = m.py();
        rt::frame!(ns, "Errors", 161, |frame| {
            match (|| -> PyResult<()> {
                frame.line(164);
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
                    let _handling = rt::Handling::new(&exception);
                    frame.line(168);
                    ns.get("print")?.call1(("TypeError:", ns.get("err")?))?;
                    ns.unbind("err")?;
                }
                Err(err) => return Err(err),
            }
            match (|| -> PyResult<()> {
                frame.line(171);
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
                    let _handling = rt::Handling::new(&exception);
                    frame.line(175);
                    ns.get("print")?.call1(("TypeError:", ns.get("err")?))?;
                    ns.unbind("err")?;
                }
                Err(err) => return Err(err),
            }
            match (|| -> PyResult<()> {
                frame.line(178);
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
                    let _handling = rt::Handling::new(&exception);
                    frame.line(182);
                    ns.get("print")?.call1(("TypeError:", ns.get("err")?))?;
                    ns.unbind("err")?;
                }
                Err(err) => return Err(err),
            }
            Ok(())
        })
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
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "BadEntries", 210, |frame| {
            frame.line(211);
            ns.set(
                "__mro_entries__",
                rt::function!(
                    ns,
                    __mro_entries__(slf as "self", bases),
                    qualname = "BadEntries.__mro_entries__"
                )?,
            )?;
            Ok(())
        })
    }

    // classes_build.py:211
    pub fn __mro_entries__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        bases: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "BadEntries.__mro_entries__", 211, |frame| {
            frame.line(212);
            Ok(PyList::new(m.py(), [&m.global("Base")?])?.into_any())
        })
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
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "BadPrepare", 224, |frame| {
            frame.line(226);
            let decorator = ns.get("classmethod")?;
            let function = rt::function!(
                ns,
                __prepare__(mcls, name, bases),
                qualname = "BadPrepare.__prepare__"
            )?;
            ns.set("__prepare__", decorator.call1((&function,))?)?;
            Ok(())
        })
    }

    // classes_build.py:226
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
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "DropCell", 239, |_| {
            // classes_build.py:240, kept as Python: dict comprehension at 241:14
            rt::exec_python!(
                ns,
                line = 240,
                r#"
    def __new__(mcls, name, bases, ns):
        ns = {k: v for k, v in ns.items() if k != "__classcell__"}
        return super().__new__(mcls, name, bases, ns)
"#
            )?;
            Ok(())
        })
    }
}

pub mod bad6 {
    use super::*;

    // classes_build.py:247
    /// The body of `class Bad6`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Bad6", 247, |frame| {
            let __class__ = ns.class_cell();
            frame.line(248);
            ns.set(
                "method",
                rt::function!(
                    ns,
                    method[&__class__](slf as "self"),
                    qualname = "Bad6.method"
                )?,
            )?;
            Ok(())
        })
    }

    // classes_build.py:248
    pub fn method<'py>(
        m: &rt::Module<'py>,
        __class__: &rt::Cell<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Bad6.method", 248, |frame| {
            frame.line(249);
            __class__.get()
        })
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
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "NotAClass", 269, |frame| {
            frame.line(270);
            ns.set("value", 1)?;
            Ok(())
        })
    }
}

pub mod conditional {
    use super::*;

    // classes_build.py:278
    /// The body of `class Conditional`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Conditional", 278, |frame| {
            frame.line(279);
            ns.set("branch", "then")?;
            Ok(())
        })
    }
}

pub mod conditional_ {
    use super::*;

    // classes_build.py:283
    /// The body of `class Conditional`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Conditional", 283, |frame| {
            frame.line(284);
            ns.set("branch", "else")?;
            Ok(())
        })
    }
}

pub mod outer {
    use super::*;

    // classes_build.py:290
    /// The body of `class Outer`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Outer", 290, |frame| {
            frame.line(291);
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
            frame.line(296);
            ns.set(
                "make",
                rt::function!(ns, make(slf as "self"), qualname = "Outer.make")?,
            )?;
            Ok(())
        })
    }

    pub mod inner {
        use super::*;

        // classes_build.py:291
        /// The body of `class Outer.Inner`, run in its namespace `ns`.
        pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
            rt::frame!(ns, "Outer.Inner", 291, |frame| {
                frame.line(292);
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
            })
        }

        pub mod deepest {
            use super::*;

            // classes_build.py:292
            /// The body of `class Outer.Inner.Deepest`, run in its namespace `ns`.
            pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
                rt::frame!(ns, "Outer.Inner.Deepest", 292, |frame| {
                    frame.line(293);
                    ns.set(
                        "where",
                        rt::function!(
                            ns,
                            r#where(slf as "self"),
                            qualname = "Outer.Inner.Deepest.where"
                        )?,
                    )?;
                    Ok(())
                })
            }

            // classes_build.py:293
            pub fn r#where<'py>(
                m: &rt::Module<'py>,
                slf: &Bound<'py, PyAny>,
            ) -> PyResult<Bound<'py, PyAny>> {
                rt::frame!(m, "Outer.Inner.Deepest.where", 293, |frame| {
                    frame.line(294);
                    slf.get_type().getattr("__qualname__")
                })
            }
        }
    }

    // classes_build.py:296
    pub fn make<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Outer.make", 296, |frame| {
            frame.line(297);
            slf.getattr("Inner")?.call_method0("Deepest")
        })
    }
}

pub mod in_loop {
    use super::*;

    // classes_build.py:304
    /// The body of `class InLoop`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "InLoop", 304, |frame| {
            frame.line(305);
            ns.set("index", ns.get("i")?)?;
            Ok(())
        })
    }
}

pub mod original {
    use super::*;

    // classes_build.py:319
    /// The body of `class Original`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Original", 318, |frame| {
            let __class__ = ns.class_cell();
            frame.line(320);
            ns.set(
                "hello",
                rt::function!(
                    ns,
                    hello[&__class__](slf as "self"),
                    qualname = "Original.hello"
                )?,
            )?;
            frame.line(323);
            ns.set(
                "via_super",
                rt::function!(
                    ns,
                    via_super[&__class__](slf as "self"),
                    qualname = "Original.via_super"
                )?,
            )?;
            Ok(())
        })
    }

    // classes_build.py:320
    pub fn hello<'py>(
        m: &rt::Module<'py>,
        __class__: &rt::Cell<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Original.hello", 320, |frame| {
            frame.line(321);
            rt::str(m.py(), "original:").add(__class__.get()?.getattr("__name__")?)
        })
    }

    // classes_build.py:323
    pub fn via_super<'py>(
        m: &rt::Module<'py>,
        __class__: &rt::Cell<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "Original.via_super", 323, |frame| {
            frame.line(324);
            rt::super_(__class__, slf)?
                .call_method0("__repr__")?
                .get_item(rt::slice(py, py.None(), 10)?)
        })
    }
}

/// Executes the body of `classes_build.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        m.set_global("__doc__", "How a class statement runs: order of evaluation, metaclasses,\n`__prepare__`, `__mro_entries__`, `__init_subclass__`, `__set_name__`.")?;
        rt::def!(m, trace(label, value = py.None()))?;
        rt::def!(m, deco(label))?;
        frame.line(21);
        m.set_global("Base", rt::class!(m, "Base"(), base::body, line = 21)?)?;
        frame.line(25);
        let decorator = deco(m, &rt::str(py, "outer"))?;
        frame.line(26);
        let decorator2 = deco(m, &rt::str(py, "inner"))?;
        frame.line(27);
        let ordered = rt::class!(
            m,
            "Ordered"(
                trace(m, &rt::str(py, "base"), &m.global("Base")?)?,
                metaclass = trace(m, &rt::str(py, "meta"), &rt::builtin(py, "type")?)?
            ),
            ordered::body,
            line = 25
        )?;
        frame.line(26);
        let ordered = decorator2.call1((ordered,))?;
        frame.line(25);
        let ordered = decorator.call1((ordered,))?;
        m.set_global("Ordered", ordered)?;
        frame.line(34);
        rt::builtins::print(
            py,
            (
                m.global("Ordered")?.getattr("decorated")?,
                m.global("Ordered")?.getattr("x")?,
                m.global("Ordered")?.getattr("y")?,
                m.global("Ordered")?.getattr("__firstlineno__")?,
            ),
        )?;
        frame.line(37);
        m.set_global(
            "Logged",
            rt::class!(
                m,
                "Logged"(rt::builtin(py, "dict")?),
                logged::body,
                line = 37
            )?,
        )?;
        frame.line(43);
        m.set_global(
            "Meta",
            rt::class!(m, "Meta"(rt::builtin(py, "type")?), meta::body, line = 43)?,
        )?;
        frame.line(58);
        m.set_global(
            "WithMeta",
            rt::class!(
                m,
                "WithMeta"(metaclass = m.global("Meta")?, flag = true),
                with_meta::body,
                line = 58
            )?,
        )?;
        frame.line(67);
        rt::builtins::print(
            py,
            (
                m.global("WithMeta")?.get_type().getattr("__name__")?,
                m.global("WithMeta")?.getattr("a")?,
                m.global("WithMeta")?.call0()?.call_method0("method")?,
            ),
        )?;
        frame.line(70);
        m.set_global(
            "Plugin",
            rt::class!(m, "Plugin"(), plugin::body, line = 70)?,
        )?;
        frame.line(79);
        m.set_global(
            "Csv",
            rt::class!(
                m,
                "Csv"(m.global("Plugin")?, name = "csv"),
                csv::body,
                line = 79
            )?,
        )?;
        frame.line(83);
        m.set_global(
            "Json",
            rt::class!(m, "Json"(m.global("Plugin")?), json::body, line = 83)?,
        )?;
        frame.line(87);
        rt::builtins::print(
            py,
            (
                m.global("Plugin")?.getattr("registry")?,
                m.global("Csv")?.getattr("plugin_name")?,
                m.global("Json")?.getattr("plugin_name")?,
            ),
        )?;
        frame.line(90);
        m.set_global(
            "Field",
            rt::class!(m, "Field"(), field::body, line = 90, attributes = ["name"])?,
        )?;
        frame.line(101);
        m.set_global(
            "Record",
            rt::class!(m, "Record"(), record::body, line = 101)?,
        )?;
        frame.line(106);
        m.set_global("r", m.global("Record")?.call0()?)?;
        frame.line(107);
        m.global("r")?
            .getattr("__dict__")?
            .set_item("title", "hello")?;
        frame.line(108);
        rt::builtins::print(
            py,
            (
                m.global("r")?.getattr("title")?,
                m.global("r")?.getattr("body")?,
                m.global("Record")?.getattr("title")?.getattr("name")?,
            ),
        )?;
        frame.line(111);
        m.set_global("Alias", rt::class!(m, "Alias"(), alias::body, line = 111)?)?;
        frame.line(117);
        m.set_global("alias", m.global("Alias")?.call0()?)?;
        frame.line(120);
        m.set_global(
            "FromAlias",
            rt::class!(
                m,
                "FromAlias"(m.global("alias")?),
                from_alias::body,
                line = 120
            )?,
        )?;
        frame.line(124);
        rt::builtins::print(
            py,
            (
                m.global("FromAlias")?
                    .getattr("__bases__")?
                    .rich_compare(PyTuple::new(py, [&m.global("Base")?])?, CompareOp::Eq)?,
                m.global("FromAlias")?
                    .getattr("__orig_bases__")?
                    .rich_compare(PyTuple::new(py, [&m.global("alias")?])?, CompareOp::Eq)?,
            ),
        )?;
        frame.line(126);
        m.set_global("bases", PyTuple::new(py, [&m.global("Base")?])?)?;
        frame.line(127);
        m.set_global(
            "extra",
            [("metaclass", &rt::builtin(py, "type")?)].into_py_dict(py)?,
        )?;
        frame.line(130);
        m.set_global(
            "Starred",
            rt::class!(
                m,
                "Starred"(*m.global("bases")?, **m.global("extra")?),
                starred::body,
                line = 130
            )?,
        )?;
        frame.line(134);
        rt::builtins::print(
            py,
            (m.global("Starred")?
                .getattr("__bases__")?
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
        frame.line(161);
        m.set_global(
            "Errors",
            rt::class!(m, "Errors"(), errors::body, line = 161)?,
        )?;
        frame.line(185);
        m.set_global(
            "M1",
            rt::class!(m, "M1"(rt::builtin(py, "type")?), m1::body, line = 185)?,
        )?;
        frame.line(189);
        m.set_global(
            "M2",
            rt::class!(m, "M2"(rt::builtin(py, "type")?), m2::body, line = 189)?,
        )?;
        frame.line(193);
        m.set_global(
            "A1",
            rt::class!(m, "A1"(metaclass = m.global("M1")?), a1::body, line = 193)?,
        )?;
        frame.line(197);
        m.set_global(
            "A2",
            rt::class!(m, "A2"(metaclass = m.global("M2")?), a2::body, line = 197)?,
        )?;
        match (|| -> PyResult<()> {
            frame.line(203);
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
                let _handling = rt::Handling::new(&exception);
                frame.line(207);
                rt::builtins::print(py, ("TypeError:", m.global("err")?))?;
                m.unbind_global("err")?;
            }
            Err(err) => return Err(err),
        }
        frame.line(210);
        m.set_global(
            "BadEntries",
            rt::class!(m, "BadEntries"(), bad_entries::body, line = 210)?,
        )?;
        match (|| -> PyResult<()> {
            frame.line(217);
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
                let _handling = rt::Handling::new(&exception);
                frame.line(221);
                rt::builtins::print(py, ("TypeError:", m.global("err")?))?;
                m.unbind_global("err")?;
            }
            Err(err) => return Err(err),
        }
        frame.line(224);
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
            frame.line(232);
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
                let _handling = rt::Handling::new(&exception);
                frame.line(236);
                rt::builtins::print(py, ("TypeError:", m.global("err")?))?;
                m.unbind_global("err")?;
            }
            Err(err) => return Err(err),
        }
        frame.line(239);
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
            frame.line(247);
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
                let _handling = rt::Handling::new(&exception);
                frame.line(252);
                rt::builtins::print(py, ("RuntimeError:", m.global("err")?))?;
                m.unbind_global("err")?;
            }
            Err(err) => return Err(err),
        }
        match (|| -> PyResult<()> {
            frame.line(257);
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
                let _handling = rt::Handling::new(&exception);
                frame.line(261);
                rt::builtins::print(py, ("TypeError:", m.global("err")?))?;
                m.unbind_global("err")?;
            }
            Err(err) => return Err(err),
        }
        // classes_build.py:264, kept as Python: generator expression at 265:43
        rt::exec_python!(
            m,
            line = 264,
            r#"
def fake_meta(name, bases, ns, **kwargs):
    print("fake_meta", name, bases, sorted(k for k in ns if not k.startswith("__")), kwargs)
    return 42
"#
        )?;
        frame.line(269);
        m.set_global(
            "NotAClass",
            rt::class!(
                m,
                "NotAClass"(metaclass = m.global("fake_meta")?, option = 1),
                not_a_class::body,
                line = 269
            )?,
        )?;
        frame.line(273);
        rt::builtins::print(py, (m.global("NotAClass")?,))?;
        frame.line(276);
        if m.global("Ordered")?.getattr("x")?.is_truthy()? {
            frame.line(278);
            m.set_global(
                "Conditional",
                rt::class!(m, "Conditional"(), conditional::body, line = 278)?,
            )?;
        } else {
            frame.line(283);
            m.set_global(
                "Conditional",
                rt::class!(m, "Conditional"(), conditional_::body, line = 283)?,
            )?;
        }
        frame.line(287);
        rt::builtins::print(py, (m.global("Conditional")?.getattr("branch")?,))?;
        frame.line(290);
        m.set_global("Outer", rt::class!(m, "Outer"(), outer::body, line = 290)?)?;
        frame.line(300);
        rt::builtins::print(
            py,
            (
                m.global("Outer")?
                    .getattr("Inner")?
                    .getattr("__qualname__")?,
                m.global("Outer")?
                    .call0()?
                    .call_method0("make")?
                    .call_method0("where")?,
                m.global("Outer")?
                    .getattr("Inner")?
                    .getattr("Deepest")?
                    .getattr("where")?
                    .getattr("__qualname__")?,
            ),
        )?;
        frame.line(302);
        for item in rt::builtin(py, "range")?.call1((2,))?.try_iter()? {
            m.set_global("i", item?)?;
            frame.line(304);
            m.set_global(
                "InLoop",
                rt::class!(m, "InLoop"(), in_loop::body, line = 304)?,
            )?;
            frame.line(307);
            rt::builtins::print(
                py,
                (
                    m.global("InLoop")?.getattr("index")?,
                    m.global("InLoop")?.getattr("__qualname__")?,
                ),
            )?;
            py.check_signals()?;
            frame.line(302);
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
        frame.line(318);
        let replaced = m.global("replaced")?;
        frame.line(319);
        let original = rt::class!(m, "Original"(), original::body, line = 318)?;
        frame.line(318);
        let original = replaced.call1((original,))?;
        m.set_global("Original", original)?;
        frame.line(327);
        rt::builtins::print(
            py,
            (
                m.global("Original")?.call0()?.call_method0("hello")?,
                m.global("Original")?.getattr("__name__")?,
                m.global("Original")?.call0()?.call_method0("via_super")?,
            ),
        )?;
        Ok(())
    })
}
