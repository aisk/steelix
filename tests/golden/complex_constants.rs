use pyo3::prelude::*;
use pyo3::pyclass::CompareOp;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// complex_constants.py:1
#[rt::frame(m, "parts", 1)]
pub fn parts<'py>(m: &rt::Module<'py>, z: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    Ok(PyTuple::new(m.py(), [&z.attr("real")?, &z.attr("imag")?])?.into_any())
}

// complex_constants.py:5
#[rt::frame(m, "scale", 5)]
pub fn scale<'py>(
    m: &rt::Module<'py>,
    z: &Bound<'py, PyAny>,
    factor: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    z.mul(factor)
}

/// Executes the body of `complex_constants.py` in the module `m`.
#[rt::frame(m, "<module>", 1)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::def!(m, parts(z))?;
    rt::def!(m, scale(z, factor = rt::complex(py, 0.0, 2.0)))?;
    rt::builtins::print(
        py,
        (
            rt::complex(py, 0.0, 1.0),
            rt::complex(py, 0.0, 2.5),
            rt::complex(py, 0.0, 0.0),
            rt::complex(py, 0.0, f64::INFINITY),
            rt::complex(py, 0.0, 3.0).neg()?,
            rt::complex(py, 0.0, 1.0).get_type().attr("__name__")?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            rt::int(py, 1).add(rt::complex(py, 0.0, 2.0))?,
            rt::int(py, 1)
                .add(rt::complex(py, 0.0, 2.0))?
                .mul(rt::complex(py, 0.0, 1.0))?,
            rt::int(py, 3).add(rt::complex(py, 0.0, 4.0))?.abs()?,
            parts(m, &rt::complex(py, 0.0, 1.5))?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            scale(m, &rt::int(py, 1), &rt::complex(py, 0.0, 2.0))?,
            scale(m, &rt::complex(py, 0.0, 1.0), &rt::complex(py, 0.0, 2.0))?,
            scale(m, &rt::int(py, 1), &rt::complex(py, 0.0, 0.5))?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            rt::complex(py, 0.0, 1.0)
                .rich_compare(rt::builtin(py, "complex")?.call1((0, 1))?, CompareOp::Eq)?,
            "falsy",
            "truthy",
        ),
    )?;
    m.set_global("x", rt::complex(py, 0.0, 4.0))?;
    rt::builtins::print(
        py,
        (
            m.global("x")?.method0("conjugate")?,
            m.global("x")?.pow(2, py.None())?,
            m.global("x")?.neg()?,
        ),
    )?;
    Ok(())
}
