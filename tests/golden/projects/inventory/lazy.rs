use pyo3::prelude::*;
use pyo3::types::PyTuple;
use steelix_runtime as rt;

/// Executes the body of `lazy.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        rt::builtins::print(py, ("lazy: imported",))?;
        frame.line(3);
        m.set_global("sys", rt::import(m, "sys")?)?;
        frame.line(5);
        m.set_global("app", rt::import(m, "app")?)?;
        frame.line(7);
        m.set_global("VALUE", 42)?;
        frame.line(8);
        m.set_global(
            "app_is_main",
            PyTuple::new(
                py,
                [
                    &rt::bool(
                        py,
                        m.global("app")?
                            .is(m.global("sys")?.getattr("modules")?.get_item("__main__")?),
                    ),
                    &m.global("app")?.getattr("__name__")?,
                    &rt::bool(
                        py,
                        m.global("app")?.getattr("Discounted")?.is(m
                            .global("sys")?
                            .getattr("modules")?
                            .get_item("__main__")?
                            .getattr("Discounted")?),
                    ),
                ],
            )?,
        )?;
        Ok(())
    })
}
