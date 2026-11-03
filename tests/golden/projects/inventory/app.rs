use pyo3::exceptions::PyImportError;
use pyo3::prelude::*;
use pyo3::types::PyIterator;
use pyo3::types::PyList;
use pyo3::types::PySet;
use steelix_runtime as rt;

// app.py:44
pub fn build<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "build", 44, |frame| {
        frame.line(45);
        let inventory = m.global("Inventory")?.call0()?;
        frame.line(46);
        inventory
            .getattr("add")?
            .call1((m.global("Item")?.call1(("apple", 3, 10))?,))?;
        frame.line(47);
        inventory
            .getattr("add")?
            .call1((m.global("Discounted")?.call1(("pear", 10, 4, 25))?,))?;
        frame.line(48);
        inventory.getattr("add")?.call1((rt::call!(
            m.global("store")?.getattr("items")?.getattr("Perishable")?,
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
        frame.line(54);
        match inventory.call_method1("take", ("pear", 10)) {
            Ok(_) => {}
            Err(err) if frame.matches(&err, 55, || m.global("OutOfStock"))? => {
                let err = frame.catch(err);
                let _handling = rt::Handling::new(&err);
                frame.line(56);
                rt::builtins::print(
                    py,
                    (
                        "app: caught",
                        err.get_type().getattr("__module__")?,
                        err.get_type().getattr("__qualname__")?,
                        &err,
                        err.getattr("wanted")?,
                    ),
                )?;
            }
            Err(err2) => return Err(err2),
        }
        frame.line(58);
        match inventory.call_method1("take", ("plum", 1)) {
            Ok(_) => {}
            Err(err) if frame.matches(&err, 59, || m.global("StoreError"))? => {
                let err = frame.catch(err);
                let _handling = rt::Handling::new(&err);
                frame.line(60);
                rt::builtins::print(
                    py,
                    (
                        "app: caught base class",
                        err.get_type().getattr("__name__")?,
                        &err,
                    ),
                )?;
            }
            Err(err2) => return Err(err2),
        }
        frame.line(61);
        inventory.call_method1("take", ("apple", 4))?;
        Ok(rt::none(py))
    })
}

// app.py:64
pub fn plugins<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "plugins", 64, |frame| {
        let mut plugin: Bound<'py, PyAny>;
        // A computed name: transpiled only with `--include plugins`.
        frame.line(66);
        for name in PyList::new(py, ["greet", "count"])?.try_iter()? {
            let name = name?;
            frame.line(67);
            plugin = m
                .global("importlib")?
                .getattr("import_module")?
                .call1((rt::str(py, "plugins.").add(&name)?,))?;
            frame.line(68);
            rt::builtins::print(
                py,
                (
                    "app: plugin",
                    plugin.getattr("__name__")?,
                    plugin.call_method0("run")?,
                    plugin.getattr("__loader__")?.getattr("name")?,
                ),
            )?;
            py.check_signals()?;
            frame.line(66);
        }
        frame.line(69);
        let r#static = m
            .global("importlib")?
            .call_method1("import_module", ("plugins.static",))?;
        frame.line(70);
        rt::builtins::print(
            py,
            (
                "app: plugin",
                r#static.getattr("__name__")?,
                r#static.call_method0("run")?,
            ),
        )?;
        Ok(rt::none(py))
    })
}

