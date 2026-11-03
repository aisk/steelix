use pyo3::exceptions::PyImportError;
use pyo3::exceptions::PyIndexError;
use pyo3::exceptions::PyKeyError;
use pyo3::exceptions::PyStopIteration;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::IntoPyDict;
use pyo3::types::PyDict;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;

// json/decoder.py:63
pub fn _decode_uXXXX<'py>(
    module: &rt::Module<'py>,
    s: &Bound<'py, PyAny>,
    pos: &Bound<'py, PyAny>,
    _m: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = module.py();
    rt::frame!(module, "_decode_uXXXX", 63, |frame| {
        frame.line(64);
        let esc = _m.call1((s, pos.add(1)?))?;
        if !esc.is_none() {
            frame.line(67);
            match (|| rt::builtin(py, "int")?.call1((esc.call_method0("group")?, 16)))() {
                Ok(value) => return Ok(value),
                Err(err) if err.is_instance_of::<PyValueError>(py) => {}
                Err(err) => return Err(err),
            }
        }
        let msg = rt::str(py, "Invalid \\uXXXX escape");
        frame.line(71);
        Err(frame.raise(module.global("JSONDecodeError")?.call1((&msg, s, pos))?))
    })
}

// json/decoder.py:73
/// Scan the string s for a JSON string. End is the index of the
/// character in s after the quote that started the JSON string.
/// Unescapes all valid JSON string escape sequences and raises ValueError
/// on attempt to decode an invalid string. If strict is False then literal
/// control characters are allowed in the string.
///
/// Returns a tuple of the decoded string and the index of the character in s
/// after the end quote.
pub fn py_scanstring<'py>(
    module: &rt::Module<'py>,
    s: &Bound<'py, PyAny>,
    end: &Bound<'py, PyAny>,
    strict: &Bound<'py, PyAny>,
    _b: &Bound<'py, PyAny>,
    _m: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = module.py();
    rt::frame!(module, "py_scanstring", 73, |frame| {
        let mut end = end.clone();
        let mut chunk: Bound<'py, PyAny>;
        let mut content: Bound<'py, PyAny>;
        let mut terminator: Bound<'py, PyAny>;
        let msg: Bound<'py, PyAny>;
        let mut esc: Bound<'py, PyAny>;
        let mut char: Bound<'py, PyAny>;
        let mut uni: Bound<'py, PyAny>;
        let mut uni2: Bound<'py, PyAny>;
        let chunks = PyList::empty(py).into_any();
        frame.line(84);
        let _append = chunks.getattr("append")?;
        frame.line(85);
        let begin = end.sub(1)?;
        loop {
            frame.line(87);
            chunk = _m.call1((s, &end))?;
            if chunk.is_none() {
                frame.line(89);
                return Err(frame.raise(module.global("JSONDecodeError")?.call1((
                    "Unterminated string starting at",
                    s,
                    &begin,
                ))?));
            }
            frame.line(90);
            end = chunk.call_method0("end")?;
            frame.line(91);
            [content, terminator] = rt::unpack(&chunk.call_method0("groups")?)?;
            // Content is contains zero or more unescaped string characters
            frame.line(93);
            if content.is_truthy()? {
                frame.line(94);
                _append.call1((&content,))?;
            }
            // Terminator is the end of string, a literal control character,
            // or a backslash denoting that an escape sequence follows
            frame.line(97);
            if terminator.eq("\"")? {
                break;
            } else {
                frame.line(99);
                if terminator.ne("\\")? {
                    frame.line(100);
                    if strict.is_truthy()? {
                        // msg = "Invalid control character %r at" % (terminator,)
                        frame.line(102);
                        msg = rt::str(py, "Invalid control character {0!r} at")
                            .call_method1("format", (&terminator,))?;
                        frame.line(103);
                        return Err(frame.raise(module.global("JSONDecodeError")?.call1((
                            &msg,
                            s,
                            end.sub(1)?,
                        ))?));
                    } else {
                        frame.line(105);
                        _append.call1((&terminator,))?;
                        py.check_signals()?;
                        continue;
                    }
                }
            }
            frame.line(108);
            match s.get_item(&end) {
                Ok(value) => esc = value,
                Err(err) if err.is_instance_of::<PyIndexError>(py) => {
                    let _handling = rt::Handling::new(&frame.catch(err));
                    frame.line(110);
                    return Err(frame.raise_from(
                        module.global("JSONDecodeError")?.call1((
                            "Unterminated string starting at",
                            s,
                            &begin,
                        ))?,
                        py.None(),
                    ));
                }
                Err(err) => return Err(err),
            }
            // If not a unicode escape sequence, must be in the lookup table
            frame.line(113);
            if esc.ne("u")? {
                frame.line(115);
                match _b.get_item(&esc) {
                    Ok(value) => char = value,
                    Err(err) if err.is_instance_of::<PyKeyError>(py) => {
                        let _handling = rt::Handling::new(&frame.catch(err));
                        frame.line(117);
                        msg = rt::str(py, "Invalid \\escape: {0!r}")
                            .call_method1("format", (&esc,))?;
                        frame.line(118);
                        return Err(
                            frame.raise(module.global("JSONDecodeError")?.call1((&msg, s, &end))?)
                        );
                    }
                    Err(err) => return Err(err),
                }
                frame.line(119);
                end = rt::iadd(&end, 1)?;
            } else {
                frame.line(121);
                uni = _decode_uXXXX(module, s, &end, &module.default("_decode_uXXXX", "_m")?)?;
                frame.line(122);
                end = rt::iadd(&end, 5)?;
                frame.line(123);
                if rt::int(py, 55296).le(&uni)?
                    && uni.le(56319)?
                    && s.get_item(rt::slice(py, &end, end.add(2)?)?)?.eq("\\u")?
                {
                    frame.line(124);
                    uni2 = _decode_uXXXX(
                        module,
                        s,
                        &end.add(1)?,
                        &module.default("_decode_uXXXX", "_m")?,
                    )?;
                    frame.line(125);
                    if rt::int(py, 56320).le(&uni2)? && uni2.le(57343)? {
                        frame.line(126);
                        uni = rt::int(py, 65536)
                            .add(uni.sub(55296)?.lshift(10)?.bitor(uni2.sub(56320)?)?)?;
                        frame.line(127);
                        end = rt::iadd(&end, 6)?;
                    }
                }
                frame.line(128);
                char = rt::builtin(py, "chr")?.call1((&uni,))?;
            }
            frame.line(129);
            _append.call1((&char,))?;
            py.check_signals()?;
        }
        frame.line(130);
        Ok(PyTuple::new(
            py,
            [&rt::str(py, "").call_method1("join", (&chunks,))?, &end],
        )?
        .into_any())
    })
}

