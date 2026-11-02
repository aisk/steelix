use pyo3::prelude::*;
use pyo3::types::PyDict;
use steelix_runtime as rt;

// control_flow.py:7
/// Count the steps until the Collatz sequence reaches 1.
pub fn collatz_steps<'py>(py: Python<'py>, n: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let mut n = n.clone();
    let mut steps = rt::int(py, 0);
    while n.ne(1)? {
        if n.rem(2)?.eq(0)? {
            n = n.floor_div(2)?;
        } else {
            n = rt::int(py, 3).mul(&n)?.add(1)?;
        }
        steps = steps.add(1)?;
    }
    Ok(steps)
}

// control_flow.py:19
pub fn classify<'py>(
    py: Python<'py>,
    globals: &Bound<'py, PyDict>,
    n: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let kind: Bound<'py, PyAny>;
    if n.lt(0)? {
        kind = rt::str(py, "negative");
    } else if n.eq(0)? {
        kind = rt::str(py, "zero");
    } else if n.lt(rt::global(globals, "LIMIT")?)? {
        kind = rt::str(py, "small");
    } else {
        kind = rt::str(py, "large");
    }
    Ok(kind)
}

// control_flow.py:31
pub fn count_up<'py>(
    py: Python<'py>,
    globals: &Bound<'py, PyDict>,
    start: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let mut i = start.clone();
    let mut total = rt::int(py, 0);
    while i.lt(rt::global(globals, "LIMIT")?)? {
        total = total.add(&i)?;
        i = i.add(rt::global(globals, "STEP")?)?;
    }
    Ok(total)
}

// control_flow.py:40
pub fn first_square_above<'py>(
    py: Python<'py>,
    bound: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let mut k = rt::int(py, 0);
    loop {
        k = k.add(1)?;
        if k.mul(&k)?.gt(bound)? {
            return Ok(k);
        }
    }
}

/// Executes the body of `control_flow.py` in the module `m`.
pub fn exec_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let globals = &m.dict();
    globals.set_item("__doc__", "Loops and branches over module level state.")?;
    globals.set_item("LIMIT", 20)?;
    globals.set_item("STEP", 3)?;
    rt::def!(
        globals,
        collatz_steps(n),
        doc = "Count the steps until the Collatz sequence reaches 1."
    )?;
    rt::def!(globals, classify(globals, n))?;
    rt::def!(globals, count_up(globals, start))?;
    rt::def!(globals, first_square_above(bound))?;
    rt::builtins::print(py, (collatz_steps(py, &rt::int(py, 27))?,))?;
    rt::builtins::print(
        py,
        (
            classify(py, globals, &rt::int(py, -5))?,
            classify(py, globals, &rt::int(py, 0))?,
            classify(py, globals, &rt::int(py, 7))?,
            classify(py, globals, &rt::int(py, 100))?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            count_up(py, globals, &rt::int(py, 1))?,
            count_up(py, globals, &rt::global(globals, "LIMIT")?)?,
        ),
    )?;
    rt::builtins::print(py, (first_square_above(py, &rt::int(py, 50))?,))?;
    globals.set_item("n", 10)?;
    while rt::global(globals, "n")?.gt(0)? {
        if rt::global(globals, "n")?.rem(4)?.eq(0)? {
            rt::builtins::print(py, ("four", rt::global(globals, "n")?))?;
        } else if rt::global(globals, "n")?.rem(3)?.eq(0)? {
            rt::builtins::print(py, ("three", rt::global(globals, "n")?))?;
        }
        globals.set_item(
            "n",
            rt::global(globals, "n")?.sub(rt::global(globals, "STEP")?)?,
        )?;
    }
    rt::builtins::print(py, (rt::global(globals, "n")?,))?;
    Ok(())
}