// app.py:73
pub fn state<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "state", 73, |frame| {
        frame.line(74);
        rt::builtins::print(
            py,
            (
                "app: shared state",
                m.global("config")?.getattr("settings")?,
                m.global("config")?.call_method0("bump")?,
                m.global("config")?.call_method0("bump")?,
            ),
        )?;
        frame.line(75);
        m.global("config")?
            .getattr("settings")?
            .set_item("currency", "USD")?;
        frame.line(76);
        rt::builtins::print(
            py,
            (
                "app: label after change",
                m.global("store")?
                    .getattr("items")?
                    .getattr("label")?
                    .call1((m.global("Item")?.call1(("fig", 7, 1))?,))?,
            ),
        )?;
        // Replacing a function of another module: its direct callers see it.
        frame.line(78);
        let original = m.global("store")?.getattr("items")?.getattr("fmt_price")?;
        // app.py:79
        fn lambda<'py>(
            m: &rt::Module<'py>,
            price: &Bound<'py, PyAny>,
        ) -> PyResult<Bound<'py, PyAny>> {
            rt::frame!(m, "state.<locals>.<lambda>", 79, |_| rt::fstring(
                m.py(),
                ("<", rt::format(price, "")?, ">")
            ))
        }
        frame.line(79);
        let value = rt::function!(m, lambda(price), qualname = "state.<locals>.<lambda>")?;
        m.global("store")?
            .getattr("items")?
            .setattr("fmt_price", value)?;
        frame.line(80);
        rt::builtins::print(
            py,
            (
                "app: patched",
                m.global("store")?
                    .getattr("items")?
                    .getattr("label")?
                    .call1((m.global("Item")?.call1(("fig", 7, 1))?,))?,
            ),
        )?;
        frame.line(81);
        m.global("store")?
            .getattr("items")?
            .setattr("fmt_price", &original)?;
        frame.line(82);
        let reloaded = m
            .global("importlib")?
            .getattr("reload")?
            .call1((m.global("config")?,))?;
        frame.line(83);
        rt::builtins::print(
            py,
            (
                "app: reloaded",
                reloaded.is(m.global("config")?),
                m.global("config")?.getattr("counter")?,
                m.global("config")?.getattr("settings")?,
            ),
        )?;
        Ok(rt::none(py))
    })
}

// app.py:86
pub fn lazy<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "lazy", 86, |frame| {
        frame.line(87);
        rt::builtins::print(py, ("app: before lazy import",))?;
        frame.line(88);
        let lazy = rt::import(m, "lazy")?;
        frame.line(90);
        rt::builtins::print(
            py,
            (
                "app: after lazy import",
                lazy.getattr("VALUE")?,
                lazy.getattr("app_is_main")?,
            ),
        )?;
        Ok(rt::none(py))
    })
}

// app.py:93
pub fn main<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "main", 93, |frame| {
        frame.line(94);
        let inventory = build(m)?;
        frame.line(95);
        shop(m, &inventory)?;
        frame.line(96);
        rt::builtins::print(
            py,
            ("app: total", m.global("total_value")?.call1((&inventory,))?),
        )?;
        frame.line(97);
        rt::builtins::print(py, (m.global("render")?.call1((&inventory,))?,))?;
        frame.line(98);
        plugins(m)?;
        frame.line(99);
        state(m)?;
        frame.line(100);
        lazy(m)?;
        frame.line(101);
        let legacy = rt::import(m, "legacy")?;
        frame.line(103);
        rt::builtins::print(
            py,
            (
                "app: legacy",
                legacy.call_method0("where")?,
                legacy.getattr("__loader__")?.getattr("name")?,
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
                    let item = m.call_method1("split", (".",))?.get_item(0)?;
                    if PySet::new(py, ["store", "plugins", "config", "lazy", "app"])?
                        .contains(&item)?
                    {
                        return Ok(Some(m.clone()));
                    }
                    py.check_signals()?;
                }
                Ok(None)
            })
        }
        frame.line(104);
        rt::builtins::print(
            py,
            (
                "app: modules",
                rt::builtin(py, "sorted")?.call1((rt::genexpr!(
                    m,
                    genexpr(m.global("sys")?.getattr("modules")?.try_iter()?),
                    qualname = "main.<locals>.<genexpr>"
                )?,))?,
            ),
        )?;
        frame.line(105);
        if m.global("sys")?
            .getattr("argv")?
            .get_item(rt::slice(py, 1, py.None())?)?
            .eq(PyList::new(py, ["crash"])?)?
        {
            frame.line(106);
            inventory
                .getattr("add")?
                .call1((m.global("Item")?.call1(("bad", 1, -1))?,))?;
            frame.line(107);
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
        rt::frame!(ns, "Discounted", 33, |frame| {
            let __class__ = ns.class_cell();
            frame.line(34);
            ns.set("__doc__", "A subclass of a class from another module.")?;
            frame.line(36);
            ns.set(
                "__init__",
                rt::function!(
                    ns,
                    __init__[&__class__](slf as "self", name, price, quantity, percent),
                    qualname = "Discounted.__init__"
                )?,
            )?;
            frame.line(40);
            ns.set(
                "unit_price",
                rt::function!(
                    ns,
                    unit_price[&__class__](slf as "self"),
                    qualname = "Discounted.unit_price"
                )?,
            )?;
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
        rt::frame!(m, "Discounted.__init__", 36, |frame| {
            frame.line(37);
            rt::super_(__class__, slf)?.call_method1("__init__", (name, price, quantity))?;
            frame.line(38);
            slf.setattr("percent", percent)?;
            Ok(rt::none(m.py()))
        })
    }

    // app.py:40
    pub fn unit_price<'py>(
        m: &rt::Module<'py>,
        __class__: &rt::Cell<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Discounted.unit_price", 40, |frame| {
            frame.line(41);
            rt::super_(__class__, slf)?
                .call_method0("unit_price")?
                .mul(rt::int(m.py(), 100).sub(slf.getattr("percent")?)?)?
                .floor_div(100)
        })
    }
}

