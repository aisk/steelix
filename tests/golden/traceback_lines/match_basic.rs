use pyo3::exceptions::PyUnboundLocalError;
use pyo3::prelude::*;
use pyo3::pyclass::CompareOp;
use pyo3::types::PyIterator;
use pyo3::types::PyList;
use pyo3::types::PySet;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// match_basic.py:7
#[rt::frame(m, "command", 7, frame)]
pub fn command<'py>(m: &rt::Module<'py>, text: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    frame.line(8);
    let subject = text.method0("split")?;
    frame.line(9);
    if rt::match_seq_len(&subject, 0)? {
        Ok(rt::str(py, "nothing"))
    } else {
        frame.line(11);
        if let Some([item2, direction]) = rt::match_seq(&subject)?
            && item2.eq("go")?
            && m.global("EXITS")?.contains(&direction)?
        {
            frame.line(12);
            rt::fstring(py, ("going ", rt::format(&direction, "")?))
        } else {
            frame.line(13);
            if let Some([item2, _]) = rt::match_seq(&subject)?
                && item2.eq("go")?
            {
                Ok(rt::str(py, "no exit there"))
            } else {
                frame.line(15);
                if let Some([item2]) = rt::match_seq(&subject)?
                    && item2.eq("look")?
                {
                    Ok(rt::str(py, "you see a room"))
                } else {
                    frame.line(17);
                    if let Some(([item2], items, [])) = rt::match_seq_star(&subject)?
                        && item2.eq("take")?
                        && items.is_truthy()?
                    {
                        frame.line(18);
                        rt::str(py, "taking ").add(rt::str(py, ", ").method1("join", (&items,))?)
                    } else {
                        frame.line(19);
                        if rt::match_seq_min(&subject, 2)?
                            && let item2 = subject.get_item(0)?
                            && item2.eq("drop")?
                            && let item = subject.get_item(1)?
                        {
                            frame.line(20);
                            rt::fstring(py, ("dropping ", rt::format(&item, "")?))
                        } else {
                            frame.line(21);
                            if rt::match_seq_min(&subject, 2)?
                                && let verb = subject.get_item(0)?
                                && let last = rt::item_from_end(&subject, 1)?
                            {
                                frame.line(22);
                                rt::fstring(
                                    py,
                                    (rt::format(&verb, "")?, "? (", rt::format(&last, "")?, ")"),
                                )
                            } else {
                                Ok(rt::str(py, "?"))
                            }
                        }
                    }
                }
            }
        }
    }
}

// match_basic.py:31
/// A named star at each position, and `*_` with the items around it.
#[rt::frame(m, "stars", 31, frame)]
pub fn stars<'py>(m: &rt::Module<'py>, value: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    frame.line(34);
    if let Some(([], rest, [])) = rt::match_seq_star(value)? {
        frame.line(35);
        rt::builtins::print(py, ("all", &rest))?;
    }
    frame.line(37);
    if let Some(([first], rest, [])) = rt::match_seq_star(value)? {
        frame.line(38);
        rt::builtins::print(py, ("first", &first, &rest))?;
    } else {
        frame.line(40);
        rt::builtins::print(py, ("empty",))?;
    }
    frame.line(42);
    if let Some(([], init, [last])) = rt::match_seq_star(value)? {
        frame.line(43);
        rt::builtins::print(py, ("last", &init, &last))?;
    } else {
        frame.line(45);
        rt::builtins::print(py, ("no last",))?;
    }
    frame.line(47);
    if let Some(([a], middle, [b, c])) = rt::match_seq_star(value)? {
        frame.line(48);
        rt::builtins::print(py, ("ends", &a, &middle, &b, &c))?;
    } else {
        frame.line(49);
        if rt::match_seq_min(value, 1)?
            && let item = rt::item_from_end(value, 1)?
            && item.eq(9)?
        {
            frame.line(50);
            rt::builtins::print(py, ("ends with 9",))?;
        } else {
            frame.line(51);
            if rt::match_seq_min(value, 1)?
                && let item = value.get_item(0)?
                && item.eq(0)?
            {
                frame.line(52);
                rt::builtins::print(py, ("starts with 0",))?;
            } else {
                frame.line(53);
                if rt::match_seq_min(value, 3)?
                    && let item = rt::item_from_end(value, 1)?
                    && item.eq(1)?
                {
                    frame.line(54);
                    rt::builtins::print(py, ("at least three, last 1",))?;
                } else {
                    frame.line(56);
                    rt::builtins::print(py, ("other",))?;
                }
            }
        }
    }
    Ok(rt::none(py))
}

