pub mod decoder;
pub mod encoder;
pub mod scanner;
pub mod tool;

use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;

// json/__init__.py:123
/// Serialize ``obj`` as a JSON formatted stream to ``fp`` (a
/// ``.write()``-supporting file-like object).
///
/// If ``skipkeys`` is true then ``dict`` keys that are not basic types
/// (``str``, ``int``, ``float``, ``bool``, ``None``) will be skipped
/// instead of raising a ``TypeError``.
///
/// If ``ensure_ascii`` is false, then the strings written to ``fp`` can
/// contain non-ASCII and non-printable characters if they appear in strings
/// contained in ``obj``. Otherwise, all such characters are escaped in JSON
/// strings.
///
/// If ``check_circular`` is false, then the circular reference check
/// for container types will be skipped and a circular reference will
/// result in an ``RecursionError`` (or worse).
///
/// If ``allow_nan`` is false, then it will be a ``ValueError`` to
/// serialize out of range ``float`` values (``nan``, ``inf``, ``-inf``)
/// in strict compliance of the JSON specification, instead of using the
/// JavaScript equivalents (``NaN``, ``Infinity``, ``-Infinity``).
///
/// If ``indent`` is a non-negative integer, then JSON array elements and
/// object members will be pretty-printed with that indent level. An indent
/// level of 0 will only insert newlines. ``None`` is the default and gives
/// a representation with no newlines inserted.
///
/// If specified, ``separators`` should be an ``(item_separator,
/// key_separator)`` tuple.  The default is ``(', ', ': ')`` if *indent* is
/// ``None`` and ``(',', ': ')`` otherwise.  To get the most compact JSON
/// representation, you should specify ``(',', ':')`` to eliminate
/// whitespace.
///
/// ``default(obj)`` is a function that should return a serializable version
/// of obj or raise TypeError. The default simply raises TypeError.
///
/// If *sort_keys* is true (default: ``False``), then the output of
/// dictionaries will be sorted by key.
///
/// To use a custom ``JSONEncoder`` subclass (e.g. one that overrides the
/// ``.default()`` method to serialize additional types), specify it with
/// the ``cls`` kwarg; otherwise ``JSONEncoder`` is used.
///
pub fn dump<'py>(
    m: &rt::Module<'py>,
    obj: &Bound<'py, PyAny>,
    fp: &Bound<'py, PyAny>,
    skipkeys: &Bound<'py, PyAny>,
    ensure_ascii: &Bound<'py, PyAny>,
    check_circular: &Bound<'py, PyAny>,
    allow_nan: &Bound<'py, PyAny>,
    cls: &Bound<'py, PyAny>,
    indent: &Bound<'py, PyAny>,
    separators: &Bound<'py, PyAny>,
    default: &Bound<'py, PyAny>,
    sort_keys: &Bound<'py, PyAny>,
    kw: &Bound<'py, PyDict>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "dump", 123, |frame| {
        let mut cls = cls.clone();
        let iterable: Bound<'py, PyAny>;
        // cached encoder
        frame.line(170);
        if !skipkeys.is_truthy()?
            && ensure_ascii.is_truthy()?
            && check_circular.is_truthy()?
            && allow_nan.is_truthy()?
            && cls.is_none()
            && indent.is_none()
            && separators.is_none()
            && default.is_none()
            && !sort_keys.is_truthy()?
            && !kw.is_truthy()?
        {
            frame.line(174);
            iterable = m
                .global("_default_encoder")?
                .call_method1("iterencode", (obj,))?;
        } else {
            if cls.is_none() {
                frame.line(177);
                cls = m.global("JSONEncoder")?;
            }
            frame.line(178);
            iterable = rt::call!(
                cls,
                skipkeys = skipkeys,
                ensure_ascii = ensure_ascii,
                check_circular = check_circular,
                allow_nan = allow_nan,
                indent = indent,
                separators = separators,
                default = default,
                sort_keys = sort_keys,
                **kw
            )?
            .call_method1("iterencode", (obj,))?;
        }
        // could accelerate with writelines in some versions of Python, at
        // a debuggability cost
        frame.line(184);
        for chunk in iterable.try_iter()? {
            let chunk = chunk?;
            frame.line(185);
            fp.call_method1("write", (&chunk,))?;
            py.check_signals()?;
            frame.line(184);
        }
        Ok(rt::none(py))
    })
}

