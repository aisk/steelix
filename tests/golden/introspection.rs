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
    rt::frame!(m, "area", 8, |frame| {
        frame.line(10);
        width.mul(height)
    })
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
    rt::frame!(m, "<module>", 1, |frame| {
        m.set_global(
            "__doc__",
            "Function attributes used by the standard library: signature, pickle, annotations.",
        )?;
        frame.line(3);
        m.set_global("annotationlib", rt::import(m, "annotationlib")?)?;
        frame.line(4);
        m.set_global("inspect", rt::import(m, "inspect")?)?;
        frame.line(5);
        m.set_global("pickle", rt::import(m, "pickle")?)?;
        rt::def!(
            m,
            area(width: "float", height: "'Length'" = 2.0, /, *args: "int", unit: "str" = "m", **kw) -> "Shape",
            doc = "Compute an area."
        )?;
        rt::def!(m, outer())?;
        frame.line(21);
        m.set_global("nested", m.global("outer")?.call0()?)?;
        frame.line(22);
        m.global("print")?.call1((
            m.global("inspect")?
                .getattr("signature")?
                .call1((m.global("nested")?,))?,
            m.global("nested")?.getattr("__doc__")?,
            m.global("nested")?.getattr("__qualname__")?,
        ))?;
        frame.line(23);
        m.global("print")?.call1((
            m.global("area")?.getattr("__doc__")?,
            m.global("area")?.getattr("__module__")?,
            m.global("area")?
                .getattr("__globals__")?
                .is(m.globals().clone().into_any()),
        ))?;
        frame.line(24);
        m.global("print")?.call1((
            m.global("area")?.getattr("__defaults__")?,
            m.global("area")?.getattr("__kwdefaults__")?,
        ))?;
        frame.line(25);
        m.global("print")?.call1((m
            .global("pickle")?
            .getattr("loads")?
            .call1((m
                .global("pickle")?
                .getattr("dumps")?
                .call1((m.global("area")?,))?,))?
            .is(m.global("area")?),))?;
        // Annotations are evaluated lazily, so `Shape` may be defined after `area`.
        frame.line(27);
        m.global("print")?.call1((rt::call!(
            m.global("annotationlib")?.getattr("get_annotations")?,
            m.global("area")?,
            format = m
                .global("annotationlib")?
                .getattr("Format")?
                .getattr("STRING")?
        )?,))?;
        frame.line(28);
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
        frame.line(29);
        m.set_global("Shape", "a shape")?;
        frame.line(30);
        m.set_global("Length", m.global("float")?)?;
        frame.line(31);
        m.global("print")?
            .call1((m.global("area")?.getattr("__annotations__")?,))?;
        frame.line(32);
        m.global("print")?.call1((m
            .global("inspect")?
            .getattr("signature")?
            .call1((m.global("area")?,))?,))?;
        frame.line(33);
        m.global("print")?.call1((
            m.global("area")?.call1((3.0,))?,
            m.global("callable")?.call1((m.global("area")?,))?,
            m.global("area")?.getattr("__name__")?,
        ))?;
        frame.line(34);
        m.global("area")?.setattr("__name__", "surface")?;
        frame.line(35);
        m.global("print")?.call1((
            m.global("area")?.getattr("__name__")?,
            m.global("repr")?
                .call1((m.global("area")?,))?
                .call_method1("startswith", ("<function area at ",))?,
        ))?;
        frame.line(36);
        m.global("area")?.setattr("extra", 1)?;
        frame.line(37);
        m.global("print")?
            .call1((m.global("area")?.getattr("__dict__")?,))?;
        frame.line(38);
        m.global("print")?
            .call1((m.global("outer")?.getattr("__annotations__")?,))?;
        Ok(())
    })
}
