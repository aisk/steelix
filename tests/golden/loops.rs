use pyo3::prelude::*;
use pyo3::types::IntoPyDict;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;

// loops.py:4
pub fn find<'py>(
    py: Python<'py>,
    items: &Bound<'py, PyAny>,
    target: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    'for_else: {
        for item in items.try_iter()? {
            let item = item?;
            if item.eq(target)? {
                rt::builtins::print(py, ("found", &item))?;
                break 'for_else;
            }
        }
        rt::builtins::print(py, ("not found", target))?;
    }
    Ok(rt::none(py))
}

// loops.py:13
pub fn first_even<'py>(
    py: Python<'py>,
    numbers: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let mut result = rt::none(py);
    for n in numbers.try_iter()? {
        let n = n?;
        if n.rem(2)?.is_truthy()? {
            continue;
        }
        result = n.clone();
        break;
    }
    Ok(result)
}

// loops.py:23
pub fn last_item<'py>(py: Python<'py>, items: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let mut item: Option<Bound<'py, PyAny>> = None;
    for item2 in items.try_iter()? {
        item = Some(item2?);
    }
    Ok(rt::local(&item, "item")?.clone())
}

// loops.py:29
pub fn count_down<'py>(py: Python<'py>, n: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let mut n = n.clone();
    'while_else: {
        while n.gt(0)? {
            n = n.sub(1)?;
            if n.eq(2)? {
                rt::builtins::print(py, ("stop at", &n))?;
                break 'while_else;
            }
        }
        rt::builtins::print(py, ("ran out",))?;
    }
    Ok(n)
}

// loops.py:40
pub fn pairs<'py>(py: Python<'py>, mapping: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let mut total = rt::int(py, 0);
    for item in mapping.call_method0("items")?.try_iter()? {
        let [key, value] = rt::unpack(&item?)?;
        rt::builtins::print(py, (&key, &value))?;
        total = total.add(&value)?;
    }
    Ok(total)
}

// loops.py:48
pub fn nested<'py>(py: Python<'py>, rows: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    for row in rows.try_iter()? {
        let row = row?;
        'for_else: {
            for cell in row.try_iter()? {
                let cell = cell?;
                if cell.lt(0)? {
                    rt::builtins::print(py, ("negative in", &row))?;
                    break 'for_else;
                }
                if cell.eq(0)? {
                    continue;
                }
                rt::builtins::print(py, ("cell", &cell))?;
            }
            rt::builtins::print(py, ("row done",))?;
        }
    }
    Ok(rt::none(py))
}

// loops.py:61
pub fn search<'py>(
    py: Python<'py>,
    rows: &Bound<'py, PyAny>,
    target: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let mut j: Bound<'py, PyAny>;
    let mut i = rt::int(py, 0);
    for row in rows.try_iter()? {
        let row = row?;
        j = rt::int(py, 0);
        for cell in row.try_iter()? {
            let cell = cell?;
            if cell.eq(target)? {
                return Ok(PyTuple::new(py, [&i, &j])?.into_any());
            }
            j = j.add(1)?;
        }
        i = i.add(1)?;
    }
    Ok(rt::none(py))
}

// loops.py:73
pub fn spin<'py>(py: Python<'py>, limit: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let mut k = rt::int(py, 0);
    loop {
        k = k.add(1)?;
        if k.ge(limit)? {
            break;
        }
    }
    Ok(k)
}

/// Executes the body of `loops.py` in the module `m`.
pub fn exec_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let globals = &m.dict();
    globals.set_item("__doc__", "for/while loops with break, continue and else.")?;
    rt::def!(globals, find(items, target))?;
    rt::def!(globals, first_even(numbers))?;
    rt::def!(globals, last_item(items))?;
    rt::def!(globals, count_down(n))?;
    rt::def!(globals, pairs(mapping))?;
    rt::def!(globals, nested(rows))?;
    rt::def!(globals, search(rows, target))?;
    rt::def!(globals, spin(limit))?;
    find(py, PyList::new(py, [1, 2, 3])?.as_any(), &rt::int(py, 2))?;
    find(py, PyList::new(py, [1, 2, 3])?.as_any(), &rt::int(py, 5))?;
    rt::builtins::print(
        py,
        (
            first_even(py, PyList::new(py, [1, 3, 4, 6])?.as_any())?,
            first_even(py, PyList::new(py, [1, 3])?.as_any())?,
        ),
    )?;
    rt::builtins::print(py, (last_item(py, &rt::str(py, "abc"))?,))?;
    rt::builtins::print(
        py,
        (
            count_down(py, &rt::int(py, 5))?,
            count_down(py, &rt::int(py, 1))?,
        ),
    )?;
    rt::builtins::print(
        py,
        (pairs(py, [("a", 1), ("b", 2)].into_py_dict(py)?.as_any())?,),
    )?;
    nested(
        py,
        PyList::new(
            py,
            [
                PyList::new(py, [1, 0, 2])?.as_any(),
                PyList::new(py, [3, -1, 4])?.as_any(),
                PyList::empty(py).as_any(),
            ],
        )?
        .as_any(),
    )?;
    rt::builtins::print(
        py,
        (
            search(
                py,
                PyList::new(
                    py,
                    [
                        PyList::new(py, [1, 2])?.as_any(),
                        PyList::new(py, [3, 4])?.as_any(),
                    ],
                )?
                .as_any(),
                &rt::int(py, 4),
            )?,
            search(
                py,
                PyList::new(py, [PyList::new(py, [1])?.as_any()])?.as_any(),
                &rt::int(py, 9),
            )?,
        ),
    )?;
    rt::builtins::print(py, (spin(py, &rt::int(py, 4))?,))?;
    globals.set_item("total", 0)?;
    for item in rt::builtin(py, "range")?.call1((5,))?.try_iter()? {
        globals.set_item("x", item?)?;
        if rt::global(globals, "x")?.eq(3)? {
            continue;
        }
        globals.set_item(
            "total",
            rt::global(globals, "total")?.add(rt::global(globals, "x")?)?,
        )?;
    }
    rt::builtins::print(py, ("loop finished with x =", rt::global(globals, "x")?))?;
    rt::builtins::print(py, (rt::global(globals, "total")?,))?;
    'for_else: {
        for item2 in PyList::new(py, ["alpha", "beta"])?.try_iter()? {
            globals.set_item("word", item2?)?;
            rt::builtins::print(py, (rt::global(globals, "word")?,))?;
            break 'for_else;
        }
        rt::builtins::print(py, ("unreachable else",))?;
    }
    Ok(())
}
