use pyo3::exceptions::PyAssertionError;
use pyo3::exceptions::PyAttributeError;
use pyo3::exceptions::PyException;
use pyo3::exceptions::PyKeyError;
use pyo3::exceptions::PyRuntimeError;
use pyo3::exceptions::PyStopIteration;
use pyo3::exceptions::PyTypeError;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::pyclass::CompareOp;
use pyo3::types::IntoPyDict;
use pyo3::types::PyDict;
use pyo3::types::PyIterator;
use pyo3::types::PyList;
use pyo3::types::PySet;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// container_locals.py:22
#[rt::frame(m, "report", 22, frame)]
pub fn report<'py>(
    m: &rt::Module<'py>,
    f: &Bound<'py, PyAny>,
    args: &Bound<'py, PyTuple>,
) -> PyResult<()> {
    let py = m.py();
    match (|| rt::builtins::print(py, (f.attr("__name__")?, "->", rt::call!(f, *args)?)))() {
        Ok(_) => {}
        Err(err) if err.is_instance_of::<PyException>(py) => {
            let e = frame.catch(err);
            rt::builtins::print(
                py,
                (
                    f.attr("__name__")?,
                    "raised",
                    e.get_type().attr("__name__")?,
                    &e,
                ),
            )?;
        }
        Err(err) => return Err(err),
    }
    Ok(())
}

// --- qualifying locals ---
// container_locals.py:32
#[rt::frame(m, "build", 32)]
pub fn build<'py>(m: &rt::Module<'py>, n: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let xs = PyList::empty(py).into_any();
    let d = PyDict::new(py).into_any();
    let s = PySet::new(py, [0])?;
    for i in rt::iter(&rt::builtins::range(py, (n,))?)? {
        let i = i?;
        xs.method1("append", (&i,))?;
        xs.attr("append")?.call1((i.mul(&i)?,))?;
        d.set_item(&i, i.str()?)?;
        s.add(i.rem(3)?)?;
    }
    rt::builtins::print(
        py,
        (
            xs.len()?,
            d.len()?,
            s.len(),
            xs.contains(2)?,
            d.contains(7)?,
            s.as_any().contains(1)?,
            !s.as_any().contains(9)?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            xs.get_item(0)?,
            xs.get_item(-1)?,
            xs.get_item(rt::slice(py, 1, 3)?)?,
            d.get_item(0)?,
            d.method1("get", (99,))?,
            d.method1("get", (99, "none"))?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            rt::builtins::sorted(py, (&s,))?,
            rt::builtins::sorted(py, (d.method0("items")?,))?,
            xs.method1("index", (4,))?,
            xs.method1("count", (1,))?,
        ),
    )?;
    xs.set_item(0, "first")?;
    d.set_item("k", &xs)?;
    d.del_item(0)?;
    xs.del_item(1)?;
    xs.attr("extend")?.call1((PyList::new(py, [7, 8])?,))?;
    xs.method1("insert", (0, "zero"))?;
    let value = xs.method0("pop")?;
    let value2 = d.method1("pop", ("k",))?.is(&xs);
    let item = s.method0("pop")?;
    rt::builtins::print(
        py,
        (
            value,
            value2,
            PyTuple::new(py, [0, 1, 2])?.contains(item)?,
            &xs,
        ),
    )?;
    rt::call!(xs.attr("sort")?, key = rt::builtin(py, "str")?)?;
    xs.method0("reverse")?;
    s.method1("discard", (1,))?;
    s.attr("update")?.call1((PyList::new(py, [5, 6])?,))?;
    rt::call!(d.attr("update")?, a = 1)?;
    d.attr("setdefault")?
        .call1(("b", PyList::empty(py)))?
        .method1("append", (2,))?;
    Ok(PyTuple::new(py, [&xs, &d, &rt::builtins::sorted(py, (&s,))?])?.into_any())
}

// container_locals.py:60
#[rt::frame(m, "comprehensions", 60)]
pub fn comprehensions<'py>(
    m: &rt::Module<'py>,
    n: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let list = PyList::empty(py);
    for i in rt::iter(&rt::builtins::range(py, (n,))?)? {
        let i = i?;
        list.append(i.mul(&i)?)?;
    }
    let squares = list;
    let dict = PyDict::new(py);
    for i in rt::iter(&rt::builtins::range(py, (n,))?)? {
        let i = i?;
        dict.set_item(i.mul(&i)?, &i)?;
    }
    let by_value = dict;
    let set = PySet::empty(py)?;
    for i in rt::iter(&rt::builtins::range(py, (n,))?)? {
        let i = i?;
        if i.rem(2)?.is_truthy()? {
            set.add(&i)?;
        }
    }
    let odd = set;
    squares.append(-1)?;
    odd.add(-1)?;
    by_value.set_item(-1, py.None())?;
    let list = PyList::empty(py);
    for i in rt::iter(&squares)? {
        let i = i?;
        if i.ge(0)? {
            let list2 = PyList::empty(py);
            for j in rt::iter(&rt::builtins::range(py, (&i,))?)? {
                let j = j?;
                list2.append(&j)?;
            }
            list.append(&list2)?;
        }
    }
    let nested = list;
    let value = rt::builtins::sorted(py, (&odd,))?;
    let list = PyList::empty(py);
    for x in rt::iter(&squares)? {
        let x = x?;
        if odd.as_any().contains(&x)? {
            list.append(&x)?;
        }
    }
    Ok(PyTuple::new(
        py,
        [
            squares.as_any(),
            by_value.as_any(),
            &value,
            nested.as_any(),
            list.as_any(),
        ],
    )?
    .into_any())
}

// container_locals.py:71
#[rt::frame(m, "unpacking_displays", 71)]
pub fn unpacking_displays<'py>(
    m: &rt::Module<'py>,
    a: &Bound<'py, PyAny>,
    b: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let joined = rt::list!(py, *a, *b)?;
    let merged = rt::dict!(py, **b, ("z", 26))?;
    let union = rt::set!(py, *a, *b)?;
    joined.append(merged.len())?;
    union.add("u")?;
    Ok(PyTuple::new(
        py,
        [
            joined.as_any(),
            merged.as_any(),
            &rt::builtins::sorted(py, (&union,))?,
        ],
    )?
    .into_any())
}

// container_locals.py:80
#[rt::frame(m, "aliases", 80)]
pub fn aliases<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let mut xs = PyList::new(py, [1])?;
    let mut ys = xs.clone();
    let zs = ys.clone();
    zs.append(2)?;
    ys.append(3)?;
    rt::builtins::print(py, (xs.is(&ys), ys.is(&zs), &xs))?;
    ys = PyList::new(py, [9])?;
    ys.append(10)?;
    xs = ys.clone();
    xs.append(11)?;
    Ok(PyTuple::new(py, [&xs, &ys, &zs])?.into_any())
}

// container_locals.py:94
#[rt::frame(m, "branches", 94)]
pub fn branches<'py>(m: &rt::Module<'py>, flag: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let mut out = if flag.is_truthy()? {
        PyList::empty(py)
    } else {
        PyList::new(py, ["else"])?
    };
    out.append(flag)?;
    while out.len() < 4 {
        out = rt::list!(py, *out, out.len())?;
        py.check_signals()?;
    }
    Ok(out.into_any())
}

