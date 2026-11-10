use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// introspection.py:8
/// Compute an area.
#[rt::frame(m, "area", 8)]
pub fn area<'py>(
    m: &rt::Module<'py>,
    width: &Bound<'py, PyAny>,
    height: &Bound<'py, PyAny>,
    args: &Bound<'py, PyTuple>,
    unit: &Bound<'py, PyAny>,
    kw: &Bound<'py, PyDict>,
) -> PyResult<Bound<'py, PyAny>> {
    width.mul(height)
}

// introspection.py:13
#[rt::frame(m, "outer", 13, frame)]
pub fn outer<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    // introspection.py:14
    /// Nested docstring.
    fn nested<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        Ok(x.clone())
    }
    let nested = rt::function!(frame, nested(x), doc = "Nested docstring.")?;
    Ok(nested)
}

/// Executes the body of `introspection.py` in the module `m`.
#[rt::frame(m, "<module>", 1)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
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
            .attr("signature")?
            .call1((m.global("nested")?,))?,
        m.global("nested")?.attr("__doc__")?,
        m.global("nested")?.attr("__qualname__")?,
    ))?;
    m.global("print")?.call1((
        m.global("area")?.attr("__doc__")?,
        m.global("area")?.attr("__module__")?,
        m.global("area")?.attr("__globals__")?.is(m.globals()),
    ))?;
    m.global("print")?.call1((
        m.global("area")?.attr("__defaults__")?,
        m.global("area")?.attr("__kwdefaults__")?,
    ))?;
    m.global("print")?.call1((m
        .global("pickle")?
        .attr("loads")?
        .call1((m
            .global("pickle")?
            .attr("dumps")?
            .call1((m.global("area")?,))?,))?
        .is(m.global("area")?),))?;
    // Annotations are evaluated lazily, so `Shape` may be defined after `area`.
    m.global("print")?.call1((rt::call!(
        m.global("annotationlib")?.attr("get_annotations")?,
        m.global("area")?,
        format = m.global("annotationlib")?.attr("Format")?.attr("STRING")?
    )?,))?;
    m.global("print")?.call1((rt::call!(
        m.global("annotationlib")?.attr("get_annotations")?,
        m.global("area")?,
        format = m
            .global("annotationlib")?
            .attr("Format")?
            .attr("FORWARDREF")?
    )?
    .get_item("return")?
    .attr("__forward_arg__")?,))?;
    m.set_global("Shape", "a shape")?;
    m.set_global("Length", m.global("float")?)?;
    m.global("print")?
        .call1((m.global("area")?.attr("__annotations__")?,))?;
    m.global("print")?.call1((m
        .global("inspect")?
        .attr("signature")?
        .call1((m.global("area")?,))?,))?;
    m.global("print")?.call1((
        m.global("area")?.call1((3.0,))?,
        m.global("callable")?.call1((m.global("area")?,))?,
        m.global("area")?.attr("__name__")?,
    ))?;
    m.global("area")?.set_attr("__name__", "surface")?;
    m.global("print")?.call1((
        m.global("area")?.attr("__name__")?,
        m.global("repr")?
            .call1((m.global("area")?,))?
            .method1("startswith", ("<function area at ",))?,
    ))?;
    m.global("area")?.set_attr("extra", 1)?;
    m.global("print")?
        .call1((m.global("area")?.attr("__dict__")?,))?;
    m.global("print")?
        .call1((m.global("outer")?.attr("__annotations__")?,))?;
    Ok(())
}
