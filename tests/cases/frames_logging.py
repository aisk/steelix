# Native functions have no Python frame, so logging and warnings attribute
# their records to the nearest Python frame instead (design §12): logging's
# own frame, the frame of kept code calling the native function, or none.
# `KNOWN_DIFFERENCES` in tests/test_e2e.py spells out the current output.
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
    try:
        logging.warning("kept step %d", n)
    finally:
        pass
    return n + 1


def show_warning(message, category, filename, lineno, file=None, line=None):
    print(f"{category.__name__} {filename.rsplit('/', 1)[-1]}:{lineno}: {message}")


warnings.showwarning = show_warning
warnings.simplefilter("always")


def warn_native(message):
    warnings.warn(message)


def warn_kept():
    try:
        warnings.warn("from kept code")
    finally:
        pass


def kept_calls_native():
    try:
        warn_native("from native code, called by kept code")
    finally:
        pass


step(1)
kept_step(2)
warn_native("from native code")
warn_kept()
kept_calls_native()
