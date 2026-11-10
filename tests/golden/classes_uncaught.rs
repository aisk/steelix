use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

pub mod account {
    use super::*;

    // classes_uncaught.py:5
    /// The body of `class Account`, run in its namespace `ns`.
    #[rt::frame(ns, "Account", 5)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::def!(ns, __init__(slf as "self", balance))?;
        rt::def!(ns, withdraw(slf as "self", amount))?;
        rt::def!(ns, @classmethod opened(cls, balance))?;
        Ok(())
    }

    // classes_uncaught.py:6
    #[rt::frame(m, "Account.__init__", 6)]
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        balance: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        slf.set_attr("balance", balance)?;
        Ok(rt::none(m.py()))
    }

    // classes_uncaught.py:9
    #[rt::frame(m, "Account.withdraw", 9, frame)]
    pub fn withdraw<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        amount: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        if amount.gt(slf.attr("balance")?)? {
            return Err(frame.raise(PyValueError::new_err((rt::fstring(
                m.py(),
                (
                    "insufficient funds: ",
                    rt::format(&slf.attr("balance")?, "")?,
                    " < ",
                    rt::format(amount, "")?,
                ),
            )?
            .unbind(),))));
        }
        slf.set_attr("balance", rt::isub(&slf.attr("balance")?, amount)?)?;
        slf.attr("balance")
    }

    // classes_uncaught.py:15
    #[rt::frame(m, "Account.opened", 15)]
    pub fn opened<'py>(
        m: &rt::Module<'py>,
        cls: &Bound<'py, PyAny>,
        balance: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let account = cls.call1((balance,))?;
        account.attr("withdraw")?.call1((balance.mul(2)?,))?;
        Ok(account)
    }
}

/// Executes the body of `classes_uncaught.py` in the module `m`.
#[rt::frame(m, "<module>", 1)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    m.set_global(
        "__doc__",
        "An uncaught exception raised in a method: the traceback has the frames of\n\
            the methods and the module.",
    )?;
    m.set_global(
        "Account",
        rt::class!(
            m,
            "Account"(),
            account::body,
            line = 5,
            attributes = ["balance"]
        )?,
    )?;
    rt::builtins::print(m.py(), ("opening",))?;
    m.set_global("first", m.global("Account")?.method1("opened", (5,))?)?;
    Ok(())
}
