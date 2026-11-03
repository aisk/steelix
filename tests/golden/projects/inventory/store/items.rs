use pyo3::prelude::*;
use steelix_runtime as rt;

// store/items.py:6
pub fn fmt_price<'py>(
    m: &rt::Module<'py>,
    price: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "fmt_price", 6, |frame| {
        frame.line(7);
        rt::fstring(
            m.py(),
            (
                rt::format(price, "")?,
                " ",
                rt::format(
                    &m.global("config")?
                        .getattr("settings")?
                        .get_item("currency")?,
                    "",
                )?,
            ),
        )
    })
}

// store/items.py:10
pub fn label<'py>(m: &rt::Module<'py>, item: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "label", 10, |frame| {
        frame.line(11);
        rt::fstring(
            m.py(),
            (
                rt::format(&item.getattr("name")?, "")?,
                ": ",
                rt::format(
                    &m.global("fmt_price")?
                        .call1((item.call_method0("unit_price")?,))?,
                    "",
                )?,
            ),
        )
    })
}

pub mod item {
    use super::*;

    // store/items.py:14
    /// The body of `class Item`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Item", 14, |frame| {
            frame.line(15);
            ns.set(
                "__init__",
                rt::function!(
                    ns,
                    __init__(slf as "self", name, price, quantity),
                    qualname = "Item.__init__"
                )?,
            )?;
            frame.line(20);
            ns.set(
                "unit_price",
                rt::function!(
                    ns,
                    unit_price(slf as "self"),
                    qualname = "Item.unit_price"
                )?,
            )?;
            frame.line(23);
            ns.set(
                "value",
                rt::function!(ns, value(slf as "self"), qualname = "Item.value")?,
            )?;
            frame.line(26);
            ns.set(
                "__repr__",
                rt::function!(ns, __repr__(slf as "self"), qualname = "Item.__repr__")?,
            )?;
            Ok(())
        })
    }

    // store/items.py:15
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        name: &Bound<'py, PyAny>,
        price: &Bound<'py, PyAny>,
        quantity: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Item.__init__", 15, |frame| {
            frame.line(16);
            slf.setattr("name", name)?;
            frame.line(17);
            slf.setattr("price", price)?;
            frame.line(18);
            slf.setattr("quantity", quantity)?;
            Ok(rt::none(m.py()))
        })
    }

    // store/items.py:20
    pub fn unit_price<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Item.unit_price", 20, |frame| {
            frame.line(21);
            slf.getattr("price")
        })
    }

    // store/items.py:23
    pub fn value<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Item.value", 23, |frame| {
            frame.line(24);
            slf.call_method0("unit_price")?
                .mul(slf.getattr("quantity")?)
        })
    }

    // store/items.py:26
    pub fn __repr__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Item.__repr__", 26, |frame| {
            frame.line(27);
            rt::fstring(
                m.py(),
                (
                    rt::format(&slf.get_type().getattr("__name__")?, "")?,
                    "(",
                    slf.getattr("name")?.repr()?,
                    ", ",
                    rt::format(&slf.getattr("quantity")?, "")?,
                    ")",
                ),
            )
        })
    }
}

pub mod perishable {
    use super::*;

    // store/items.py:30
    /// The body of `class Perishable`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Perishable", 30, |frame| {
            let __class__ = ns.class_cell();
            frame.line(31);
            ns.set(
                "__init__",
                rt::function!(
                    ns,
                    __init__[&__class__](slf as "self", name, price, quantity, days),
                    qualname = "Perishable.__init__"
                )?,
            )?;
            Ok(())
        })
    }

    // store/items.py:31
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        __class__: &rt::Cell<'py>,
        slf: &Bound<'py, PyAny>,
        name: &Bound<'py, PyAny>,
        price: &Bound<'py, PyAny>,
        quantity: &Bound<'py, PyAny>,
        days: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Perishable.__init__", 31, |frame| {
            frame.line(32);
            rt::super_(__class__, slf)?.call_method1("__init__", (name, price, quantity))?;
            frame.line(33);
            slf.setattr("days", days)?;
            Ok(rt::none(m.py()))
        })
    }
}

/// Executes the body of `store/items.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    rt::frame!(m, "<module>", 1, |frame| {
        m.set_global("config", rt::import(m, "config")?)?;
        frame.line(3);
        rt::builtins::print(m.py(), ("store.items: imported",))?;
        rt::def!(m, fmt_price(price))?;
        rt::def!(m, label(item))?;
        frame.line(14);
        m.set_global(
            "Item",
            rt::class!(
                m,
                "Item"(),
                item::body,
                line = 14,
                attributes = ["name", "price", "quantity"]
            )?,
        )?;
        frame.line(30);
        m.set_global(
            "Perishable",
            rt::class!(
                m,
                "Perishable"(m.global("Item")?),
                perishable::body,
                line = 30,
                attributes = ["days"]
            )?,
        )?;
        Ok(())
    })
}
