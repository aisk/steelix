use pyo3::prelude::*;
use pyo3::types::PyList;
use steelix_runtime as rt;

// loop_else_jumps.py:4
pub fn skip_rows<'py>(
    m: &rt::Module<'py>,
    rows: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "skip_rows", 4, |frame| {
        frame.line(5);
        'outer: for row in rows.try_iter()? {
            let row = row?;
            frame.line(6);
            'for_else: {
                for cell in row.try_iter()? {
                    let cell = cell?;
                    frame.line(7);
                    if cell.lt(0)? {
                        break 'for_else;
                    }
                    py.check_signals()?;
                    frame.line(6);
                }
                py.check_signals()?;
                frame.line(5);
                continue 'outer;
            }
            frame.line(11);
            rt::builtins::print(py, ("row with a negative cell", &row))?;
            py.check_signals()?;
            frame.line(5);
        }
        Ok(rt::none(py))
    })
}

// loop_else_jumps.py:14
pub fn first_complete<'py>(
    m: &rt::Module<'py>,
    rows: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "first_complete", 14, |frame| {
        frame.line(15);
        'for_else: {
            for row in rows.try_iter()? {
                let row = row?;
                frame.line(16);
                'for_else2: {
                    for cell in row.try_iter()? {
                        let cell = cell?;
                        if cell.is_none() {
                            break 'for_else2;
                        }
                        py.check_signals()?;
                    }
                    frame.line(20);
                    rt::builtins::print(py, ("complete", &row))?;
                    break 'for_else;
                }
                py.check_signals()?;
                frame.line(15);
            }
            frame.line(23);
            rt::builtins::print(py, ("none complete",))?;
        }
        Ok(rt::none(py))
    })
}

// loop_else_jumps.py:26
pub fn countdown<'py>(m: &rt::Module<'py>, n: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "countdown", 26, |frame| {
        let mut n = n.clone();
        let mut i: Bound<'py, PyAny>;
        frame.line(27);
        'outer: while n.gt(0)? {
            frame.line(28);
            n = rt::isub(&n, 1)?;
            i = rt::int(py, 0);
            frame.line(30);
            'while_else: {
                while i.lt(&n)? {
                    frame.line(31);
                    if i.eq(2)? {
                        break 'while_else;
                    }
                    frame.line(33);
                    i = rt::iadd(&i, 1)?;
                    py.check_signals()?;
                    frame.line(30);
                }
                frame.line(35);
                rt::builtins::print(py, ("short", &n))?;
                py.check_signals()?;
                frame.line(27);
                continue 'outer;
            }
            frame.line(37);
            rt::builtins::print(py, ("long", &n))?;
            py.check_signals()?;
            frame.line(27);
        }
        Ok(rt::none(py))
    })
}

// loop_else_jumps.py:40
pub fn nested<'py>(m: &rt::Module<'py>, groups: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "nested", 40, |frame| {
        frame.line(41);
        'outer2: for group in groups.try_iter()? {
            let group = group?;
            frame.line(42);
            'for_else: {
                'outer: for row in group.try_iter()? {
                    let row = row?;
                    frame.line(43);
                    'for_else2: {
                        for cell in row.try_iter()? {
                            let cell = cell?;
                            frame.line(44);
                            if cell.eq(0)? {
                                break 'for_else2;
                            }
                            py.check_signals()?;
                            frame.line(43);
                        }
                        py.check_signals()?;
                        frame.line(42);
                        continue 'outer;
                    }
                    frame.line(48);
                    rt::builtins::print(py, ("zero in", &row))?;
                    break 'for_else;
                }
                frame.line(51);
                rt::builtins::print(py, ("no zero in", &group))?;
                py.check_signals()?;
                frame.line(41);
                continue 'outer2;
            }
            frame.line(53);
            rt::builtins::print(py, ("done with", &group))?;
            py.check_signals()?;
            frame.line(41);
        }
        Ok(rt::none(py))
    })
}

/// Executes the body of `loop_else_jumps.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        m.set_global(
            "__doc__",
            "`break`/`continue` in a loop's `else` belong to the enclosing loop.",
        )?;
        rt::def!(m, skip_rows(rows))?;
        rt::def!(m, first_complete(rows))?;
        rt::def!(m, countdown(n))?;
        rt::def!(m, nested(groups))?;
        frame.line(56);
        skip_rows(
            m,
            PyList::new(
                py,
                [
                    PyList::new(py, [1, 2])?.as_any(),
                    PyList::new(py, [3, -1])?.as_any(),
                    PyList::new(py, [4])?.as_any(),
                ],
            )?
            .as_any(),
        )?;
        frame.line(57);
        first_complete(
            m,
            PyList::new(
                py,
                [
                    PyList::new(py, [&rt::int(py, 1), &rt::none(py)])?.as_any(),
                    PyList::new(py, [2, 3])?.as_any(),
                    PyList::new(py, [4])?.as_any(),
                ],
            )?
            .as_any(),
        )?;
        frame.line(58);
        first_complete(
            m,
            PyList::new(py, [PyList::new(py, [&rt::none(py)])?.as_any()])?.as_any(),
        )?;
        frame.line(59);
        countdown(m, &rt::int(py, 5))?;
        frame.line(60);
        nested(
            m,
            PyList::new(
                py,
                [
                    PyList::new(
                        py,
                        [
                            PyList::new(py, [1])?.as_any(),
                            PyList::new(py, [0, 1])?.as_any(),
                        ],
                    )?
                    .as_any(),
                    PyList::new(py, [PyList::new(py, [2])?.as_any()])?.as_any(),
                ],
            )?
            .as_any(),
        )?;
        Ok(())
    })
}
