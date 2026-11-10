use pyo3::prelude::*;
use pyo3::types::PyList;
use steelix_runtime as rt;

// loop_else_jumps.py:4
#[rt::frame(m, "skip_rows", 4)]
pub fn skip_rows<'py>(
    m: &rt::Module<'py>,
    rows: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    'outer: for row in rt::iter(rows)? {
        let row = row?;
        'for_else: {
            for cell in rt::iter(&row)? {
                let cell = cell?;
                if cell.lt(0)? {
                    break 'for_else;
                }
            }
            continue 'outer;
        }
        rt::builtins::print(py, ("row with a negative cell", &row))?;
    }
    Ok(rt::none(py))
}

// loop_else_jumps.py:14
#[rt::frame(m, "first_complete", 14)]
pub fn first_complete<'py>(
    m: &rt::Module<'py>,
    rows: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    'for_else: {
        for row in rt::iter(rows)? {
            let row = row?;
            'for_else2: {
                for cell in rt::iter(&row)? {
                    let cell = cell?;
                    if cell.is_none() {
                        break 'for_else2;
                    }
                }
                rt::builtins::print(py, ("complete", &row))?;
                break 'for_else;
            }
        }
        rt::builtins::print(py, ("none complete",))?;
    }
    Ok(rt::none(py))
}

// loop_else_jumps.py:26
#[rt::frame(m, "countdown", 26)]
pub fn countdown<'py>(m: &rt::Module<'py>, n: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let mut n = n.clone();
    let mut i: Bound<'py, PyAny>;
    'outer: while n.gt(0)? {
        n = rt::isub(&n, 1)?;
        i = rt::int(py, 0);
        'while_else: {
            while i.lt(&n)? {
                if i.eq(2)? {
                    break 'while_else;
                }
                i = rt::iadd(&i, 1)?;
                py.check_signals()?;
            }
            rt::builtins::print(py, ("short", &n))?;
            py.check_signals()?;
            continue 'outer;
        }
        rt::builtins::print(py, ("long", &n))?;
        py.check_signals()?;
    }
    Ok(rt::none(py))
}

// loop_else_jumps.py:40
#[rt::frame(m, "nested", 40)]
pub fn nested<'py>(m: &rt::Module<'py>, groups: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    'outer2: for group in rt::iter(groups)? {
        let group = group?;
        'for_else: {
            'outer: for row in rt::iter(&group)? {
                let row = row?;
                'for_else2: {
                    for cell in rt::iter(&row)? {
                        let cell = cell?;
                        if cell.eq(0)? {
                            break 'for_else2;
                        }
                    }
                    continue 'outer;
                }
                rt::builtins::print(py, ("zero in", &row))?;
                break 'for_else;
            }
            rt::builtins::print(py, ("no zero in", &group))?;
            continue 'outer2;
        }
        rt::builtins::print(py, ("done with", &group))?;
    }
    Ok(rt::none(py))
}

/// Executes the body of `loop_else_jumps.py` in the module `m`.
#[rt::frame(m, "<module>", 1)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    m.set_global(
        "__doc__",
        "`break`/`continue` in a loop's `else` belong to the enclosing loop.",
    )?;
    rt::def!(m, skip_rows(rows))?;
    rt::def!(m, first_complete(rows))?;
    rt::def!(m, countdown(n))?;
    rt::def!(m, nested(groups))?;
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
    first_complete(
        m,
        PyList::new(py, [PyList::new(py, [&rt::none(py)])?.as_any()])?.as_any(),
    )?;
    countdown(m, &rt::int(py, 5))?;
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
}
