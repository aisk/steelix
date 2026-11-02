# Native functions get a traceback frame only when an exception passes, so
# logging and warnings attribute their records to the nearest Python frame
# (design §12): logging's own frame, the frame of kept code calling the
# native function, or none. `KNOWN_DIFFERENCES` in tests/test_e2e.py has it.
import logging
import sys
import warnings

logging.basicConfig(
    stream=sys.stdout,
    format="%(levelname)s %(filename)s:%(lineno)d %(funcName)s: %(message)s",
)


def step(n):
    logging.warning("native step %d", n)
    return n + 1


def kept_step(n):
    match n:  # kept as Python
        case _:
            logging.warning("kept step %d", n)
    return n + 1


def show_warning(message, category, filename, lineno, file=None, line=None):
    print(f"{category.__name__} {filename.rsplit('/', 1)[-1]}:{lineno}: {message}")


warnings.showwarning = show_warning
warnings.simplefilter("always")


def warn_native(message):
    warnings.warn(message)


def warn_kept():
    match 0:  # kept as Python
        case _:
            warnings.warn("from kept code")


def kept_calls_native():
    match 0:  # kept as Python
        case _:
            warn_native("from native code, called by kept code")


step(1)
kept_step(2)
warn_native("from native code")
warn_kept()
kept_calls_native()
