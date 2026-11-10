use pyo3::prelude::*;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

/// Executes the body of `lazy.py` in the module `m`.
#[rt::frame(m, "<module>", 1)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::builtins::print(py, ("lazy: imported",))?;
    m.set_global("sys", rt::import(m, "sys")?)?;
    m.set_global("app", rt::import(m, "app")?)?;
    m.set_global("VALUE", 42)?;
    m.set_global(
        "app_is_main",
        PyTuple::new(
            py,
            [
                &rt::bool(
                    py,
                    m.global("app")?
                        .is(m.global("sys")?.attr("modules")?.get_item("__main__")?),
                ),
                &m.global("app")?.attr("__name__")?,
                &rt::bool(
                    py,
                    m.global("app")?.attr("Discounted")?.is(m
                        .global("sys")?
                        .attr("modules")?
                        .get_item("__main__")?
                        .attr("Discounted")?),
                ),
            ],
        )?,
    )?;
    Ok(())
}
