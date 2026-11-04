use pyo3::exceptions::PyImportError;
use pyo3::prelude::*;
use pyo3::types::PyIterator;
use pyo3::types::PyList;
use pyo3::types::PySet;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// app.py:44
pub fn build<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "build", 44, |_| {
        let inventory = m.global("Inventory")?.call0()?;
        inventory
            .attr("add")?
            .call1((m.global("Item")?.call1(("apple", 3, 10))?,))?;
        inventory
            .attr("add")?
            .call1((m.global("Discounted")?.call1(("pear", 10, 4, 25))?,))?;
        inventory.attr("add")?.call1((rt::call!(
            m.global("store")?.attr("items")?.attr("Perishable")?,
            "milk",
            2,
            5,
            days = 3
        )?,))?;
        Ok(inventory)
    })
}

// app.py:52
pub fn shop<'py>(
    m: &rt::Module<'py>,
    inventory: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "shop", 52, |frame| {
        match inventory.method1("take", ("pear", 10)) {
            Ok(_) => {}
            Err(err) if frame.matches(&err, 52, || m.global("OutOfStock"))? => {
                let err = frame.catch(err);
                rt::builtins::print(
                    py,
                    (
                        "app: caught",
                        err.get_type().attr("__module__")?,
                        err.get_type().attr("__qualname__")?,
                        &err,
                        err.attr("wanted")?,
                    ),
                )?;
            }
            Err(err2) => return Err(err2),
        }
        match inventory.method1("take", ("plum", 1)) {
            Ok(_) => {}
            Err(err) if frame.matches(&err, 52, || m.global("StoreError"))? => {
                let err = frame.catch(err);
                rt::builtins::print(
                    py,
                    (
                        "app: caught base class",
                        err.get_type().attr("__name__")?,
                        &err,
                    ),
                )?;
            }
            Err(err2) => return Err(err2),
        }
        inventory.method1("take", ("apple", 4))?;
        Ok(rt::none(py))
    })
}

// app.py:64
pub fn plugins<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "plugins", 64, |_| {
        let mut plugin: Bound<'py, PyAny>;
        // A computed name: transpiled only with `--include plugins`.
        for name in PyList::new(py, ["greet", "count"])?.try_iter()? {
            let name = name?;
            plugin = m
                .global("importlib")?
                .attr("import_module")?
                .call1((rt::str(py, "plugins.").add(&name)?,))?;
            rt::builtins::print(
                py,
                (
                    "app: plugin",
                    plugin.attr("__name__")?,
                    plugin.method0("run")?,
                    plugin.attr("__loader__")?.attr("name")?,
                ),
            )?;
            py.check_signals()?;
        }
        let r#static = m
            .global("importlib")?
            .method1("import_module", ("plugins.static",))?;
        rt::builtins::print(
            py,
            (
                "app: plugin",
                r#static.attr("__name__")?,
                r#static.method0("run")?,
            ),
        )?;
        Ok(rt::none(py))
    })
}

// app.py:73
pub fn state<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "state", 73, |_| {
        rt::builtins::print(
            py,
            (
                "app: shared state",
                m.global("config")?.attr("settings")?,
                m.global("config")?.method0("bump")?,
                m.global("config")?.method0("bump")?,
            ),
        )?;
        m.global("config")?
            .attr("settings")?
            .set_item("currency", "USD")?;
        rt::builtins::print(
            py,
            (
                "app: label after change",
                m.global("store")?
                    .attr("items")?
                    .attr("label")?
                    .call1((m.global("Item")?.call1(("fig", 7, 1))?,))?,
            ),
        )?;
        // Replacing a function of another module: its direct callers see it.
        let original = m.global("store")?.attr("items")?.attr("fmt_price")?;
        // app.py:79
        fn lambda<'py>(
            m: &rt::Module<'py>,
            price: &Bound<'py, PyAny>,
        ) -> PyResult<Bound<'py, PyAny>> {
            rt::frame!(m, "state.<locals>.<lambda>", 79, |_| {
                rt::fstring(m.py(), ("<", rt::format(price, "")?, ">"))
            })
        }
        let value = rt::function!(m, lambda(price), qualname = "state.<locals>.<lambda>")?;
        m.global("store")?
            .attr("items")?
            .set_attr("fmt_price", value)?;
        rt::builtins::print(
            py,
            (
                "app: patched",
                m.global("store")?
                    .attr("items")?
                    .attr("label")?
                    .call1((m.global("Item")?.call1(("fig", 7, 1))?,))?,
            ),
        )?;
        m.global("store")?
            .attr("items")?
            .set_attr("fmt_price", &original)?;
        let reloaded = m
            .global("importlib")?
            .attr("reload")?
            .call1((m.global("config")?,))?;
        rt::builtins::print(
            py,
            (
                "app: reloaded",
                reloaded.is(m.global("config")?),
                m.global("config")?.attr("counter")?,
                m.global("config")?.attr("settings")?,
            ),
        )?;
        Ok(rt::none(py))
    })
}