// json/decoder.py:140
pub fn JSONObject<'py>(
    m: &rt::Module<'py>,
    s_and_end: &Bound<'py, PyAny>,
    strict: &Bound<'py, PyAny>,
    scan_once: &Bound<'py, PyAny>,
    object_hook: &Bound<'py, PyAny>,
    object_pairs_hook: &Bound<'py, PyAny>,
    memo: &Bound<'py, PyAny>,
    _w: &Bound<'py, PyAny>,
    _ws: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "JSONObject", 140, |frame| {
        let mut memo = memo.clone();
        let result: Bound<'py, PyAny>;
        let mut key: Bound<'py, PyAny>;
        let mut value: Option<Bound<'py, PyAny>> = None;
        let mut comma_idx: Bound<'py, PyAny>;
        frame.line(142);
        let [s, mut end] = rt::unpack(s_and_end)?;
        let mut pairs = PyList::empty(py).into_any();
        frame.line(144);
        let pairs_append = pairs.getattr("append")?;
        // Backwards compatibility
        if memo.is_none() {
            memo = PyDict::new(py).into_any();
        }
        frame.line(148);
        let memo_get = memo.getattr("setdefault")?;
        // Use a slice to prevent IndexError from being raised, the following
        // check will raise a more specific ValueError if the string is empty
        frame.line(151);
        let mut nextchar = s.get_item(rt::slice(py, &end, end.add(1)?)?)?;
        // Normally we expect nextchar == '"'
        frame.line(153);
        if nextchar.ne("\"")? {
            frame.line(154);
            if _ws.contains(&nextchar)? {
                frame.line(155);
                end = _w.call1((&s, &end))?.call_method0("end")?;
                frame.line(156);
                nextchar = s.get_item(rt::slice(py, &end, end.add(1)?)?)?;
            }
            // Trivial empty object
            frame.line(158);
            if nextchar.eq("}")? {
                if !object_pairs_hook.is_none() {
                    frame.line(160);
                    result = object_pairs_hook.call1((&pairs,))?;
                    frame.line(161);
                    return Ok(PyTuple::new(py, [&result, &end.add(1)?])?.into_any());
                }
                pairs = PyDict::new(py).into_any();
                if !object_hook.is_none() {
                    frame.line(164);
                    pairs = object_hook.call1((&pairs,))?;
                }
                frame.line(165);
                return Ok(PyTuple::new(py, [&pairs, &end.add(1)?])?.into_any());
            } else {
                frame.line(166);
                if nextchar.ne("\"")? {
                    frame.line(167);
                    return Err(frame.raise(m.global("JSONDecodeError")?.call1((
                        "Expecting property name enclosed in double quotes",
                        &s,
                        &end,
                    ))?));
                }
            }
        }
        frame.line(169);
        end = rt::iadd(&end, 1)?;
        loop {
            frame.line(171);
            [key, end] = rt::unpack(&m.global("scanstring")?.call1((&s, &end, strict))?)?;
            frame.line(172);
            key = memo_get.call1((&key, &key))?;
            // To skip some function call overhead we optimize the fast paths where
            // the JSON key separator is ": " or just ":".
            frame.line(175);
            if s.get_item(rt::slice(py, &end, end.add(1)?)?)?.ne(":")? {
                frame.line(176);
                end = _w.call1((&s, &end))?.call_method0("end")?;
                frame.line(177);
                if s.get_item(rt::slice(py, &end, end.add(1)?)?)?.ne(":")? {
                    frame.line(178);
                    return Err(frame.raise(m.global("JSONDecodeError")?.call1((
                        "Expecting ':' delimiter",
                        &s,
                        &end,
                    ))?));
                }
            }
            frame.line(179);
            end = rt::iadd(&end, 1)?;
            match (|| -> PyResult<()> {
                frame.line(182);
                if _ws.contains(s.get_item(&end)?)? {
                    frame.line(183);
                    end = rt::iadd(&end, 1)?;
                    frame.line(184);
                    if _ws.contains(s.get_item(&end)?)? {
                        frame.line(185);
                        end = _w.call1((&s, end.add(1)?))?.call_method0("end")?;
                    }
                }
                Ok(())
            })() {
                Ok(()) => {}
                Err(err2) if err2.is_instance_of::<PyIndexError>(py) => {}
                Err(err2) => return Err(err2),
            }
            match (|| -> PyResult<()> {
                frame.line(190);
                let [value2, end2] = rt::unpack(&scan_once.call1((&s, &end))?)?;
                value = Some(value2);
                end = end2;
                Ok(())
            })() {
                Ok(()) => {}
                Err(err) if err.is_instance_of::<PyStopIteration>(py) => {
                    let err = frame.catch(err);
                    let _handling = rt::Handling::new(&err);
                    frame.line(192);
                    return Err(frame.raise_from(
                        m.global("JSONDecodeError")?.call1((
                            "Expecting value",
                            &s,
                            err.getattr("value")?,
                        ))?,
                        py.None(),
                    ));
                }
                Err(err2) => return Err(err2),
            }
            frame.line(193);
            pairs_append.call1((PyTuple::new(py, [&key, rt::local(&value, "value")?])?,))?;
            match (|| -> PyResult<()> {
                frame.line(195);
                nextchar = s.get_item(&end)?;
                frame.line(196);
                if _ws.contains(&nextchar)? {
                    frame.line(197);
                    end = _w.call1((&s, end.add(1)?))?.call_method0("end")?;
                    frame.line(198);
                    nextchar = s.get_item(&end)?;
                }
                Ok(())
            })() {
                Ok(()) => {}
                Err(err2) if err2.is_instance_of::<PyIndexError>(py) => nextchar = rt::str(py, ""),
                Err(err2) => return Err(err2),
            }
            frame.line(201);
            end = rt::iadd(&end, 1)?;
            frame.line(203);
            if nextchar.eq("}")? {
                break;
            } else {
                frame.line(205);
                if nextchar.ne(",")? {
                    frame.line(206);
                    return Err(frame.raise(m.global("JSONDecodeError")?.call1((
                        "Expecting ',' delimiter",
                        &s,
                        end.sub(1)?,
                    ))?));
                }
            }
            frame.line(207);
            comma_idx = end.sub(1)?;
            frame.line(208);
            end = _w.call1((&s, &end))?.call_method0("end")?;
            frame.line(209);
            nextchar = s.get_item(rt::slice(py, &end, end.add(1)?)?)?;
            frame.line(210);
            end = rt::iadd(&end, 1)?;
            frame.line(211);
            if nextchar.ne("\"")? {
                frame.line(212);
                if nextchar.eq("}")? {
                    frame.line(213);
                    return Err(frame.raise(m.global("JSONDecodeError")?.call1((
                        "Illegal trailing comma before end of object",
                        &s,
                        &comma_idx,
                    ))?));
                }
                frame.line(214);
                return Err(frame.raise(m.global("JSONDecodeError")?.call1((
                    "Expecting property name enclosed in double quotes",
                    &s,
                    end.sub(1)?,
                ))?));
            }
            py.check_signals()?;
        }
        if !object_pairs_hook.is_none() {
            frame.line(217);
            result = object_pairs_hook.call1((&pairs,))?;
            frame.line(218);
            return Ok(PyTuple::new(py, [&result, &end])?.into_any());
        }
        frame.line(219);
        pairs = rt::builtin(py, "dict")?.call1((&pairs,))?;
        if !object_hook.is_none() {
            frame.line(221);
            pairs = object_hook.call1((&pairs,))?;
        }
        frame.line(222);
        Ok(PyTuple::new(py, [&pairs, &end])?.into_any())
    })
}

