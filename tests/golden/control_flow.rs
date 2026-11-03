use pyo3::prelude::*;
use steelix_runtime as rt;

// control_flow.py:7
/// Count the steps until the Collatz sequence reaches 1.
pub fn collatz_steps<'py>(
    m: &rt::Module<'py>,
    n: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "collatz_steps", 7, |_| {
        let mut n = n.clone();
        let mut steps = rt::int(py, 0);
        while n.ne(1)? {
            if n.rem(2)?.eq(0)? {
                n = n.floor_div(2)?;
            } else {
                n = rt::int(py, 3).mul(&n)?.add(1)?;
            }
            steps = steps.add(1)?;
            py.check_signals()?;
        }
        Ok(steps)
    })
}

// control_flow.py:19
pub fn classify<'py>(m: &rt::Module<'py>, n: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "classify", 19, |_| {
        let kind = if n.lt(0)? {
            rt::str(py, "negative")
        } else if n.eq(0)? {
            rt::str(py, "zero")
        } else if n.lt(m.global("LIMIT")?)? {
            rt::str(py, "small")
        } else {
            rt::str(py, "large")
        };
        Ok(kind)
    })
}

// control_flow.py:31
pub fn count_up<'py>(
    m: &rt::Module<'py>,
    start: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "count_up", 31, |_| {
        let mut i = start.clone();
        let mut total = rt::int(py, 0);
        while i.lt(m.global("LIMIT")?)? {
            total = total.add(&i)?;
            i = i.add(m.global("STEP")?)?;
            py.check_signals()?;
        }
        Ok(total)
    })
}

// control_flow.py:40
pub fn first_square_above<'py>(
    m: &rt::Module<'py>,
    bound: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "first_square_above", 40, |_| {
        let mut k = rt::int(py, 0);
        loop {
            k = k.add(1)?;
            if k.mul(&k)?.gt(bound)? {
                return Ok(k);
            }
            py.check_signals()?;
        }
    })
}

/// Executes the body of `control_flow.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |_| {
        m.set_global("__doc__", "Loops and branches over module level state.")?;
        m.set_global("LIMIT", 20)?;
        m.set_global("STEP", 3)?;
        rt::def!(
            m,
            collatz_steps(n),
            doc = "Count the steps until the Collatz sequence reaches 1."
        )?;
        rt::def!(m, classify(n))?;
        rt::def!(m, count_up(start))?;
        rt::def!(m, first_square_above(bound))?;
        rt::builtins::print(py, (collatz_steps(m, &rt::int(py, 27))?,))?;
        rt::builtins::print(
            py,
            (
                classify(m, &rt::int(py, -5))?,
                classify(m, &rt::int(py, 0))?,
                classify(m, &rt::int(py, 7))?,
                classify(m, &rt::int(py, 100))?,
            ),
        )?;
        rt::builtins::print(
            py,
            (
                count_up(m, &rt::int(py, 1))?,
                count_up(m, &m.global("LIMIT")?)?,
            ),
        )?;
        rt::builtins::print(py, (first_square_above(m, &rt::int(py, 50))?,))?;
        m.set_global("n", 10)?;
        while m.global("n")?.gt(0)? {
            if m.global("n")?.rem(4)?.eq(0)? {
                rt::builtins::print(py, ("four", m.global("n")?))?;
            } else if m.global("n")?.rem(3)?.eq(0)? {
                rt::builtins::print(py, ("three", m.global("n")?))?;
            }
            m.set_global("n", m.global("n")?.sub(m.global("STEP")?)?)?;
            py.check_signals()?;
        }
        rt::builtins::print(py, (m.global("n")?,))?;
        Ok(())
    })
}
