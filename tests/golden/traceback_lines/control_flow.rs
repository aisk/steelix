use pyo3::prelude::*;
use steelix_runtime as rt;

// control_flow.py:7
/// Count the steps until the Collatz sequence reaches 1.
pub fn collatz_steps<'py>(
    m: &rt::Module<'py>,
    n: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "collatz_steps", 7, |frame| {
        let mut n = n.clone();
        let mut steps = rt::int(py, 0);
        frame.line(10);
        while n.ne(1)? {
            frame.line(11);
            if n.rem(2)?.eq(0)? {
                frame.line(12);
                n = n.floor_div(2)?;
            } else {
                frame.line(14);
                n = rt::int(py, 3).mul(&n)?.add(1)?;
            }
            frame.line(15);
            steps = steps.add(1)?;
            py.check_signals()?;
            frame.line(10);
        }
        Ok(steps)
    })
}

// control_flow.py:19
pub fn classify<'py>(m: &rt::Module<'py>, n: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "classify", 19, |frame| {
        frame.line(20);
        let kind = if n.lt(0)? {
            rt::str(py, "negative")
        } else {
            frame.line(22);
            if n.eq(0)? {
                rt::str(py, "zero")
            } else {
                frame.line(24);
                if n.lt(m.global("LIMIT")?)? {
                    rt::str(py, "small")
                } else {
                    rt::str(py, "large")
                }
            }
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
    rt::frame!(m, "count_up", 31, |frame| {
        let mut i = start.clone();
        let mut total = rt::int(py, 0);
        frame.line(34);
        while i.lt(m.global("LIMIT")?)? {
            frame.line(35);
            total = total.add(&i)?;
            frame.line(36);
            i = i.add(m.global("STEP")?)?;
            py.check_signals()?;
            frame.line(34);
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
    rt::frame!(m, "first_square_above", 40, |frame| {
        let mut k = rt::int(py, 0);
        loop {
            frame.line(43);
            k = k.add(1)?;
            frame.line(44);
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
    rt::frame!(m, "<module>", 1, |frame| {
        m.set_global("__doc__", "Loops and branches over module level state.")?;
        frame.line(3);
        m.set_global("LIMIT", 20)?;
        frame.line(4);
        m.set_global("STEP", 3)?;
        rt::def!(
            m,
            collatz_steps(n),
            doc = "Count the steps until the Collatz sequence reaches 1."
        )?;
        rt::def!(m, classify(n))?;
        rt::def!(m, count_up(start))?;
        rt::def!(m, first_square_above(bound))?;
        frame.line(48);
        rt::builtins::print(py, (collatz_steps(m, &rt::int(py, 27))?,))?;
        frame.line(49);
        rt::builtins::print(
            py,
            (
                classify(m, &rt::int(py, -5))?,
                classify(m, &rt::int(py, 0))?,
                classify(m, &rt::int(py, 7))?,
                classify(m, &rt::int(py, 100))?,
            ),
        )?;
        frame.line(50);
        rt::builtins::print(
            py,
            (
                count_up(m, &rt::int(py, 1))?,
                count_up(m, &m.global("LIMIT")?)?,
            ),
        )?;
        frame.line(51);
        rt::builtins::print(py, (first_square_above(m, &rt::int(py, 50))?,))?;
        frame.line(52);
        m.set_global("n", 10)?;
        frame.line(53);
        while m.global("n")?.gt(0)? {
            frame.line(54);
            if m.global("n")?.rem(4)?.eq(0)? {
                frame.line(55);
                rt::builtins::print(py, ("four", m.global("n")?))?;
            } else {
                frame.line(56);
                if m.global("n")?.rem(3)?.eq(0)? {
                    frame.line(57);
                    rt::builtins::print(py, ("three", m.global("n")?))?;
                }
            }
            frame.line(58);
            m.set_global("n", m.global("n")?.sub(m.global("STEP")?)?)?;
            py.check_signals()?;
            frame.line(53);
        }
        frame.line(59);
        rt::builtins::print(py, (m.global("n")?,))?;
        Ok(())
    })
}