// json/decoder.py:224
pub fn JSONArray<'py>(
    m: &rt::Module<'py>,
    s_and_end: &Bound<'py, PyAny>,
    scan_once: &Bound<'py, PyAny>,
    _w: &Bound<'py, PyAny>,
    _ws: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "JSONArray", 224, |frame| {
        let mut value: Option<Bound<'py, PyAny>> = None;
        let mut comma_idx: Bound<'py, PyAny>;
        frame.line(225);
        let [s, mut end] = rt::unpack(s_and_end)?;
        let values = PyList::empty(py).into_any();
        frame.line(227);
        let mut nextchar = s.get_item(rt::slice(py, &end, end.add(1)?)?)?;
        frame.line(228);
        if _ws.contains(&nextchar)? {
            frame.line(229);
            end = _w.call1((&s, end.add(1)?))?.call_method0("end")?;
            frame.line(230);
            nextchar = s.get_item(rt::slice(py, &end, end.add(1)?)?)?;
        }
        // Look-ahead for trivial empty array
        frame.line(232);
        if nextchar.eq("]")? {
            frame.line(233);
            return Ok(PyTuple::new(py, [&values, &end.add(1)?])?.into_any());
        }
        frame.line(234);
        let _append = values.getattr("append")?;
        loop {
            match (|| -> PyResult<()> {
                frame.line(237);
                let [value2, end2] = rt::unpack(&scan_once.call1((&s, &end))?)?;
                value = Some(value2);
                end = end2;
                Ok(())
            })() {
                Ok(()) => {}
                Err(err) if err.is_instance_of::<PyStopIteration>(py) => {
                    let err = frame.catch(err);
                    let _handling = rt::Handling::new(&err);
                    frame.line(239);
                    return Err(frame.raise_from(
                        m.global("JSONDecodeError")?.call1((
                            "Expecting value",
                            &s,
                            err.getattr("value")?,
                        ))?,
                        py.None(),
                    ));
                }
                Err(err2) => return Err(err2),
            }
            frame.line(240);
            _append.call1((rt::local(&value, "value")?,))?;
            frame.line(241);
            nextchar = s.get_item(rt::slice(py, &end, end.add(1)?)?)?;
            frame.line(242);
            if _ws.contains(&nextchar)? {
                frame.line(243);
                end = _w.call1((&s, end.add(1)?))?.call_method0("end")?;
                frame.line(244);
                nextchar = s.get_item(rt::slice(py, &end, end.add(1)?)?)?;
            }
            frame.line(245);
            end = rt::iadd(&end, 1)?;
            frame.line(246);
            if nextchar.eq("]")? {
                break;
            } else {
                frame.line(248);
                if nextchar.ne(",")? {
                    frame.line(249);
                    return Err(frame.raise(m.global("JSONDecodeError")?.call1((
                        "Expecting ',' delimiter",
                        &s,
                        end.sub(1)?,
                    ))?));
                }
            }
            frame.line(250);
            comma_idx = end.sub(1)?;
            match (|| -> PyResult<()> {
                frame.line(252);
                if _ws.contains(s.get_item(&end)?)? {
                    frame.line(253);
                    end = rt::iadd(&end, 1)?;
                    frame.line(254);
                    if _ws.contains(s.get_item(&end)?)? {
                        frame.line(255);
                        end = _w.call1((&s, end.add(1)?))?.call_method0("end")?;
                    }
                }
                frame.line(256);
                nextchar = s.get_item(rt::slice(py, &end, end.add(1)?)?)?;
                Ok(())
            })() {
                Ok(()) => {}
                Err(err2) if err2.is_instance_of::<PyIndexError>(py) => {}
                Err(err2) => return Err(err2),
            }
            frame.line(259);
            if nextchar.eq("]")? {
                frame.line(260);
                return Err(frame.raise(m.global("JSONDecodeError")?.call1((
                    "Illegal trailing comma before end of array",
                    &s,
                    &comma_idx,
                ))?));
            }
            py.check_signals()?;
        }
        frame.line(262);
        Ok(PyTuple::new(py, [&values, &end])?.into_any())
    })
}

