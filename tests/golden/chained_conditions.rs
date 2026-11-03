use pyo3::prelude::*;
use steelix_runtime as rt;

// chained_conditions.py:4
pub fn f<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "f", 4, |frame| {
        frame.line(5);
        rt::builtins::print(m.py(), ("f", x))?;
        Ok(x.clone())
    })
}

// chained_conditions.py:9
pub fn check<'py>(
    m: &rt::Module<'py>,
    a: &Bound<'py, PyAny>,
    b: &Bound<'py, PyAny>,
    d: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "check", 9, |frame| {
        frame.line(10);
        if a.lt(b)? && {
            let rhs = f(m, &rt::int(py, 3))?;
            b.lt(&rhs)? && rhs.lt(d)?
        } {
            frame.line(11);
            rt::builtins::print(py, ("all ascending",))?;
        } else {
            frame.line(13);
            rt::builtins::print(py, ("not ascending",))?;
        }
        {
            frame.line(14);
            let rhs2 = f(m, b)?;
            if a.lt(&rhs2)? && {
                let rhs3 = f(m, &rt::int(py, 3))?;
                rhs2.lt(&rhs3)? && rhs3.le(d)? && d.lt(f(m, &rt::int(py, 10))?)?
            } {
                frame.line(15);
                rt::builtins::print(py, ("long chain",))?;
            }
        }
        frame.line(16);
        if f(m, a)?.eq(1)? && rt::int(py, 1).lt(f(m, b)?)? && {
            let rhs2 = f(m, &rt::int(py, 5))?;
            b.lt(&rhs2)? && rhs2.lt(d)?
        } {
            frame.line(17);
            rt::builtins::print(py, ("mixed",))?;
        } else {
            {
                frame.line(18);
                let rhs4 = f(m, &rt::int(py, 0))?;
                if a.lt(&rhs4)? && rhs4.lt(d)? {
                    frame.line(19);
                    rt::builtins::print(py, ("elif",))?;
                }
            }
        }
        let mut n = rt::int(py, 0);
        loop {
            frame.line(21);
            let rhs4 = f(m, &rt::int(py, 2))?;
            if !(n.lt(&rhs4)? && rhs4.lt(d)?) {
                break;
            }
            frame.line(22);
            n = rt::iadd(&n, 1)?;
            py.check_signals()?;
        }
        frame.line(23);
        rt::builtins::print(py, ("n", &n))?;
        Ok(rt::none(py))
    })
}

/// Executes the body of `chained_conditions.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        m.set_global(
            "__doc__",
            "Chained comparisons in conditions whose operands need temporaries.",
        )?;
        rt::def!(m, f(x))?;
        rt::def!(m, check(a, b, d))?;
        frame.line(26);
        check(m, &rt::int(py, 1), &rt::int(py, 2), &rt::int(py, 4))?;
        frame.line(27);
        check(m, &rt::int(py, 1), &rt::int(py, 2), &rt::int(py, 3))?;
        frame.line(28);
        check(m, &rt::int(py, 5), &rt::int(py, 2), &rt::int(py, 4))?;
        Ok(())
    })
}
