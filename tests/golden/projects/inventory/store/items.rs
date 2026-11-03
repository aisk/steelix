use pyo3::prelude::*;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// store/items.py:6
pub fn fmt_price<'py>(
    m: &rt::Module<'py>,
    price: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "fmt_price", 6, |_| {
        rt::fstring(
            m.py(),
            (
                rt::format(price, "")?,
                " ",
                rt::format(
                    &m.global("config")?.attr("settings")?.get_item("currency")?,
                    "",
                )?,
            ),
        )
    })
}

// store/items.py:10
pub fn label<'py>(m: &rt::Module<'py>, item: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "label", 10, |_| {
        rt::fstring(
            m.py(),
            (
                rt::format(&item.attr("name")?, "")?,
                ": ",
                rt::format(
                    &m.global("fmt_price")?
                        .call1((item.method0("unit_price")?,))?,
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
        rt::frame!(ns, "Item", 14, |_| {
            rt::def!(ns, __init__(slf as "self", name, price, quantity))?;
            rt::def!(ns, unit_price(slf as "self"))?;
            rt::def!(ns, value(slf as "self"))?;
            rt::def!(ns, __repr__(slf as "self"))?;
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
        rt::frame!(m, "Item.__init__", 15, |_| {
            slf.set_attr("name", name)?;
            slf.set_attr("price", price)?;
            slf.set_attr("quantity", quantity)?;
            Ok(rt::none(m.py()))
        })
    }

    // store/items.py:20
    pub fn unit_price<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Item.unit_price", 20, |_| { slf.attr("price") })
    }

    // store/items.py:23
    pub fn value<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Item.value", 23, |_| {
            slf.method0("unit_price")?.mul(slf.attr("quantity")?)
        })
    }

    // store/items.py:26
    pub fn __repr__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Item.__repr__", 26, |_| {
            rt::fstring(
                m.py(),
                (
                    rt::format(&slf.get_type().attr("__name__")?, "")?,
                    "(",
                    slf.attr("name")?.repr()?,
                    ", ",
                    rt::format(&slf.attr("quantity")?, "")?,
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
        rt::frame!(ns, "Perishable", 30, |_| {
            let __class__ = ns.class_cell();
            rt::def!(
                ns,
                __init__[&__class__](slf as "self", name, price, quantity, days)
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
        rt::frame!(m, "Perishable.__init__", 31, |_| {
            rt::super_(__class__, slf)?.method1("__init__", (name, price, quantity))?;
            slf.set_attr("days", days)?;
            Ok(rt::none(m.py()))
        })
    }
}

/// Executes the body of `store/items.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    rt::frame!(m, "<module>", 1, |_| {
        m.set_global("config", rt::import(m, "config")?)?;
        rt::builtins::print(m.py(), ("store.items: imported",))?;
        rt::def!(m, fmt_price(price))?;
        rt::def!(m, label(item))?;
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