/// Subclass of ValueError with the following additional properties:
///
/// msg: The unformatted error message
/// doc: The JSON document being parsed
/// pos: The start index of doc where parsing failed
/// lineno: The line corresponding to pos
/// colno: The column corresponding to pos
///
pub mod json_decode_error {
    use super::*;

    // json/decoder.py:23
    /// The body of `class JSONDecodeError`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "JSONDecodeError", 23, |frame| {
            frame.line(24);
            ns.set("__doc__", "Subclass of ValueError with the following additional properties:\n\nmsg: The unformatted error message\ndoc: The JSON document being parsed\npos: The start index of doc where parsing failed\nlineno: The line corresponding to pos\ncolno: The column corresponding to pos\n\n")?;
            frame.line(34);
            ns.set(
                "__init__",
                rt::function!(
                    ns,
                    __init__(slf as "self", msg, doc, pos),
                    qualname = "JSONDecodeError.__init__"
                )?,
            )?;
            frame.line(45);
            ns.set(
                "__reduce__",
                rt::function!(
                    ns,
                    __reduce__(slf as "self"),
                    qualname = "JSONDecodeError.__reduce__"
                )?,
            )?;
            Ok(())
        })
    }

    // Note that this exception is used from _json
    // json/decoder.py:34
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        msg: &Bound<'py, PyAny>,
        doc: &Bound<'py, PyAny>,
        pos: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "JSONDecodeError.__init__", 34, |frame| {
            frame.line(35);
            let lineno = doc.call_method1("count", ("\n", 0, pos))?.add(1)?;
            frame.line(36);
            let colno = pos.sub(doc.call_method1("rfind", ("\n", 0, pos))?)?;
            frame.line(37);
            let errmsg = rt::str(py, "%s: line %d column %d (char %d)")
                .rem(PyTuple::new(py, [msg, &lineno, &colno, pos])?)?;
            frame.line(38);
            rt::builtin(py, "ValueError")?.call_method1("__init__", (slf, &errmsg))?;
            frame.line(39);
            slf.setattr("msg", msg)?;
            frame.line(40);
            slf.setattr("doc", doc)?;
            frame.line(41);
            slf.setattr("pos", pos)?;
            frame.line(42);
            slf.setattr("lineno", &lineno)?;
            frame.line(43);
            slf.setattr("colno", &colno)?;
            Ok(rt::none(py))
        })
    }

    // json/decoder.py:45
    pub fn __reduce__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "JSONDecodeError.__reduce__", 45, |frame| {
            frame.line(46);
            Ok(PyTuple::new(
                py,
                [
                    &slf.getattr("__class__")?,
                    PyTuple::new(
                        py,
                        [
                            &slf.getattr("msg")?,
                            &slf.getattr("doc")?,
                            &slf.getattr("pos")?,
                        ],
                    )?
                    .as_any(),
                ],
            )?
            .into_any())
        })
    }
}

