use pyo3::prelude::*;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// docstrings.py:22
/// Summary line.
///
/// Details here with "quotes" and \ backslash  tab.
///   indented two
pub fn f<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    Ok(x.clone())
}

// docstrings.py:31
/// One line only.
pub fn g<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    Ok(rt::none(m.py()))
}

// docstrings.py:35
/// Tabs    inside and a tab
///     starting a line, trailing spaces
/// ünïcødé, a lone \ and an escaped " quote.
///
///
///  form feed
///  vertical tab
pub fn h<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    Ok(rt::none(m.py()))
}

// docstrings.py:44
/// Ends with blank lines.
pub fn k<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    Ok(rt::none(m.py()))
}

/// Class doc.
///
/// More.
pub mod k {
    use super::*;

    // docstrings.py:8
    /// The body of `class K`, run in its namespace `ns`.
    #[rt::frame(ns, "K", 8)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        ns.set(
            "__doc__",
            "Class doc.\n\
                \n\
                More.\n",
        )?;
        rt::def!(
            ns,
            method(slf as "self", a, b = 1),
            doc = "Method doc.\n\
                \n\
                \x20   code block\n\
                trailing"
        )?;
        Ok(())
    }

    // docstrings.py:14
    /// Method doc.
    ///
    ///     code block
    /// trailing
    pub fn method<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        a: &Bound<'py, PyAny>,
        b: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(a.clone())
    }
}

/// Executes the body of `docstrings.py` in the module `m`.
#[rt::frame(m, "<module>", 1)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    m.set_global(
        "__doc__",
        "Docstrings of several lines are printed one line per source line in the\n\
            Rust literal, joined by `\\` continuations, and keep their exact value.\n\
            \n\
            \x20  Indented line.\n",
    )?;
    m.set_global("K", rt::class!(m, "K"(), k::body, line = 8)?)?;
    rt::def!(
        m,
        f(x),
        doc = "Summary line.\n\
            \n\
            Details here with \"quotes\" and \\ backslash  tab.\n\
            \x20 indented two\n"
    )?;
    rt::def!(m, g(), doc = "One line only.")?;
    rt::def!(
        m,
        h(),
        doc = "Tabs    inside and a tab        \n\
            \x20   starting a line, trailing spaces  \n\
            ünïcødé, a lone \\ and an escaped \" quote.\n\
            \n\
            \u{c} form feed\u{b} vertical tab\n"
    )?;
    rt::def!(m, k(), doc = "Ends with blank lines.")?;
    rt::builtins::print(py, (m.global("__doc__")?.repr()?,))?;
    rt::builtins::print(py, (m.global("K")?.attr("__doc__")?.repr()?,))?;
    rt::builtins::print(
        py,
        (m.global("K")?.attr("method")?.attr("__doc__")?.repr()?,),
    )?;
    rt::builtins::print(py, (m.global("f")?.attr("__doc__")?.repr()?,))?;
    rt::builtins::print(py, (m.global("g")?.attr("__doc__")?.repr()?,))?;
    rt::builtins::print(py, (m.global("h")?.attr("__doc__")?.repr()?,))?;
    rt::builtins::print(py, (m.global("k")?.attr("__doc__")?.repr()?,))?;
    Ok(())
}
