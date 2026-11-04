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
pub fn show<'py>(m: &rt::Module<'py>, exc: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "show", 9, |_| {
        m.global("print")?
            .call1((m.global("type")?.call1((exc,))?.attr("__name__")?, exc))?;
        for frame_ in m
            .global("traceback")?
            .attr("extract_tb")?
            .call1((exc.attr("__traceback__")?,))?
            .try_iter()?
        {
            let frame_ = frame_?;
            m.global("print")?.call1((
                " ",
                frame_.attr("name")?,
                frame_.attr("lineno")?,
                frame_.attr("line")?,
            ))?;
            py.check_signals()?;
        }
        Ok(rt::none(py))
    })
}

// comprehensions.py:15
pub fn squares<'py>(m: &rt::Module<'py>, xs: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "squares", 15, |_| {
        let list = PyList::empty(py);
        for x in xs.try_iter()? {
            let x = x?;
            list.append(x.mul(&x)?)?;
            py.check_signals()?;
        }
        Ok(list.into_any())
    })
}

// comprehensions.py:19
pub fn evens<'py>(m: &rt::Module<'py>, d: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "evens", 19, |_| {
        let dict = PyDict::new(py);
        for item in d.method0("items")?.try_iter()? {
            let [k, v] = rt::unpack(&item?)?;
            if v.rem(2)?.eq(0)? {
                dict.set_item(&k, &v)?;
            }
            py.check_signals()?;
        }
        Ok(dict.into_any())
    })
}

// comprehensions.py:23
pub fn letters<'py>(m: &rt::Module<'py>, words: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "letters", 23, |_| {
        let set = PySet::empty(py)?;
        for w in words.try_iter()? {
            let w = w?;
            for c in w.try_iter()? {
                let c = c?;
                if c.method0("isalpha")?.is_truthy()? && c.ne("z")? {
                    set.add(&c)?;
                }
                py.check_signals()?;
            }
            py.check_signals()?;
        }
        Ok(set.into_any())
    })
}

// comprehensions.py:27
pub fn pairs<'py>(m: &rt::Module<'py>, n: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "pairs", 27, |_| {
        let list = PyList::empty(py);
        for i in m.global("range")?.call1((n,))?.try_iter()? {
            let i = i?;
            for j in m.global("range")?.call1((&i,))?.try_iter()? {
                let j = j?;
                if i.add(&j)?.rem(2)?.is_truthy()? {
                    list.append(PyTuple::new(py, [&i, &j])?)?;
                }
                py.check_signals()?;
            }
            py.check_signals()?;
        }
        Ok(list.into_any())
    })
}

// comprehensions.py:31
pub fn nested<'py>(m: &rt::Module<'py>, rows: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "nested", 31, |_| {
        // The inner comprehension sees the variable of the outer one.
        let list = PyList::empty(py);
        for row in rows.try_iter()? {
            let row = row?;
            let list2 = PyList::empty(py);
            for cell in row.try_iter()? {
                let cell = cell?;
                list2.append(cell.mul(row.get_item(0)?)?)?;
                py.check_signals()?;
            }
            list.append(&list2)?;
            py.check_signals()?;
        }
        Ok(list.into_any())
    })
}

// comprehensions.py:36
pub fn unpacking<'py>(
    m: &rt::Module<'py>,
    items: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "unpacking", 36, |_| {
        let list = PyList::empty(py);
        for item in items.try_iter()? {
            let ([a], rest, [b]) = rt::unpack_star(&item?)?;
            list.append(PyTuple::new(py, [&a, &rest, &b])?)?;
            py.check_signals()?;
        }
        Ok(list.into_any())
    })
}

// comprehensions.py:40
pub fn attribute_target<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "attribute_target", 40, |_| {
        let r#box = m.global("types")?.method0("SimpleNamespace")?;
        let list = PyList::empty(py);
        for item in m.global("range")?.call1((3,))?.try_iter()? {
            r#box.set_attr("value", item?)?;
            list.append(r#box.attr("value")?)?;
            py.check_signals()?;
        }
        let values = list.into_any();
        Ok(PyTuple::new(py, [&values, &r#box.attr("value")?])?.into_any())
    })
}

// comprehensions.py:46
pub fn subscript_target<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "subscript_target", 46, |_| {
        let slots = PyList::new(py, [0])?.into_any();
        let list = PyList::empty(py);
        for item in rt::str(py, "ab").try_iter()? {
            let value = item?;
            slots.set_item(0, &value)?;
            list.append(slots.get_item(0)?)?;
            py.check_signals()?;
        }
        Ok(PyTuple::new(py, [list.as_any(), &slots])?.into_any())
    })
}