/// Simple JSON <https://json.org> decoder
///
/// Performs the following translations in decoding by default:
///
/// +---------------+-------------------+
/// | JSON          | Python            |
/// +===============+===================+
/// | object        | dict              |
/// +---------------+-------------------+
/// | array         | list              |
/// +---------------+-------------------+
/// | string        | str               |
/// +---------------+-------------------+
/// | number (int)  | int               |
/// +---------------+-------------------+
/// | number (real) | float             |
/// +---------------+-------------------+
/// | true          | True              |
/// +---------------+-------------------+
/// | false         | False             |
/// +---------------+-------------------+
/// | null          | None              |
/// +---------------+-------------------+
///
/// It also understands ``NaN``, ``Infinity``, and ``-Infinity`` as
/// their corresponding ``float`` values, which is outside the JSON spec.
///
pub mod json_decoder {
    use super::*;

    // json/decoder.py:265
    /// The body of `class JSONDecoder`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        let py = m.py();
        rt::frame!(ns, "JSONDecoder", 265, |frame| {
            frame.line(266);
            ns.set("__doc__", "Simple JSON <https://json.org> decoder\n\nPerforms the following translations in decoding by default:\n\n+---------------+-------------------+\n| JSON          | Python            |\n+===============+===================+\n| object        | dict              |\n+---------------+-------------------+\n| array         | list              |\n+---------------+-------------------+\n| string        | str               |\n+---------------+-------------------+\n| number (int)  | int               |\n+---------------+-------------------+\n| number (real) | float             |\n+---------------+-------------------+\n| true          | True              |\n+---------------+-------------------+\n| false         | False             |\n+---------------+-------------------+\n| null          | None              |\n+---------------+-------------------+\n\nIt also understands ``NaN``, ``Infinity``, and ``-Infinity`` as\ntheir corresponding ``float`` values, which is outside the JSON spec.\n\n")?;
            frame.line(295);
            ns.set("__init__", rt::function!(
    ns,
    __init__(slf as "self", *, object_hook = py.None(), parse_float = py.None(), parse_int = py.None(), parse_constant = py.None(), strict = true, object_pairs_hook = py.None()),
    qualname = "JSONDecoder.__init__",
    doc = "``object_hook``, if specified, will be called with the result\nof every JSON object decoded and its return value will be used in\nplace of the given ``dict``.  This can be used to provide custom\ndeserializations (e.g. to support JSON-RPC class hinting).\n\n``object_pairs_hook``, if specified will be called with the result\nof every JSON object decoded with an ordered list of pairs.  The\nreturn value of ``object_pairs_hook`` will be used instead of the\n``dict``.  This feature can be used to implement custom decoders.\nIf ``object_hook`` is also defined, the ``object_pairs_hook`` takes\npriority.\n\n``parse_float``, if specified, will be called with the string\nof every JSON float to be decoded. By default this is equivalent to\nfloat(num_str). This can be used to use another datatype or parser\nfor JSON floats (e.g. decimal.Decimal).\n\n``parse_int``, if specified, will be called with the string\nof every JSON int to be decoded. By default this is equivalent to\nint(num_str). This can be used to use another datatype or parser\nfor JSON integers (e.g. float).\n\n``parse_constant``, if specified, will be called with one of the\nfollowing strings: -Infinity, Infinity, NaN.\nThis can be used to raise an exception if invalid JSON numbers\nare encountered.\n\nIf ``strict`` is false (true is the default), then control\ncharacters will be allowed inside strings.  Control characters in\nthis context are those with character codes in the 0-31 range,\nincluding ``'\\t'`` (tab), ``'\\n'``, ``'\\r'`` and ``'\\0'``.\n"
)?)?;
            frame.line(343);
            ns.set("decode", rt::function!(
    ns,
    decode(slf as "self", s, _w = ns.get("WHITESPACE")?.getattr("match")?),
    qualname = "JSONDecoder.decode",
    doc = "Return the Python representation of ``s`` (a ``str`` instance\ncontaining a JSON document).\n\n"
)?)?;
            frame.line(354);
            ns.set("raw_decode", rt::function!(
    ns,
    raw_decode(slf as "self", s, idx = 0),
    qualname = "JSONDecoder.raw_decode",
    doc = "Decode a JSON document from ``s`` (a ``str`` beginning with\na JSON document) and return a 2-tuple of the Python\nrepresentation and the index in ``s`` where the document ended.\n\nThis can be used to decode a JSON document from a string that may\nhave extraneous data at the end.\n\n"
)?)?;
            Ok(())
        })
    }

    // json/decoder.py:295
    /// ``object_hook``, if specified, will be called with the result
    /// of every JSON object decoded and its return value will be used in
    /// place of the given ``dict``.  This can be used to provide custom
    /// deserializations (e.g. to support JSON-RPC class hinting).
    ///
    /// ``object_pairs_hook``, if specified will be called with the result
    /// of every JSON object decoded with an ordered list of pairs.  The
    /// return value of ``object_pairs_hook`` will be used instead of the
    /// ``dict``.  This feature can be used to implement custom decoders.
    /// If ``object_hook`` is also defined, the ``object_pairs_hook`` takes
    /// priority.
    ///
    /// ``parse_float``, if specified, will be called with the string
    /// of every JSON float to be decoded. By default this is equivalent to
    /// float(num_str). This can be used to use another datatype or parser
    /// for JSON floats (e.g. decimal.Decimal).
    ///
    /// ``parse_int``, if specified, will be called with the string
    /// of every JSON int to be decoded. By default this is equivalent to
    /// int(num_str). This can be used to use another datatype or parser
    /// for JSON integers (e.g. float).
    ///
    /// ``parse_constant``, if specified, will be called with one of the
    /// following strings: -Infinity, Infinity, NaN.
    /// This can be used to raise an exception if invalid JSON numbers
    /// are encountered.
    ///
    /// If ``strict`` is false (true is the default), then control
    /// characters will be allowed inside strings.  Control characters in
    /// this context are those with character codes in the 0-31 range,
    /// including ``'\t'`` (tab), ``'\n'``, ``'\r'`` and ``'\0'``.
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        object_hook: &Bound<'py, PyAny>,
        parse_float: &Bound<'py, PyAny>,
        parse_int: &Bound<'py, PyAny>,
        parse_constant: &Bound<'py, PyAny>,
        strict: &Bound<'py, PyAny>,
        object_pairs_hook: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "JSONDecoder.__init__", 295, |frame| {
            frame.line(330);
            slf.setattr("object_hook", object_hook)?;
            frame.line(331);
            slf.setattr(
                "parse_float",
                if parse_float.is_truthy()? {
                    parse_float.clone()
                } else {
                    rt::builtin(py, "float")?
                },
            )?;
            frame.line(332);
            slf.setattr(
                "parse_int",
                if parse_int.is_truthy()? {
                    parse_int.clone()
                } else {
                    rt::builtin(py, "int")?
                },
            )?;
            frame.line(333);
            slf.setattr(
                "parse_constant",
                if parse_constant.is_truthy()? {
                    parse_constant.clone()
                } else {
                    m.global("_CONSTANTS")?.getattr("__getitem__")?
                },
            )?;
            frame.line(334);
            slf.setattr("strict", strict)?;
            frame.line(335);
            slf.setattr("object_pairs_hook", object_pairs_hook)?;
            frame.line(336);
            slf.setattr("parse_object", m.global("JSONObject")?)?;
            frame.line(337);
            slf.setattr("parse_array", m.global("JSONArray")?)?;
            frame.line(338);
            slf.setattr("parse_string", m.global("scanstring")?)?;
            frame.line(339);
            slf.setattr("memo", PyDict::new(py))?;
            frame.line(340);
            slf.setattr(
                "scan_once",
                m.global("scanner")?.call_method1("make_scanner", (slf,))?,
            )?;
            Ok(rt::none(py))
        })
    }

    // json/decoder.py:343
    /// Return the Python representation of ``s`` (a ``str`` instance
    /// containing a JSON document).
    ///
    pub fn decode<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        s: &Bound<'py, PyAny>,
        _w: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "JSONDecoder.decode", 343, |frame| {
            frame.line(348);
            let [obj, mut end] = rt::unpack(&rt::call!(
                slf.getattr("raw_decode")?,
                s,
                idx = _w.call1((s, 0))?.call_method0("end")?
            )?)?;
            frame.line(349);
            end = _w.call1((s, &end))?.call_method0("end")?;
            frame.line(350);
            if end.ne(s.len()?)? {
                frame.line(351);
                return Err(frame.raise(m.global("JSONDecodeError")?.call1((
                    "Extra data",
                    s,
                    &end,
                ))?));
            }
            Ok(obj)
        })
    }

    // json/decoder.py:354
    /// Decode a JSON document from ``s`` (a ``str`` beginning with
    /// a JSON document) and return a 2-tuple of the Python
    /// representation and the index in ``s`` where the document ended.
    ///
    /// This can be used to decode a JSON document from a string that may
    /// have extraneous data at the end.
    ///
    pub fn raw_decode<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        s: &Bound<'py, PyAny>,
        idx: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "JSONDecoder.raw_decode", 354, |frame| {
            let mut obj: Option<Bound<'py, PyAny>> = None;
            let mut end: Option<Bound<'py, PyAny>> = None;
            match (|| -> PyResult<()> {
                frame.line(364);
                let [obj2, end2] = rt::unpack(&slf.call_method1("scan_once", (s, idx))?)?;
                obj = Some(obj2);
                end = Some(end2);
                Ok(())
            })() {
                Ok(()) => {}
                Err(err) if err.is_instance_of::<PyStopIteration>(py) => {
                    let err = frame.catch(err);
                    let _handling = rt::Handling::new(&err);
                    frame.line(366);
                    return Err(frame.raise_from(
                        m.global("JSONDecodeError")?.call1((
                            "Expecting value",
                            s,
                            err.getattr("value")?,
                        ))?,
                        py.None(),
                    ));
                }
                Err(err2) => return Err(err2),
            }
            frame.line(367);
            Ok(PyTuple::new(py, [rt::local(&obj, "obj")?, rt::local(&end, "end")?])?.into_any())
        })
    }
}

