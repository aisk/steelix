use pyo3::exceptions::PyKeyError;
use pyo3::prelude::*;
use pyo3::types::PyIterator;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;

pub mod account {
    use super::*;

    // classes_mangling.py:4
    /// The body of `class Account`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Account", 4, |frame| {
            frame.line(5);
            ns.set("_Account__count", 0)?;
            frame.line(6);
            ns.set(
                "_Account__slots_like",
                PyTuple::new(m.py(), ["__a", "__b"])?,
            )?;
            frame.line(8);
            ns.set(
                "__init__",
                rt::function!(
                    ns,
                    __init__(slf as "self", owner),
                    qualname = "Account.__init__"
                )?,
            )?;
            frame.line(13);
            ns.set(
                "_Account__secret",
                rt::function!(
                    ns,
                    __secret(slf as "self"),
                    qualname = "Account.__secret"
                )?,
            )?;
            frame.line(16);
            ns.set(
                "reveal",
                rt::function!(ns, reveal(slf as "self"), qualname = "Account.reveal")?,
            )?;
            frame.line(19);
            ns.set(
                "get",
                rt::function!(
                    ns,
                    get(slf as "self", _Account__key = "default"),
                    qualname = "Account.get"
                )?,
            )?;
            frame.line(23);
            let decorator = ns.get("staticmethod")?;
            let function = rt::function!(ns, count(), qualname = "Account.count")?;
            ns.set("count", decorator.call1((&function,))?)?;
            frame.line(26);
            ns.set(
                "_Account__Inner",
                rt::class!(
                    m,
                    "__Inner"(),
                    __inner::body,
                    line = 26,
                    qualname = "Account.__Inner"
                )?,
            )?;
            frame.line(32);
            ns.set(
                "inner",
                rt::function!(ns, inner(slf as "self"), qualname = "Account.inner")?,
            )?;
            frame.line(35);
            ns.set(
                "handler",
                rt::function!(
                    ns,
                    handler(slf as "self"),
                    qualname = "Account.handler"
                )?,
            )?;
            frame.line(41);
            ns.set(
                "names",
                rt::function!(ns, names(slf as "self"), qualname = "Account.names")?,
            )?;
            Ok(())
        })
    }

    // classes_mangling.py:8
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        owner: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Account.__init__", 8, |frame| {
            frame.line(9);
            slf.setattr("_Account__owner", owner)?;
            frame.line(10);
            let __class__ = slf.getattr("__class__")?;
            __class__.setattr(
                "_Account__count",
                rt::iadd(&__class__.getattr("_Account__count")?, 1)?,
            )?;
            frame.line(11);
            let Account = m.global("Account")?;
            Account.setattr(
                "_Account__count",
                rt::iadd(&Account.getattr("_Account__count")?, 0)?,
            )?;
            Ok(rt::none(m.py()))
        })
    }

    // classes_mangling.py:13
    pub fn __secret<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Account.__secret", 13, |frame| {
            frame.line(14);
            rt::str(m.py(), "secret of ").add(slf.getattr("_Account__owner")?)
        })
    }

    // classes_mangling.py:16
    pub fn reveal<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Account.reveal", 16, |frame| {
            frame.line(17);
            slf.call_method0("_Account__secret")
        })
    }

    // classes_mangling.py:19
    pub fn get<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        _Account__key: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(_Account__key.clone())
    }

    // classes_mangling.py:23
    pub fn count<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Account.count", 22, |frame| {
            frame.line(24);
            m.global("Account")?.getattr("_Account__count")
        })
    }

    pub mod __inner {
        use super::*;

        // classes_mangling.py:26
        /// The body of `class Account.__Inner`, run in its namespace `ns`.
        pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
            rt::frame!(ns, "Account.__Inner", 26, |frame| {
                frame.line(27);
                ns.set("_Inner__level", 2)?;
                frame.line(29);
                ns.set(
                    "level",
                    rt::function!(
                        ns,
                        level(slf as "self"),
                        qualname = "Account.__Inner.level"
                    )?,
                )?;
                Ok(())
            })
        }

        // classes_mangling.py:29
        pub fn level<'py>(
            m: &rt::Module<'py>,
            slf: &Bound<'py, PyAny>,
        ) -> PyResult<Bound<'py, PyAny>> {
            rt::frame!(m, "Account.__Inner.level", 29, |frame| {
                frame.line(30);
                slf.getattr("_Inner__level")
            })
        }
    }

    // classes_mangling.py:32
    pub fn inner<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Account.inner", 32, |frame| {
            frame.line(33);
            slf.call_method0("_Account__Inner")?.call_method0("level")
        })
    }

    // classes_mangling.py:35
    pub fn handler<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Account.handler", 35, |frame| {
            match (|| -> PyResult<()> {
                frame.line(37);
                return Err(frame.raise(PyKeyError::new_err("k")));
            })() {
                Ok(()) => unreachable!(),
                Err(err) if err.is_instance_of::<PyKeyError>(m.py()) => {
                    let _Account__err = frame.catch(err);
                    let _handling = rt::Handling::new(&_Account__err);
                    frame.line(39);
                    return Ok(_Account__err.repr()?.into_any());
                }
                Err(err) => return Err(err),
            }
        })
    }

    // classes_mangling.py:41
    pub fn names<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "Account.names", 41, |frame| {
            let _Account__local = rt::int(py, 1);
            frame.line(43);
            let list = PyList::empty(py);
            for n in rt::builtin(py, "dir")?.call1((slf,))?.try_iter()? {
                let n = n?;
                if n.contains("secret")? || n.contains("owner")? {
                    list.append(&n)?;
                }
                py.check_signals()?;
            }
            list.add(PyList::new(py, [&_Account__local])?)
        })
    }
}