// json/__init__.py:188
/// Serialize ``obj`` to a JSON formatted ``str``.
///
/// If ``skipkeys`` is true then ``dict`` keys that are not basic types
/// (``str``, ``int``, ``float``, ``bool``, ``None``) will be skipped
/// instead of raising a ``TypeError``.
///
/// If ``ensure_ascii`` is false, then the return value can contain
/// non-ASCII and non-printable characters if they appear in strings
/// contained in ``obj``.  Otherwise, all such characters are escaped in
/// JSON strings.
///
/// If ``check_circular`` is false, then the circular reference check
/// for container types will be skipped and a circular reference will
/// result in an ``RecursionError`` (or worse).
///
/// If ``allow_nan`` is false, then it will be a ``ValueError`` to
/// serialize out of range ``float`` values (``nan``, ``inf``, ``-inf``) in
/// strict compliance of the JSON specification, instead of using the
/// JavaScript equivalents (``NaN``, ``Infinity``, ``-Infinity``).
///
/// If ``indent`` is a non-negative integer, then JSON array elements and
/// object members will be pretty-printed with that indent level. An indent
/// level of 0 will only insert newlines. ``None`` is the default and gives
/// a representation with no newlines inserted.
///
/// If specified, ``separators`` should be an ``(item_separator,
/// key_separator)`` tuple.  The default is ``(', ', ': ')`` if *indent* is
/// ``None`` and ``(',', ': ')`` otherwise.  To get the most compact JSON
/// representation, you should specify ``(',', ':')`` to eliminate
/// whitespace.
///
/// ``default(obj)`` is a function that should return a serializable version
/// of obj or raise TypeError. The default simply raises TypeError.
///
/// If *sort_keys* is true (default: ``False``), then the output of
/// dictionaries will be sorted by key.
///
/// To use a custom ``JSONEncoder`` subclass (e.g. one that overrides the
/// ``.default()`` method to serialize additional types), specify it with
/// the ``cls`` kwarg; otherwise ``JSONEncoder`` is used.
///
pub fn dumps<'py>(
    m: &rt::Module<'py>,
    obj: &Bound<'py, PyAny>,
    skipkeys: &Bound<'py, PyAny>,
    ensure_ascii: &Bound<'py, PyAny>,
    check_circular: &Bound<'py, PyAny>,
    allow_nan: &Bound<'py, PyAny>,
    cls: &Bound<'py, PyAny>,
    indent: &Bound<'py, PyAny>,
    separators: &Bound<'py, PyAny>,
    default: &Bound<'py, PyAny>,
    sort_keys: &Bound<'py, PyAny>,
    kw: &Bound<'py, PyDict>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "dumps", 188, |frame| {
        let mut cls = cls.clone();
        // cached encoder
        frame.line(234);
        if !skipkeys.is_truthy()?
            && ensure_ascii.is_truthy()?
            && check_circular.is_truthy()?
            && allow_nan.is_truthy()?
            && cls.is_none()
            && indent.is_none()
            && separators.is_none()
            && default.is_none()
            && !sort_keys.is_truthy()?
            && !kw.is_truthy()?
        {
            frame.line(238);
            return m.global("_default_encoder")?.call_method1("encode", (obj,));
        }
        if cls.is_none() {
            frame.line(240);
            cls = m.global("JSONEncoder")?;
        }
        frame.line(241);
        rt::call!(
            cls,
            skipkeys = skipkeys,
            ensure_ascii = ensure_ascii,
            check_circular = check_circular,
            allow_nan = allow_nan,
            indent = indent,
            separators = separators,
            default = default,
            sort_keys = sort_keys,
            **kw
        )?
        .call_method1("encode", (obj,))
    })
}