// app.py:86
pub fn lazy<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "lazy", 86, |_| {
        rt::builtins::print(py, ("app: before lazy import",))?;
        let lazy = rt::import(m, "lazy")?;
        rt::builtins::print(
            py,
            (
                "app: after lazy import",
                lazy.attr("VALUE")?,
                lazy.attr("app_is_main")?,
            ),
        )?;
        Ok(rt::none(py))
    })
}

// app.py:93
pub fn main<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "main", 93, |_| {
        let inventory = build(m)?;
        shop(m, &inventory)?;
        rt::builtins::print(
            py,
            ("app: total", m.global("total_value")?.call1((&inventory,))?),
        )?;
        rt::builtins::print(py, (m.global("render")?.call1((&inventory,))?,))?;
        plugins(m)?;
        state(m)?;
        lazy(m)?;
        let legacy = rt::import(m, "legacy")?;
        rt::builtins::print(
            py,
            (
                "app: legacy",
                legacy.method0("where")?,
                legacy.attr("__loader__")?.attr("name")?,
            ),
        )?;
        // app.py:104
        fn genexpr<'py>(
            module: &rt::Module<'py>,
            modules: &Bound<'py, PyIterator>,
        ) -> PyResult<Option<Bound<'py, PyAny>>> {
            let py = module.py();
            rt::frame!(module, "main.<locals>.<genexpr>", 104, |_| {
                for m in modules {
                    let m = m?;
                    let item = m.method1("split", (".",))?.get_item(0)?;
                    if PySet::new(py, ["store", "plugins", "config", "lazy", "app"])?
                        .contains(&item)?
                    {
                        return Ok(Some(m));
                    }
                    py.check_signals()?;
                }
                Ok(None)
            })
        }
        rt::builtins::print(
            py,
            (
                "app: modules",
                rt::builtin(py, "sorted")?.call1((rt::genexpr!(
                    m,
                    genexpr(m.global("sys")?.attr("modules")?.try_iter()?),
                    qualname = "main.<locals>.<genexpr>"
                )?,))?,
            ),
        )?;
        if m.global("sys")?
            .attr("argv")?
            .get_item(rt::slice(py, 1, py.None())?)?
            .eq(PyList::new(py, ["crash"])?)?
        {
            inventory
                .attr("add")?
                .call1((m.global("Item")?.call1(("bad", 1, -1))?,))?;
            rt::builtins::print(py, (m.global("render")?.call1((&inventory,))?,))?;
        }
        Ok(rt::none(py))
    })
}

/// A subclass of a class from another module.
pub mod discounted {
    use super::*;

