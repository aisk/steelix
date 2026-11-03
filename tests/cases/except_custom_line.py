class AppError(Exception):
    pass


def handle(n):
    try:
        if n:
            raise AppError("bad")
    except AppError as e:
        print("caught", e)
    return {}["after"]


handle(1)
