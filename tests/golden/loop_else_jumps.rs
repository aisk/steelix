use pyo3::prelude::*;
use pyo3::types::PyList;
use steelix_runtime as rt;

// loop_else_jumps.py:4
pub fn skip_rows<'py>(py: Python<'py>, rows: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    'outer: for row in rows.try_iter()? {
        let row = row?;
        'for_else: {
            for cell in row.try_iter()? {
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
pub fn first_complete<'py>(
    py: Python<'py>,
    rows: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    'for_else: {
        for row in rows.try_iter()? {
            let row = row?;
            'for_else2: {
                for cell in row.try_iter()? {
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
pub fn countdown<'py>(py: Python<'py>, n: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
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
            }
            rt::builtins::print(py, ("short", &n))?;
            continue 'outer;
        }
        rt::builtins::print(py, ("long", &n))?;
    }
    Ok(rt::none(py))
}

// loop_else_jumps.py:40
pub fn nested<'py>(py: Python<'py>, groups: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    'outer2: for group in groups.try_iter()? {
        let group = group?;
        'for_else: {
            'outer: for row in group.try_iter()? {
                let row = row?;
                'for_else2: {
                    for cell in row.try_iter()? {
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
pub fn exec_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let globals = &m.dict();
    globals.set_item(
        "__doc__",
        "`break`/`continue` in a loop's `else` belong to the enclosing loop.",
    )?;
    rt::def!(globals, skip_rows(rows))?;
    rt::def!(globals, first_complete(rows))?;
    rt::def!(globals, countdown(n))?;
    rt::def!(globals, nested(groups))?;
    skip_rows(
        py,
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
        py,
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
        py,
        PyList::new(py, [PyList::new(py, [&rt::none(py)])?.as_any()])?.as_any(),
    )?;
    countdown(py, &rt::int(py, 5))?;
    nested(
        py,
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
