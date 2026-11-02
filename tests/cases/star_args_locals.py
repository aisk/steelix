# `*` and `**` arguments that are local variables used again afterwards.


def collect(*args, **kwargs):
    return args, kwargs


def forward(items, options):
    extra = list(items)
    named = dict(options)
    first = collect(*extra, **named)
    second = collect(0, *extra, flag=True, **named)
    return first, second, extra, named


print(forward((1, 2), {"key": 3}))
