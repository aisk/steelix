# Copied unmodified (below this header) from Lib/json/__main__.py of CPython 3.14.9,
# licensed under the PSF License Agreement. A steelix test project:
# tests/test_projects.py runs it transpiled and with CPython and compares.
"""Command-line tool to validate and pretty-print JSON

Usage::

    $ echo '{"json":"obj"}' | python -m json
    {
        "json": "obj"
    }
    $ echo '{ 1.2:3.4}' | python -m json
    Expecting property name enclosed in double quotes: line 1 column 3 (char 2)

"""
import json.tool


if __name__ == '__main__':
    try:
        json.tool.main()
    except BrokenPipeError as exc:
        raise SystemExit(exc.errno)