// container_locals.py:105
#[rt::frame(m, "used_as_values", 105)]
pub fn used_as_values<'py>(
    m: &rt::Module<'py>,
    xs_in: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let xs = PyList::new(py, [3, 1, 2])?;
    let d = [("sep", "-"), ("end", "!\n")].into_py_dict(py)?;
    let s = PySet::new(py, [1])?;
    rt::call!(rt::builtin(py, "print")?, *xs, **d)?;
    rt::builtins::print(
        py,
        (rt::fstring(
            py,
            (
                rt::format(&xs, "")?,
                " ",
                d.repr()?,
                " ",
                rt::format(s.str()?.as_any(), ">5")?,
                " ",
                rt::format(&rt::int(py, xs.len()), "03")?,
            ),
        )?,),
    )?;
    rt::builtins::print(
        py,
        (
            xs.str()?,
            d.repr()?,
            xs.is_truthy()?,
            !s.is_truthy()?,
            xs.rich_compare(PyList::new(py, [3, 1, 2])?, CompareOp::Eq)?,
            xs.rich_compare(xs_in, CompareOp::Ne)?,
            xs.rich_compare(PyList::new(py, [4])?, CompareOp::Lt)?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            xs.add(PyList::new(py, [4])?)?,
            xs.mul(2)?,
            s.bitor(PySet::new(py, [2])?)?,
            s.bitand(PySet::new(py, [1])?)?,
            d.bitor([("x", 1)].into_py_dict(py)?)?,
            xs.get_type().is(rt::builtin(py, "list")?),
            s.get_type().attr("__name__")?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            xs.is_instance(&rt::builtin(py, "list")?)?,
            d.is_instance(
                PyTuple::new(py, [&rt::builtin(py, "list")?, &rt::builtin(py, "dict")?])?.as_any(),
            )?,
            rt::builtins::hash(py, (rt::builtins::tuple(py, (&xs,))?,))?.rich_compare(
                rt::builtins::hash(py, (PyTuple::new(py, [3, 1, 2])?,))?,
                CompareOp::Eq,
            )?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            rt::builtins::sorted(py, (&xs,))?,
            rt::builtins::max(py, (&xs,))?,
            rt::builtins::sum(py, (&xs,))?,
            rt::builtins::list(py, (rt::builtins::reversed(py, (&xs,))?,))?,
            rt::builtins::dict(py, (&d,))?,
            rt::builtins::set(py, (&s,))?,
            rt::builtins::tuple(py, (&xs,))?,
        ),
    )?;
    let [a, b, c] = rt::unpack(&xs)?;
    let ([first], rest, []) = rt::unpack_star(&xs)?;
    rt::builtins::print(
        py,
        (
            &a,
            &b,
            &c,
            &first,
            &rest,
            if xs.is_truthy()? {
                xs.clone().into_any()
            } else {
                rt::none(py)
            },
            if xs.is_truthy()? {
                xs.clone().into_any()
            } else {
                rt::none(py)
            },
            if xs.is_truthy()? {
                d.clone().into_any()
            } else {
                xs.clone().into_any()
            },
        ),
    )?;
    let holder = PyList::new(py, [xs.as_any(), d.as_any(), s.as_any()])?.into_any();
    let pair = PyTuple::new(py, [xs.as_any(), s.as_any()])?.into_any();
    let mapping = [
        (&rt::str(py, "xs"), xs.as_any()),
        (&rt::int(py, 1), d.as_any()),
    ]
    .into_py_dict(py)?;
    rt::builtins::print(py, (&holder, &pair, &mapping, holder.get_item(0)?.is(&xs)))?;
    Ok(xs.into_any())
}

// container_locals.py:125
#[rt::frame(m, "echo", 125)]
pub fn echo<'py>(
    m: &rt::Module<'py>,
    args: &Bound<'py, PyTuple>,
    kwargs: &Bound<'py, PyDict>,
) -> PyResult<Bound<'py, PyAny>> {
    Ok(PyTuple::new(m.py(), [args.as_any(), kwargs.as_any()])?.into_any())
}

// container_locals.py:129
#[rt::frame(m, "passing", 129)]
pub fn passing<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let xs = PyList::new(py, [1, 2])?;
    let d = [("k", "v")].into_py_dict(py)?;
    rt::builtins::print(
        py,
        (
            echo(
                m,
                &PyTuple::new(py, [xs.as_any(), d.as_any()])?,
                &PyDict::new(py),
            )?,
            rt::call!(m.global("echo")?, *xs, **d)?,
            echo(
                m,
                &PyTuple::new(py, [&xs])?,
                &[("key", &d)].into_py_dict(py)?,
            )?,
            echo(m, &PyTuple::empty(py), &[("xs", &xs)].into_py_dict(py)?)?,
        ),
    )?;
    rt::builtins::print(py, (takes(m, &xs, &d)?, takes(m, &d, &xs)?))?;
    let it = xs.try_iter()?.into_any();
    rt::builtins::print(
        py,
        (
            rt::builtins::next(py, (&it,))?,
            rt::builtins::next(py, (&it,))?,
            rt::builtins::next(py, (&it, "done"))?,
        ),
    )?;
    Ok(d.into_any())
}

// container_locals.py:139
#[rt::frame(m, "takes", 139)]
pub fn takes<'py>(
    m: &rt::Module<'py>,
    a: &Bound<'py, PyAny>,
    b: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let a_is = a.get_type().attr("__name__")?;
    Ok(PyTuple::new(py, [&a_is, &rt::int(py, b.len()?)])?.into_any())
}

// container_locals.py:144
#[rt::frame(m, "iteration", 144, frame)]
pub fn iteration<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let xs = PyList::new(py, [1, 2, 3])?.into_any();
    let seen = PyList::empty(py);
    let mut x: Option<Bound<'py, PyAny>> = None;
    for item in rt::iter(&xs)? {
        x = Some(item?);
        seen.append(rt::local(&x, "x")?)?;
        if rt::local(&x, "x")?.lt(3)? && xs.len()? < 6 {
            xs.attr("append")?.call1((rt::local(&x, "x")?.mul(10)?,))?;
        }
    }
    rt::builtins::print(py, (&seen, &xs))?;
    let s = PySet::new(py, [1, 2])?;
    match (|| -> PyResult<()> {
        for item2 in rt::iter(&s)? {
            x = Some(item2?);
            s.add(rt::local(&x, "x")?.add(10)?)?;
        }
        Ok(())
    })() {
        Ok(()) => {}
        Err(err) if err.is_instance_of::<PyRuntimeError>(py) => {
            let e = frame.catch(err);
            rt::builtins::print(py, ("RuntimeError", &e))?;
        }
        Err(err) => return Err(err),
    }
    let d = [(1, "a")].into_py_dict(py)?;
    match (|| -> PyResult<()> {
        for k in rt::iter(&d)? {
            let k = k?;
            d.set_item(k.add(1)?, "b")?;
        }
        Ok(())
    })() {
        Ok(()) => {}
        Err(err) if err.is_instance_of::<PyRuntimeError>(py) => {
            let e = frame.catch(err);
            rt::builtins::print(py, ("RuntimeError", &e))?;
        }
        Err(err) => return Err(err),
    }
    let mut i: Bound<'py, PyAny>;
    for item3 in rt::iter(&rt::builtins::enumerate(py, (&xs,))?)? {
        let [i2, x2] = rt::unpack(&item3?)?;
        i = i2;
        x = Some(x2);
        if rt::local(&x, "x")?.eq(2)? {
            xs.del_item(&i)?;
        }
    }
    Ok(PyTuple::new(py, [&xs, &rt::builtins::sorted(py, (&s,))?, &d])?.into_any())
}

// container_locals.py:170
#[rt::frame(m, "lifetimes", 170)]
pub fn lifetimes<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let mut xs = PyList::new(py, [&m.global("Loud")?.call1(("a",))?])?;
    xs.append(m.global("Loud")?.call1(("b",))?)?;
    rt::builtins::print(py, ("rebinding",))?;
    xs = PyList::new(py, [&m.global("Loud")?.call1(("c",))?])?;
    rt::builtins::print(py, ("rebound",))?;
    let d = [("k", &m.global("Loud")?.call1(("d",))?)].into_py_dict(py)?;
    d.set_item("k", m.global("Loud")?.call1(("e",))?)?;
    rt::builtins::print(py, ("replaced",))?;
    let s = PySet::new(py, [1])?;
    s.add(m.global("Loud")?.call1(("f",))?)?;
    let mut ys = xs.clone();
    xs = PyList::empty(py);
    rt::builtins::print(py, ("still alive", &ys))?;
    ys = xs.clone();
    rt::builtins::print(py, ("gone",))?;
    xs.append(m.global("Loud")?.call1(("g",))?)?;
    rt::int(py, xs.len()).add(d.len())?.add(s.len())
}

// container_locals.py:190
#[rt::frame(m, "evaluation_order", 190, frame)]
pub fn evaluation_order<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let seen = rt::Cell::empty(py, "seen")?;
    let xs = PyList::empty(py);
    // container_locals.py:193
    #[rt::frame(m, "evaluation_order.<locals>.note", 193)]
    fn note<'py>(
        m: &rt::Module<'py>,
        seen: &rt::Cell<'py>,
        x: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::builtins::print(m.py(), ("note", x, seen.get()?.len()?))?;
        seen.get()?.method1("append", (x,))?;
        Ok(x.clone())
    }
    let note = rt::function!(frame, note[&seen](x))?;
    seen.set(PyList::empty(py))?;
    xs.append(note.call1((1,))?)?;
    xs.append(note.call1((&xs,))?)?;
    let d = PyDict::new(py);
    let value = note.call1(("value",))?;
    d.set_item(note.call1(("key",))?, value)?;
    let s = rt::builtins::set(py, ())?;
    s.attr("add")?.call1((note.call1(("s",))?,))?;
    Ok(PyTuple::new(py, [&rt::int(py, xs.len()), &d, &s, &seen.get()?])?.into_any())
}

