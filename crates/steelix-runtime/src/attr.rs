//! Attribute access and method calls with names spelled in the source.

use pyo3::prelude::*;

/// `PyAnyMethods::getattr` and friends for attribute names written in the
/// Python source, which generated code spells as string literals.
///
/// PyO3 turns a `&str` name into a new Python string on every call, and
/// CPython then hashes it and can not use its attribute caches, which only
/// know interned names. These methods take `'static` names and reuse one
/// interned string per name, like a name in CPython bytecode. Otherwise they
/// are the PyO3 methods: `o.attr("x")` is `o.getattr("x")`, `o.method1("m",
/// (a,))` is `o.call_method1("m", (a,))`.
///
/// ```ignore
/// let ip = ctx.attr("ip")?;            // ctx.ip
/// ctx.set_attr("ip", ip.add(1)?)?;    // ctx.ip = ctx.ip + 1
/// ctx.method1("push", (&value,))?;    // ctx.push(value)
/// ```
pub trait Attributes<'py> {
    /// `o.name`
    fn attr(&self, name: &'static str) -> PyResult<Bound<'py, PyAny>>;
    /// `o.name = value`
    fn set_attr(&self, name: &'static str, value: impl IntoPyObject<'py>) -> PyResult<()>;
    /// `del o.name`
    fn del_attr(&self, name: &'static str) -> PyResult<()>;
    /// `o.name()`
    fn method0(&self, name: &'static str) -> PyResult<Bound<'py, PyAny>>;
    /// `o.name(args..)`
    fn method1(
        &self,
        name: &'static str,
        args: impl pyo3::call::PyCallArgs<'py>,
    ) -> PyResult<Bound<'py, PyAny>>;
}

impl<'py> Attributes<'py> for Bound<'py, PyAny> {
    #[inline]
    fn attr(&self, name: &'static str) -> PyResult<Bound<'py, PyAny>> {
        self.getattr(crate::interned(self.py(), name))
    }

    #[inline]
    fn set_attr(&self, name: &'static str, value: impl IntoPyObject<'py>) -> PyResult<()> {
        self.setattr(crate::interned(self.py(), name), value)
    }

    #[inline]
    fn del_attr(&self, name: &'static str) -> PyResult<()> {
        self.delattr(crate::interned(self.py(), name))
    }

    #[inline]
    fn method0(&self, name: &'static str) -> PyResult<Bound<'py, PyAny>> {
        self.call_method0(crate::interned(self.py(), name))
    }

    #[inline]
    fn method1(
        &self,
        name: &'static str,
        args: impl pyo3::call::PyCallArgs<'py>,
    ) -> PyResult<Bound<'py, PyAny>> {
        self.call_method1(crate::interned(self.py(), name), args)
    }
}
