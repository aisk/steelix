use pyo3::prelude::*;
use pyo3::types::IntoPyDict;
use pyo3::types::PyDict;
use pyo3::types::PyIterator;
use pyo3::types::PyList;
use pyo3::types::PySet;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// comprehensions.py:9
#[rt::frame(m, "show", 9)]
pub fn show<'py>(m: &rt::Module<'py>, exc: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    m.global("print")?
        .call1((m.global("type")?.call1((exc,))?.attr("__name__")?, exc))?;
    for frame_ in rt::iter(
        &m.global("traceback")?
            .attr("extract_tb")?
            .call1((exc.attr("__traceback__")?,))?,
    )? {
        let frame_ = frame_?;
        m.global("print")?.call1((
            " ",
            frame_.attr("name")?,
            frame_.attr("lineno")?,
            frame_.attr("line")?,
        ))?;
    }
    Ok(rt::none(m.py()))
}

// comprehensions.py:15
#[rt::frame(m, "squares", 15)]
pub fn squares<'py>(m: &rt::Module<'py>, xs: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let list = PyList::empty(m.py());
    for x in rt::iter(xs)? {
        let x = x?;
        list.append(x.mul(&x)?)?;
    }
    Ok(list.into_any())
}

// comprehensions.py:19
#[rt::frame(m, "evens", 19)]
pub fn evens<'py>(m: &rt::Module<'py>, d: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let dict = PyDict::new(m.py());
    for item in rt::iter(&d.method0("items")?)? {
        let [k, v] = rt::unpack(&item?)?;
        if v.rem(2)?.eq(0)? {
            dict.set_item(&k, &v)?;
        }
    }
    Ok(dict.into_any())
}

// comprehensions.py:23
#[rt::frame(m, "letters", 23)]
pub fn letters<'py>(m: &rt::Module<'py>, words: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let set = PySet::empty(m.py())?;
    for w in rt::iter(words)? {
        let w = w?;
        for c in rt::iter(&w)? {
            let c = c?;
            if c.method0("isalpha")?.is_truthy()? && c.ne("z")? {
                set.add(&c)?;
            }
        }
    }
    Ok(set.into_any())
}

// comprehensions.py:27
#[rt::frame(m, "pairs", 27)]
pub fn pairs<'py>(m: &rt::Module<'py>, n: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let list = PyList::empty(py);
    for i in rt::iter(&m.global("range")?.call1((n,))?)? {
        let i = i?;
        for j in rt::iter(&m.global("range")?.call1((&i,))?)? {
            let j = j?;
            if i.add(&j)?.rem(2)?.is_truthy()? {
                list.append(PyTuple::new(py, [&i, &j])?)?;
            }
        }
    }
    Ok(list.into_any())
}

// comprehensions.py:31
#[rt::frame(m, "nested", 31)]
pub fn nested<'py>(m: &rt::Module<'py>, rows: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    // The inner comprehension sees the variable of the outer one.
    let list = PyList::empty(py);
    for row in rt::iter(rows)? {
        let row = row?;
        let list2 = PyList::empty(py);
        for cell in rt::iter(&row)? {
            let cell = cell?;
            list2.append(cell.mul(row.get_item(0)?)?)?;
        }
        list.append(&list2)?;
    }
    Ok(list.into_any())
}

// comprehensions.py:36
#[rt::frame(m, "unpacking", 36)]
pub fn unpacking<'py>(
    m: &rt::Module<'py>,
    items: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let list = PyList::empty(py);
    for item in rt::iter(items)? {
        let ([a], rest, [b]) = rt::unpack_star(&item?)?;
        list.append(PyTuple::new(py, [&a, &rest, &b])?)?;
    }
    Ok(list.into_any())
}