// comprehensions.py:51
pub fn dict_order<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "dict_order", 51, |_| {
        // comprehensions.py:52
        fn key<'py>(m: &rt::Module<'py>, k: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
            rt::frame!(m, "dict_order.<locals>.key", 52, |_| {
                m.global("print")?.call1(("key", k))?;
                k.rem(2)
            })
        }
        let key = rt::function!(m, key(k), qualname = "dict_order.<locals>.key")?;
        // comprehensions.py:56
        fn value<'py>(m: &rt::Module<'py>, v: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
            rt::frame!(m, "dict_order.<locals>.value", 56, |_| {
                m.global("print")?.call1(("value", v))?;
                Ok(v.clone())
            })
        }
        let value = rt::function!(m, value(v), qualname = "dict_order.<locals>.value")?;
        // The key is evaluated before the value; a later duplicate key wins.
        let dict = PyDict::new(py);
        for k in m.global("range")?.call1((4,))?.try_iter()? {
            let k = k?;
            dict.set_item(key.call1((&k,))?, value.call1((&k,))?)?;
            py.check_signals()?;
        }
        Ok(dict.into_any())
    })
}

// comprehensions.py:67
pub fn shadow<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "shadow", 67, |_| {
        let list = PyList::empty(py);
        for x in m.global("range")?.call1((3,))?.try_iter()? {
            let x = x?;
            list.append(x.mul(&x)?)?;
            py.check_signals()?;
        }
        let squares = list.into_any();
        Ok(PyTuple::new(py, [&squares, &m.global("x")?])?.into_any())
    })
}

// comprehensions.py:72
pub fn first_iterable_outside<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "first_iterable_outside", 72, |_| {
        let x = PyList::new(py, [1, 2])?.into_any();
        // The first iterable is the function's `x`, then `x` is the item.
        let list = PyList::empty(py);
        for x in x.try_iter()? {
            let x = x?;
            list.append(x.mul(10)?)?;
            py.check_signals()?;
        }
        Ok(list.into_any())
    })
}

// comprehensions.py:78
pub fn late_binding<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "late_binding", 78, |_| {
        let i: Bound<'py, PyAny>;
        // comprehensions.py:79
        fn lambda<'py>(m: &rt::Module<'py>, i: &rt::Cell<'py>) -> PyResult<Bound<'py, PyAny>> {
            rt::frame!(m, "late_binding.<locals>.<lambda>", 79, |_| { i.get() })
        }
        let funcs = {
            let i_ = rt::Cell::empty(py, "i")?;
            let list = PyList::empty(py);
            for item in m.global("range")?.call1((3,))?.try_iter()? {
                i_.set(item?)?;
                list.append(rt::function!(
                    m,
                    lambda[&i_ as "i"](),
                    qualname = "late_binding.<locals>.<lambda>"
                )?)?;
                py.check_signals()?;
            }
            list.into_any()
        };
        let list = PyList::empty(py);
        for f in funcs.try_iter()? {
            let f = f?;
            list.append(f.call0()?)?;
            py.check_signals()?;
        }
        Ok(list.into_any())
    })
}

// comprehensions.py:83
pub fn default_binding<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "default_binding", 83, |_| {
        // comprehensions.py:84
        fn lambda<'py>(m: &rt::Module<'py>, i: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
            Ok(i.clone())
        }
        let list = PyList::empty(py);
        for i in m.global("range")?.call1((3,))?.try_iter()? {
            let i = i?;
            list.append(rt::function!(
                m,
                lambda(i = &i),
                qualname = "default_binding.<locals>.<lambda>"
            )?)?;
            py.check_signals()?;
        }
        let funcs = list.into_any();
        let list = PyList::empty(py);
        for f in funcs.try_iter()? {
            let f = f?;
            list.append(f.call0()?)?;
            py.check_signals()?;
        }
        Ok(list.into_any())
    })
}

// comprehensions.py:88
pub fn generator_in_comprehension<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "generator_in_comprehension", 88, |_| {
        let x: Bound<'py, PyAny>;
        // comprehensions.py:89
        fn genexpr<'py>(
            m: &rt::Module<'py>,
            x: &rt::Cell<'py>,
            iter: &Bound<'py, PyIterator>,
        ) -> PyResult<Option<Bound<'py, PyAny>>> {
            rt::frame!(
                m,
                "generator_in_comprehension.<locals>.<genexpr>",
                89,
                |_| {
                    if let Some(y) = rt::next(iter)? {
                        return Ok(Some(x.get()?.add(&y)?));
                    }
                    Ok(None)
                }
            )
        }
        let x_ = rt::Cell::empty(py, "x")?;
        let list = PyList::empty(py);
        for item in PyTuple::new(py, [10, 20])?.try_iter()? {
            x_.set(item?)?;
            list.append(m.global("list")?.call1((rt::genexpr!(
                m,
                genexpr[&x_ as "x"](m.global("range")?.call1((2,))?.try_iter()?),
                qualname = "generator_in_comprehension.<locals>.<genexpr>"
            )?,))?)?;
            py.check_signals()?;
        }
        Ok(list.into_any())
    })
}

