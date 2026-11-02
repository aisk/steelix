# Names that are not globals are looked up in the namespace of `builtins`,
# like `LOAD_GLOBAL` does, not among the attributes of the module object:
# `__annotations__` is an attribute of every module but not a builtin.
import builtins

builtins.answer = 42


def show():
    print(answer)
    print(__annotations__)


show()
