# Module level annotations in several blocks kept as Python make one
# `__annotate__`, defined before the first statement as in CPython, which
# only reports the annotations whose assignments have run.
import annotationlib
import sys

print("__annotate__" in dir())
A: int = 1
print(__annotate__(1))


class Box:
    pass


B: list[
    str
] = []
if len(sys.argv) > 5:
    C: float
D: "Box" = Box()
print(annotationlib.get_annotations(sys.modules[__name__]))
print(annotationlib.get_annotations(sys.modules[__name__], format=annotationlib.Format.STRING))
E: undefined_name
print(sorted(__conditional_annotations__))
print(__annotate__(1))
