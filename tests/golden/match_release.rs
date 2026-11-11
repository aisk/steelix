use pyo3::prelude::*;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// match_release.py:44
#[rt::frame(m, "captures", 44)]
pub fn captures<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    {
        let subject2 = m.global("Fresh")?.call1(("a",))?;
        if let Some([x]) = rt::match_seq(&subject2)? {
            rt::builtins::print(py, ("matched", x.attr("name")?))?;
        }
    }
    // A capture only read in its case is released at the end of the case.
    rt::builtins::print(
        py,
        (
            "captures after the match:",
            rt::builtins::sorted(py, (m.global("alive")?,))?,
        ),
    )?;
    Ok(rt::none(py))
}

// match_release.py:52
#[rt::frame(m, "unused", 52)]
pub fn unused<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    {
        let subject2 = m.global("Fresh")?.call1(("b",))?;
        if let Some([_]) = rt::match_seq(&subject2)? {
            rt::builtins::print(
                py,
                (
                    "unused capture:",
                    rt::builtins::sorted(py, (m.global("alive")?,))?,
                ),
            )?;
        }
    }
    Ok(rt::none(py))
}

// match_release.py:58
#[rt::frame(m, "subject", 58)]
pub fn subject<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    // CPython has released the subject and the attribute here.
    {
        let subject2 = m.global("Fresh")?.call1(("c",))?;
        if let Some([item]) = rt::match_class(&subject2, &m.global("Fresh")?, 1, [])?
            && rt::match_instance(&item, &m.global("T")?)?
        {
            rt::builtins::print(
                py,
                (
                    "in the body:",
                    rt::builtins::sorted(py, (m.global("alive")?,))?,
                ),
            )?;
        }
    }
    Ok(rt::none(py))
}

pub mod t {
    use super::*;

    // match_release.py:9
    /// The body of `class T`, run in its namespace `ns`.
    #[rt::frame(ns, "T", 9)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::def!(ns, __init__(slf as "self", name))?;
        rt::def!(ns, __del__(slf as "self"))?;
        Ok(())
    }

    // match_release.py:10
    #[rt::frame(m, "T.__init__", 10)]
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        name: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        slf.set_attr("name", name)?;
        m.global("alive")?.method1("add", (name,))?;
        Ok(rt::none(m.py()))
    }

    // match_release.py:14
    #[rt::frame(m, "T.__del__", 14)]
    pub fn __del__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        m.global("alive")?
            .attr("discard")?
            .call1((slf.attr("name")?,))?;
        Ok(rt::none(m.py()))
    }
}

/// A sequence whose items are new objects, referenced by nothing else.
pub mod fresh {
    use super::*;

    // match_release.py:18
    /// The body of `class Fresh`, run in its namespace `ns`.
    #[rt::frame(ns, "Fresh", 18)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        ns.set(
            "__doc__",
            "A sequence whose items are new objects, referenced by nothing else.",
        )?;
        ns.set("__match_args__", PyTuple::new(m.py(), ["item"])?)?;
        rt::def!(ns, __init__(slf as "self", *names))?;
        rt::def!(ns, __del__(slf as "self"))?;
        rt::def!(ns, __len__(slf as "self"))?;
        rt::def!(ns, __getitem__(slf as "self", index))?;
        rt::def!(ns, @property item(slf as "self"))?;
        Ok(())
    }

    // match_release.py:23
    #[rt::frame(m, "Fresh.__init__", 23)]
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        names: &Bound<'py, PyTuple>,
    ) -> PyResult<Bound<'py, PyAny>> {
        slf.set_attr("names", names)?;
        m.global("alive")?.method1("add", ("subject",))?;
        Ok(rt::none(m.py()))
    }

    // match_release.py:27
    #[rt::frame(m, "Fresh.__del__", 27)]
    pub fn __del__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        m.global("alive")?.method1("discard", ("subject",))?;
        Ok(rt::none(m.py()))
    }

    // match_release.py:30
    #[rt::frame(m, "Fresh.__len__", 30)]
    pub fn __len__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(rt::int(m.py(), slf.attr("names")?.len()?))
    }

    // match_release.py:33
    #[rt::frame(m, "Fresh.__getitem__", 33)]
    pub fn __getitem__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        index: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        m.global("T")?.call1((slf.attr("names")?.get_item(index)?,))
    }

    // match_release.py:36
    #[rt::frame(m, "Fresh.item", 36)]
    pub fn item<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        m.global("T")?.call1(("attribute",))
    }
}

/// Executes the body of `match_release.py` in the module `m`.
#[rt::frame(m, "<module>", 1)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    m.set_global(
        "__doc__",
        "When the objects a `match` takes apart are released (design §12): the\n\
            known differences are listed in `KNOWN_DIFFERENCES` in tests/test_e2e.py.",
    )?;
    m.set_global("collections", rt::import(m, "collections.abc")?)?;
    m.set_global("alive", rt::builtins::set(py, ())?)?;
    m.set_global(
        "T",
        rt::class!(m, "T"(), t::body, line = 9, attributes = ["name"])?,
    )?;
    m.set_global(
        "Fresh",
        rt::class!(m, "Fresh"(), fresh::body, line = 18, attributes = ["names"])?,
    )?;
    m.global("collections")?
        .attr("abc")?
        .attr("Sequence")?
        .attr("register")?
        .call1((m.global("Fresh")?,))?;
    rt::def!(m, captures())?;
    rt::def!(m, unused())?;
    rt::def!(m, subject())?;
    captures(m)?;
    unused(m)?;
    subject(m)?;
    rt::builtins::print(
        py,
        (
            "at the end:",
            rt::builtins::sorted(py, (m.global("alive")?,))?,
        ),
    )?;
    Ok(())
}