// container_locals.py:208
#[rt::frame(m, "attempt", 208, frame)]
pub fn attempt<'py>(
    m: &rt::Module<'py>,
    label: &Bound<'py, PyAny>,
    f: &Bound<'py, PyAny>,
) -> PyResult<()> {
    let py = m.py();
    match (|| rt::builtins::print(py, (label, f.call0()?)))() {
        Ok(_) => {}
        Err(err) if err.is_instance_of::<PyException>(py) => {
            let e = frame.catch(err);
            rt::builtins::print(py, (label, e.get_type().attr("__name__")?, &e))?;
        }
        Err(err) => return Err(err),
    }
    Ok(())
}

// container_locals.py:215
#[rt::frame(m, "protocols", 215, frame)]
pub fn protocols<'py>(
    m: &rt::Module<'py>,
    flag: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let xs = PyList::new(py, [1, 2])?;
    let ys = PyList::new(py, [3])?;
    let d = [("k", 1)].into_py_dict(py)?;
    let s = PySet::new(py, [1])?;
    {
        let value = xs.rich_compare(&ys, CompareOp::Eq)?;
        let result = PyList::new(py, [1, 2])?.rich_compare(&xs, CompareOp::Eq)?;
        let value2 = if result.is_truthy()? {
            xs.rich_compare(&ys, CompareOp::Ne)?
        } else {
            result
        };
        let result2 = xs.rich_compare(&ys, CompareOp::Lt)?;
        rt::builtins::print(
            py,
            (
                xs.is_none(),
                !xs.is(&ys),
                value,
                value2,
                if result2.is_truthy()? {
                    ys.rich_compare(&ys, CompareOp::Le)?
                } else {
                    result2
                },
                d.rich_compare([("k", 1)].into_py_dict(py)?, CompareOp::Eq)?,
            ),
        )?;
    }
    rt::builtins::print(
        py,
        (
            !xs.is_truthy()?,
            d.is_truthy()?,
            if xs.is_truthy()? {
                ys.clone().into_any()
            } else {
                xs.clone().into_any()
            },
            if s.is_truthy()? {
                s.clone().into_any()
            } else {
                xs.clone().into_any()
            },
            if flag.is_truthy()? {
                xs.clone().into_any()
            } else {
                d.clone().into_any()
            },
            rt::int(py, xs.len()).rich_compare(1, CompareOp::Gt)?,
            rt::int(py, s.len()).rich_compare(d.len(), CompareOp::Eq)?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            xs.method0("__len__")?,
            xs.attr("__class__")?.attr("__name__")?,
            d.method0("keys")?.bitand(&s)?,
            s.method1("__contains__", (1,))?,
            d.attr("__doc__")?.get_item(rt::slice(py, py.None(), 4)?)?,
        ),
    )?;
    let list = PyList::empty(py);
    for underscore in rt::iter(&rt::builtins::range(py, (2,))?)? {
        let underscore = underscore?;
        list.append(&xs)?;
    }
    let dict = PyDict::new(py);
    for v in rt::iter(PyTuple::new(py, [&xs, &ys])?.as_any())? {
        let v = v?;
        dict.set_item(v.len()?, &v)?;
    }
    let n = xs.clone().into_any();
    rt::builtins::print(py, (list, dict, n.is(&xs), &n))?;
    // noqa: UP031
    rt::builtins::print(
        py,
        (
            rt::fstring(
                py,
                (
                    rt::ascii(&xs)?,
                    " ",
                    rt::format(d.repr()?.as_any(), ">12")?,
                    " ",
                    rt::format(&s, "")?,
                ),
            )?,
            rt::str(py, "%s %r").rem(PyTuple::new(py, [xs.as_any(), d.as_any()])?)?,
            xs.str()?.add(s.repr()?)?,
        ),
    )?;
    while ys.is_truthy()? {
        rt::builtins::print(py, ("pop", ys.method0("pop")?))?;
        py.check_signals()?;
    }
    if d.is_truthy()? {
        rt::builtins::print(py, ("dict is true",))?;
    }
    if !xs.is_truthy()? {
        return Err(frame.raise(PyAssertionError::new_err((d.clone().into_any().unbind(),))));
    }
    // container_locals.py:230
    fn lambda<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
        Ok(rt::int(m.py(), 0))
    }
    for item in rt::iter(
        PyList::new(
            py,
            [PyTuple::new(
                py,
                [
                    &rt::str(py, "call"),
                    &rt::function!(frame, lambda as "<lambda>"())?,
                ],
            )?
            .as_any()],
        )?
        .as_any(),
    )? {
        let [label, f] = rt::unpack(&item?)?;
        attempt(m, &label, &f)?;
    }
    match xs.call0() {
        Ok(_) => {}
        Err(err) if err.is_instance_of::<PyTypeError>(py) => {
            let e = frame.catch(err);
            rt::builtins::print(py, ("TypeError", &e))?;
        }
        Err(err) => return Err(err),
    }
    match (|| rt::builtins::print(py, (xs.attr("missing")?,)))() {
        Ok(_) => {}
        Err(err) if err.is_instance_of::<PyAttributeError>(py) => {
            let e = frame.catch(err);
            rt::builtins::print(py, ("AttributeError", &e))?;
        }
        Err(err) => return Err(err),
    }
    match (|| -> PyResult<()> {
        d.set_attr("attr", 1)?;
        Ok(())
    })() {
        Ok(()) => {}
        Err(err) if err.is_instance_of::<PyAttributeError>(py) => {
            let e = frame.catch(err);
            rt::builtins::print(py, ("AttributeError", &e))?;
        }
        Err(err) => return Err(err),
    }
    match (|| -> PyResult<()> {
        s.del_attr("attr")?;
        Ok(())
    })() {
        Ok(()) => {}
        Err(err) if err.is_instance_of::<PyAttributeError>(py) => {
            let e = frame.catch(err);
            rt::builtins::print(py, ("AttributeError", &e))?;
        }
        Err(err) => return Err(err),
    }
    match (|| rt::builtins::print(py, (xs.neg()?,)))() {
        Ok(_) => {}
        Err(err) if err.is_instance_of::<PyTypeError>(py) => {
            let e = frame.catch(err);
            rt::builtins::print(py, ("TypeError", &e))?;
        }
        Err(err) => return Err(err),
    }
    match (|| rt::builtins::print(py, (s.as_any().add(1)?,)))() {
        Ok(_) => {}
        Err(err) if err.is_instance_of::<PyTypeError>(py) => {
            let e = frame.catch(err);
            rt::builtins::print(py, ("TypeError", &e))?;
        }
        Err(err) => return Err(err),
    }
    match (|| rt::builtins::print(py, (xs.matmul(&d)?, xs.sub(&s)?)))() {
        Ok(_) => {}
        Err(err) if err.is_instance_of::<PyTypeError>(py) => {
            let e = frame.catch(err);
            rt::builtins::print(py, ("TypeError", &e))?;
        }
        Err(err) => return Err(err),
    }
    match (|| rt::builtins::print(py, ([(&xs, 1)].into_py_dict(py)?,)))() {
        Ok(_) => {}
        Err(err) if err.is_instance_of::<PyTypeError>(py) => {
            let e = frame.catch(err);
            rt::builtins::print(py, ("TypeError", &e))?;
        }
        Err(err) => return Err(err),
    }
    match (|| rt::builtins::print(py, (rt::int(py, 1).is_instance(&xs)?,)))() {
        Ok(_) => {}
        Err(err) if err.is_instance_of::<PyTypeError>(py) => {
            let e = frame.catch(err);
            rt::builtins::print(py, ("TypeError", &e))?;
        }
        Err(err) => return Err(err),
    }
    match Err(frame.raise(xs.as_any())) {
        Ok(()) => unreachable!(),
        Err(err) if err.is_instance_of::<PyTypeError>(py) => {
            let e = frame.catch(err);
            rt::builtins::print(py, ("TypeError", &e))?;
        }
        Err(err) => return Err(err),
    }
    match Err(frame.raise_from(PyValueError::new_err("cause"), &d)) {
        Ok(()) => unreachable!(),
        Err(err) if err.is_instance_of::<PyTypeError>(py) => {
            let e = frame.catch(err);
            rt::builtins::print(py, ("TypeError", &e))?;
        }
        Err(err) => return Err(err),
    }
    match (|| -> PyResult<()> {
        frame.with(&xs, |_| Ok(()))?;
        Ok(())
    })() {
        Ok(()) => {}
        Err(err) if err.is_instance_of::<PyTypeError>(py) => {
            let e = frame.catch(err);
            rt::builtins::print(py, ("TypeError", &e))?;
        }
        Err(err) => return Err(err),
    }
    match (|| -> PyResult<()> {
        if s.is_truthy()? {
            return Err(frame.raise(PyAssertionError::new_err((xs.clone().into_any().unbind(),))));
        }
        Ok(())
    })() {
        Ok(()) => {}
        Err(err) if err.is_instance_of::<PyAssertionError>(py) => {
            let e = frame.catch(err);
            rt::builtins::print(
                py,
                ("AssertionError", &e, e.attr("args")?.get_item(0)?.is(&xs)),
            )?;
        }
        Err(err) => return Err(err),
    }
    match (|| -> PyResult<()> {
        let [a, b] = rt::unpack(&s)?;
        Ok(())
    })() {
        Ok(()) => {}
        Err(err) if err.is_instance_of::<PyValueError>(py) => {
            let e = frame.catch(err);
            rt::builtins::print(py, ("ValueError", &e))?;
        }
        Err(err) => return Err(err),
    }
    Ok(if flag.is_truthy()? {
        xs.clone().into_any()
    } else {
        d.clone().into_any()
    })
}