// match_basic.py:63
#[rt::frame(m, "literals", 63, frame)]
pub fn literals<'py>(
    m: &rt::Module<'py>,
    value: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    if value.is_none() {
        return Ok(rt::str(py, "None"));
    } else if value.is(rt::bool(py, true)) {
        return Ok(rt::str(py, "True"));
    } else if value.is(rt::bool(py, false)) {
        return Ok(rt::str(py, "False"));
    } else {
        frame.line(71);
        if value.eq(1)? {
            return Ok(rt::str(py, "one (== 1)"));
        } else {
            frame.line(73);
            if value.eq(2.5)? {
                return Ok(rt::str(py, "two and a half"));
            } else {
                frame.line(75);
                if value.eq(-3)? {
                    return Ok(rt::str(py, "minus three"));
                } else {
                    frame.line(77);
                    if value.eq(rt::complex(py, 1.0, 2.0))? {
                        return Ok(rt::str(py, "complex"));
                    } else {
                        frame.line(79);
                        if value.eq("text")? {
                            return Ok(rt::str(py, "the string"));
                        } else {
                            frame.line(81);
                            if value.eq(rt::bytes(py, b"bytes"))? {
                                return Ok(rt::str(py, "the bytes"));
                            } else {
                                frame.line(83);
                                if value.eq("a")? || value.eq("b")? || value.eq("c")? {
                                    return Ok(rt::str(py, "a letter"));
                                } else {
                                    frame.line(85);
                                    if value.eq(10)? || value.eq(20)? || value.eq(30)? {
                                        let tens = value.clone();
                                        frame.line(86);
                                        return rt::fstring(py, ("tens ", rt::format(&tens, "")?));
                                    } else {
                                        frame.line(87);
                                        if value.eq(4)? || value.eq(5)? {
                                            let n = value.clone();
                                            frame.line(88);
                                            return rt::fstring(
                                                py,
                                                ("four or five: ", rt::format(&n, "")?),
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(rt::str(py, "no match"))
}

// match_basic.py:96
#[rt::frame(m, "nested", 96, frame)]
pub fn nested<'py>(m: &rt::Module<'py>, point: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    frame.line(98);
    if let Some([item, label]) = rt::match_seq(point)?
        && let Some([item2, item3]) = rt::match_seq(&item)?
        && item2.eq(0)?
        && item3.eq(0)?
    {
        frame.line(99);
        return rt::fstring(py, ("origin ", rt::format(&label, "")?));
    } else {
        frame.line(100);
        if let Some([item, _]) = rt::match_seq(point)?
            && (matches!(rt::match_seq(&item)?, Some([item2, item3]) if item2.eq(1)? && item3.eq(0)?)
                || matches!(rt::match_seq(&item)?, Some([item4, item5]) if item4.eq(0)? && item5.eq(1)?))
        {
            return Ok(rt::str(py, "unit"));
        } else {
            frame.line(102);
            if let Some([item, item2]) = rt::match_seq(point)?
                && let Some([x, y]) = rt::match_seq(&item)?
                && let Some(([], tags, [])) = rt::match_seq_star(&item2)?
                && x.eq(&y)?
            {
                frame.line(103);
                return rt::fstring(
                    py,
                    (
                        "diagonal ",
                        rt::format(&x, "")?,
                        " ",
                        rt::format(&tags, "")?,
                    ),
                );
            } else {
                frame.line(104);
                if let Some([item, label]) = rt::match_seq(point)?
                    && let Some([x, y]) = rt::match_seq(&item)?
                    && rt::match_instance(&label, &rt::builtin(py, "str")?)?
                {
                    frame.line(105);
                    return rt::fstring(
                        py,
                        (
                            "point ",
                            rt::format(&x, "")?,
                            ",",
                            rt::format(&y, "")?,
                            " ",
                            rt::format(&label, "")?,
                        ),
                    );
                } else {
                    frame.line(106);
                    if let Some([pair, _]) = rt::match_seq(point)?
                        && rt::match_seq_len(&pair, 2)?
                    {
                        frame.line(107);
                        return rt::fstring(py, ("pair ", rt::format(&pair, "")?));
                    }
                }
            }
        }
    }
    Ok(rt::str(py, "other"))
}

// match_basic.py:115
/// A guard that fails still leaves the captures bound (CPython stores
/// them before the guard runs).
#[rt::frame(m, "guard_binds", 115, frame)]
pub fn guard_binds<'py>(
    m: &rt::Module<'py>,
    values: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let mut x: Option<Bound<'py, PyAny>> = None;
    let mut y: Option<Bound<'py, PyAny>> = None;
    'match_: {
        frame.line(119);
        if let Some([x2, y2]) = rt::match_seq(values)? {
            x = Some(x2);
            y = Some(y2);
            if rt::local(&x, "x")?.gt(rt::local(&y, "y")?)? {
                frame.line(120);
                rt::builtins::print(py, ("descending",))?;
                break 'match_;
            }
        }
        frame.line(121);
        if rt::match_seq_min(values, 1)?
            && let x2 = values.get_item(0)?
        {
            x = Some(x2);
            if rt::local(&x, "x")?.lt(0)? {
                frame.line(122);
                rt::builtins::print(py, ("negative start",))?;
                break 'match_;
            }
        }
    }
    frame.line(125);
    rt::builtins::print(py, ("x after the match:", rt::local(&x, "x")?))?;
    frame.line(127);
    match (|| rt::builtins::print(py, (rt::local(&y, "y")?,)))() {
        Ok(_) => {}
        Err(err) if err.is_instance_of::<PyUnboundLocalError>(py) => {
            let err = frame.catch(err);
            frame.line(129);
            rt::builtins::print(py, ("UnboundLocalError:", &err))?;
        }
        Err(err2) => return Err(err2),
    }
    Ok(rt::none(py))
}

// match_basic.py:137
/// Captures used after the match are stored in the function's locals.
#[rt::frame(m, "counts", 137, frame)]
pub fn counts<'py>(m: &rt::Module<'py>, items: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let mut seen = rt::int(py, 0);
    frame.line(140);
    let mut n: Option<Bound<'py, PyAny>> = None;
    for item in items.try_iter()? {
        let item = item?;
        frame.line(142);
        if let Some([item2, n2]) = rt::match_seq(&item)?
            && item2.eq("add")?
        {
            n = Some(n2);
            frame.line(143);
            seen = rt::iadd(&seen, rt::local(&n, "n")?)?;
        } else {
            frame.line(144);
            if let Some([item2, n2]) = rt::match_seq(&item)?
                && item2.eq("sub")?
            {
                n = Some(n2);
                frame.line(145);
                seen = rt::isub(&seen, rt::local(&n, "n")?)?;
            } else {
                frame.line(146);
                if let Some([item2]) = rt::match_seq(&item)?
                    && item2.eq("reset")?
                {
                    seen = rt::int(py, 0);
                } else {
                    let other = item.clone();
                    frame.line(149);
                    rt::builtins::print(py, ("skipping", &other))?;
                }
            }
        }
        py.check_signals()?;
        frame.line(140);
    }
    frame.line(150);
    rt::builtins::print(py, ("total", &seen, "last n", rt::local(&n, "n")?))?;
    Ok(rt::none(py))
}

// match_basic.py:156
#[rt::frame(m, "walrus_guard", 156, frame)]
pub fn walrus_guard<'py>(
    m: &rt::Module<'py>,
    value: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    frame.line(158);
    let mut doubled: Option<Bound<'py, PyAny>> = None;
    if let Some([a]) = rt::match_seq(value)?
        && {
            let doubled2 = a.mul(2)?;
            doubled = Some(doubled2.clone());
            doubled2.gt(10)?
        }
    {
        frame.line(159);
        rt::builtins::print(py, ("big", rt::local(&doubled, "doubled")?))?;
    } else {
        frame.line(161);
        rt::builtins::print(py, ("small", rt::local(&doubled, "doubled")?))?;
    }
    Ok(rt::none(py))
}

// match_basic.py:168
#[rt::frame(m, "loop_jumps", 168, frame)]
pub fn loop_jumps<'py>(
    m: &rt::Module<'py>,
    rows: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    frame.line(169);
    let mut total: Bound<'py, PyAny>;
    'outer: for row in rows.try_iter()? {
        let row = row?;
        frame.line(171);
        if rt::match_seq_min(&row, 1)?
            && let item = row.get_item(0)?
            && item.eq("skip")?
        {
            py.check_signals()?;
            frame.line(169);
            continue;
        } else {
            frame.line(173);
            if let Some([item]) = rt::match_seq(&row)?
                && item.eq("stop")?
            {
                break;
            } else {
                frame.line(175);
                if let Some([x]) = rt::match_seq(&row)?
                    && x.gt(100)?
                {
                    frame.line(176);
                    rt::builtins::print(py, ("large",))?;
                } else {
                    frame.line(177);
                    if let Some(([name], values, [])) = rt::match_seq_star(&row)?
                        && values.is_truthy()?
                    {
                        total = rt::int(py, 0);
                        frame.line(179);
                        'for_else: {
                            for v in values.try_iter()? {
                                let v = v?;
                                frame.line(180);
                                if v.lt(0)? {
                                    break 'for_else;
                                }
                                frame.line(182);
                                total = rt::iadd(&total, &v)?;
                                py.check_signals()?;
                                frame.line(179);
                            }
                            frame.line(184);
                            rt::builtins::print(py, (&name, "sum", &total))?;
                            py.check_signals()?;
                            frame.line(169);
                            continue 'outer;
                        }
                        frame.line(186);
                        rt::builtins::print(py, (&name, "has a negative value"))?;
                    }
                }
            }
        }
        py.check_signals()?;
        frame.line(169);
    }
    frame.line(187);
    rt::builtins::print(py, ("done",))?;
    Ok(rt::none(py))
}

// match_basic.py:226
/// A generator expression or a lambda in a guard is a nested `fn` item
/// that the guard calls.
#[rt::frame(m, "nested_functions", 226, frame)]
pub fn nested_functions<'py>(
    m: &rt::Module<'py>,
    value: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let first = rt::Cell::empty(py, "first")?;
    // match_basic.py:230
    #[rt::frame(m, "nested_functions.<locals>.<genexpr>", 230)]
    fn genexpr<'py>(
        m: &rt::Module<'py>,
        items: &Bound<'py, PyIterator>,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        let py = m.py();
        if let Some(item) = rt::next(items)? {
            return Ok(Some(rt::bool(
                py,
                item.is_instance(&rt::builtin(py, "str")?)?,
            )));
        }
        Ok(None)
    }
    // match_basic.py:232
    #[rt::frame(m, "nested_functions.<locals>.<lambda>", 232)]
    fn lambda<'py>(m: &rt::Module<'py>, first: &rt::Cell<'py>) -> PyResult<Bound<'py, PyAny>> {
        first.get()?.rich_compare(0, CompareOp::Gt)
    }
    frame.line(230);
    if let Some([items]) = rt::match_class(value, &rt::builtin(py, "list")?, 1, [])?
        && rt::builtin(py, "all")?
            .call1((rt::genexpr!(
                m,
                genexpr(items.try_iter()?),
                qualname = "nested_functions.<locals>.<genexpr>"
            )?,))?
            .is_truthy()?
    {
        return Ok(rt::str(py, "strings"));
    }
    frame.line(232);
    if rt::match_seq_min(value, 1)?
        && let first2 = value.get_item(0)?
    {
        first.set(&first2)?;
        if rt::function!(
            m,
            lambda[&first](),
            qualname = "nested_functions.<locals>.<lambda>"
        )?
        .call0()?
        .is_truthy()?
        {
            return Ok(rt::str(py, "starts positive"));
        }
    }
    Ok(rt::str(py, "other"))
}