/// Executes the body of `app.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        m.set_global("__doc__", "A multi-module test application (tests/test_projects.py).\n\npython app.py          runs everything and exits normally\npython app.py crash    ends with an exception raised by code kept as\n                       Python in a submodule, through native frames of\n                       other modules\n")?;
        frame.line(9);
        m.set_global("importlib", rt::import(m, "importlib")?)?;
        frame.line(10);
        m.set_global("sys", rt::import(m, "sys")?)?;
        frame.line(12);
        rt::builtins::print(py, ("app: start", m.global("__name__")?))?;
        frame.line(14);
        m.set_global("config", rt::import(m, "config")?)?;
        frame.line(15);
        m.set_global("store", rt::import(m, "store")?)?;
        frame.line(16);
        let store = rt::import_names(m, "store", &["Inventory", "Item", "total_value"])?;
        m.set_global("Inventory", rt::import_from(&store, "Inventory")?)?;
        m.set_global("Item", rt::import_from(&store, "Item")?)?;
        m.set_global("total_value", rt::import_from(&store, "total_value")?)?;
        frame.line(17);
        let errors = rt::import_names(m, "store.errors", &["OutOfStock", "StoreError"])?;
        m.set_global("OutOfStock", rt::import_from(&errors, "OutOfStock")?)?;
        m.set_global("StoreError", rt::import_from(&errors, "StoreError")?)?;
        frame.line(18);
        let reports = rt::import_names(m, "store.reports", &["render"])?;
        m.set_global("render", rt::import_from(&reports, "render")?)?;
        frame.line(20);
        rt::builtins::print(py, ("app: imported store",))?;
        match (|| -> PyResult<()> {
            frame.line(23);
            let package = rt::import_names(m, ".", &["config"])?;
            m.set_global("relative_config", rt::import_from(&package, "config")?)?;
            Ok(())
        })() {
            Ok(()) => {}
            Err(err) if err.is_instance_of::<PyImportError>(py) => {
                let exception = frame.catch(err);
                m.set_global("err", &exception)?;
                let _handling = rt::Handling::new(&exception);
                frame.line(25);
                rt::builtins::print(py, ("app: relative import:", m.global("err")?))?;
                m.unbind_global("err")?;
            }
            Err(err) => return Err(err),
        }
        match (|| -> PyResult<()> {
            frame.line(28);
            let store = rt::import_names(m, "store", &["nothing"])?;
            m.set_global("nothing", rt::import_from(&store, "nothing")?)?;
            Ok(())
        })() {
            Ok(()) => {}
            Err(err) if err.is_instance_of::<PyImportError>(py) => {
                let exception = frame.catch(err);
                m.set_global("err", &exception)?;
                let _handling = rt::Handling::new(&exception);
                frame.line(30);
                rt::builtins::print(
                    py,
                    (
                        "app: missing name:",
                        m.global("err")?.str()?.getattr("replace")?.call1((
                            m.global("__file__")?
                                .call_method1("rsplit", ("/", 1))?
                                .get_item(0)?,
                            "<dir>",
                        ))?,
                    ),
                )?;
                m.unbind_global("err")?;
            }
            Err(err) => return Err(err),
        }
        frame.line(33);
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
        frame.line(110);
        if m.global("__name__")?.eq("__main__")? {
            frame.line(111);
            main(m)?;
        }
        Ok(())
    })
}