// comprehensions.py:40
#[rt::frame(m, "attribute_target", 40)]
pub fn attribute_target<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let r#box = m.global("types")?.method0("SimpleNamespace")?;
    let list = PyList::empty(py);
    for item in rt::iter(&m.global("range")?.call1((3,))?)? {
        r#box.set_attr("value", item?)?;
        list.append(r#box.attr("value")?)?;
    }
    let values = list.into_any();
    Ok(PyTuple::new(py, [&values, &r#box.attr("value")?])?.into_any())
}

// comprehensions.py:46
#[rt::frame(m, "subscript_target", 46)]
pub fn subscript_target<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let slots = PyList::new(py, [0])?.into_any();
    let list = PyList::empty(py);
    for item in rt::iter(&rt::str(py, "ab"))? {
        let value = item?;
        slots.set_item(0, &value)?;
        list.append(slots.get_item(0)?)?;
    }
    Ok(PyTuple::new(py, [list.as_any(), &slots])?.into_any())
}

// comprehensions.py:51
#[rt::frame(m, "dict_order", 51, frame)]
pub fn dict_order<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    // comprehensions.py:52
    #[rt::frame(m, "dict_order.<locals>.key", 52)]
    fn key<'py>(m: &rt::Module<'py>, k: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        m.global("print")?.call1(("key", k))?;
        k.rem(2)
    }
    let key = rt::function!(frame, key(k))?;
    // comprehensions.py:56
    #[rt::frame(m, "dict_order.<locals>.value", 56)]
    fn value<'py>(m: &rt::Module<'py>, v: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        m.global("print")?.call1(("value", v))?;
        Ok(v.clone())
    }
    let value = rt::function!(frame, value(v))?;
    // The key is evaluated before the value; a later duplicate key wins.
    let dict = PyDict::new(m.py());
    for k in rt::iter(&m.global("range")?.call1((4,))?)? {
        let k = k?;
        dict.set_item(key.call1((&k,))?, value.call1((&k,))?)?;
    }
    Ok(dict.into_any())
}

// comprehensions.py:67
#[rt::frame(m, "shadow", 67)]
pub fn shadow<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let list = PyList::empty(py);
    for x in rt::iter(&m.global("range")?.call1((3,))?)? {
        let x = x?;
        list.append(x.mul(&x)?)?;
    }
    let squares = list.into_any();
    Ok(PyTuple::new(py, [&squares, &m.global("x")?])?.into_any())
}

// comprehensions.py:72
#[rt::frame(m, "first_iterable_outside", 72)]
pub fn first_iterable_outside<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let x = PyList::new(py, [1, 2])?.into_any();
    // The first iterable is the function's `x`, then `x` is the item.
    let list = PyList::empty(py);
    for x in rt::iter(&x)? {
        let x = x?;
        list.append(x.mul(10)?)?;
    }
    Ok(list.into_any())
}

// comprehensions.py:78
#[rt::frame(m, "late_binding", 78, frame)]
pub fn late_binding<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let i: Bound<'py, PyAny>;
    // comprehensions.py:79
    #[rt::frame(m, "late_binding.<locals>.<lambda>", 79)]
    fn lambda<'py>(m: &rt::Module<'py>, i: &rt::Cell<'py>) -> PyResult<Bound<'py, PyAny>> {
        i.get()
    }
    let funcs = {
        let i_ = rt::Cell::empty(py, "i")?;
        let list = PyList::empty(py);
        for item in rt::iter(&m.global("range")?.call1((3,))?)? {
            i_.set(item?)?;
            list.append(rt::function!(frame, lambda as "<lambda>"[&i_ as "i"]())?)?;
        }
        list.into_any()
    };
    let list = PyList::empty(py);
    for f in rt::iter(&funcs)? {
        let f = f?;
        list.append(f.call0()?)?;
    }
    Ok(list.into_any())
}

// comprehensions.py:83
#[rt::frame(m, "default_binding", 83, frame)]
pub fn default_binding<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    // comprehensions.py:84
    fn lambda<'py>(m: &rt::Module<'py>, i: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        Ok(i.clone())
    }
    let list = PyList::empty(py);
    for i in rt::iter(&m.global("range")?.call1((3,))?)? {
        let i = i?;
        list.append(rt::function!(frame, lambda as "<lambda>"(i = &i))?)?;
    }
    let funcs = list.into_any();
    let list = PyList::empty(py);
    for f in rt::iter(&funcs)? {
        let f = f?;
        list.append(f.call0()?)?;
    }
    Ok(list.into_any())
}

// comprehensions.py:88
#[rt::frame(m, "generator_in_comprehension", 88, frame)]
pub fn generator_in_comprehension<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let x: Bound<'py, PyAny>;
    // comprehensions.py:89
    #[rt::frame(m, "generator_in_comprehension.<locals>.<genexpr>", 89)]
    fn genexpr<'py>(
        m: &rt::Module<'py>,
        x: &rt::Cell<'py>,
        iter: &Bound<'py, PyIterator>,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        if let Some(y) = rt::next(iter)? {
            return Ok(Some(x.get()?.add(&y)?));
        }
        Ok(None)
    }
    let x_ = rt::Cell::empty(py, "x")?;
    let list = PyList::empty(py);
    for item in rt::iter(PyTuple::new(py, [10, 20])?.as_any())? {
        x_.set(item?)?;
        list.append(m.global("list")?.call1((rt::genexpr!(
            frame,
            genexpr[&x_ as "x"](m.global("range")?.call1((2,))?.try_iter()?)
        )?,))?)?;
    }
    Ok(list.into_any())
}

