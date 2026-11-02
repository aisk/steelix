use pyo3::prelude::*;
use pyo3::types::IntoPyDict;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;

// loops.py:4
pub fn find<'py>(
    m: &rt::Module<'py>,
    items: &Bound<'py, PyAny>,
    target: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
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
    m: &rt::Module<'py>,
    numbers: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let mut result = rt::none(m.py());
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
pub fn last_item<'py>(
    m: &rt::Module<'py>,
    items: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let mut item: Option<Bound<'py, PyAny>> = None;
    for item2 in items.try_iter()? {
        item = Some(item2?);
    }
    Ok(rt::local(&item, "item")?.clone())
}

// loops.py:29
pub fn count_down<'py>(m: &rt::Module<'py>, n: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
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
pub fn pairs<'py>(m: &rt::Module<'py>, mapping: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let mut total = rt::int(py, 0);
    for item in mapping.call_method0("items")?.try_iter()? {
        let [key, value] = rt::unpack(&item?)?;
        rt::builtins::print(py, (&key, &value))?;
        total = total.add(&value)?;
    }
    Ok(total)
}

// loops.py:48
pub fn nested<'py>(m: &rt::Module<'py>, rows: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
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
    m: &rt::Module<'py>,
    rows: &Bound<'py, PyAny>,
    target: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
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
pub fn spin<'py>(m: &rt::Module<'py>, limit: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let mut k = rt::int(m.py(), 0);
    loop {
        k = k.add(1)?;
        if k.ge(limit)? {
            break;
        }
    }
    Ok(k)
}

/// Executes the body of `loops.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    m.set_global("__doc__", "for/while loops with break, continue and else.")?;
    rt::def!(m, find(items, target))?;
    rt::def!(m, first_even(numbers))?;
    rt::def!(m, last_item(items))?;
    rt::def!(m, count_down(n))?;
    rt::def!(m, pairs(mapping))?;
    rt::def!(m, nested(rows))?;
    rt::def!(m, search(rows, target))?;
    rt::def!(m, spin(limit))?;
    find(m, PyList::new(py, [1, 2, 3])?.as_any(), &rt::int(py, 2))?;
    find(m, PyList::new(py, [1, 2, 3])?.as_any(), &rt::int(py, 5))?;
    rt::builtins::print(
        py,
        (
            first_even(m, PyList::new(py, [1, 3, 4, 6])?.as_any())?,
            first_even(m, PyList::new(py, [1, 3])?.as_any())?,
        ),
    )?;
    rt::builtins::print(py, (last_item(m, &rt::str(py, "abc"))?,))?;
    rt::builtins::print(
        py,
        (
            count_down(m, &rt::int(py, 5))?,
            count_down(m, &rt::int(py, 1))?,
        ),
    )?;
    rt::builtins::print(
        py,
        (pairs(m, [("a", 1), ("b", 2)].into_py_dict(py)?.as_any())?,),
    )?;
    nested(
        m,
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
                m,
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
                m,
                PyList::new(py, [PyList::new(py, [1])?.as_any()])?.as_any(),
                &rt::int(py, 9),
            )?,
        ),
    )?;
    rt::builtins::print(py, (spin(m, &rt::int(py, 4))?,))?;
    m.set_global("total", 0)?;
    for item in rt::builtin(py, "range")?.call1((5,))?.try_iter()? {
        m.set_global("x", item?)?;
        if m.global("x")?.eq(3)? {
            continue;
        }
        m.set_global("total", m.global("total")?.add(m.global("x")?)?)?;
    }
    rt::builtins::print(py, ("loop finished with x =", m.global("x")?))?;
    rt::builtins::print(py, (m.global("total")?,))?;
    'for_else: {
        for item2 in PyList::new(py, ["alpha", "beta"])?.try_iter()? {
            m.set_global("word", item2?)?;
            rt::builtins::print(py, (m.global("word")?,))?;
            break 'for_else;
        }
        rt::builtins::print(py, ("unreachable else",))?;
    }
    Ok(())
}
