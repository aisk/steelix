pub mod __main__;
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
use steelix_runtime::Attributes;

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
#[rt::frame(m, "dump", 123)]
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
    let mut cls = cls.clone();
    // cached encoder
    let iterable = if !skipkeys.is_truthy()?
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
        m.global("_default_encoder")?
            .method1("iterencode", (obj,))?
    } else {
        if cls.is_none() {
            cls = m.global("JSONEncoder")?;
        }
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
        .method1("iterencode", (obj,))?
    };
    // could accelerate with writelines in some versions of Python, at
    // a debuggability cost
    for chunk in rt::iter(&iterable)? {
        let chunk = chunk?;
        fp.method1("write", (&chunk,))?;
    }
    Ok(rt::none(m.py()))
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
#[rt::frame(m, "dumps", 188)]
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
    let mut cls = cls.clone();
    // cached encoder
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
        return m.global("_default_encoder")?.method1("encode", (obj,));
    }
    if cls.is_none() {
        cls = m.global("JSONEncoder")?;
    }
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
    .method1("encode", (obj,))
}

// json/__init__.py:251
#[rt::frame(m, "detect_encoding", 251)]
pub fn detect_encoding<'py>(
    m: &rt::Module<'py>,
    b: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let bstartswith = b.attr("startswith")?;
    if bstartswith
        .call1((PyTuple::new(
            py,
            [
                &m.global("codecs")?.attr("BOM_UTF32_BE")?,
                &m.global("codecs")?.attr("BOM_UTF32_LE")?,
            ],
        )?,))?
        .is_truthy()?
    {
        return Ok(rt::str(py, "utf-32"));
    }
    if bstartswith
        .call1((PyTuple::new(
            py,
            [
                &m.global("codecs")?.attr("BOM_UTF16_BE")?,
                &m.global("codecs")?.attr("BOM_UTF16_LE")?,
            ],
        )?,))?
        .is_truthy()?
    {
        return Ok(rt::str(py, "utf-16"));
    }
    if bstartswith
        .call1((m.global("codecs")?.attr("BOM_UTF8")?,))?
        .is_truthy()?
    {
        return Ok(rt::str(py, "utf-8-sig"));
    }
    if b.len()? >= 4 {
        if !b.get_item(0)?.is_truthy()? {
            // 00 00 -- -- - utf-32-be
            // 00 XX -- -- - utf-16-be
            return Ok(if b.get_item(1)?.is_truthy()? {
                rt::str(py, "utf-16-be")
            } else {
                rt::str(py, "utf-32-be")
            });
        }
        if !b.get_item(1)?.is_truthy()? {
            // XX 00 00 00 - utf-32-le
            // XX 00 00 XX - utf-16-le
            // XX 00 XX -- - utf-16-le
            return Ok(
                if b.get_item(2)?.is_truthy()? || b.get_item(3)?.is_truthy()? {
                    rt::str(py, "utf-16-le")
                } else {
                    rt::str(py, "utf-32-le")
                },
            );
        }
    } else if b.len()? == 2 {
        if !b.get_item(0)?.is_truthy()? {
            // 00 XX - utf-16-be
            return Ok(rt::str(py, "utf-16-be"));
        }
        if !b.get_item(1)?.is_truthy()? {
            // XX 00 - utf-16-le
            return Ok(rt::str(py, "utf-16-le"));
        }
    }
    // default
    Ok(rt::str(py, "utf-8"))
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
#[rt::frame(m, "load", 281)]
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
    rt::call!(
        m.global("loads")?,
        fp.method0("read")?,
        cls = cls,
        object_hook = object_hook,
        parse_float = parse_float,
        parse_int = parse_int,
        parse_constant = parse_constant,
        object_pairs_hook = object_pairs_hook,
        **kw
    )
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
#[rt::frame(m, "loads", 307, frame)]
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
    let mut s = s.clone();
    let mut cls = cls.clone();
    if s.is_instance(&rt::builtin(py, "str")?)? {
        if s.method1("startswith", ("\u{feff}",))?.is_truthy()? {
            return Err(frame.raise(m.global("JSONDecodeError")?.call1((
                "Unexpected UTF-8 BOM (decode using utf-8-sig)",
                &s,
                0,
            ))?));
        }
    } else {
        if !s.is_instance(
            PyTuple::new(
                py,
                [&rt::builtin(py, "bytes")?, &rt::builtin(py, "bytearray")?],
            )?
            .as_any(),
        )? {
            return Err(frame.raise(PyTypeError::new_err((rt::fstring(
                py,
                (
                    "the JSON object must be str, bytes or bytearray, not ",
                    rt::format(&s.attr("__class__")?.attr("__name__")?, "")?,
                ),
            )?
            .unbind(),))));
        }
        s = s
            .attr("decode")?
            .call1((detect_encoding(m, &s)?, "surrogatepass"))?;
    }
    if cls.is_none()
        && object_hook.is_none()
        && parse_int.is_none()
        && parse_float.is_none()
        && parse_constant.is_none()
        && object_pairs_hook.is_none()
        && !kw.is_truthy()?
    {
        return m.global("_default_decoder")?.method1("decode", (&s,));
    }
    if cls.is_none() {
        cls = m.global("JSONDecoder")?;
    }
    if !object_hook.is_none() {
        kw.set_item("object_hook", object_hook)?;
    }
    if !object_pairs_hook.is_none() {
        kw.set_item("object_pairs_hook", object_pairs_hook)?;
    }
    if !parse_float.is_none() {
        kw.set_item("parse_float", parse_float)?;
    }
    if !parse_int.is_none() {
        kw.set_item("parse_int", parse_int)?;
    }
    if !parse_constant.is_none() {
        kw.set_item("parse_constant", parse_constant)?;
    }
    rt::call!(cls, **kw)?.method1("decode", (&s,))
}

