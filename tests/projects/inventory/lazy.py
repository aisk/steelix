print("lazy: imported")

import sys

import app

VALUE = 42
app_is_main = (app is sys.modules["__main__"], app.__name__, app.Discounted is sys.modules["__main__"].Discounted)