// comprehensions.py:92
pub fn stop_iteration<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "stop_iteration", 92, |frame| {
        // comprehensions.py:93
        fn boom<'py>(m: &rt::Module<'py>, v: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
            rt::frame!(m, "stop_iteration.<locals>.boom", 93, |frame| {
                if v.eq(2)? {
                    return Err(frame.raise(
                        m.global("StopIteration")?
                            .call1(("not converted in a comprehension",))?,
                    ));
                }
                Ok(v.clone())
            })
        }
        let boom = rt::function!(m, boom(v), qualname = "stop_iteration.<locals>.boom")?;
        match (|| -> PyResult<Bound<'py, PyAny>> {
            let print = m.global("print")?;
            let list = PyList::empty(py);
            for v in m.global("range")?.call1((4,))?.try_iter()? {
                let v = v?;
                list.append(boom.call1((&v,))?)?;
                py.check_signals()?;
            }
            print.call1((&list,))
        })() {
            Ok(_) => {}
            Err(err) if frame.matches(&err, 92, || m.global("StopIteration"))? => {
                let exc = frame.catch(err);
                m.global("print")?.call1(("StopIteration", &exc))?;
            }
            Err(err) => return Err(err),
        }
        Ok(rt::none(py))
    })
}

// comprehensions.py:104
pub fn multiline<'py>(m: &rt::Module<'py>, xs: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "multiline", 104, |_| {
        let list = PyList::empty(py);
        for x in xs.try_iter()? {
            let x = x?;
            if x.ne(1)? {
                list.append(rt::int(py, 10).floor_div(&x)?)?;
            }
            py.check_signals()?;
        }
        Ok(list.into_any())
    })
}

// comprehensions.py:112
pub fn failing_condition<'py>(
    m: &rt::Module<'py>,
    xs: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "failing_condition", 112, |_| {
        let list = PyList::empty(py);
        for x in xs.try_iter()? {
            let x = x?;
            if rt::int(py, 10).floor_div(&x)?.is_truthy()? {
                list.append(&x)?;
            }
            py.check_signals()?;
        }
        Ok(list.into_any())
    })
}

// comprehensions.py:125
pub fn failing_iteration<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "failing_iteration", 125, |_| {
        let list = PyList::empty(py);
        for x in m.global("Bad")?.call0()?.try_iter()? {
            let x = x?;
            list.append(&x)?;
            py.check_signals()?;
        }
        Ok(list.into_any())
    })
}

pub mod bad {
    use super::*;

    // comprehensions.py:117
    /// The body of `class Bad`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Bad", 117, |_| {
            rt::def!(ns, __iter__(slf as "self"))?;
            rt::def!(ns, __next__(slf as "self"))?;
            Ok(())
        })
    }

    // comprehensions.py:118
    pub fn __iter__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(slf.clone())
    }

    // comprehensions.py:121
    pub fn __next__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Bad.__next__", 121, |frame| {
            Err(frame.raise(m.global("ValueError")?.call1(("next failed",))?))
        })
    }
}

pub mod config {
    use super::*;

    // comprehensions.py:130
    /// The body of `class Config`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        let py = m.py();
        rt::frame!(ns, "Config", 130, |frame| {
            ns.set("size", 3)?;
            let list = PyList::empty(py);
            for i in ns.get("range")?.call1((ns.get("size")?,))?.try_iter()? {
                let i = i?;
                list.append(i.mul(2)?)?;
                py.check_signals()?;
            }
            ns.set("doubled", list)?;
            match (|| -> PyResult<Bound<'py, PyAny>> {
                let list = PyList::empty(py);
                for i in ns.get("range")?.call1((2,))?.try_iter()? {
                    let i = i?;
                    list.append(m.global("size")?)?;
                    py.check_signals()?;
                }
                Ok(list.into_any())
            })() {
                Ok(value) => {
                    ns.set("broken", &value)?;
                }
                Err(err) if frame.matches(&err, 130, || ns.get("NameError"))? => {
                    let exception = frame.catch(err);
                    ns.set("exc", &exception)?;
                    ns.get("print")?.call1(("class body:", ns.get("exc")?))?;
                    ns.unbind("exc")?;
                }
                Err(err) => return Err(err),
            }
            let dict = PyDict::new(py);
            for n in PyTuple::new(py, ["a", "bb"])?.try_iter()? {
                let n = n?;
                dict.set_item(&n, m.global("len")?.call1((&n,))?)?;
                py.check_signals()?;
            }
            ns.set("names", dict)?;
            Ok(())
        })
    }
}

pub mod base {
    use super::*;

