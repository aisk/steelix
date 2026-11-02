"""Function attributes used by the standard library: signature, pickle, annotations."""

import annotationlib
import inspect
import pickle


def area(width: float, height: "Length" = 2.0, /, *args: int, unit: str = "m", **kw) -> Shape:
    """Compute an area."""
    return width * height


def outer():
    def nested(x):
        """Nested docstring."""
        return x

    return nested


nested = outer()
print(inspect.signature(nested), nested.__doc__, nested.__qualname__)
print(area.__doc__, area.__module__, area.__globals__ is globals())
print(area.__defaults__, area.__kwdefaults__)
print(pickle.loads(pickle.dumps(area)) is area)
# Annotations are evaluated lazily, so `Shape` may be defined after `area`.
print(annotationlib.get_annotations(area, format=annotationlib.Format.STRING))
print(annotationlib.get_annotations(area, format=annotationlib.Format.FORWARDREF)["return"].__forward_arg__)
Shape = "a shape"
Length = float
print(area.__annotations__)
print(inspect.signature(area))
print(area(3.0), callable(area), area.__name__)
area.__name__ = "surface"
print(area.__name__, repr(area).startswith("<function area at "))
area.extra = 1
print(area.__dict__)
print(outer.__annotations__)