// comprehensions.py:92
#[rt::frame(m, "stop_iteration", 92, frame)]
pub fn stop_iteration<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    // comprehensions.py:93
    #[rt::frame(m, "stop_iteration.<locals>.boom", 93, frame)]
    fn boom<'py>(m: &rt::Module<'py>, v: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        if v.eq(2)? {
            return Err(frame.raise(
                m.global("StopIteration")?
                    .call1(("not converted in a comprehension",))?,
            ));
        }
        Ok(v.clone())
    }
    let boom = rt::function!(frame, boom(v))?;
    match (|| -> PyResult<Bound<'py, PyAny>> {
        let print = m.global("print")?;
        let list = PyList::empty(py);
        for v in rt::iter(&m.global("range")?.call1((4,))?)? {
            let v = v?;
            list.append(boom.call1((&v,))?)?;
        }
        print.call1((&list,))
    })() {
        Ok(_) => {}
        Err(err) if frame.matches(&err, || m.global("StopIteration"))? => {
            let exc = frame.catch(err);
            m.global("print")?.call1(("StopIteration", &exc))?;
        }
        Err(err) => return Err(err),
    }
    Ok(rt::none(py))
}

// comprehensions.py:104
#[rt::frame(m, "multiline", 104)]
pub fn multiline<'py>(m: &rt::Module<'py>, xs: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let list = PyList::empty(py);
    for x in rt::iter(xs)? {
        let x = x?;
        if x.ne(1)? {
            list.append(rt::int(py, 10).floor_div(&x)?)?;
        }
    }
    Ok(list.into_any())
}

// comprehensions.py:112
#[rt::frame(m, "failing_condition", 112)]
pub fn failing_condition<'py>(
    m: &rt::Module<'py>,
    xs: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let list = PyList::empty(py);
    for x in rt::iter(xs)? {
        let x = x?;
        if rt::int(py, 10).floor_div(&x)?.is_truthy()? {
            list.append(&x)?;
        }
    }
    Ok(list.into_any())
}

// comprehensions.py:125
#[rt::frame(m, "failing_iteration", 125)]
pub fn failing_iteration<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let list = PyList::empty(m.py());
    for x in rt::iter(&m.global("Bad")?.call0()?)? {
        let x = x?;
        list.append(&x)?;
    }
    Ok(list.into_any())
}

pub mod bad {
    use super::*;

    // comprehensions.py:117
    /// The body of `class Bad`, run in its namespace `ns`.
    #[rt::frame(ns, "Bad", 117)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::def!(ns, __iter__(slf as "self"))?;
        rt::def!(ns, __next__(slf as "self"))?;
        Ok(())
    }

    // comprehensions.py:118
    pub fn __iter__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(slf.clone())
    }

    // comprehensions.py:121
    #[rt::frame(m, "Bad.__next__", 121, frame)]
    pub fn __next__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(frame.raise(m.global("ValueError")?.call1(("next failed",))?))
    }
}

pub mod config {
    use super::*;

    // comprehensions.py:130
    /// The body of `class Config`, run in its namespace `ns`.
    #[rt::frame(ns, "Config", 130, frame)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        let py = m.py();
        ns.set("size", 3)?;
        let list = PyList::empty(py);
        for i in rt::iter(&ns.get("range")?.call1((ns.get("size")?,))?)? {
            let i = i?;
            list.append(i.mul(2)?)?;
        }
        ns.set("doubled", list)?;
        match (|| -> PyResult<Bound<'py, PyAny>> {
            let list = PyList::empty(py);
            for i in rt::iter(&ns.get("range")?.call1((2,))?)? {
                let i = i?;
                list.append(m.global("size")?)?;
            }
            Ok(list.into_any())
        })() {
            Ok(value) => {
                ns.set("broken", &value)?;
            }
            Err(err) if frame.matches(&err, || ns.get("NameError"))? => {
                let exception = frame.catch(err);
                ns.set("exc", &exception)?;
                ns.get("print")?.call1(("class body:", ns.get("exc")?))?;
                ns.unbind("exc")?;
            }
            Err(err) => return Err(err),
        }
        let dict = PyDict::new(py);
        for n in rt::iter(PyTuple::new(py, ["a", "bb"])?.as_any())? {
            let n = n?;
            dict.set_item(&n, m.global("len")?.call1((&n,))?)?;
        }
        ns.set("names", dict)?;
        Ok(())
    }
}

pub mod base {
    use super::*;

