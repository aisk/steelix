def parts(z):
    return z.real, z.imag


def scale(z, factor=2j):
    return z * factor


print(1j, 2.5j, 0j, 1e999j, -3j, type(1j).__name__)
print(1 + 2j, (1 + 2j) * 1j, abs(3 + 4j), parts(1.5j))
print(scale(1), scale(1j), scale(1, 0.5j))
print(1j == complex(0, 1), 0j or "falsy", 1j and "truthy")
x = 4j
print(x.conjugate(), x**2, -x)