// container_locals.py:292
#[rt::frame(m, "returns_from_blocks", 292, frame)]
pub fn returns_from_blocks<'py>(
    m: &rt::Module<'py>,
    flag: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let xs = PyList::new(py, [1])?;
    let result = (|| -> PyResult<rt::Flow<'py>> {
        xs.append(2)?;
        if flag.is_truthy()? {
            return Ok(rt::Flow::Return(xs.clone().into_any()));
        }
        if let rt::Flow::Return(value) = frame
            .with(&rt::builtins::open(py, (m.global("__file__")?,))?, |_| {
                Ok(rt::Flow::Return(xs.clone().into_any()))
            })?
        {
            return Ok(rt::Flow::Return(value));
        }
        Ok(rt::Flow::Next)
    })();
    let finally = frame.finally(result);
    xs.append("finally")?;
    if let rt::Flow::Return(value) = finally.resume()? {
        return Ok(value);
    }
    Ok(rt::none(py))
}

// --- errors keep CPython's types and messages ---
// container_locals.py:307
#[rt::frame(m, "subscripted", 307)]
pub fn subscripted<'py>(m: &rt::Module<'py>, i: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let xs = PyList::new(py, [1, 2, 3])?.into_any();
    let d = [(1, "one")].into_py_dict(py)?.into_any();
    let keys = [("a", 1)].into_py_dict(py)?;
    keys.del_item("a")?;
    keys.set_item(i, &xs)?;
    xs.set_item(i, rt::iadd(&xs.get_item(i)?, 10)?)?;
    rt::builtins::print(
        py,
        (
            xs.get_item(i)?,
            xs.get_item(-1)?,
            xs.get_item(rt::slice(py, i, py.None())?)?,
            d.get_item(i)?,
            &keys,
        ),
    )?;
    xs.del_item(0)?;
    d.get_item(xs.get_item(0)?)
}

// container_locals.py:319
#[rt::frame(m, "missing_key", 319)]
pub fn missing_key<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let d = [("a", 1)].into_py_dict(m.py())?.into_any();
    d.get_item("b")
}

// container_locals.py:324
#[rt::frame(m, "missing_index", 324)]
pub fn missing_index<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let xs = PyList::new(m.py(), [1])?.into_any();
    xs.get_item(5)
}

// container_locals.py:329
#[rt::frame(m, "bad_index", 329)]
pub fn bad_index<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let xs = PyList::new(m.py(), [1])?.into_any();
    xs.get_item("x")
}

// container_locals.py:334
#[rt::frame(m, "unhashable_key", 334)]
pub fn unhashable_key<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let d = PyDict::new(py);
    d.set_item(PyList::new(py, [1])?, 2)?;
    Ok(d.into_any())
}

// container_locals.py:340
#[rt::frame(m, "unhashable_element", 340)]
pub fn unhashable_element<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let s = PySet::new(py, [1])?;
    s.add(PyList::new(py, [1])?)?;
    Ok(s.into_any())
}

// container_locals.py:346
#[rt::frame(m, "unhashable_lookup", 346)]
pub fn unhashable_lookup<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let d = PyDict::new(py);
    Ok(rt::bool(py, d.contains(PyList::empty(py))?))
}

// container_locals.py:351
#[rt::frame(m, "append_arity", 351)]
pub fn append_arity<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let xs = PyList::empty(m.py());
    xs.method1("append", (1, 2))?;
    Ok(xs.into_any())
}

// container_locals.py:357
#[rt::frame(m, "append_nothing", 357)]
pub fn append_nothing<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let xs = PyList::empty(m.py());
    xs.method0("append")?;
    Ok(xs.into_any())
}

// container_locals.py:363
#[rt::frame(m, "append_keyword", 363)]
pub fn append_keyword<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let xs = PyList::empty(m.py());
    rt::call!(xs.attr("append")?, object = 1)?;
    Ok(xs.into_any())
}

// container_locals.py:369
#[rt::frame(m, "append_star", 369)]
pub fn append_star<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let xs = PyList::empty(py);
    let args = PyTuple::new(py, [1])?.into_any();
    rt::call!(xs.attr("append")?, *args)?;
    rt::call!(xs.attr("append")?, *PyTuple::empty(py))?;
    Ok(xs.into_any())
}

// container_locals.py:377
#[rt::frame(m, "add_arity", 377)]
pub fn add_arity<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let s = PySet::new(m.py(), [1])?;
    s.method1("add", (1, 2))?;
    Ok(s.into_any())
}

// container_locals.py:383
#[rt::frame(m, "no_such_method", 383)]
pub fn no_such_method<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let xs = PyList::empty(py);
    let d = PyDict::new(py);
    d.method1("add", (1,))?;
    xs.method1("add", (1,))?;
    Ok(xs.into_any())
}

// container_locals.py:391
#[rt::frame(m, "append_result", 391)]
pub fn append_result<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let xs = PyList::empty(py);
    let r = xs.method1("append", (1,))?;
    let value = xs.method1("append", (2,))?;
    let value2 = PySet::new(py, [3])?.method1("add", (4,))?;
    let list = PyList::empty(py);
    for i in rt::iter(&rt::builtins::range(py, (2,))?)? {
        let i = i?;
        list.append(xs.method1("append", (&i,))?)?;
    }
    rt::builtins::print(py, (value, value2, list))?;
    let f = xs.attr("append")?;
    f.call1((5,))?;
    let g = xs.attr("append")?;
    xs.method0("clear")?;
    g.call1((6,))?;
    Ok(PyTuple::new(
        py,
        [
            &r,
            &xs,
            &rt::bool(py, f.attr("__self__")?.is(&xs)),
            &f.attr("__name__")?,
        ],
    )?
    .into_any())
}

// container_locals.py:403
#[rt::frame(m, "delete_missing", 403, frame)]
pub fn delete_missing<'py>(m: &rt::Module<'py>) -> PyResult<()> {
    let py = m.py();
    let d = PyDict::new(py);
    let xs = PyList::empty(py).into_any();
    match (|| -> PyResult<()> {
        d.del_item("k")?;
        Ok(())
    })() {
        Ok(()) => {}
        Err(err) if err.is_instance_of::<PyKeyError>(py) => {
            let e = frame.catch(err);
            rt::builtins::print(py, ("KeyError", &e))?;
        }
        Err(err) => return Err(err),
    }
    xs.del_item(0)?;
    Ok(())
}

// container_locals.py:413
#[rt::frame(m, "assign_missing", 413)]
pub fn assign_missing<'py>(m: &rt::Module<'py>) -> PyResult<()> {
    let xs = PyList::empty(m.py()).into_any();
    xs.set_item(0, 1)?;
    Ok(())
}