    // app.py:33
    /// The body of `class Discounted`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Discounted", 33, |_| {
            let __class__ = ns.class_cell();
            ns.set("__doc__", "A subclass of a class from another module.")?;
            rt::def!(
                ns,
                __init__[&__class__](slf as "self", name, price, quantity, percent)
            )?;
            rt::def!(ns, unit_price[&__class__](slf as "self"))?;
            Ok(())
        })
    }

    // app.py:36
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        __class__: &rt::Cell<'py>,
        slf: &Bound<'py, PyAny>,
        name: &Bound<'py, PyAny>,
        price: &Bound<'py, PyAny>,
        quantity: &Bound<'py, PyAny>,
        percent: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Discounted.__init__", 36, |_| {
            rt::super_(__class__, slf)?.method1("__init__", (name, price, quantity))?;
            slf.set_attr("percent", percent)?;
            Ok(rt::none(m.py()))
        })
    }

    // app.py:40
    pub fn unit_price<'py>(
        m: &rt::Module<'py>,
        __class__: &rt::Cell<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Discounted.unit_price", 40, |_| {
            rt::super_(__class__, slf)?
                .method0("unit_price")?
                .mul(rt::int(m.py(), 100).sub(slf.attr("percent")?)?)?
                .floor_div(100)
        })
    }
}

/// Executes the body of `app.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        m.set_global(
            "__doc__",
            "A multi-module test application (tests/test_projects.py).\n\
                \n\
                python app.py          runs everything and exits normally\n\
                python app.py crash    ends with an exception raised by code kept as\n\
                \x20                      Python in a submodule, through native frames of\n\
                \x20                      other modules\n",
        )?;
        m.set_global("importlib", rt::import(m, "importlib")?)?;
        m.set_global("sys", rt::import(m, "sys")?)?;
        rt::builtins::print(py, ("app: start", m.global("__name__")?))?;
        m.set_global("config", rt::import(m, "config")?)?;
        m.set_global("store", rt::import(m, "store")?)?;
        let store = rt::import_names(m, "store", &["Inventory", "Item", "total_value"])?;
        m.set_global("Inventory", rt::import_from(&store, "Inventory")?)?;
        m.set_global("Item", rt::import_from(&store, "Item")?)?;
        m.set_global("total_value", rt::import_from(&store, "total_value")?)?;
        let errors = rt::import_names(m, "store.errors", &["OutOfStock", "StoreError"])?;
        m.set_global("OutOfStock", rt::import_from(&errors, "OutOfStock")?)?;
        m.set_global("StoreError", rt::import_from(&errors, "StoreError")?)?;
        let reports = rt::import_names(m, "store.reports", &["render"])?;
        m.set_global("render", rt::import_from(&reports, "render")?)?;
        rt::builtins::print(py, ("app: imported store",))?;
        match (|| -> PyResult<()> {
            let package = rt::import_names(m, ".", &["config"])?;
            m.set_global("relative_config", rt::import_from(&package, "config")?)?;
            Ok(())
        })() {
            Ok(()) => {}
            Err(err) if err.is_instance_of::<PyImportError>(py) => {
                let exception = frame.catch(err);
                m.set_global("err", &exception)?;
                rt::builtins::print(py, ("app: relative import:", m.global("err")?))?;
                m.unbind_global("err")?;
            }
            Err(err) => return Err(err),
        }
        match (|| -> PyResult<()> {
            let store = rt::import_names(m, "store", &["nothing"])?;
            m.set_global("nothing", rt::import_from(&store, "nothing")?)?;
            Ok(())
        })() {
            Ok(()) => {}
            Err(err) if err.is_instance_of::<PyImportError>(py) => {
                let exception = frame.catch(err);
                m.set_global("err", &exception)?;
                rt::builtins::print(
                    py,
                    (
                        "app: missing name:",
                        m.global("err")?.str()?.attr("replace")?.call1((
                            m.global("__file__")?
                                .method1("rsplit", ("/", 1))?
                                .get_item(0)?,
                            "<dir>",
                        ))?,
                    ),
                )?;
                m.unbind_global("err")?;
            }
            Err(err) => return Err(err),
        }
        m.set_global(
            "Discounted",
            rt::class!(
                m,
                "Discounted"(m.global("Item")?),
                discounted::body,
                line = 33,
                attributes = ["percent"]
            )?,
        )?;
        rt::def!(m, build())?;
        rt::def!(m, shop(inventory))?;
        rt::def!(m, plugins())?;
        rt::def!(m, state())?;
        rt::def!(m, lazy())?;
        rt::def!(m, main())?;
        if m.global("__name__")?.eq("__main__")? {
            main(m)?;
        }
        Ok(())
    })
}