    // comprehensions.py:140
    /// The body of `class Base`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Base", 140, |_| {
            rt::def!(ns, items(slf as "self"))?;
            Ok(())
        })
    }

    // comprehensions.py:141
    pub fn items<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Base.items", 141, |_| {
            Ok(PyList::new(m.py(), [1, 2])?.into_any())
        })
    }
}

pub mod derived {
    use super::*;

    // comprehensions.py:145
    /// The body of `class Derived`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Derived", 145, |_| {
            let __class__ = ns.class_cell();
            ns.set(
                "_Derived__secret",
                rt::function!(ns, __secret(slf as "self"))?,
            )?;
            rt::def!(ns, values[&__class__](slf as "self"))?;
            rt::def!(ns, secrets(slf as "self"))?;
            rt::def!(ns, supers[&__class__](slf as "self"))?;
            Ok(())
        })
    }

    // comprehensions.py:146
    pub fn __secret<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(rt::str(m.py(), "secret"))
    }

    // comprehensions.py:149
    pub fn values<'py>(
        m: &rt::Module<'py>,
        __class__: &rt::Cell<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "Derived.values", 149, |_| {
            let list = PyList::empty(py);
            for v in rt::super_(__class__, slf)?.method0("items")?.try_iter()? {
                let v = v?;
                list.append(v.mul(2)?)?;
                py.check_signals()?;
            }
            let list2 = PyList::empty(py);
            for underscore in rt::str(py, "x").try_iter()? {
                let underscore = underscore?;
                list2.append(__class__.get()?.attr("__name__")?)?;
                py.check_signals()?;
            }
            list.add(&list2)
        })
    }

    // comprehensions.py:152
    pub fn secrets<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "Derived.secrets", 152, |_| {
            let list = PyList::empty(py);
            for underscore in m.global("range")?.call1((2,))?.try_iter()? {
                let underscore = underscore?;
                list.append(slf.method0("_Derived__secret")?)?;
                py.check_signals()?;
            }
            Ok(list.into_any())
        })
    }

    // comprehensions.py:155
    pub fn supers<'py>(
        m: &rt::Module<'py>,
        __class__: &rt::Cell<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "Derived.supers", 155, |_| {
            let list = PyList::empty(py);
            for underscore in rt::str(py, "x").try_iter()? {
                let underscore = underscore?;
                list.append(rt::super_(__class__, slf)?.method0("items")?)?;
                py.check_signals()?;
            }
            Ok(list.into_any())
        })
    }
}

/// Executes the body of `comprehensions.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
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
        for item in PyTuple::new(
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
        .try_iter()?
        {
            let [call, arg] = rt::unpack(&item?)?;
            m.set_global("call", call)?;
            m.set_global("arg", arg)?;
            match (|| m.global("call")?.call1((m.global("arg")?,)))() {
                Ok(_) => {}
                Err(err) if frame.matches(&err, 1, || m.global("ZeroDivisionError"))? => {
                    let exception = frame.catch(err);
                    m.set_global("exc", &exception)?;
                    m.global("show")?.call1((m.global("exc")?,))?;
                    m.unbind_global("exc")?;
                }
                Err(err) => return Err(err),
            }
            py.check_signals()?;
        }
        match (|| m.global("failing_iteration")?.call0())() {
            Ok(_) => {}
            Err(err) if frame.matches(&err, 1, || m.global("ValueError"))? => {
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
        for n in m.global("range")?.call1((3,))?.try_iter()? {
            let n = n?;
            if n.is_truthy()? {
                list.append(&n)?;
            }
            py.check_signals()?;
        }
        m.set_global("module_level", list)?;
        {
            let print = m.global("print")?;
            let module_level = m.global("module_level")?;
            let list = PyList::empty(py);
            for w in PyTuple::new(py, ["ab", "c"])?.try_iter()? {
                let w = w?;
                let list2 = PyList::empty(py);
                for c in w.try_iter()? {
                    let c = c?;
                    list2.append(&c)?;
                    py.check_signals()?;
                }
                list.append(&list2)?;
                py.check_signals()?;
            }
            let set = PySet::empty(py)?;
            for n in m.global("range")?.call1((5,))?.try_iter()? {
                let n = n?;
                set.add(n.rem(2)?)?;
                py.check_signals()?;
            }
            print.call1((module_level, list, set))?;
        }
        let print = m.global("print")?;
        let value = m.globals().contains("n")?;
        let dict = PyDict::new(py);
        for item2 in m
            .global("zip")?
            .call1(("ab", m.global("range")?.call1((2,))?))?
            .try_iter()?
        {
            let [k, v] = rt::unpack(&item2?)?;
            dict.set_item(&k, &v)?;
            py.check_signals()?;
        }
        print.call1((value, dict))?;
        Ok(())
    })
}