// --- locals that are not typed ---
// container_locals.py:421
#[rt::frame(m, "rebound_to_other_type", 421, frame)]
pub fn rebound_to_other_type<'py>(
    m: &rt::Module<'py>,
    flag: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let mut xs = PyList::empty(py).into_any();
    if flag.is_truthy()? {
        xs = PyTuple::empty(py).into_any();
    }
    match xs.method1("append", (1,)) {
        Ok(_) => {}
        Err(err) if err.is_instance_of::<PyAttributeError>(py) => {
            let e = frame.catch(err);
            rt::builtins::print(py, ("AttributeError", &e))?;
        }
        Err(err) => return Err(err),
    }
    let mut d = PyDict::new(py).into_any();
    d = d.method0("items")?;
    let mut s = PySet::new(py, [1])?.into_any();
    s = rt::builtins::frozenset(py, (&s,))?;
    match s.method1("add", (2,)) {
        Ok(_) => {}
        Err(err) if err.is_instance_of::<PyAttributeError>(py) => {
            let e = frame.catch(err);
            rt::builtins::print(py, ("AttributeError", &e))?;
        }
        Err(err) => return Err(err),
    }
    let mut mixed = PyList::empty(py).into_any();
    mixed = PyDict::new(py).into_any();
    mixed.set_item("k", 1)?;
    Ok(PyTuple::new(py, [&xs, &rt::builtins::list(py, (&d,))?, &s, &mixed])?.into_any())
}

// container_locals.py:449
#[rt::frame(m, "subclass_or_call", 449)]
pub fn subclass_or_call<'py>(
    m: &rt::Module<'py>,
    flag: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let xs = if flag.is_truthy()? {
        m.global("Appender")?.call0()?
    } else {
        PyList::empty(py).into_any()
    };
    xs.method1("append", (1,))?;
    let ys = rt::builtins::list(py, ())?;
    ys.method1("append", (2,))?;
    let zs = {
        let value = PyList::empty(py);
        if value.is_truthy()? {
            value.into_any()
        } else {
            m.global("Appender")?.call0()?
        }
    };
    zs.method1("append", (3,))?;
    let ws = if flag.is_truthy()? {
        xs.clone()
    } else {
        ys.clone()
    };
    ws.method1("append", (4,))?;
    Ok(PyTuple::new(py, [&xs, &ys, &zs, &ws.get_type().attr("__name__")?])?.into_any())
}

// container_locals.py:461
#[rt::frame(m, "chained", 461)]
pub fn chained<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let value = PyList::empty(py).into_any();
    let a = value.clone();
    let b = value.clone();
    a.method1("append", (1,))?;
    b.method1("append", (2,))?;
    let value = PyDict::new(py).into_any();
    let c = value.clone();
    let d = value.clone();
    c.set_item("k", &a)?;
    Ok(PyTuple::new(
        py,
        [&rt::bool(py, a.is(&b)), &b, &rt::bool(py, c.is(&d)), &d],
    )?
    .into_any())
}

// container_locals.py:470
#[rt::frame(m, "captured", 470, frame)]
pub fn captured<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let xs = PyList::empty(py).into_any();
    let s = rt::builtins::set(py, ())?;
    // container_locals.py:474
    #[rt::frame(m, "captured.<locals>.push", 474)]
    fn push<'py>(
        m: &rt::Module<'py>,
        xs: &Bound<'py, PyAny>,
        s: &Bound<'py, PyAny>,
        x: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        xs.method1("append", (x,))?;
        s.method1("add", (x,))?;
        Ok(rt::int(m.py(), xs.len()?))
    }
    let push = rt::function!(frame, push[xs, s](x))?;
    push.call1((1,))?;
    xs.method1("append", (2,))?;
    s.method1("add", (3,))?;
    Ok(PyTuple::new(
        py,
        [&xs, &rt::builtins::sorted(py, (&s,))?, &push.call1((4,))?],
    )?
    .into_any())
}

// container_locals.py:485
#[rt::frame(m, "rebound_by_nested", 485, frame)]
pub fn rebound_by_nested<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let xs = rt::Cell::new(py, "xs", PyList::empty(py))?;
    // container_locals.py:488
    #[rt::frame(m, "rebound_by_nested.<locals>.replace", 488)]
    fn replace<'py>(m: &rt::Module<'py>, xs: &rt::Cell<'py>) -> PyResult<()> {
        xs.set(m.global("Appender")?.call0()?)?;
        Ok(())
    }
    let replace = rt::function!(frame, replace[&xs]())?;
    xs.get()?.method1("append", (1,))?;
    replace.call0()?;
    xs.get()?.method1("append", (2,))?;
    xs.get()
}

// container_locals.py:498
#[rt::frame(m, "maybe_unassigned", 498)]
pub fn maybe_unassigned<'py>(
    m: &rt::Module<'py>,
    flag: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let mut xs: Option<Bound<'py, PyAny>> = None;
    if flag.is_truthy()? {
        xs = Some(PyList::empty(m.py()).into_any());
    }
    rt::local(&xs, "xs")?.method1("append", (1,))?;
    Ok(rt::local(&xs, "xs")?.clone())
}

// container_locals.py:505
#[rt::frame(m, "deleted", 505)]
pub fn deleted<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let mut xs = Some(PyList::empty(py).into_any());
    rt::local(&xs, "xs")?.method1("append", (1,))?;
    xs = None;
    xs = Some(PyDict::new(py).into_any());
    rt::local(&xs, "xs")?.set_item("k", 1)?;
    Ok(rt::local(&xs, "xs")?.clone())
}

// container_locals.py:514
#[rt::frame(m, "other_targets", 514, frame)]
pub fn other_targets<'py>(
    m: &rt::Module<'py>,
    pairs: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let mut xs = PyList::empty(py).into_any();
    let n: Bound<'py, PyAny>;
    [xs, n] = [m.global("Appender")?.call0()?, rt::int(py, 1)];
    xs.method1("append", (&n,))?;
    let mut ys = PyList::empty(py).into_any();
    for item in rt::iter(pairs)? {
        ys = item?;
        rt::builtins::print(py, ("item", &ys))?;
    }
    let mut zs = PyList::empty(py).into_any();
    zs = m.global("Appender")?.call0()?;
    if !zs.is_none() {
        zs.method1("append", (3,))?;
    }
    let mut ws = PyList::empty(py).into_any();
    ws = rt::iadd(&ws, PyTuple::new(py, [5])?)?;
    ws.method1("append", (6,))?;
    let mut us: Option<Bound<'py, PyAny>> = None;
    frame.with(
        &rt::builtins::open(py, (m.global("__file__")?,))?,
        |value| {
            us = Some(value.clone());
            us = Some(PyList::empty(py).into_any());
            rt::local(&us, "us")?.method1("append", (7,))?;
            Ok(())
        },
    )?;
    let mut vs = rt::import(m, "os")?;
    rt::builtins::print(py, (vs.attr("sep")?,))?;
    vs = PyList::empty(py).into_any();
    vs.method1("append", (8,))?;
    Ok(PyTuple::new(py, [&xs, &ys, &zs, &ws, rt::local(&us, "us")?, &vs])?.into_any())
}

// container_locals.py:538
#[rt::frame(m, "parameter", 538)]
pub fn parameter<'py>(
    m: &rt::Module<'py>,
    xs: &Bound<'py, PyAny>,
    d: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let mut xs = xs.clone();
    let mut d = d.clone();
    xs = PyList::empty(py).into_any();
    xs.method1("append", (1,))?;
    if d.is_none() {
        d = PyDict::new(py).into_any();
    }
    d.set_item("k", &xs)?;
    Ok(PyTuple::new(py, [&xs, &d])?.into_any())
}

// container_locals.py:547
#[rt::frame(m, "matching", 547)]
pub fn matching<'py>(
    m: &rt::Module<'py>,
    command: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let xs = PyList::new(py, [command, &rt::int(py, 1)])?;
    let d = [("cmd", command), ("n", &rt::int(py, 2))].into_py_dict(py)?;
    if let Some([item, n]) = rt::match_seq(&xs)?
        && item.eq("go")?
    {
        rt::builtins::print(py, ("list go", &n))?;
    } else if let Some(([first], rest, [])) = rt::match_seq_star(&xs)? {
        rt::builtins::print(py, ("list", &first, &rest))?;
    }
    if let Some([cmd]) = rt::match_map(&d, ["cmd"])?
        && cmd.eq("go")?
        && let rest = rt::match_rest(&d, ["cmd"])?
    {
        rt::builtins::print(py, ("dict go", &rest))?;
    } else if let Some([other]) = rt::match_map(&d, ["cmd"])? {
        rt::builtins::print(py, ("dict", &other))?;
    }
    let found = if command.eq("stop")? {
        PyList::empty(py)
    } else {
        PyList::new(py, [command])?
    };
    found.append(xs.len())?;
    if let Some([items]) = rt::match_class(&found, &rt::builtin(py, "list")?, 1, [])? {
        items.method1("append", ("matched",))?;
    }
    Ok(found.into_any())
}