    // comprehensions.py:140
    /// The body of `class Base`, run in its namespace `ns`.
    #[rt::frame(ns, "Base", 140)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::def!(ns, items(slf as "self"))?;
        Ok(())
    }

    // comprehensions.py:141
    #[rt::frame(m, "Base.items", 141)]
    pub fn items<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        Ok(PyList::new(m.py(), [1, 2])?.into_any())
    }
}

pub mod derived {
    use super::*;

    // comprehensions.py:145
    /// The body of `class Derived`, run in its namespace `ns`.
    #[rt::frame(ns, "Derived", 145)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        let __class__ = ns.class_cell();
        ns.set(
            "_Derived__secret",
            rt::function!(ns, __secret(slf as "self"))?,
        )?;
        rt::def!(ns, values[&__class__](slf as "self"))?;
        rt::def!(ns, secrets(slf as "self"))?;
        rt::def!(ns, supers[&__class__](slf as "self"))?;
        Ok(())
    }

    // comprehensions.py:146
    pub fn __secret<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(rt::str(m.py(), "secret"))
    }

    // comprehensions.py:149
    #[rt::frame(m, "Derived.values", 149)]
    pub fn values<'py>(
        m: &rt::Module<'py>,
        __class__: &rt::Cell<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        let list = PyList::empty(py);
        for v in rt::iter(&rt::super_(__class__, slf)?.method0("items")?)? {
            let v = v?;
            list.append(v.mul(2)?)?;
        }
        let list2 = PyList::empty(py);
        for underscore in rt::iter(&rt::str(py, "x"))? {
            let underscore = underscore?;
            list2.append(__class__.get()?.attr("__name__")?)?;
        }
        list.add(&list2)
    }

    // comprehensions.py:152
    #[rt::frame(m, "Derived.secrets", 152)]
    pub fn secrets<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let list = PyList::empty(m.py());
        for underscore in rt::iter(&m.global("range")?.call1((2,))?)? {
            let underscore = underscore?;
            list.append(slf.method0("_Derived__secret")?)?;
        }
        Ok(list.into_any())
    }

    // comprehensions.py:155
    #[rt::frame(m, "Derived.supers", 155)]
    pub fn supers<'py>(
        m: &rt::Module<'py>,
        __class__: &rt::Cell<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        let list = PyList::empty(py);
        for underscore in rt::iter(&rt::str(py, "x"))? {
            let underscore = underscore?;
            list.append(rt::super_(__class__, slf)?.method0("items")?)?;
        }
        Ok(list.into_any())
    }
}