// json/__init__.py:251
pub fn detect_encoding<'py>(
    m: &rt::Module<'py>,
    b: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "detect_encoding", 251, |frame| {
        frame.line(252);
        let bstartswith = b.getattr("startswith")?;
        frame.line(253);
        if bstartswith
            .call1((PyTuple::new(
                py,
                [
                    &m.global("codecs")?.getattr("BOM_UTF32_BE")?,
                    &m.global("codecs")?.getattr("BOM_UTF32_LE")?,
                ],
            )?,))?
            .is_truthy()?
        {
            return Ok(rt::str(py, "utf-32"));
        }
        frame.line(255);
        if bstartswith
            .call1((PyTuple::new(
                py,
                [
                    &m.global("codecs")?.getattr("BOM_UTF16_BE")?,
                    &m.global("codecs")?.getattr("BOM_UTF16_LE")?,
                ],
            )?,))?
            .is_truthy()?
        {
            return Ok(rt::str(py, "utf-16"));
        }
        frame.line(257);
        if bstartswith
            .call1((m.global("codecs")?.getattr("BOM_UTF8")?,))?
            .is_truthy()?
        {
            return Ok(rt::str(py, "utf-8-sig"));
        }
        frame.line(260);
        if b.len()? >= 4 {
            frame.line(261);
            if !b.get_item(0)?.is_truthy()? {
                // 00 00 -- -- - utf-32-be
                // 00 XX -- -- - utf-16-be
                frame.line(264);
                return Ok(if b.get_item(1)?.is_truthy()? {
                    rt::str(py, "utf-16-be")
                } else {
                    rt::str(py, "utf-32-be")
                });
            }
            frame.line(265);
            if !b.get_item(1)?.is_truthy()? {
                // XX 00 00 00 - utf-32-le
                // XX 00 00 XX - utf-16-le
                // XX 00 XX -- - utf-16-le
                frame.line(269);
                return Ok(
                    if b.get_item(2)?.is_truthy()? || b.get_item(3)?.is_truthy()? {
                        rt::str(py, "utf-16-le")
                    } else {
                        rt::str(py, "utf-32-le")
                    },
                );
            }
        } else {
            frame.line(270);
            if b.len()? == 2 {
                frame.line(271);
                if !b.get_item(0)?.is_truthy()? {
                    // 00 XX - utf-16-be
                    return Ok(rt::str(py, "utf-16-be"));
                }
                frame.line(274);
                if !b.get_item(1)?.is_truthy()? {
                    // XX 00 - utf-16-le
                    return Ok(rt::str(py, "utf-16-le"));
                }
            }
        }
        // default
        Ok(rt::str(py, "utf-8"))
    })
}

// json/__init__.py:281
/// Deserialize ``fp`` (a ``.read()``-supporting file-like object containing
/// a JSON document) to a Python object.
///
/// ``object_hook`` is an optional function that will be called with the
/// result of any object literal decode (a ``dict``). The return value of
/// ``object_hook`` will be used instead of the ``dict``. This feature
/// can be used to implement custom decoders (e.g. JSON-RPC class hinting).
///
/// ``object_pairs_hook`` is an optional function that will be called with
/// the result of any object literal decoded with an ordered list of pairs.
/// The return value of ``object_pairs_hook`` will be used instead of the
/// ``dict``.  This feature can be used to implement custom decoders.  If
/// ``object_hook`` is also defined, the ``object_pairs_hook`` takes
/// priority.
///
/// To use a custom ``JSONDecoder`` subclass, specify it with the ``cls``
/// kwarg; otherwise ``JSONDecoder`` is used.
pub fn load<'py>(
    m: &rt::Module<'py>,
    fp: &Bound<'py, PyAny>,
    cls: &Bound<'py, PyAny>,
    object_hook: &Bound<'py, PyAny>,
    parse_float: &Bound<'py, PyAny>,
    parse_int: &Bound<'py, PyAny>,
    parse_constant: &Bound<'py, PyAny>,
    object_pairs_hook: &Bound<'py, PyAny>,
    kw: &Bound<'py, PyDict>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "load", 281, |frame| {
        frame.line(301);
        rt::call!(
            m.global("loads")?,
            fp.call_method0("read")?,
            cls = cls,
            object_hook = object_hook,
            parse_float = parse_float,
            parse_int = parse_int,
            parse_constant = parse_constant,
            object_pairs_hook = object_pairs_hook,
            **kw
        )
    })
}