// container_locals.py:572
#[rt::frame(m, "in_try", 572, frame)]
pub fn in_try<'py>(m: &rt::Module<'py>, key: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let xs = PyList::empty(py);
    let d = PyDict::new(py).into_any();
    let mut ys: Option<Bound<'py, PyAny>> = None;
    let result = (|| -> PyResult<()> {
        match (|| -> PyResult<()> {
            xs.append(1)?;
            d.set_item("a", &xs)?;
            ys = Some(PyList::new(py, [&d.get_item(key)?])?.into_any());
            rt::local(&ys, "ys")?.method1("append", (2,))?;
            Ok(())
        })() {
            Ok(()) => {}
            Err(err) if err.is_instance_of::<PyKeyError>(py) => {
                let e = frame.catch(err);
                xs.append(e.attr("args")?)?;
                ys = Some(PyList::empty(py).into_any());
            }
            Err(err) => return Err(err),
        }
        Ok(())
    })();
    let finally = frame.finally(result);
    xs.append("finally")?;
    finally.resume()?;
    let mut zs: Option<Bound<'py, PyAny>> = None;
    frame.with(&rt::builtins::open(py, (m.global("__file__")?,))?, |f| {
        zs = Some(
            PyList::new(
                py,
                [&rt::int(py, f.method0("readline")?.len()?).rich_compare(0, CompareOp::Gt)?],
            )?
            .into_any(),
        );
        rt::local(&zs, "zs")?.attr("append")?.call1((xs.len(),))?;
        xs.append("with")?;
        Ok(())
    })?;
    Ok(PyTuple::new(
        py,
        [xs.as_any(), rt::local(&ys, "ys")?, rt::local(&zs, "zs")?],
    )?
    .into_any())
}

// container_locals.py:592
#[rt::frame(m, "generator", 592)]
pub async fn generator<'py>(
    m: &rt::Module<'py>,
    n: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let mut xs = PyList::empty(py);
    let s = PySet::new(py, [-1])?;
    rt::yield_from(&xs).await?;
    let mut got: Bound<'py, PyAny>;
    for i in rt::iter(&rt::builtins::range(py, (n,))?)? {
        let i = i?;
        xs.append(&i)?;
        s.add(&i)?;
        got = rt::yield_(&xs).await?;
        if got.is_truthy()? {
            xs = PyList::new(py, [&got])?;
            xs.append("sent")?;
        }
    }
    Ok(PyTuple::new(py, [xs.as_any(), &rt::builtins::sorted(py, (&s,))?])?.into_any())
}

// container_locals.py:606
#[rt::frame(m, "drive", 606, frame)]
pub fn drive<'py>(m: &rt::Module<'py>) -> PyResult<()> {
    let py = m.py();
    let r#gen = m.global("generator")?.call1((3,))?;
    rt::builtins::print(
        py,
        (
            rt::builtins::next(py, (&r#gen,))?,
            r#gen.method1("send", ("x",))?,
            rt::builtins::next(py, (&r#gen,))?,
        ),
    )?;
    match rt::builtins::next(py, (&r#gen,)) {
        Ok(_) => {}
        Err(err) if err.is_instance_of::<PyStopIteration>(py) => {
            let e = frame.catch(err);
            rt::builtins::print(py, ("returned", e.attr("value")?))?;
        }
        Err(err) => return Err(err),
    }
    // container_locals.py:613
    #[rt::frame(m, "drive.<locals>.<genexpr>", 613)]
    fn genexpr<'py>(
        m: &rt::Module<'py>,
        iter: &Bound<'py, PyIterator>,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        if let Some(v) = rt::next(iter)? {
            return Ok(Some(rt::builtins::tuple(m.py(), (&v,))?));
        }
        Ok(None)
    }
    // container_locals.py:613
    #[rt::frame(m, "drive.<locals>.<genexpr>", 613)]
    fn genexpr2<'py>(
        m: &rt::Module<'py>,
        iter: &Bound<'py, PyIterator>,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        if let Some(v) = rt::next(iter)? {
            return Ok(Some(rt::int(m.py(), v.len()?)));
        }
        Ok(None)
    }
    rt::builtins::print(
        py,
        (
            rt::builtins::list(
                py,
                (rt::genexpr!(
                    frame,
                    genexpr(m.global("generator")?.call1((2,))?.try_iter()?)
                )?,),
            )?,
            rt::builtins::sum(
                py,
                (rt::genexpr!(
                    frame,
                    genexpr2(m.global("generator")?.call1((3,))?.try_iter()?)
                )?,),
            )?,
        ),
    )?;
    Ok(())
}

// container_locals.py:616
#[rt::frame(m, "coroutine", 616, frame)]
pub async fn coroutine<'py>(
    m: &rt::Module<'py>,
    n: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let xs = PyList::empty(py);
    let d = PyDict::new(py);
    for i in rt::iter(&rt::builtins::range(py, (n,))?)? {
        let i = i?;
        let sleep = m.global("asyncio")?.method1("sleep", (0,))?;
        rt::await_(sleep).await?;
        xs.append(&i)?;
        let sleep = m.global("asyncio")?.method1("sleep", (0, &i))?;
        xs.append(rt::await_(sleep).await?)?;
        d.set_item(&i, rt::delegate(helper(m, &i)).await?)?;
    }
    match rt::await_(&xs).await {
        Ok(_) => {}
        Err(err) if err.is_instance_of::<PyTypeError>(py) => {
            let e = frame.catch(err);
            rt::builtins::print(py, ("TypeError", &e))?;
        }
        Err(err) => return Err(err),
    }
    let result: PyResult<()> = async {
        {
            let iter = rt::aiter(&xs)?;
            while let Some(x) = rt::anext(&iter).await? {
                rt::builtins::print(py, (&x,))?;
                py.check_signals()?;
            }
        }
        Ok(())
    }
    .await;
    match result {
        Ok(()) => {}
        Err(err) if err.is_instance_of::<PyTypeError>(py) => {
            let e = frame.catch(err);
            rt::builtins::print(py, ("TypeError", &e))?;
        }
        Err(err) => return Err(err),
    }
    let sleep = m.global("asyncio")?.method1("sleep", (0, &xs))?;
    Ok(PyTuple::new(py, [&rt::await_(sleep).await?, &d])?.into_any())
}

// container_locals.py:636
#[rt::frame(m, "helper", 636)]
pub async fn helper<'py>(
    m: &rt::Module<'py>,
    n: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let seen = PyList::empty(py);
    for i in rt::iter(&rt::builtins::range(py, (n,))?)? {
        let i = i?;
        seen.append(&i)?;
        let sleep = m.global("asyncio")?.method1("sleep", (0,))?;
        rt::await_(sleep).await?;
    }
    Ok(seen.into_any())
}

// container_locals.py:644
#[rt::frame(m, "lambdas_and_defaults", 644, frame)]
pub fn lambdas_and_defaults<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let xs = PyList::new(py, [1])?;
    // noqa: E731
    // container_locals.py:646
    #[rt::frame(m, "lambdas_and_defaults.<locals>.<lambda>", 646)]
    fn lambda<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
        PyList::empty(m.py()).method1("append", (1,))
    }
    let f = rt::function!(frame, lambda as "<lambda>"())?;
    let ys = PyList::new(py, [&f.call0()?])?;
    // container_locals.py:648
    fn lambda2<'py>(m: &rt::Module<'py>, v: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        Ok(v.clone())
    }
    ys.append(rt::function!(frame, lambda2 as "<lambda>"(v = &xs))?.call0()?)?;
    // container_locals.py:650
    fn default<'py>(m: &rt::Module<'py>, v: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        Ok(v.clone())
    }
    let default = rt::function!(frame, default(v = &xs))?;
    xs.append(default.call0()?.is(&xs))?;
    Ok(PyTuple::new(py, [&xs, &ys])?.into_any())
}

