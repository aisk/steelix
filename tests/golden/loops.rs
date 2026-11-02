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
    rt::frame!(m, "find", 4, |frame| {
        frame.line(5);
        'for_else: {
            for item in items.try_iter()? {
                let item = item?;
                frame.line(6);
                if item.eq(target)? {
                    frame.line(7);
                    rt::builtins::print(py, ("found", &item))?;
                    break 'for_else;
                }
                py.check_signals()?;
                frame.line(5);
            }
            frame.line(10);
            rt::builtins::print(py, ("not found", target))?;
        }
        Ok(rt::none(py))
    })
}

// loops.py:13
pub fn first_even<'py>(
    m: &rt::Module<'py>,
    numbers: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "first_even", 13, |frame| {
        let mut result = rt::none(py);
        frame.line(15);
        for n in numbers.try_iter()? {
            let n = n?;
            frame.line(16);
            if n.rem(2)?.is_truthy()? {
                py.check_signals()?;
                frame.line(15);
                continue;
            }
            result = n.clone();
            break;
        }
        Ok(result)
    })
}

// loops.py:23
pub fn last_item<'py>(
    m: &rt::Module<'py>,
    items: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "last_item", 23, |frame| {
        let mut item: Option<Bound<'py, PyAny>> = None;
        frame.line(24);
        for item2 in items.try_iter()? {
            item = Some(item2?);
            m.py().check_signals()?;
        }
        frame.line(26);
        Ok(rt::local(&item, "item")?.clone())
    })
}

// loops.py:29
pub fn count_down<'py>(m: &rt::Module<'py>, n: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "count_down", 29, |frame| {
        let mut n = n.clone();
        frame.line(30);
        'while_else: {
            while n.gt(0)? {
                frame.line(31);
                n = n.sub(1)?;
                frame.line(32);
                if n.eq(2)? {
                    frame.line(33);
                    rt::builtins::print(py, ("stop at", &n))?;
                    break 'while_else;
                }
                py.check_signals()?;
                frame.line(30);
            }
            frame.line(36);
            rt::builtins::print(py, ("ran out",))?;
        }
        Ok(n)
    })
}

// loops.py:40
pub fn pairs<'py>(m: &rt::Module<'py>, mapping: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "pairs", 40, |frame| {
        let mut total = rt::int(py, 0);
        frame.line(42);
        for item in mapping.call_method0("items")?.try_iter()? {
            let [key, value] = rt::unpack(&item?)?;
            frame.line(43);
            rt::builtins::print(py, (&key, &value))?;
            frame.line(44);
            total = total.add(&value)?;
            py.check_signals()?;
            frame.line(42);
        }
        Ok(total)
    })
}

// loops.py:48
pub fn nested<'py>(m: &rt::Module<'py>, rows: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "nested", 48, |frame| {
        frame.line(49);
        for row in rows.try_iter()? {
            let row = row?;
            frame.line(50);
            'for_else: {
                for cell in row.try_iter()? {
                    let cell = cell?;
                    frame.line(51);
                    if cell.lt(0)? {
                        frame.line(52);
                        rt::builtins::print(py, ("negative in", &row))?;
                        break 'for_else;
                    }
                    frame.line(54);
                    if cell.eq(0)? {
                        py.check_signals()?;
                        frame.line(50);
                        continue;
                    }
                    frame.line(56);
                    rt::builtins::print(py, ("cell", &cell))?;
                    py.check_signals()?;
                    frame.line(50);
                }
                frame.line(58);
                rt::builtins::print(py, ("row done",))?;
            }
            py.check_signals()?;
            frame.line(49);
        }
        Ok(rt::none(py))
    })
}

// loops.py:61
pub fn search<'py>(
    m: &rt::Module<'py>,
    rows: &Bound<'py, PyAny>,
    target: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "search", 61, |frame| {
        let mut j: Bound<'py, PyAny>;
        let mut i = rt::int(py, 0);
        frame.line(63);
        for row in rows.try_iter()? {
            let row = row?;
            j = rt::int(py, 0);
            frame.line(65);
            for cell in row.try_iter()? {
                let cell = cell?;
                frame.line(66);
                if cell.eq(target)? {
                    frame.line(67);
                    return Ok(PyTuple::new(py, [&i, &j])?.into_any());
                }
                frame.line(68);
                j = j.add(1)?;
                py.check_signals()?;
                frame.line(65);
            }
            frame.line(69);
            i = i.add(1)?;
            py.check_signals()?;
            frame.line(63);
        }
        Ok(rt::none(py))
    })
}

// loops.py:73
pub fn spin<'py>(m: &rt::Module<'py>, limit: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "spin", 73, |frame| {
        let mut k = rt::int(py, 0);
        loop {
            frame.line(76);
            k = k.add(1)?;
            frame.line(77);
            if k.ge(limit)? {
                break;
            }
            py.check_signals()?;
        }
        Ok(k)
    })
}

/// Executes the body of `loops.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        m.set_global("__doc__", "for/while loops with break, continue and else.")?;
        rt::def!(m, find(items, target))?;
        rt::def!(m, first_even(numbers))?;
        rt::def!(m, last_item(items))?;
        rt::def!(m, count_down(n))?;
        rt::def!(m, pairs(mapping))?;
        rt::def!(m, nested(rows))?;
        rt::def!(m, search(rows, target))?;
        rt::def!(m, spin(limit))?;
        frame.line(82);
        find(m, PyList::new(py, [1, 2, 3])?.as_any(), &rt::int(py, 2))?;
        frame.line(83);
        find(m, PyList::new(py, [1, 2, 3])?.as_any(), &rt::int(py, 5))?;
        frame.line(84);
        rt::builtins::print(
            py,
            (
                first_even(m, PyList::new(py, [1, 3, 4, 6])?.as_any())?,
                first_even(m, PyList::new(py, [1, 3])?.as_any())?,
            ),
        )?;
        frame.line(85);
        rt::builtins::print(py, (last_item(m, &rt::str(py, "abc"))?,))?;
        frame.line(86);
        rt::builtins::print(
            py,
            (
                count_down(m, &rt::int(py, 5))?,
                count_down(m, &rt::int(py, 1))?,
            ),
        )?;
        frame.line(87);
        rt::builtins::print(
            py,
            (pairs(m, [("a", 1), ("b", 2)].into_py_dict(py)?.as_any())?,),
        )?;
        frame.line(88);
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
        frame.line(89);
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
        frame.line(90);
        rt::builtins::print(py, (spin(m, &rt::int(py, 4))?,))?;
        frame.line(92);
        m.set_global("total", 0)?;
        frame.line(93);
        for item in rt::builtin(py, "range")?.call1((5,))?.try_iter()? {
            m.set_global("x", item?)?;
            frame.line(94);
            if m.global("x")?.eq(3)? {
                py.check_signals()?;
                frame.line(93);
                continue;
            }
            frame.line(96);
            m.set_global("total", m.global("total")?.add(m.global("x")?)?)?;
            py.check_signals()?;
            frame.line(93);
        }
        frame.line(98);
        rt::builtins::print(py, ("loop finished with x =", m.global("x")?))?;
        frame.line(99);
        rt::builtins::print(py, (m.global("total")?,))?;
        frame.line(100);
        'for_else: {
            for item2 in PyList::new(py, ["alpha", "beta"])?.try_iter()? {
                m.set_global("word", item2?)?;
                frame.line(101);
                rt::builtins::print(py, (m.global("word")?,))?;
                break 'for_else;
            }
            frame.line(104);
            rt::builtins::print(py, ("unreachable else",))?;
        }
        Ok(())
    })
}