// json/__init__.py:307
/// Deserialize ``s`` (a ``str``, ``bytes`` or ``bytearray`` instance
/// containing a JSON document) to a Python object.
///
/// ``object_hook`` is an optional function that will be called with the
/// result of any object literal decode (a ``dict``). The return value of
/// ``object_hook`` will be used instead of the ``dict``. This feature
/// can be used to implement custom decoders (e.g. JSON-RPC class hinting).
///
/// ``object_pairs_hook`` is an optional function that will be called with
/// the result of any object literal decoded with an ordered list of pairs.
/// The return value of ``object_pairs_hook`` will be used instead of the
/// ``dict``.  This feature can be used to implement custom decoders.  If
/// ``object_hook`` is also defined, the ``object_pairs_hook`` takes
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
/// To use a custom ``JSONDecoder`` subclass, specify it with the ``cls``
/// kwarg; otherwise ``JSONDecoder`` is used.
pub fn loads<'py>(
    m: &rt::Module<'py>,
    s: &Bound<'py, PyAny>,
    cls: &Bound<'py, PyAny>,
    object_hook: &Bound<'py, PyAny>,
    parse_float: &Bound<'py, PyAny>,
    parse_int: &Bound<'py, PyAny>,
    parse_constant: &Bound<'py, PyAny>,
    object_pairs_hook: &Bound<'py, PyAny>,
    kw: &Bound<'py, PyDict>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "loads", 307, |frame| {
        let mut s = s.clone();
        let mut cls = cls.clone();
        frame.line(342);
        if s.is_instance(&rt::builtin(py, "str")?)? {
            frame.line(343);
            if s.call_method1("startswith", ("\u{feff}",))?.is_truthy()? {
                frame.line(344);
                return Err(frame.raise(m.global("JSONDecodeError")?.call1((
                    "Unexpected UTF-8 BOM (decode using utf-8-sig)",
                    &s,
                    0,
                ))?));
            }
        } else {
            frame.line(347);
            if !s.is_instance(
                PyTuple::new(
                    py,
                    [&rt::builtin(py, "bytes")?, &rt::builtin(py, "bytearray")?],
                )?
                .as_any(),
            )? {
                frame.line(348);
                return Err(frame.raise(PyTypeError::new_err((rt::fstring(
                    py,
                    (
                        "the JSON object must be str, bytes or bytearray, not ",
                        rt::format(&s.getattr("__class__")?.getattr("__name__")?, "")?,
                    ),
                )?
                .unbind(),))));
            }
            frame.line(350);
            s = s
                .getattr("decode")?
                .call1((detect_encoding(m, &s)?, "surrogatepass"))?;
        }
        frame.line(352);
        if cls.is_none()
            && object_hook.is_none()
            && parse_int.is_none()
            && parse_float.is_none()
            && parse_constant.is_none()
            && object_pairs_hook.is_none()
            && !kw.is_truthy()?
        {
            frame.line(355);
            return m.global("_default_decoder")?.call_method1("decode", (&s,));
        }
        if cls.is_none() {
            frame.line(357);
            cls = m.global("JSONDecoder")?;
        }
        if !object_hook.is_none() {
            frame.line(359);
            kw.set_item("object_hook", object_hook)?;
        }
        if !object_pairs_hook.is_none() {
            frame.line(361);
            kw.set_item("object_pairs_hook", object_pairs_hook)?;
        }
        if !parse_float.is_none() {
            frame.line(363);
            kw.set_item("parse_float", parse_float)?;
        }
        if !parse_int.is_none() {
            frame.line(365);
            kw.set_item("parse_int", parse_int)?;
        }
        if !parse_constant.is_none() {
            frame.line(367);
            kw.set_item("parse_constant", parse_constant)?;
        }
        frame.line(368);
        rt::call!(cls, **kw)?.call_method1("decode", (&s,))
    })
}