// container_locals.py:723
#[rt::frame(m, "set_key_in_set", 723, frame)]
pub fn set_key_in_set<'py>(m: &rt::Module<'py>) -> PyResult<()> {
    let py = m.py();
    // `in` looks an unhashable set up as a frozenset, `PySet_Contains` does not.
    let s = PySet::new(
        py,
        [
            &rt::builtins::frozenset(py, (PySet::new(py, [1])?,))?,
            &rt::int(py, 2),
        ],
    )?;
    s.add(3)?;
    let key = PySet::new(py, [1])?;
    let value = s.as_any().contains(&key)?;
    let value2 = s.as_any().contains(PySet::new(py, [9])?)?;
    let value3 = !s.as_any().contains(&key)?;
    let item = PySet::new(py, [1])?;
    rt::builtins::print(
        py,
        (
            value,
            value2,
            value3,
            PySet::new(py, [&rt::builtins::frozenset(py, (PySet::new(py, [1])?,))?])?
                .as_any()
                .contains(item)?,
        ),
    )?;
    match (|| rt::builtins::print(py, (s.as_any().contains(PyList::new(py, [1])?)?,)))() {
        Ok(_) => {}
        Err(err) if err.is_instance_of::<PyTypeError>(py) => {
            let e = frame.catch(err);
            rt::builtins::print(py, (&e,))?;
        }
        Err(err) => return Err(err),
    }
    Ok(())
}

// container_locals.py:735
#[rt::frame(m, "str_equal", 735)]
pub fn str_equal<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    // `Bound<PyString>` has its own `eq`.
    Ok(PyTuple::new(
        m.py(),
        [
            &x.str()?.rich_compare(x, CompareOp::Eq)?,
            &x.str()?.rich_compare(x, CompareOp::Ne)?,
            &x.repr()?.rich_compare(x, CompareOp::Eq)?,
        ],
    )?
    .into_any())
}

pub mod loud {
    use super::*;

    // container_locals.py:11
    /// The body of `class Loud`, run in its namespace `ns`.
    #[rt::frame(ns, "Loud", 11)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::def!(ns, __init__(slf as "self", name))?;
        rt::def!(ns, __repr__(slf as "self"))?;
        rt::def!(ns, __del__(slf as "self"))?;
        Ok(())
    }

    // container_locals.py:12
    #[rt::frame(m, "Loud.__init__", 12)]
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        name: &Bound<'py, PyAny>,
    ) -> PyResult<()> {
        slf.set_attr("name", name)?;
        Ok(())
    }

    // container_locals.py:15
    #[rt::frame(m, "Loud.__repr__", 15)]
    pub fn __repr__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::fstring(m.py(), ("Loud(", rt::format(&slf.attr("name")?, "")?, ")"))
    }

    // container_locals.py:18
    #[rt::frame(m, "Loud.__del__", 18)]
    pub fn __del__<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<()> {
        rt::builtins::print(m.py(), ("del", slf.attr("name")?))?;
        Ok(())
    }
}

pub mod appender {
    use super::*;

    // container_locals.py:443
    /// The body of `class Appender`, run in its namespace `ns`.
    #[rt::frame(ns, "Appender", 443)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        let __class__ = ns.class_cell();
        rt::def!(ns, append[&__class__](slf as "self", x))?;
        Ok(())
    }

    // container_locals.py:444
    #[rt::frame(m, "Appender.append", 444)]
    pub fn append<'py>(
        m: &rt::Module<'py>,
        __class__: &rt::Cell<'py>,
        slf: &Bound<'py, PyAny>,
        x: &Bound<'py, PyAny>,
    ) -> PyResult<()> {
        rt::builtins::print(m.py(), ("Appender.append", x))?;
        rt::super_(__class__, slf)?
            .attr("append")?
            .call1((x.mul(2)?,))?;
        Ok(())
    }
}

