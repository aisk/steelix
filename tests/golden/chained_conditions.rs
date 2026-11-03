use pyo3::prelude::*;
use steelix_runtime as rt;

// chained_conditions.py:4
pub fn f<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "f", 4, |_| {
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
    rt::frame!(m, "check", 9, |_| {
        if a.lt(b)? && {
            let rhs = f(m, &rt::int(py, 3))?;
            b.lt(&rhs)? && rhs.lt(d)?
        } {
            rt::builtins::print(py, ("all ascending",))?;
        } else {
            rt::builtins::print(py, ("not ascending",))?;
        }
        {
            let rhs2 = f(m, b)?;
            if a.lt(&rhs2)? && {
                let rhs3 = f(m, &rt::int(py, 3))?;
                rhs2.lt(&rhs3)? && rhs3.le(d)? && d.lt(f(m, &rt::int(py, 10))?)?
            } {
                rt::builtins::print(py, ("long chain",))?;
            }
        }
        if f(m, a)?.eq(1)? && rt::int(py, 1).lt(f(m, b)?)? && {
            let rhs2 = f(m, &rt::int(py, 5))?;
            b.lt(&rhs2)? && rhs2.lt(d)?
        } {
            rt::builtins::print(py, ("mixed",))?;
        } else {
            {
                let rhs4 = f(m, &rt::int(py, 0))?;
                if a.lt(&rhs4)? && rhs4.lt(d)? {
                    rt::builtins::print(py, ("elif",))?;
                }
            }
        }
        let mut n = rt::int(py, 0);
        loop {
            let rhs4 = f(m, &rt::int(py, 2))?;
            if !(n.lt(&rhs4)? && rhs4.lt(d)?) {
                break;
            }
            n = rt::iadd(&n, 1)?;
            py.check_signals()?;
        }
        rt::builtins::print(py, ("n", &n))?;
        Ok(rt::none(py))
    })
}

/// Executes the body of `chained_conditions.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |_| {
        m.set_global(
            "__doc__",
            "Chained comparisons in conditions whose operands need temporaries.",
        )?;
        rt::def!(m, f(x))?;
        rt::def!(m, check(a, b, d))?;
        check(m, &rt::int(py, 1), &rt::int(py, 2), &rt::int(py, 4))?;
        check(m, &rt::int(py, 1), &rt::int(py, 2), &rt::int(py, 3))?;
        check(m, &rt::int(py, 5), &rt::int(py, 2), &rt::int(py, 4))?;
        Ok(())
    })
}
