pub mod text;

use pyo3::prelude::*;
use steelix_runtime as rt;

/// Executes the body of `store/reports/__init__.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    rt::frame!(m, "<module>", 1, |_| {
        let text = rt::import_names(m, ".text", &["render"])?;
        m.set_global("render", rt::import_from(&text, "render")?)?;
        let items = rt::import_names(m, "..items", &["Item"])?;
        m.set_global("Item", rt::import_from(&items, "Item")?)?;
        rt::builtins::print(
            m.py(),
            (
                "store.reports: imported",
                m.global("Item")?.getattr("__module__")?,
            ),
        )?;
        Ok(())
    })
}