/// Executes the body of `container_locals.py` in the module `m`.
#[rt::frame(m, "<module>", 1)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    m.set_global(
        "__doc__",
        "Locals that only ever hold a list, dict or set display are typed Rust\n\
            locals (`Bound<'py, PyList>`, ..), and `xs.append(x)`, `s.add(x)` call\n\
            PyO3's methods. Everything else about them stays what Python does. Locals\n\
            that may hold something else stay objects, and so do those that are\n\
            subscripted (`xs[i]`, `d[k]`, `xs[i] = v`), which reads better on an object.\n",
    )?;
    m.set_global("asyncio", rt::import(m, "asyncio")?)?;
    m.set_global(
        "Loud",
        rt::class!(m, "Loud"(), loud::body, line = 11, attributes = ["name"])?,
    )?;
    rt::def!(m, report(f, *args))?;
    rt::def!(m, build(n))?;
    rt::def!(m, comprehensions(n))?;
    rt::def!(m, unpacking_displays(a, b))?;
    rt::def!(m, aliases())?;
    rt::def!(m, branches(flag))?;
    rt::def!(m, used_as_values(xs_in))?;
    rt::def!(m, echo(*args, **kwargs))?;
    rt::def!(m, passing())?;
    rt::def!(m, takes(a, b))?;
    rt::def!(m, iteration())?;
    rt::def!(m, lifetimes())?;
    rt::def!(m, evaluation_order())?;
    rt::def!(m, attempt(label, f))?;
    rt::def!(m, protocols(flag))?;
    rt::def!(m, returns_from_blocks(flag))?;
    rt::def!(m, subscripted(i))?;
    rt::def!(m, missing_key())?;
    rt::def!(m, missing_index())?;
    rt::def!(m, bad_index())?;
    rt::def!(m, unhashable_key())?;
    rt::def!(m, unhashable_element())?;
    rt::def!(m, unhashable_lookup())?;
    rt::def!(m, append_arity())?;
    rt::def!(m, append_nothing())?;
    rt::def!(m, append_keyword())?;
    rt::def!(m, append_star())?;
    rt::def!(m, add_arity())?;
    rt::def!(m, no_such_method())?;
    rt::def!(m, append_result())?;
    rt::def!(m, delete_missing())?;
    rt::def!(m, assign_missing())?;
    rt::def!(m, rebound_to_other_type(flag))?;
    m.set_global(
        "Appender",
        rt::class!(
            m,
            "Appender"(rt::builtin(py, "list")?),
            appender::body,
            line = 443
        )?,
    )?;
    rt::def!(m, subclass_or_call(flag))?;
    rt::def!(m, chained())?;
    rt::def!(m, captured())?;
    rt::def!(m, rebound_by_nested())?;
    rt::def!(m, maybe_unassigned(flag))?;
    rt::def!(m, deleted())?;
    rt::def!(m, other_targets(pairs))?;
    rt::def!(m, parameter(xs, d = py.None()))?;
    rt::def!(m, matching(command))?;
    rt::def!(m, in_try(key))?;
    rt::def!(m, gen generator(n))?;
    rt::def!(m, drive())?;
    rt::def!(m, async coroutine(n))?;
    rt::def!(m, async helper(n))?;
    rt::def!(m, lambdas_and_defaults())?;
    m.set_global("total", PyList::empty(py))?;
    m.global("total")?.method1("append", (1,))?;
    m.set_global("seen", rt::builtins::set(py, ())?)?;
    m.global("seen")?.method1("add", ("module",))?;
    m.set_global("table", PyDict::new(py))?;
    let total = m.global("total")?;
    m.global("table")?.set_item("k", total)?;
    for item in rt::iter(
        PyList::new(
            py,
            [
                PyTuple::new(py, [&m.global("build")?, PyTuple::new(py, [4])?.as_any()])?.as_any(),
                PyTuple::new(
                    py,
                    [
                        &m.global("comprehensions")?,
                        PyTuple::new(py, [5])?.as_any(),
                    ],
                )?
                .as_any(),
                PyTuple::new(
                    py,
                    [
                        &m.global("unpacking_displays")?,
                        PyTuple::new(
                            py,
                            [&rt::str(py, "ab"), [("c", 1)].into_py_dict(py)?.as_any()],
                        )?
                        .as_any(),
                    ],
                )?
                .as_any(),
                PyTuple::new(py, [&m.global("aliases")?, PyTuple::empty(py).as_any()])?.as_any(),
                PyTuple::new(
                    py,
                    [&m.global("branches")?, PyTuple::new(py, [true])?.as_any()],
                )?
                .as_any(),
                PyTuple::new(
                    py,
                    [&m.global("branches")?, PyTuple::new(py, [false])?.as_any()],
                )?
                .as_any(),
                PyTuple::new(
                    py,
                    [
                        &m.global("used_as_values")?,
                        PyTuple::new(py, [PyList::new(py, [3, 1, 2])?.as_any()])?.as_any(),
                    ],
                )?
                .as_any(),
                PyTuple::new(py, [&m.global("passing")?, PyTuple::empty(py).as_any()])?.as_any(),
                PyTuple::new(py, [&m.global("iteration")?, PyTuple::empty(py).as_any()])?.as_any(),
                PyTuple::new(py, [&m.global("lifetimes")?, PyTuple::empty(py).as_any()])?.as_any(),
                PyTuple::new(
                    py,
                    [&m.global("evaluation_order")?, PyTuple::empty(py).as_any()],
                )?
                .as_any(),
                PyTuple::new(
                    py,
                    [&m.global("protocols")?, PyTuple::new(py, [true])?.as_any()],
                )?
                .as_any(),
                PyTuple::new(
                    py,
                    [&m.global("protocols")?, PyTuple::new(py, [false])?.as_any()],
                )?
                .as_any(),
                PyTuple::new(
                    py,
                    [
                        &m.global("returns_from_blocks")?,
                        PyTuple::new(py, [true])?.as_any(),
                    ],
                )?
                .as_any(),
                PyTuple::new(
                    py,
                    [
                        &m.global("returns_from_blocks")?,
                        PyTuple::new(py, [false])?.as_any(),
                    ],
                )?
                .as_any(),
                PyTuple::new(
                    py,
                    [&m.global("subscripted")?, PyTuple::new(py, [1])?.as_any()],
                )?
                .as_any(),
                PyTuple::new(
                    py,
                    [&m.global("subscripted")?, PyTuple::new(py, [0])?.as_any()],
                )?
                .as_any(),
                PyTuple::new(py, [&m.global("missing_key")?, PyTuple::empty(py).as_any()])?
                    .as_any(),
                PyTuple::new(
                    py,
                    [&m.global("missing_index")?, PyTuple::empty(py).as_any()],
                )?
                .as_any(),
                PyTuple::new(py, [&m.global("bad_index")?, PyTuple::empty(py).as_any()])?.as_any(),
                PyTuple::new(
                    py,
                    [&m.global("unhashable_key")?, PyTuple::empty(py).as_any()],
                )?
                .as_any(),
                PyTuple::new(
                    py,
                    [
                        &m.global("unhashable_element")?,
                        PyTuple::empty(py).as_any(),
                    ],
                )?
                .as_any(),
                PyTuple::new(
                    py,
                    [&m.global("unhashable_lookup")?, PyTuple::empty(py).as_any()],
                )?
                .as_any(),
                PyTuple::new(
                    py,
                    [&m.global("append_arity")?, PyTuple::empty(py).as_any()],
                )?
                .as_any(),
                PyTuple::new(
                    py,
                    [&m.global("append_nothing")?, PyTuple::empty(py).as_any()],
                )?
                .as_any(),
                PyTuple::new(
                    py,
                    [&m.global("append_keyword")?, PyTuple::empty(py).as_any()],
                )?
                .as_any(),
                PyTuple::new(py, [&m.global("append_star")?, PyTuple::empty(py).as_any()])?
                    .as_any(),
                PyTuple::new(py, [&m.global("add_arity")?, PyTuple::empty(py).as_any()])?.as_any(),
                PyTuple::new(
                    py,
                    [&m.global("no_such_method")?, PyTuple::empty(py).as_any()],
                )?
                .as_any(),
                PyTuple::new(
                    py,
                    [&m.global("append_result")?, PyTuple::empty(py).as_any()],
                )?
                .as_any(),
                PyTuple::new(
                    py,
                    [&m.global("delete_missing")?, PyTuple::empty(py).as_any()],
                )?
                .as_any(),
                PyTuple::new(
                    py,
                    [&m.global("assign_missing")?, PyTuple::empty(py).as_any()],
                )?
                .as_any(),
                PyTuple::new(
                    py,
                    [
                        &m.global("rebound_to_other_type")?,
                        PyTuple::new(py, [true])?.as_any(),
                    ],
                )?
                .as_any(),
                PyTuple::new(
                    py,
                    [
                        &m.global("rebound_to_other_type")?,
                        PyTuple::new(py, [false])?.as_any(),
                    ],
                )?
                .as_any(),
                PyTuple::new(
                    py,
                    [
                        &m.global("subclass_or_call")?,
                        PyTuple::new(py, [true])?.as_any(),
                    ],
                )?
                .as_any(),
                PyTuple::new(
                    py,
                    [
                        &m.global("subclass_or_call")?,
                        PyTuple::new(py, [false])?.as_any(),
                    ],
                )?
                .as_any(),
                PyTuple::new(py, [&m.global("chained")?, PyTuple::empty(py).as_any()])?.as_any(),
                PyTuple::new(py, [&m.global("captured")?, PyTuple::empty(py).as_any()])?.as_any(),
                PyTuple::new(
                    py,
                    [&m.global("rebound_by_nested")?, PyTuple::empty(py).as_any()],
                )?
                .as_any(),
                PyTuple::new(
                    py,
                    [
                        &m.global("maybe_unassigned")?,
                        PyTuple::new(py, [true])?.as_any(),
                    ],
                )?
                .as_any(),
                PyTuple::new(
                    py,
                    [
                        &m.global("maybe_unassigned")?,
                        PyTuple::new(py, [false])?.as_any(),
                    ],
                )?
                .as_any(),
                PyTuple::new(py, [&m.global("deleted")?, PyTuple::empty(py).as_any()])?.as_any(),
                PyTuple::new(
                    py,
                    [
                        &m.global("other_targets")?,
                        PyTuple::new(
                            py,
                            [PyList::new(
                                py,
                                [
                                    PyList::new(py, [1])?.as_any(),
                                    PyList::new(py, [2])?.as_any(),
                                ],
                            )?
                            .as_any()],
                        )?
                        .as_any(),
                    ],
                )?
                .as_any(),
                PyTuple::new(
                    py,
                    [
                        &m.global("parameter")?,
                        PyTuple::new(py, [PyTuple::empty(py).as_any()])?.as_any(),
                    ],
                )?
                .as_any(),
                PyTuple::new(
                    py,
                    [&m.global("matching")?, PyTuple::new(py, ["go"])?.as_any()],
                )?
                .as_any(),
                PyTuple::new(
                    py,
                    [&m.global("matching")?, PyTuple::new(py, ["stop"])?.as_any()],
                )?
                .as_any(),
                PyTuple::new(
                    py,
                    [&m.global("in_try")?, PyTuple::new(py, ["a"])?.as_any()],
                )?
                .as_any(),
                PyTuple::new(
                    py,
                    [&m.global("in_try")?, PyTuple::new(py, ["b"])?.as_any()],
                )?
                .as_any(),
                PyTuple::new(py, [&m.global("drive")?, PyTuple::empty(py).as_any()])?.as_any(),
                PyTuple::new(
                    py,
                    [
                        &m.global("lambdas_and_defaults")?,
                        PyTuple::empty(py).as_any(),
                    ],
                )?
                .as_any(),
            ],
        )?
        .as_any(),
    )? {
        let [f, args] = rt::unpack(&item?)?;
        m.set_global("f", f)?;
        m.set_global("args", args)?;
        rt::call!(m.global("report")?, m.global("f")?, *m.global("args")?)?;
        m.global("total")?
            .attr("append")?
            .call1((m.global("f")?.attr("__name__")?,))?;
    }
    rt::builtins::print(
        py,
        (m.global("asyncio")?
            .attr("run")?
            .call1((m.global("coroutine")?.call1((3,))?,))?,),
    )?;
    rt::builtins::print(
        py,
        (
            m.global("total")?.len()?,
            m.global("seen")?,
            m.global("table")?.get_item("k")?.is(m.global("total")?),
        ),
    )?;
    rt::def!(m, set_key_in_set())?;
    rt::def!(m, str_equal(x))?;
    set_key_in_set(m)?;
    rt::builtins::print(
        py,
        (
            str_equal(m, &rt::str(py, "a"))?,
            str_equal(m, &rt::int(py, 1))?,
        ),
    )?;
    missing_key(m)?;
    Ok(())
}
