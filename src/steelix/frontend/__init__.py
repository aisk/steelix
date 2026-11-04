"""Frontend: parse Python source and run scope / name analysis.

Planned modules:
    parse.py     read source files and build `ast` trees, collect comments via `tokenize`
    scope.py     symtable based analysis: local / global / nonlocal / free / cell names
    module_bindings.py  AST walk counting module-level bindings (enables direct Rust
                 calls), detecting `import *` / `globals()`
    locals.py    definite-assignment analysis of function locals
"""