pub mod _under {
    use super::*;

    // classes_mangling.py:46
    /// The body of `class _Under`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "_Under", 46, |frame| {
            frame.line(47);
            ns.set(
                "_Under__x",
                rt::function!(ns, __x(slf as "self"), qualname = "_Under.__x")?,
            )?;
            frame.line(50);
            ns.set(
                "call",
                rt::function!(ns, call(slf as "self"), qualname = "_Under.call")?,
            )?;
            Ok(())
        })
    }

    // classes_mangling.py:47
    pub fn __x<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        Ok(rt::str(m.py(), "under"))
    }

    // classes_mangling.py:50
    pub fn call<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "_Under.call", 50, |frame| {
            frame.line(51);
            slf.call_method0("_Under__x")
        })
    }
}

pub mod ___ {
    use super::*;

    // classes_mangling.py:54
    /// The body of `class ___`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "___", 54, |frame| {
            frame.line(55);
            ns.set(
                "__y",
                rt::function!(ns, __y(slf as "self"), qualname = "___.__y")?,
            )?;
            frame.line(58);
            ns.set(
                "call",
                rt::function!(ns, call(slf as "self"), qualname = "___.call")?,
            )?;
            Ok(())
        })
    }

    // classes_mangling.py:55
    pub fn __y<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        Ok(rt::str(m.py(), "only underscores"))
    }

    // classes_mangling.py:58
    pub fn call<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "___.call", 58, |frame| {
            frame.line(59);
            slf.call_method0("__y")
        })
    }
}

/// Executes the body of `classes_mangling.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        m.set_global("__doc__", "Private names are mangled like CPython does.")?;
        frame.line(4);
        m.set_global(
            "Account",
            rt::class!(
                m,
                "Account"(),
                account::body,
                line = 4,
                attributes = ["__owner"]
            )?,
        )?;
        frame.line(46);
        m.set_global(
            "_Under",
            rt::class!(m, "_Under"(), _under::body, line = 46)?,
        )?;
        frame.line(54);
        m.set_global("___", rt::class!(m, "___"(), ___::body, line = 54)?)?;
        frame.line(62);
        m.set_global("a", m.global("Account")?.call1(("ann",))?)?;
        frame.line(63);
        m.global("Account")?.call1(("bob",))?;
        frame.line(64);
        rt::builtins::print(
            py,
            (
                m.global("a")?.call_method0("reveal")?,
                m.global("a")?.getattr("_Account__owner")?,
                m.global("Account")?.call_method0("count")?,
                m.global("a")?.call_method0("inner")?,
                m.global("a")?.call_method0("get")?,
                rt::call!(m.global("a")?.getattr("get")?, _Account__key = 1)?,
            ),
        )?;
        frame.line(65);
        rt::builtins::print(
            py,
            (
                m.global("a")?.call_method0("handler")?,
                m.global("a")?.call_method0("names")?,
                rt::builtin(py, "hasattr")?.call1((m.global("a")?, "__owner"))?,
                m.global("Account")?
                    .getattr("__dict__")?
                    .contains("_Account__secret")?,
            ),
        )?;
        frame.line(66);
        rt::builtins::print(
            py,
            (
                m.global("Account")?
                    .getattr("_Account__Inner")?
                    .getattr("__qualname__")?,
                m.global("Account")?.getattr("__static_attributes__")?,
            ),
        )?;
        // classes_mangling.py:67
        fn genexpr<'py>(
            m: &rt::Module<'py>,
            __dict__: &Bound<'py, PyIterator>,
        ) -> PyResult<Option<Bound<'py, PyAny>>> {
            rt::frame!(m, "<genexpr>", 67, |_| {
                for k in __dict__ {
                    let k = k?;
                    if k.contains("y")? {
                        return Ok(Some(k.clone()));
                    }
                    m.py().check_signals()?;
                }
                Ok(None)
            })
        }
        frame.line(67);
        rt::builtins::print(
            py,
            (
                m.global("_Under")?.call0()?.call_method0("call")?,
                m.global("___")?.call0()?.call_method0("call")?,
                rt::builtin(py, "sorted")?.call1((rt::genexpr!(
                    m,
                    genexpr(m.global("___")?.getattr("__dict__")?.try_iter()?)
                )?,))?,
            ),
        )?;
        frame.line(68);
        rt::builtins::print(py, (m.global("Account")?.getattr("_Account__slots_like")?,))?;
        Ok(())
    })
}
