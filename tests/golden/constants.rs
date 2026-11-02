use pyo3::prelude::*;
use pyo3::pyclass::CompareOp;
use steelix_runtime as rt;

// constants.py:1
pub fn describe<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::builtins::print(
        py,
        (
            0,
            7,
            -7,
            2147483648i64,
            -9223372036854775808i64,
            rt::int(py, 2).pow(64, py.None())?,
            rt::bigint(py, "123456789012345678901234567890123456789012")?,
        ),
    )?;
    rt::builtins::print(py, (1.5, -0.0, 1e+100, 2.5e-10, f64::INFINITY))?;
    rt::builtins::print(
        py,
        (
            "plain",
            "quote \" and \\ backslash",
            "tab\tnewline\n",
            "unicode é 中文 \u{200b}",
        ),
    )?;
    rt::builtins::print(py, (rt::bytes(py, b"bytes\0\xff"), true, false, py.None()))?;
    rt::builtins::print(
        py,
        (
            rt::int(py, 7).div(2)?,
            rt::int(py, 7).floor_div(2)?,
            rt::int(py, -7).rem(3)?,
            rt::int(py, 2).pow(10, py.None())?,
            rt::int(py, 1).lshift(5)?,
            rt::int(py, 255).rshift(2)?,
            rt::int(py, 6).bitand(3)?,
            rt::int(py, 6).bitor(3)?,
            rt::int(py, 6).bitxor(3)?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            rt::str(py, "ab").add("cd")?,
            rt::str(py, "ab").mul(3)?,
            rt::str(py, "%d items").rem(5)?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            rt::int(py, 3).rich_compare(4, CompareOp::Lt)?,
            rt::int(py, 3).rich_compare(2, CompareOp::Le)?,
            rt::int(py, 3).rich_compare(4, CompareOp::Gt)?,
            rt::int(py, 3).rich_compare(3, CompareOp::Ge)?,
            rt::int(py, 3).rich_compare(3.0, CompareOp::Eq)?,
            rt::str(py, "a").rich_compare("b", CompareOp::Ne)?,
        ),
    )?;
    rt::builtins::print(py, (rt::int(py, 1).add(2.5)?, rt::int(py, 10).sub(true)?))?;
    rt::builtins::print(
        py,
        (
            rt::str_surrogatepass(py, b"a\xed\xa0\x80b")?.len()?,
            rt::bytes(py, b"").len()?,
            rt::int(py, -3).abs()?,
        ),
    )?;
    Ok(rt::none(py))
}

/// Executes the body of `constants.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::def!(m, describe())?;
    m.set_global("x", rt::int(py, 1).add(2)?)?;
    let value = m.global("x")?.mul(2)?;
    m.set_global("y", &value)?;
    m.set_global("z", &value)?;
    rt::builtins::print(py, (m.global("x")?, m.global("y")?, m.global("z")?))?;
    rt::builtins::print(py, (describe(m)?,))?;
    Ok(())
}