/// Executes the body of `json/decoder.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        m.set_global("__doc__", "Implementation of JSONDecoder\n")?;
        // Copied unmodified (below this header) from Lib/json/decoder.py of CPython 3.14.9,
        // licensed under the PSF License Agreement. A steelix test project:
        // tests/test_projects.py runs it transpiled and with CPython and compares.
        frame.line(6);
        m.set_global("re", rt::import(m, "re")?)?;
        frame.line(8);
        let json = rt::import_names(m, "json", &["scanner"])?;
        m.set_global("scanner", rt::import_from(&json, "scanner")?)?;
        match (|| -> PyResult<()> {
            frame.line(10);
            let _json = rt::import_names(m, "_json", &["scanstring"])?;
            m.set_global("c_scanstring", rt::import_from(&_json, "scanstring")?)?;
            Ok(())
        })() {
            Ok(()) => {}
            Err(err) if err.is_instance_of::<PyImportError>(py) => {
                let _handling = rt::Handling::new(&frame.catch(err));
                frame.line(12);
                m.set_global("c_scanstring", py.None())?;
            }
            Err(err) => return Err(err),
        }
        frame.line(14);
        m.set_global(
            "__all__",
            PyList::new(py, ["JSONDecoder", "JSONDecodeError"])?,
        )?;
        frame.line(16);
        m.set_global(
            "FLAGS",
            m.global("re")?
                .getattr("VERBOSE")?
                .bitor(m.global("re")?.getattr("MULTILINE")?)?
                .bitor(m.global("re")?.getattr("DOTALL")?)?,
        )?;
        frame.line(18);
        m.set_global("NaN", rt::builtin(py, "float")?.call1(("nan",))?)?;
        frame.line(19);
        m.set_global("PosInf", rt::builtin(py, "float")?.call1(("inf",))?)?;
        frame.line(20);
        m.set_global("NegInf", rt::builtin(py, "float")?.call1(("-inf",))?)?;
        frame.line(23);
        m.set_global(
            "JSONDecodeError",
            rt::class!(
                m,
                "JSONDecodeError"(rt::builtin(py, "ValueError")?),
                json_decode_error::body,
                line = 23,
                attributes = ["colno", "doc", "lineno", "msg", "pos"]
            )?,
        )?;
        frame.line(49);
        m.set_global(
            "_CONSTANTS",
            [
                ("-Infinity", &m.global("NegInf")?),
                ("Infinity", &m.global("PosInf")?),
                ("NaN", &m.global("NaN")?),
            ]
            .into_py_dict(py)?,
        )?;
        frame.line(56);
        m.set_global(
            "HEXDIGITS",
            m.global("re")?
                .getattr("compile")?
                .call1(("[0-9A-Fa-f]{4}", m.global("FLAGS")?))?,
        )?;
        frame.line(57);
        m.set_global(
            "STRINGCHUNK",
            m.global("re")?
                .getattr("compile")?
                .call1(("(.*?)([\"\\\\\\x00-\\x1f])", m.global("FLAGS")?))?,
        )?;
        frame.line(58);
        m.set_global(
            "BACKSLASH",
            [
                ("\"", "\""),
                ("\\", "\\"),
                ("/", "/"),
                ("b", "\u{8}"),
                ("f", "\u{c}"),
                ("n", "\n"),
                ("r", "\r"),
                ("t", "\t"),
            ]
            .into_py_dict(py)?,
        )?;
        frame.line(63);
        rt::def!(
            m,
            _decode_uXXXX(s, pos, _m = m.global("HEXDIGITS")?.getattr("match")?)
        )?;
        frame.line(73);
        rt::def!(
            m,
            py_scanstring(
                s,
                end,
                strict = true,
                _b = m.global("BACKSLASH")?,
                _m = m.global("STRINGCHUNK")?.getattr("match")?
            ),
            doc = "Scan the string s for a JSON string. End is the index of the\ncharacter in s after the quote that started the JSON string.\nUnescapes all valid JSON string escape sequences and raises ValueError\non attempt to decode an invalid string. If strict is False then literal\ncontrol characters are allowed in the string.\n\nReturns a tuple of the decoded string and the index of the character in s\nafter the end quote."
        )?;
        // Use speedup if available
        frame.line(134);
        {
            let c_scanstring = m.global("c_scanstring")?;
            m.set_global(
                "scanstring",
                if c_scanstring.is_truthy()? {
                    c_scanstring
                } else {
                    m.global("py_scanstring")?
                },
            )?;
        }
        frame.line(136);
        m.set_global(
            "WHITESPACE",
            m.global("re")?
                .getattr("compile")?
                .call1(("[ \\t\\n\\r]*", m.global("FLAGS")?))?,
        )?;
        frame.line(137);
        m.set_global("WHITESPACE_STR", " \t\n\r")?;
        frame.line(140);
        rt::def!(
            m,
            JSONObject(
                s_and_end,
                strict,
                scan_once,
                object_hook,
                object_pairs_hook,
                memo = py.None(),
                _w = m.global("WHITESPACE")?.getattr("match")?,
                _ws = m.global("WHITESPACE_STR")?
            )
        )?;
        frame.line(224);
        rt::def!(
            m,
            JSONArray(
                s_and_end,
                scan_once,
                _w = m.global("WHITESPACE")?.getattr("match")?,
                _ws = m.global("WHITESPACE_STR")?
            )
        )?;
        frame.line(265);
        m.set_global(
            "JSONDecoder",
            rt::class!(
                m,
                "JSONDecoder"(rt::builtin(py, "object")?),
                json_decoder::body,
                line = 265,
                attributes = [
                    "memo",
                    "object_hook",
                    "object_pairs_hook",
                    "parse_array",
                    "parse_constant",
                    "parse_float",
                    "parse_int",
                    "parse_object",
                    "parse_string",
                    "scan_once",
                    "strict"
                ]
            )?,
        )?;
        Ok(())
    })
}