/// Executes the body of `json/__init__.py` in the module `m`.
#[rt::frame(m, "<module>", 1)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    m.set_global(
        "__doc__",
        "JSON (JavaScript Object Notation) <https://json.org> is a subset of\n\
            JavaScript syntax (ECMA-262 3rd edition) used as a lightweight data\n\
            interchange format.\n\
            \n\
            :mod:`json` exposes an API familiar to users of the standard library\n\
            :mod:`marshal` and :mod:`pickle` modules.  It is derived from a\n\
            version of the externally maintained simplejson library.\n\
            \n\
            Encoding basic Python object hierarchies::\n\
            \n\
            \x20   >>> import json\n\
            \x20   >>> json.dumps(['foo', {'bar': ('baz', None, 1.0, 2)}])\n\
            \x20   '[\"foo\", {\"bar\": [\"baz\", null, 1.0, 2]}]'\n\
            \x20   >>> print(json.dumps(\"\\\"foo\\bar\"))\n\
            \x20   \"\\\"foo\\bar\"\n\
            \x20   >>> print(json.dumps('\\u1234'))\n\
            \x20   \"\\u1234\"\n\
            \x20   >>> print(json.dumps('\\\\'))\n\
            \x20   \"\\\\\"\n\
            \x20   >>> print(json.dumps({\"c\": 0, \"b\": 0, \"a\": 0}, sort_keys=True))\n\
            \x20   {\"a\": 0, \"b\": 0, \"c\": 0}\n\
            \x20   >>> from io import StringIO\n\
            \x20   >>> io = StringIO()\n\
            \x20   >>> json.dump(['streaming API'], io)\n\
            \x20   >>> io.getvalue()\n\
            \x20   '[\"streaming API\"]'\n\
            \n\
            Compact encoding::\n\
            \n\
            \x20   >>> import json\n\
            \x20   >>> mydict = {'4': 5, '6': 7}\n\
            \x20   >>> json.dumps([1,2,3,mydict], separators=(',', ':'))\n\
            \x20   '[1,2,3,{\"4\":5,\"6\":7}]'\n\
            \n\
            Pretty printing::\n\
            \n\
            \x20   >>> import json\n\
            \x20   >>> print(json.dumps({'4': 5, '6': 7}, sort_keys=True, indent=4))\n\
            \x20   {\n\
            \x20       \"4\": 5,\n\
            \x20       \"6\": 7\n\
            \x20   }\n\
            \n\
            Decoding JSON::\n\
            \n\
            \x20   >>> import json\n\
            \x20   >>> obj = ['foo', {'bar': ['baz', None, 1.0, 2]}]\n\
            \x20   >>> json.loads('[\"foo\", {\"bar\":[\"baz\", null, 1.0, 2]}]') == obj\n\
            \x20   True\n\
            \x20   >>> json.loads('\"\\\\\"foo\\\\bar\"') == '\"foo\\x08ar'\n\
            \x20   True\n\
            \x20   >>> from io import StringIO\n\
            \x20   >>> io = StringIO('[\"streaming API\"]')\n\
            \x20   >>> json.load(io)[0] == 'streaming API'\n\
            \x20   True\n\
            \n\
            Specializing JSON object decoding::\n\
            \n\
            \x20   >>> import json\n\
            \x20   >>> def as_complex(dct):\n\
            \x20   ...     if '__complex__' in dct:\n\
            \x20   ...         return complex(dct['real'], dct['imag'])\n\
            \x20   ...     return dct\n\
            \x20   ...\n\
            \x20   >>> json.loads('{\"__complex__\": true, \"real\": 1, \"imag\": 2}',\n\
            \x20   ...     object_hook=as_complex)\n\
            \x20   (1+2j)\n\
            \x20   >>> from decimal import Decimal\n\
            \x20   >>> json.loads('1.1', parse_float=Decimal) == Decimal('1.1')\n\
            \x20   True\n\
            \n\
            Specializing JSON object encoding::\n\
            \n\
            \x20   >>> import json\n\
            \x20   >>> def encode_complex(obj):\n\
            \x20   ...     if isinstance(obj, complex):\n\
            \x20   ...         return [obj.real, obj.imag]\n\
            \x20   ...     raise TypeError(f'Object of type {obj.__class__.__name__} '\n\
            \x20   ...                     f'is not JSON serializable')\n\
            \x20   ...\n\
            \x20   >>> json.dumps(2 + 1j, default=encode_complex)\n\
            \x20   '[2.0, 1.0]'\n\
            \x20   >>> json.JSONEncoder(default=encode_complex).encode(2 + 1j)\n\
            \x20   '[2.0, 1.0]'\n\
            \x20   >>> ''.join(json.JSONEncoder(default=encode_complex).iterencode(2 + 1j))\n\
            \x20   '[2.0, 1.0]'\n\
            \n\
            \n\
            Using json from the shell to validate and pretty-print::\n\
            \n\
            \x20   $ echo '{\"json\":\"obj\"}' | python -m json\n\
            \x20   {\n\
            \x20       \"json\": \"obj\"\n\
            \x20   }\n\
            \x20   $ echo '{ 1.2:3.4}' | python -m json\n\
            \x20   Expecting property name enclosed in double quotes: line 1 column 3 (char 2)\n",
    )?;
    // Copied unmodified (below this header) from Lib/json/__init__.py of CPython 3.14.9,
    // licensed under the PSF License Agreement. A steelix test project:
    // tests/test_projects.py runs it transpiled and with CPython and compares.
    m.set_global("__version__", "2.0.9")?;
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
    m.set_global("__author__", "Bob Ippolito <bob@redivi.com>")?;
    let decoder = rt::import_names(m, ".decoder", &["JSONDecoder", "JSONDecodeError"])?;
    m.set_global("JSONDecoder", rt::import_from(&decoder, "JSONDecoder")?)?;
    m.set_global(
        "JSONDecodeError",
        rt::import_from(&decoder, "JSONDecodeError")?,
    )?;
    let encoder = rt::import_names(m, ".encoder", &["JSONEncoder"])?;
    m.set_global("JSONEncoder", rt::import_from(&encoder, "JSONEncoder")?)?;
    m.set_global("codecs", rt::import(m, "codecs")?)?;
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
        doc = "Serialize ``obj`` as a JSON formatted stream to ``fp`` (a\n\
            ``.write()``-supporting file-like object).\n\
            \n\
            If ``skipkeys`` is true then ``dict`` keys that are not basic types\n\
            (``str``, ``int``, ``float``, ``bool``, ``None``) will be skipped\n\
            instead of raising a ``TypeError``.\n\
            \n\
            If ``ensure_ascii`` is false, then the strings written to ``fp`` can\n\
            contain non-ASCII and non-printable characters if they appear in strings\n\
            contained in ``obj``. Otherwise, all such characters are escaped in JSON\n\
            strings.\n\
            \n\
            If ``check_circular`` is false, then the circular reference check\n\
            for container types will be skipped and a circular reference will\n\
            result in an ``RecursionError`` (or worse).\n\
            \n\
            If ``allow_nan`` is false, then it will be a ``ValueError`` to\n\
            serialize out of range ``float`` values (``nan``, ``inf``, ``-inf``)\n\
            in strict compliance of the JSON specification, instead of using the\n\
            JavaScript equivalents (``NaN``, ``Infinity``, ``-Infinity``).\n\
            \n\
            If ``indent`` is a non-negative integer, then JSON array elements and\n\
            object members will be pretty-printed with that indent level. An indent\n\
            level of 0 will only insert newlines. ``None`` is the default and gives\n\
            a representation with no newlines inserted.\n\
            \n\
            If specified, ``separators`` should be an ``(item_separator,\n\
            key_separator)`` tuple.  The default is ``(', ', ': ')`` if *indent* is\n\
            ``None`` and ``(',', ': ')`` otherwise.  To get the most compact JSON\n\
            representation, you should specify ``(',', ':')`` to eliminate\n\
            whitespace.\n\
            \n\
            ``default(obj)`` is a function that should return a serializable version\n\
            of obj or raise TypeError. The default simply raises TypeError.\n\
            \n\
            If *sort_keys* is true (default: ``False``), then the output of\n\
            dictionaries will be sorted by key.\n\
            \n\
            To use a custom ``JSONEncoder`` subclass (e.g. one that overrides the\n\
            ``.default()`` method to serialize additional types), specify it with\n\
            the ``cls`` kwarg; otherwise ``JSONEncoder`` is used.\n\
            \n"
    )?;
    rt::def!(
        m,
        dumps(obj, *, skipkeys = false, ensure_ascii = true, check_circular = true, allow_nan = true, cls = py.None(), indent = py.None(), separators = py.None(), default = py.None(), sort_keys = false, **kw),
        doc = "Serialize ``obj`` to a JSON formatted ``str``.\n\
            \n\
            If ``skipkeys`` is true then ``dict`` keys that are not basic types\n\
            (``str``, ``int``, ``float``, ``bool``, ``None``) will be skipped\n\
            instead of raising a ``TypeError``.\n\
            \n\
            If ``ensure_ascii`` is false, then the return value can contain\n\
            non-ASCII and non-printable characters if they appear in strings\n\
            contained in ``obj``.  Otherwise, all such characters are escaped in\n\
            JSON strings.\n\
            \n\
            If ``check_circular`` is false, then the circular reference check\n\
            for container types will be skipped and a circular reference will\n\
            result in an ``RecursionError`` (or worse).\n\
            \n\
            If ``allow_nan`` is false, then it will be a ``ValueError`` to\n\
            serialize out of range ``float`` values (``nan``, ``inf``, ``-inf``) in\n\
            strict compliance of the JSON specification, instead of using the\n\
            JavaScript equivalents (``NaN``, ``Infinity``, ``-Infinity``).\n\
            \n\
            If ``indent`` is a non-negative integer, then JSON array elements and\n\
            object members will be pretty-printed with that indent level. An indent\n\
            level of 0 will only insert newlines. ``None`` is the default and gives\n\
            a representation with no newlines inserted.\n\
            \n\
            If specified, ``separators`` should be an ``(item_separator,\n\
            key_separator)`` tuple.  The default is ``(', ', ': ')`` if *indent* is\n\
            ``None`` and ``(',', ': ')`` otherwise.  To get the most compact JSON\n\
            representation, you should specify ``(',', ':')`` to eliminate\n\
            whitespace.\n\
            \n\
            ``default(obj)`` is a function that should return a serializable version\n\
            of obj or raise TypeError. The default simply raises TypeError.\n\
            \n\
            If *sort_keys* is true (default: ``False``), then the output of\n\
            dictionaries will be sorted by key.\n\
            \n\
            To use a custom ``JSONEncoder`` subclass (e.g. one that overrides the\n\
            ``.default()`` method to serialize additional types), specify it with\n\
            the ``cls`` kwarg; otherwise ``JSONEncoder`` is used.\n\
            \n"
    )?;
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
        doc = "Deserialize ``fp`` (a ``.read()``-supporting file-like object containing\n\
            a JSON document) to a Python object.\n\
            \n\
            ``object_hook`` is an optional function that will be called with the\n\
            result of any object literal decode (a ``dict``). The return value of\n\
            ``object_hook`` will be used instead of the ``dict``. This feature\n\
            can be used to implement custom decoders (e.g. JSON-RPC class hinting).\n\
            \n\
            ``object_pairs_hook`` is an optional function that will be called with\n\
            the result of any object literal decoded with an ordered list of pairs.\n\
            The return value of ``object_pairs_hook`` will be used instead of the\n\
            ``dict``.  This feature can be used to implement custom decoders.  If\n\
            ``object_hook`` is also defined, the ``object_pairs_hook`` takes\n\
            priority.\n\
            \n\
            To use a custom ``JSONDecoder`` subclass, specify it with the ``cls``\n\
            kwarg; otherwise ``JSONDecoder`` is used.\n"
    )?;
    rt::def!(
        m,
        loads(s, *, cls = py.None(), object_hook = py.None(), parse_float = py.None(), parse_int = py.None(), parse_constant = py.None(), object_pairs_hook = py.None(), **kw),
        doc = "Deserialize ``s`` (a ``str``, ``bytes`` or ``bytearray`` instance\n\
            containing a JSON document) to a Python object.\n\
            \n\
            ``object_hook`` is an optional function that will be called with the\n\
            result of any object literal decode (a ``dict``). The return value of\n\
            ``object_hook`` will be used instead of the ``dict``. This feature\n\
            can be used to implement custom decoders (e.g. JSON-RPC class hinting).\n\
            \n\
            ``object_pairs_hook`` is an optional function that will be called with\n\
            the result of any object literal decoded with an ordered list of pairs.\n\
            The return value of ``object_pairs_hook`` will be used instead of the\n\
            ``dict``.  This feature can be used to implement custom decoders.  If\n\
            ``object_hook`` is also defined, the ``object_pairs_hook`` takes\n\
            priority.\n\
            \n\
            ``parse_float``, if specified, will be called with the string\n\
            of every JSON float to be decoded. By default this is equivalent to\n\
            float(num_str). This can be used to use another datatype or parser\n\
            for JSON floats (e.g. decimal.Decimal).\n\
            \n\
            ``parse_int``, if specified, will be called with the string\n\
            of every JSON int to be decoded. By default this is equivalent to\n\
            int(num_str). This can be used to use another datatype or parser\n\
            for JSON integers (e.g. float).\n\
            \n\
            ``parse_constant``, if specified, will be called with one of the\n\
            following strings: -Infinity, Infinity, NaN.\n\
            This can be used to raise an exception if invalid JSON numbers\n\
            are encountered.\n\
            \n\
            To use a custom ``JSONDecoder`` subclass, specify it with the ``cls``\n\
            kwarg; otherwise ``JSONDecoder`` is used.\n"
    )?;
    Ok(())
}
