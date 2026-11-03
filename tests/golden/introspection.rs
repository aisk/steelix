use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3::types::PyTuple;
use steelix_runtime as rt;

// introspection.py:8
/// Compute an area.
pub fn area<'py>(
    m: &rt::Module<'py>,
    width: &Bound<'py, PyAny>,
    height: &Bound<'py, PyAny>,
    args: &Bound<'py, PyTuple>,
    unit: &Bound<'py, PyAny>,
    kw: &Bound<'py, PyDict>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "area", 8, |_| { width.mul(height) })
}

// introspection.py:13
pub fn outer<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "outer", 13, |_| {
        // introspection.py:14
        /// Nested docstring.
        fn nested<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
            Ok(x.clone())
        }
        let nested = rt::function!(
            m,
            nested(x),
            qualname = "outer.<locals>.nested",
            doc = "Nested docstring."
        )?;
        Ok(nested)
    })
}

/// Executes the body of `introspection.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    rt::frame!(m, "<module>", 1, |_| {
        m.set_global(
            "__doc__",
            "Function attributes used by the standard library: signature, pickle, annotations.",
        )?;
        m.set_global("annotationlib", rt::import(m, "annotationlib")?)?;
        m.set_global("inspect", rt::import(m, "inspect")?)?;
        m.set_global("pickle", rt::import(m, "pickle")?)?;
        rt::def!(
            m,
            area(width: "float", height: "'Length'" = 2.0, /, *args: "int", unit: "str" = "m", **kw) -> "Shape",
            doc = "Compute an area."
        )?;
        rt::def!(m, outer())?;
        m.set_global("nested", m.global("outer")?.call0()?)?;
        m.global("print")?.call1((
            m.global("inspect")?
                .getattr("signature")?
                .call1((m.global("nested")?,))?,
            m.global("nested")?.getattr("__doc__")?,
            m.global("nested")?.getattr("__qualname__")?,
        ))?;
        m.global("print")?.call1((
            m.global("area")?.getattr("__doc__")?,
            m.global("area")?.getattr("__module__")?,
            m.global("area")?
                .getattr("__globals__")?
                .is(m.globals().clone().into_any()),
        ))?;
        m.global("print")?.call1((
            m.global("area")?.getattr("__defaults__")?,
            m.global("area")?.getattr("__kwdefaults__")?,
        ))?;
        m.global("print")?.call1((m
            .global("pickle")?
            .getattr("loads")?
            .call1((m
                .global("pickle")?
                .getattr("dumps")?
                .call1((m.global("area")?,))?,))?
            .is(m.global("area")?),))?;
        // Annotations are evaluated lazily, so `Shape` may be defined after `area`.
        m.global("print")?.call1((rt::call!(
            m.global("annotationlib")?.getattr("get_annotations")?,
            m.global("area")?,
            format = m
                .global("annotationlib")?
                .getattr("Format")?
                .getattr("STRING")?
        )?,))?;
        m.global("print")?.call1((rt::call!(
            m.global("annotationlib")?.getattr("get_annotations")?,
            m.global("area")?,
            format = m
                .global("annotationlib")?
                .getattr("Format")?
                .getattr("FORWARDREF")?
        )?
        .get_item("return")?
        .getattr("__forward_arg__")?,))?;
        m.set_global("Shape", "a shape")?;
        m.set_global("Length", m.global("float")?)?;
        m.global("print")?
            .call1((m.global("area")?.getattr("__annotations__")?,))?;
        m.global("print")?.call1((m
            .global("inspect")?
            .getattr("signature")?
            .call1((m.global("area")?,))?,))?;
        m.global("print")?.call1((
            m.global("area")?.call1((3.0,))?,
            m.global("callable")?.call1((m.global("area")?,))?,
            m.global("area")?.getattr("__name__")?,
        ))?;
        m.global("area")?.setattr("__name__", "surface")?;
        m.global("print")?.call1((
            m.global("area")?.getattr("__name__")?,
            m.global("repr")?
                .call1((m.global("area")?,))?
                .call_method1("startswith", ("<function area at ",))?,
        ))?;
        m.global("area")?.setattr("extra", 1)?;
        m.global("print")?
            .call1((m.global("area")?.getattr("__dict__")?,))?;
        m.global("print")?
            .call1((m.global("outer")?.getattr("__annotations__")?,))?;
        Ok(())
    })
}
