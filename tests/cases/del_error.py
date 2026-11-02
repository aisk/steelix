def drop(flag):
    if flag:
        value = 1
    del value
    return "dropped"


print(drop(True))
print(drop(False))
