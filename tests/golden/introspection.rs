use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3::types::PyTuple;
use steelix_runtime as rt;

// introspection.py:8
/// Compute an area.
pub fn area<'py>(
    py: Python<'py>,
    width: &Bound<'py, PyAny>,
    height: &Bound<'py, PyAny>,
    args: &Bound<'py, PyTuple>,
    unit: &Bound<'py, PyAny>,
    kw: &Bound<'py, PyDict>,
) -> PyResult<Bound<'py, PyAny>> {
    width.mul(height)
}

// introspection.py:13
pub fn outer<'py>(py: Python<'py>, globals: &Bound<'py, PyDict>) -> PyResult<Bound<'py, PyAny>> {
    // introspection.py:14
    /// Nested docstring.
    fn nested<'py>(py: Python<'py>, x: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        Ok(x.clone())
    }
    let nested = rt::function!(
        globals,
        nested(x),
        qualname = "outer.<locals>.nested",
        doc = "Nested docstring."
    )?;
    Ok(nested)
}

/// Executes the body of `introspection.py` in the module `m`.
pub fn exec_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let globals = &m.dict();
    globals.set_item(
        "__doc__",
        "Function attributes used by the standard library: signature, pickle, annotations.",
    )?;
    globals.set_item("annotationlib", rt::import(globals, "annotationlib")?)?;
    globals.set_item("inspect", rt::import(globals, "inspect")?)?;
    globals.set_item("pickle", rt::import(globals, "pickle")?)?;
    rt::def!(
        globals,
        area(width: "float", height: "'Length'" = 2.0, /, *args: "int", unit: "str" = "m", **kw) -> "Shape",
        doc = "Compute an area."
    )?;
    rt::def!(globals, outer(globals))?;
    globals.set_item("nested", rt::global(globals, "outer")?.call0()?)?;
    rt::global(globals, "print")?.call1((
        rt::global(globals, "inspect")?
            .getattr("signature")?
            .call1((rt::global(globals, "nested")?,))?,
        rt::global(globals, "nested")?.getattr("__doc__")?,
        rt::global(globals, "nested")?.getattr("__qualname__")?,
    ))?;
    rt::global(globals, "print")?.call1((
        rt::global(globals, "area")?.getattr("__doc__")?,
        rt::global(globals, "area")?.getattr("__module__")?,
        rt::global(globals, "area")?
            .getattr("__globals__")?
            .is(globals.clone().into_any()),
    ))?;
    rt::global(globals, "print")?.call1((
        rt::global(globals, "area")?.getattr("__defaults__")?,
        rt::global(globals, "area")?.getattr("__kwdefaults__")?,
    ))?;
    rt::global(globals, "print")?.call1((rt::global(globals, "pickle")?
        .getattr("loads")?
        .call1((rt::global(globals, "pickle")?
            .getattr("dumps")?
            .call1((rt::global(globals, "area")?,))?,))?
        .is(rt::global(globals, "area")?),))?;
    // Annotations are evaluated lazily, so `Shape` may be defined after `area`.
    rt::global(globals, "print")?.call1((rt::call!(
        rt::global(globals, "annotationlib")?.getattr("get_annotations")?,
        rt::global(globals, "area")?,
        format = rt::global(globals, "annotationlib")?
            .getattr("Format")?
            .getattr("STRING")?
    )?,))?;
    rt::global(globals, "print")?.call1((rt::call!(
        rt::global(globals, "annotationlib")?.getattr("get_annotations")?,
        rt::global(globals, "area")?,
        format = rt::global(globals, "annotationlib")?
            .getattr("Format")?
            .getattr("FORWARDREF")?
    )?
    .get_item("return")?
    .getattr("__forward_arg__")?,))?;
    globals.set_item("Shape", "a shape")?;
    globals.set_item("Length", rt::global(globals, "float")?)?;
    rt::global(globals, "print")?
        .call1((rt::global(globals, "area")?.getattr("__annotations__")?,))?;
    rt::global(globals, "print")?.call1((rt::global(globals, "inspect")?
        .getattr("signature")?
        .call1((rt::global(globals, "area")?,))?,))?;
    rt::global(globals, "print")?.call1((
        rt::global(globals, "area")?.call1((3.0,))?,
        rt::global(globals, "callable")?.call1((rt::global(globals, "area")?,))?,
        rt::global(globals, "area")?.getattr("__name__")?,
    ))?;
    rt::global(globals, "area")?.setattr("__name__", "surface")?;
    rt::global(globals, "print")?.call1((
        rt::global(globals, "area")?.getattr("__name__")?,
        rt::global(globals, "repr")?
            .call1((rt::global(globals, "area")?,))?
            .call_method1("startswith", ("<function area at ",))?,
    ))?;
    rt::global(globals, "area")?.setattr("extra", 1)?;
    rt::global(globals, "print")?.call1((rt::global(globals, "area")?.getattr("__dict__")?,))?;
    rt::global(globals, "print")?
        .call1((rt::global(globals, "outer")?.getattr("__annotations__")?,))?;
    Ok(())
}
