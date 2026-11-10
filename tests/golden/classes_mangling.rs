use pyo3::exceptions::PyKeyError;
use pyo3::prelude::*;
use pyo3::types::PyIterator;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

pub mod account {
    use super::*;

    // classes_mangling.py:4
    /// The body of `class Account`, run in its namespace `ns`.
    #[rt::frame(ns, "Account", 4)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        ns.set("_Account__count", 0)?;
        ns.set(
            "_Account__slots_like",
            PyTuple::new(m.py(), ["__a", "__b"])?,
        )?;
        rt::def!(ns, __init__(slf as "self", owner))?;
        ns.set(
            "_Account__secret",
            rt::function!(ns, __secret(slf as "self"))?,
        )?;
        rt::def!(ns, reveal(slf as "self"))?;
        rt::def!(ns, get(slf as "self", _Account__key = "default"))?;
        rt::def!(ns, @staticmethod count())?;
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
        rt::def!(ns, inner(slf as "self"))?;
        rt::def!(ns, handler(slf as "self"))?;
        rt::def!(ns, names(slf as "self"))?;
        Ok(())
    }

    // classes_mangling.py:8
    #[rt::frame(m, "Account.__init__", 8)]
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        owner: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        slf.set_attr("_Account__owner", owner)?;
        {
            let __class__ = slf.attr("__class__")?;
            __class__.set_attr(
                "_Account__count",
                rt::iadd(&__class__.attr("_Account__count")?, 1)?,
            )?;
        }
        {
            let Account = m.global("Account")?;
            Account.set_attr(
                "_Account__count",
                rt::iadd(&Account.attr("_Account__count")?, 0)?,
            )?;
        }
        Ok(rt::none(m.py()))
    }

    // classes_mangling.py:13
    #[rt::frame(m, "Account.__secret", 13)]
    pub fn __secret<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::str(m.py(), "secret of ").add(slf.attr("_Account__owner")?)
    }

    // classes_mangling.py:16
    #[rt::frame(m, "Account.reveal", 16)]
    pub fn reveal<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        slf.method0("_Account__secret")
    }

    // classes_mangling.py:19
    pub fn get<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        _Account__key: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(_Account__key.clone())
    }

    // classes_mangling.py:22
    #[rt::frame(m, "Account.count", 22)]
    pub fn count<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
        m.global("Account")?.attr("_Account__count")
    }

    pub mod __inner {
        use super::*;

        // classes_mangling.py:26
        /// The body of `class Account.__Inner`, run in its namespace `ns`.
        #[rt::frame(ns, "Account.__Inner", 26)]
        pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
            ns.set("_Inner__level", 2)?;
            rt::def!(ns, level(slf as "self"))?;
            Ok(())
        }

        // classes_mangling.py:29
        #[rt::frame(m, "Account.__Inner.level", 29)]
        pub fn level<'py>(
            m: &rt::Module<'py>,
            slf: &Bound<'py, PyAny>,
        ) -> PyResult<Bound<'py, PyAny>> {
            slf.attr("_Inner__level")
        }
    }

    // classes_mangling.py:32
    #[rt::frame(m, "Account.inner", 32)]
    pub fn inner<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        slf.method0("_Account__Inner")?.method0("level")
    }

    // classes_mangling.py:35
    #[rt::frame(m, "Account.handler", 35, frame)]
    pub fn handler<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        match Err(frame.raise(PyKeyError::new_err("k"))) {
            Ok(()) => unreachable!(),
            Err(err) if err.is_instance_of::<PyKeyError>(m.py()) => {
                let _Account__err = frame.catch(err);
                Ok(_Account__err.repr()?.into_any())
            }
            Err(err) => Err(err),
        }
    }

    // classes_mangling.py:41
    #[rt::frame(m, "Account.names", 41)]
    pub fn names<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        let _Account__local = rt::int(py, 1);
        let list = PyList::empty(py);
        for n in rt::iter(&rt::builtin(py, "dir")?.call1((slf,))?)? {
            let n = n?;
            if n.contains("secret")? || n.contains("owner")? {
                list.append(&n)?;
            }
        }
        list.add(PyList::new(py, [&_Account__local])?)
    }
}

pub mod _under {
    use super::*;

    // classes_mangling.py:46
    /// The body of `class _Under`, run in its namespace `ns`.
    #[rt::frame(ns, "_Under", 46)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        ns.set("_Under__x", rt::function!(ns, __x(slf as "self"))?)?;
        rt::def!(ns, call(slf as "self"))?;
        Ok(())
    }

    // classes_mangling.py:47
    pub fn __x<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        Ok(rt::str(m.py(), "under"))
    }

    // classes_mangling.py:50
    #[rt::frame(m, "_Under.call", 50)]
    pub fn call<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        slf.method0("_Under__x")
    }
}

pub mod ___ {
    use super::*;

    // classes_mangling.py:54
    /// The body of `class ___`, run in its namespace `ns`.
    #[rt::frame(ns, "___", 54)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::def!(ns, __y(slf as "self"))?;
        rt::def!(ns, call(slf as "self"))?;
        Ok(())
    }

    // classes_mangling.py:55
    pub fn __y<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        Ok(rt::str(m.py(), "only underscores"))
    }

    // classes_mangling.py:58
    #[rt::frame(m, "___.call", 58)]
    pub fn call<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        slf.method0("__y")
    }
}

/// Executes the body of `classes_mangling.py` in the module `m`.
#[rt::frame(m, "<module>", 1)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    m.set_global("__doc__", "Private names are mangled like CPython does.")?;
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
    m.set_global(
        "_Under",
        rt::class!(m, "_Under"(), _under::body, line = 46)?,
    )?;
    m.set_global("___", rt::class!(m, "___"(), ___::body, line = 54)?)?;
    m.set_global("a", m.global("Account")?.call1(("ann",))?)?;
    m.global("Account")?.call1(("bob",))?;
    rt::builtins::print(
        py,
        (
            m.global("a")?.method0("reveal")?,
            m.global("a")?.attr("_Account__owner")?,
            m.global("Account")?.method0("count")?,
            m.global("a")?.method0("inner")?,
            m.global("a")?.method0("get")?,
            rt::call!(m.global("a")?.attr("get")?, _Account__key = 1)?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            m.global("a")?.method0("handler")?,
            m.global("a")?.method0("names")?,
            rt::builtin(py, "hasattr")?.call1((m.global("a")?, "__owner"))?,
            m.global("Account")?
                .attr("__dict__")?
                .contains("_Account__secret")?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            m.global("Account")?
                .attr("_Account__Inner")?
                .attr("__qualname__")?,
            m.global("Account")?.attr("__static_attributes__")?,
        ),
    )?;
    // classes_mangling.py:67
    #[rt::frame(m, "<genexpr>", 67)]
    fn genexpr<'py>(
        m: &rt::Module<'py>,
        __dict__: &Bound<'py, PyIterator>,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        for k in rt::iter(__dict__)? {
            let k = k?;
            if k.contains("y")? {
                return Ok(Some(k));
            }
        }
        Ok(None)
    }
    rt::builtins::print(
        py,
        (
            m.global("_Under")?.call0()?.method0("call")?,
            m.global("___")?.call0()?.method0("call")?,
            rt::builtin(py, "sorted")?.call1((rt::genexpr!(
                m,
                genexpr(m.global("___")?.attr("__dict__")?.try_iter()?)
            )?,))?,
        ),
    )?;
    rt::builtins::print(py, (m.global("Account")?.attr("_Account__slots_like")?,))?;
    Ok(())
}