/// Executes the body of `json/__init__.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        m.set_global("__doc__", "JSON (JavaScript Object Notation) <https://json.org> is a subset of\nJavaScript syntax (ECMA-262 3rd edition) used as a lightweight data\ninterchange format.\n\n:mod:`json` exposes an API familiar to users of the standard library\n:mod:`marshal` and :mod:`pickle` modules.  It is derived from a\nversion of the externally maintained simplejson library.\n\nEncoding basic Python object hierarchies::\n\n    >>> import json\n    >>> json.dumps(['foo', {'bar': ('baz', None, 1.0, 2)}])\n    '[\"foo\", {\"bar\": [\"baz\", null, 1.0, 2]}]'\n    >>> print(json.dumps(\"\\\"foo\\bar\"))\n    \"\\\"foo\\bar\"\n    >>> print(json.dumps('\\u1234'))\n    \"\\u1234\"\n    >>> print(json.dumps('\\\\'))\n    \"\\\\\"\n    >>> print(json.dumps({\"c\": 0, \"b\": 0, \"a\": 0}, sort_keys=True))\n    {\"a\": 0, \"b\": 0, \"c\": 0}\n    >>> from io import StringIO\n    >>> io = StringIO()\n    >>> json.dump(['streaming API'], io)\n    >>> io.getvalue()\n    '[\"streaming API\"]'\n\nCompact encoding::\n\n    >>> import json\n    >>> mydict = {'4': 5, '6': 7}\n    >>> json.dumps([1,2,3,mydict], separators=(',', ':'))\n    '[1,2,3,{\"4\":5,\"6\":7}]'\n\nPretty printing::\n\n    >>> import json\n    >>> print(json.dumps({'4': 5, '6': 7}, sort_keys=True, indent=4))\n    {\n        \"4\": 5,\n        \"6\": 7\n    }\n\nDecoding JSON::\n\n    >>> import json\n    >>> obj = ['foo', {'bar': ['baz', None, 1.0, 2]}]\n    >>> json.loads('[\"foo\", {\"bar\":[\"baz\", null, 1.0, 2]}]') == obj\n    True\n    >>> json.loads('\"\\\\\"foo\\\\bar\"') == '\"foo\\x08ar'\n    True\n    >>> from io import StringIO\n    >>> io = StringIO('[\"streaming API\"]')\n    >>> json.load(io)[0] == 'streaming API'\n    True\n\nSpecializing JSON object decoding::\n\n    >>> import json\n    >>> def as_complex(dct):\n    ...     if '__complex__' in dct:\n    ...         return complex(dct['real'], dct['imag'])\n    ...     return dct\n    ...\n    >>> json.loads('{\"__complex__\": true, \"real\": 1, \"imag\": 2}',\n    ...     object_hook=as_complex)\n    (1+2j)\n    >>> from decimal import Decimal\n    >>> json.loads('1.1', parse_float=Decimal) == Decimal('1.1')\n    True\n\nSpecializing JSON object encoding::\n\n    >>> import json\n    >>> def encode_complex(obj):\n    ...     if isinstance(obj, complex):\n    ...         return [obj.real, obj.imag]\n    ...     raise TypeError(f'Object of type {obj.__class__.__name__} '\n    ...                     f'is not JSON serializable')\n    ...\n    >>> json.dumps(2 + 1j, default=encode_complex)\n    '[2.0, 1.0]'\n    >>> json.JSONEncoder(default=encode_complex).encode(2 + 1j)\n    '[2.0, 1.0]'\n    >>> ''.join(json.JSONEncoder(default=encode_complex).iterencode(2 + 1j))\n    '[2.0, 1.0]'\n\n\nUsing json from the shell to validate and pretty-print::\n\n    $ echo '{\"json\":\"obj\"}' | python -m json\n    {\n        \"json\": \"obj\"\n    }\n    $ echo '{ 1.2:3.4}' | python -m json\n    Expecting property name enclosed in double quotes: line 1 column 3 (char 2)\n")?;
        // Copied unmodified (below this header) from Lib/json/__init__.py of CPython 3.14.9,
        // licensed under the PSF License Agreement. A steelix test project:
        // tests/test_projects.py runs it transpiled and with CPython and compares.
        frame.line(101);
        m.set_global("__version__", "2.0.9")?;
        frame.line(102);
        m.set_global(
            "__all__",
            PyList::new(
                py,
                [
                    "dump",
                    "dumps",
                    "load",
                    "loads",
                    "JSONDecoder",
                    "JSONDecodeError",
                    "JSONEncoder",
                ],
            )?,
        )?;
        frame.line(107);
        m.set_global("__author__", "Bob Ippolito <bob@redivi.com>")?;
        frame.line(109);
        let decoder = rt::import_names(m, ".decoder", &["JSONDecoder", "JSONDecodeError"])?;
        m.set_global("JSONDecoder", rt::import_from(&decoder, "JSONDecoder")?)?;
        m.set_global(
            "JSONDecodeError",
            rt::import_from(&decoder, "JSONDecodeError")?,
        )?;
        frame.line(110);
        let encoder = rt::import_names(m, ".encoder", &["JSONEncoder"])?;
        m.set_global("JSONEncoder", rt::import_from(&encoder, "JSONEncoder")?)?;
        frame.line(111);
        m.set_global("codecs", rt::import(m, "codecs")?)?;
        frame.line(113);
        m.set_global(
            "_default_encoder",
            rt::call!(
                m.global("JSONEncoder")?,
                skipkeys = false,
                ensure_ascii = true,
                check_circular = true,
                allow_nan = true,
                indent = py.None(),
                separators = py.None(),
                default = py.None()
            )?,
        )?;
        rt::def!(
            m,
            dump(obj, fp, *, skipkeys = false, ensure_ascii = true, check_circular = true, allow_nan = true, cls = py.None(), indent = py.None(), separators = py.None(), default = py.None(), sort_keys = false, **kw),
            doc = "Serialize ``obj`` as a JSON formatted stream to ``fp`` (a\n``.write()``-supporting file-like object).\n\nIf ``skipkeys`` is true then ``dict`` keys that are not basic types\n(``str``, ``int``, ``float``, ``bool``, ``None``) will be skipped\ninstead of raising a ``TypeError``.\n\nIf ``ensure_ascii`` is false, then the strings written to ``fp`` can\ncontain non-ASCII and non-printable characters if they appear in strings\ncontained in ``obj``. Otherwise, all such characters are escaped in JSON\nstrings.\n\nIf ``check_circular`` is false, then the circular reference check\nfor container types will be skipped and a circular reference will\nresult in an ``RecursionError`` (or worse).\n\nIf ``allow_nan`` is false, then it will be a ``ValueError`` to\nserialize out of range ``float`` values (``nan``, ``inf``, ``-inf``)\nin strict compliance of the JSON specification, instead of using the\nJavaScript equivalents (``NaN``, ``Infinity``, ``-Infinity``).\n\nIf ``indent`` is a non-negative integer, then JSON array elements and\nobject members will be pretty-printed with that indent level. An indent\nlevel of 0 will only insert newlines. ``None`` is the default and gives\na representation with no newlines inserted.\n\nIf specified, ``separators`` should be an ``(item_separator,\nkey_separator)`` tuple.  The default is ``(', ', ': ')`` if *indent* is\n``None`` and ``(',', ': ')`` otherwise.  To get the most compact JSON\nrepresentation, you should specify ``(',', ':')`` to eliminate\nwhitespace.\n\n``default(obj)`` is a function that should return a serializable version\nof obj or raise TypeError. The default simply raises TypeError.\n\nIf *sort_keys* is true (default: ``False``), then the output of\ndictionaries will be sorted by key.\n\nTo use a custom ``JSONEncoder`` subclass (e.g. one that overrides the\n``.default()`` method to serialize additional types), specify it with\nthe ``cls`` kwarg; otherwise ``JSONEncoder`` is used.\n\n"
        )?;
        rt::def!(
            m,
            dumps(obj, *, skipkeys = false, ensure_ascii = true, check_circular = true, allow_nan = true, cls = py.None(), indent = py.None(), separators = py.None(), default = py.None(), sort_keys = false, **kw),
            doc = "Serialize ``obj`` to a JSON formatted ``str``.\n\nIf ``skipkeys`` is true then ``dict`` keys that are not basic types\n(``str``, ``int``, ``float``, ``bool``, ``None``) will be skipped\ninstead of raising a ``TypeError``.\n\nIf ``ensure_ascii`` is false, then the return value can contain\nnon-ASCII and non-printable characters if they appear in strings\ncontained in ``obj``.  Otherwise, all such characters are escaped in\nJSON strings.\n\nIf ``check_circular`` is false, then the circular reference check\nfor container types will be skipped and a circular reference will\nresult in an ``RecursionError`` (or worse).\n\nIf ``allow_nan`` is false, then it will be a ``ValueError`` to\nserialize out of range ``float`` values (``nan``, ``inf``, ``-inf``) in\nstrict compliance of the JSON specification, instead of using the\nJavaScript equivalents (``NaN``, ``Infinity``, ``-Infinity``).\n\nIf ``indent`` is a non-negative integer, then JSON array elements and\nobject members will be pretty-printed with that indent level. An indent\nlevel of 0 will only insert newlines. ``None`` is the default and gives\na representation with no newlines inserted.\n\nIf specified, ``separators`` should be an ``(item_separator,\nkey_separator)`` tuple.  The default is ``(', ', ': ')`` if *indent* is\n``None`` and ``(',', ': ')`` otherwise.  To get the most compact JSON\nrepresentation, you should specify ``(',', ':')`` to eliminate\nwhitespace.\n\n``default(obj)`` is a function that should return a serializable version\nof obj or raise TypeError. The default simply raises TypeError.\n\nIf *sort_keys* is true (default: ``False``), then the output of\ndictionaries will be sorted by key.\n\nTo use a custom ``JSONEncoder`` subclass (e.g. one that overrides the\n``.default()`` method to serialize additional types), specify it with\nthe ``cls`` kwarg; otherwise ``JSONEncoder`` is used.\n\n"
        )?;
        frame.line(248);
        m.set_global(
            "_default_decoder",
            rt::call!(
                m.global("JSONDecoder")?,
                object_hook = py.None(),
                object_pairs_hook = py.None()
            )?,
        )?;
        rt::def!(m, detect_encoding(b))?;
        rt::def!(
            m,
            load(fp, *, cls = py.None(), object_hook = py.None(), parse_float = py.None(), parse_int = py.None(), parse_constant = py.None(), object_pairs_hook = py.None(), **kw),
            doc = "Deserialize ``fp`` (a ``.read()``-supporting file-like object containing\na JSON document) to a Python object.\n\n``object_hook`` is an optional function that will be called with the\nresult of any object literal decode (a ``dict``). The return value of\n``object_hook`` will be used instead of the ``dict``. This feature\ncan be used to implement custom decoders (e.g. JSON-RPC class hinting).\n\n``object_pairs_hook`` is an optional function that will be called with\nthe result of any object literal decoded with an ordered list of pairs.\nThe return value of ``object_pairs_hook`` will be used instead of the\n``dict``.  This feature can be used to implement custom decoders.  If\n``object_hook`` is also defined, the ``object_pairs_hook`` takes\npriority.\n\nTo use a custom ``JSONDecoder`` subclass, specify it with the ``cls``\nkwarg; otherwise ``JSONDecoder`` is used.\n"
        )?;
        rt::def!(
            m,
            loads(s, *, cls = py.None(), object_hook = py.None(), parse_float = py.None(), parse_int = py.None(), parse_constant = py.None(), object_pairs_hook = py.None(), **kw),
            doc = "Deserialize ``s`` (a ``str``, ``bytes`` or ``bytearray`` instance\ncontaining a JSON document) to a Python object.\n\n``object_hook`` is an optional function that will be called with the\nresult of any object literal decode (a ``dict``). The return value of\n``object_hook`` will be used instead of the ``dict``. This feature\ncan be used to implement custom decoders (e.g. JSON-RPC class hinting).\n\n``object_pairs_hook`` is an optional function that will be called with\nthe result of any object literal decoded with an ordered list of pairs.\nThe return value of ``object_pairs_hook`` will be used instead of the\n``dict``.  This feature can be used to implement custom decoders.  If\n``object_hook`` is also defined, the ``object_pairs_hook`` takes\npriority.\n\n``parse_float``, if specified, will be called with the string\nof every JSON float to be decoded. By default this is equivalent to\nfloat(num_str). This can be used to use another datatype or parser\nfor JSON floats (e.g. decimal.Decimal).\n\n``parse_int``, if specified, will be called with the string\nof every JSON int to be decoded. By default this is equivalent to\nint(num_str). This can be used to use another datatype or parser\nfor JSON integers (e.g. float).\n\n``parse_constant``, if specified, will be called with one of the\nfollowing strings: -Infinity, Infinity, NaN.\nThis can be used to raise an exception if invalid JSON numbers\nare encountered.\n\nTo use a custom ``JSONDecoder`` subclass, specify it with the ``cls``\nkwarg; otherwise ``JSONDecoder`` is used.\n"
        )?;
        Ok(())
    })
}