/// Executes the body of `comprehensions.py` in the module `m`.
#[rt::frame(m, "<module>", 1, frame)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    m.set_global(
        "__doc__",
        "List, set and dict comprehensions run inline (PEP 709): their variables\n\
            do not leak, the first iterable is evaluated outside, and errors are\n\
            reported in the enclosing frame on the line of the failing part.",
    )?;
    m.set_global("traceback", rt::import(m, "traceback")?)?;
    m.set_global("types", rt::import(m, "types")?)?;
    rt::def!(m, show(exc))?;
    rt::def!(m, squares(xs))?;
    rt::def!(m, evens(d))?;
    rt::def!(m, letters(words))?;
    rt::def!(m, pairs(n))?;
    rt::def!(m, nested(rows))?;
    rt::def!(m, unpacking(items))?;
    rt::def!(m, attribute_target())?;
    rt::def!(m, subscript_target())?;
    rt::def!(m, dict_order())?;
    m.set_global("x", "global x")?;
    rt::def!(m, shadow())?;
    rt::def!(m, first_iterable_outside())?;
    rt::def!(m, late_binding())?;
    rt::def!(m, default_binding())?;
    rt::def!(m, generator_in_comprehension())?;
    rt::def!(m, stop_iteration())?;
    rt::def!(m, multiline(xs))?;
    rt::def!(m, failing_condition(xs))?;
    m.set_global("Bad", rt::class!(m, "Bad"(), bad::body, line = 117)?)?;
    rt::def!(m, failing_iteration())?;
    m.set_global(
        "Config",
        rt::class!(m, "Config"(), config::body, line = 130)?,
    )?;
    m.set_global("Base", rt::class!(m, "Base"(), base::body, line = 140)?)?;
    m.set_global(
        "Derived",
        rt::class!(m, "Derived"(m.global("Base")?), derived::body, line = 145)?,
    )?;
    m.global("print")?.call1((
        m.global("squares")?.call1((PyList::new(py, [1, 2, 3])?,))?,
        m.global("evens")?
            .call1(([("a", 1), ("b", 2), ("c", 4)].into_py_dict(py)?,))?,
    ))?;
    m.global("print")?.call1((
        m.global("sorted")?.call1((m
            .global("letters")?
            .call1((PyList::new(py, ["ab1", "zc"])?,))?,))?,
        m.global("pairs")?.call1((4,))?,
        m.global("nested")?.call1((PyList::new(
            py,
            [
                PyList::new(py, [1, 2])?.as_any(),
                PyList::new(py, [3, 4])?.as_any(),
            ],
        )?,))?,
    ))?;
    m.global("print")?.call1((
        m.global("unpacking")?.call1((PyList::new(
            py,
            [
                PyTuple::new(py, [1, 2, 3, 4])?.as_any(),
                PyTuple::new(py, [5, 6])?.as_any(),
            ],
        )?,))?,
        m.global("attribute_target")?.call0()?,
        m.global("subscript_target")?.call0()?,
    ))?;
    m.global("print")?
        .call1((m.global("dict_order")?.call0()?,))?;
    m.global("print")?.call1((
        m.global("shadow")?.call0()?,
        m.global("x")?,
        m.global("first_iterable_outside")?.call0()?,
    ))?;
    m.global("print")?.call1((
        m.global("late_binding")?.call0()?,
        m.global("default_binding")?.call0()?,
        m.global("generator_in_comprehension")?.call0()?,
    ))?;
    m.global("stop_iteration")?.call0()?;
    for item in rt::iter(
        PyTuple::new(
            py,
            [
                PyTuple::new(
                    py,
                    [
                        &m.global("multiline")?,
                        PyList::new(py, [1, 2, 0])?.as_any(),
                    ],
                )?
                .as_any(),
                PyTuple::new(
                    py,
                    [
                        &m.global("failing_condition")?,
                        PyList::new(py, [1, 0])?.as_any(),
                    ],
                )?
                .as_any(),
            ],
        )?
        .as_any(),
    )? {
        let [call, arg] = rt::unpack(&item?)?;
        m.set_global("call", call)?;
        m.set_global("arg", arg)?;
        match (|| m.global("call")?.call1((m.global("arg")?,)))() {
            Ok(_) => {}
            Err(err) if frame.matches(&err, || m.global("ZeroDivisionError"))? => {
                let exception = frame.catch(err);
                m.set_global("exc", &exception)?;
                m.global("show")?.call1((m.global("exc")?,))?;
                m.unbind_global("exc")?;
            }
            Err(err) => return Err(err),
        }
    }
    match (|| m.global("failing_iteration")?.call0())() {
        Ok(_) => {}
        Err(err) if frame.matches(&err, || m.global("ValueError"))? => {
            let exception = frame.catch(err);
            m.set_global("exc", &exception)?;
            m.global("show")?.call1((m.global("exc")?,))?;
            m.unbind_global("exc")?;
        }
        Err(err) => return Err(err),
    }
    m.global("print")?.call1((
        m.global("Config")?.attr("doubled")?,
        m.global("Config")?.attr("names")?,
        m.global("Config")?.attr("size")?,
    ))?;
    m.set_global("d", m.global("Derived")?.call0()?)?;
    m.global("print")?.call1((
        m.global("d")?.method0("values")?,
        m.global("d")?.method0("secrets")?,
        m.global("d")?.method0("supers")?,
    ))?;
    let list = PyList::empty(py);
    for n in rt::iter(&m.global("range")?.call1((3,))?)? {
        let n = n?;
        if n.is_truthy()? {
            list.append(&n)?;
        }
    }
    m.set_global("module_level", list)?;
    {
        let print = m.global("print")?;
        let module_level = m.global("module_level")?;
        let list = PyList::empty(py);
        for w in rt::iter(PyTuple::new(py, ["ab", "c"])?.as_any())? {
            let w = w?;
            let list2 = PyList::empty(py);
            for c in rt::iter(&w)? {
                let c = c?;
                list2.append(&c)?;
            }
            list.append(&list2)?;
        }
        let set = PySet::empty(py)?;
        for n in rt::iter(&m.global("range")?.call1((5,))?)? {
            let n = n?;
            set.add(n.rem(2)?)?;
        }
        print.call1((module_level, list, set))?;
    }
    let print = m.global("print")?;
    let value = m.globals().contains("n")?;
    let dict = PyDict::new(py);
    for item2 in rt::iter(
        &m.global("zip")?
            .call1(("ab", m.global("range")?.call1((2,))?))?,
    )? {
        let [k, v] = rt::unpack(&item2?)?;
        dict.set_item(&k, &v)?;
    }
    print.call1((value, dict))?;
    Ok(())
}