pub mod config {
    use super::*;

    // match_basic.py:209
    /// The body of `class Config`, run in its namespace `ns`.
    #[rt::frame(ns, "Config", 209, frame)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        let py = m.py();
        {
            frame.line(210);
            let subject = ns.get("len")?.call1((ns.get("EXITS")?,))?;
            frame.line(211);
            if subject.eq(3)? {
                ns.set("count", &subject)?;
                frame.line(212);
                ns.set("kind", "three")?;
            } else {
                frame.line(214);
                ns.set("kind", "other")?;
            }
        }
        frame.line(216);
        'match_: {
            let subject = PyTuple::new(py, [&rt::str(py, "debug"), &rt::int(py, 1)])?.into_any();
            frame.line(217);
            if let Some([mode, level]) = rt::match_seq(&subject)? {
                ns.set("mode", &mode)?;
                ns.set("level", &level)?;
                if ns.get("level")?.gt(5)? {
                    frame.line(218);
                    ns.set("verbose", true)?;
                    break 'match_;
                }
            }
            frame.line(219);
            if let Some([mode, _]) = rt::match_seq(&subject)? {
                ns.set("mode", &mode)?;
                frame.line(220);
                ns.set("verbose", false)?;
            }
        }
        Ok(())
    }
}

/// Executes the body of `match_basic.py` in the module `m`.
#[rt::frame(m, "<module>", 1, frame)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    // `match` statements (design §8): sequence patterns, literals, captures,
    // guards, OR and AS patterns, at module level, in functions and in a class body.
    frame.line(4);
    m.set_global("EXITS", PySet::new(py, ["north", "south", "east"])?)?;
    rt::def!(m, command(text))?;
    frame.line(27);
    for item in PyList::new(
        py,
        [
            "",
            "go north",
            "go west",
            "look",
            "take",
            "take lamp key",
            "drop a b c",
            "x y",
            "z",
        ],
    )?
    .try_iter()?
    {
        m.set_global("line", item?)?;
        frame.line(28);
        rt::builtins::print(
            py,
            (
                m.global("line")?.repr()?,
                "->",
                command(m, &m.global("line")?)?,
            ),
        )?;
        py.check_signals()?;
        frame.line(27);
    }
    rt::def!(
        m,
        stars(value),
        doc = "A named star at each position, and `*_` with the items around it."
    )?;
    frame.line(59);
    for item2 in PyList::new(
        py,
        [
            PyList::empty(py).as_any(),
            PyList::new(py, [9])?.as_any(),
            PyList::new(py, [0])?.as_any(),
            PyList::new(py, [1, 2])?.as_any(),
            PyList::new(py, [1, 2, 3])?.as_any(),
            PyTuple::new(py, [1, 2, 3, 4, 5])?.as_any(),
            &rt::builtin(py, "range")?.call1((3,))?,
            &rt::str(py, "abc"),
            PyList::new(py, [5, 6, 1])?.as_any(),
        ],
    )?
    .try_iter()?
    {
        m.set_global("value", item2?)?;
        frame.line(60);
        stars(m, &m.global("value")?)?;
        py.check_signals()?;
        frame.line(59);
    }
    rt::def!(m, literals(value))?;
    frame.line(92);
    for item3 in PyList::new(
        py,
        [
            &rt::none(py),
            &rt::bool(py, true),
            &rt::bool(py, false),
            &rt::int(py, 1),
            &rt::float(py, 1.0),
            &rt::int(py, 0),
            &rt::float(py, 2.5),
            &rt::int(py, -3),
            &rt::float(py, -3.0),
            &rt::int(py, 1).add(rt::complex(py, 0.0, 2.0))?,
            &rt::str(py, "text"),
            &rt::bytes(py, b"bytes"),
            &rt::str(py, "b"),
            &rt::int(py, 20),
            &rt::int(py, 5),
            &rt::str(py, "x"),
        ],
    )?
    .try_iter()?
    {
        m.set_global("value", item3?)?;
        frame.line(93);
        rt::builtins::print(
            py,
            (
                m.global("value")?.repr()?,
                literals(m, &m.global("value")?)?,
            ),
        )?;
        py.check_signals()?;
        frame.line(92);
    }
    rt::def!(m, nested(point))?;
    frame.line(111);
    for item4 in PyList::new(
        py,
        [
            PyList::new(py, [PyTuple::new(py, [0, 0])?.as_any(), &rt::str(py, "o")])?.as_any(),
            PyList::new(py, [PyTuple::new(py, [0, 1])?.as_any(), &rt::str(py, "u")])?.as_any(),
            PyList::new(
                py,
                [
                    PyTuple::new(py, [2, 2])?.as_any(),
                    PyList::new(py, ["a", "b"])?.as_any(),
                ],
            )?
            .as_any(),
            PyList::new(py, [PyList::new(py, [1, 2])?.as_any(), &rt::str(py, "p")])?.as_any(),
            PyList::new(py, [PyTuple::new(py, [1, 2])?.as_any(), &rt::int(py, 3)])?.as_any(),
            PyList::new(py, [1, 2])?.as_any(),
        ],
    )?
    .try_iter()?
    {
        m.set_global("point", item4?)?;
        frame.line(112);
        rt::builtins::print(py, (nested(m, &m.global("point")?)?,))?;
        py.check_signals()?;
        frame.line(111);
    }
    rt::def!(
        m,
        guard_binds(values),
        doc = "A guard that fails still leaves the captures bound (CPython stores\n\
            them before the guard runs)."
    )?;
    frame.line(132);
    guard_binds(m, PyList::new(py, [1, 2])?.as_any())?;
    frame.line(133);
    guard_binds(m, PyList::new(py, [-1, 2, 3])?.as_any())?;
    frame.line(134);
    guard_binds(m, PyList::new(py, [-5])?.as_any())?;
    rt::def!(
        m,
        counts(items),
        doc = "Captures used after the match are stored in the function's locals."
    )?;
    frame.line(153);
    counts(
        m,
        PyList::new(
            py,
            [
                PyTuple::new(py, [&rt::str(py, "add"), &rt::int(py, 5)])?.as_any(),
                PyTuple::new(py, [&rt::str(py, "sub"), &rt::int(py, 2)])?.as_any(),
                &rt::str(py, "bad"),
                PyTuple::new(py, [&rt::str(py, "add"), &rt::int(py, 10)])?.as_any(),
            ],
        )?
        .as_any(),
    )?;
    rt::def!(m, walrus_guard(value))?;
    frame.line(164);
    walrus_guard(m, PyList::new(py, [7])?.as_any())?;
    frame.line(165);
    walrus_guard(m, PyList::new(py, [2])?.as_any())?;
    rt::def!(m, loop_jumps(rows))?;
    frame.line(190);
    loop_jumps(
        m,
        PyList::new(
            py,
            [
                PyList::new(py, [&rt::str(py, "a"), &rt::int(py, 1), &rt::int(py, 2)])?.as_any(),
                PyList::new(py, [&rt::str(py, "skip"), &rt::int(py, 5)])?.as_any(),
                PyList::new(py, [500])?.as_any(),
                PyList::new(py, [&rt::str(py, "b"), &rt::int(py, 3), &rt::int(py, -1)])?.as_any(),
                PyList::new(py, ["stop"])?.as_any(),
                PyList::new(py, [&rt::str(py, "c"), &rt::int(py, 9)])?.as_any(),
            ],
        )?
        .as_any(),
    )?;
    // At module level the captures are globals.
    {
        frame.line(194);
        let subject = rt::builtin(py, "sorted")?.call1((m.global("EXITS")?,))?;
        frame.line(195);
        if let Some(([first], others, [])) = rt::match_seq_star(&subject)? {
            m.set_global("first", &first)?;
            m.set_global("others", &others)?;
            frame.line(196);
            rt::builtins::print(
                py,
                (
                    "first exit",
                    m.global("first")?,
                    "others",
                    m.global("others")?,
                ),
            )?;
        }
    }
    frame.line(198);
    for item5 in PyList::new(
        py,
        [
            PyTuple::new(py, ["hello", "world"])?.as_any(),
            PyTuple::new(py, ["bye"])?.as_any(),
            &rt::int(py, 42),
        ],
    )?
    .try_iter()?
    {
        m.set_global("message", item5?)?;
        frame.line(199);
        'match_: {
            let subject = m.global("message")?;
            frame.line(200);
            if let Some([greeting, who]) = rt::match_seq(&subject)? {
                m.set_global("greeting", &greeting)?;
                m.set_global("who", &who)?;
                if m.global("greeting")?.eq("hello")? {
                    frame.line(201);
                    rt::builtins::print(py, ("greeting", m.global("who")?))?;
                    break 'match_;
                }
            }
            frame.line(202);
            if let Some([word]) = rt::match_seq(&subject)? {
                m.set_global("word", &word)?;
                frame.line(203);
                rt::builtins::print(py, ("one word", m.global("word")?))?;
                break 'match_;
            }
            frame.line(205);
            rt::builtins::print(py, ("not a message:", m.global("message")?))?;
        }
        py.check_signals()?;
        frame.line(198);
    }
    frame.line(206);
    rt::builtins::print(
        py,
        (m.global("greeting")?, m.global("who")?, m.global("word")?),
    )?;
    frame.line(209);
    m.set_global(
        "Config",
        rt::class!(m, "Config"(), config::body, line = 209)?,
    )?;
    frame.line(223);
    rt::builtins::print(
        py,
        (
            m.global("Config")?.attr("kind")?,
            m.global("Config")?.attr("count")?,
            m.global("Config")?.attr("mode")?,
            m.global("Config")?.attr("level")?,
            m.global("Config")?.attr("verbose")?,
        ),
    )?;
    rt::def!(
        m,
        nested_functions(value),
        doc = "A generator expression or a lambda in a guard is a nested `fn` item\n\
            that the guard calls."
    )?;
    frame.line(238);
    rt::builtins::print(
        py,
        (
            nested_functions(m, PyList::new(py, ["a", "b"])?.as_any())?,
            nested_functions(
                m,
                PyList::new(py, [&rt::int(py, 1), &rt::str(py, "a")])?.as_any(),
            )?,
            nested_functions(m, PyList::new(py, [-1])?.as_any())?,
        ),
    )?;
    Ok(())
}
