"""Bank ledger simulation that processes transactions at module level with globals."""

import functools

accounts = {}
history = []
fee_total = 0
TRANSFER_FEE = 25  # cents


def logged(fn):
    @functools.wraps(fn)
    def wrapper(*args, **kwargs):
        ok = fn(*args, **kwargs)
        history.append((fn.__name__, args, ok))
        return ok

    return wrapper


@logged
def open_account(name, initial=0):
    if name in accounts:
        return False
    accounts[name] = initial
    return True


@logged
def deposit(name, amount):
    if amount <= 0 or name not in accounts:
        return False
    accounts[name] += amount
    return True


@logged
def withdraw(name, amount):
    if accounts.get(name, 0) < amount:
        return False
    accounts[name] -= amount
    return True


@logged
def transfer(src, dst, amount, *, fee=TRANSFER_FEE):
    global fee_total
    if dst not in accounts or accounts.get(src, 0) < amount + fee:
        return False
    accounts[src] -= amount + fee
    accounts[dst] += amount
    fee_total += fee
    return True


@logged
def close_account(name):
    if name not in accounts or accounts[name] != 0:
        return False
    del accounts[name]
    return True


def cents(n):
    sign = "-" if n < 0 else ""
    n = abs(n)
    return f"{sign}${n // 100:,}.{n % 100:02d}"


ops = [
    (open_account, ("alice", 100000), {}),
    (open_account, ("bob",), {}),
    (open_account, ("carol", 5000), {}),
    (open_account, ("alice",), {}),
    (deposit, ("bob", 2550), {}),
    (withdraw, ("carol", 6000), {}),
    (transfer, ("alice", "bob", 12345), {}),
    (transfer, ("bob", "carol", 1000), {"fee": 0}),
    (transfer, ("carol", "dave", 10), {}),
    (withdraw, ("carol", 6000), {}),
    (close_account, ("carol",), {}),
    (deposit, ("bob", -5), {}),
]

for op, args, kwargs in ops:
    op(*args, **kwargs)


def main():
    for i, (name, args, ok) in enumerate(history, 1):
        status = "ok" if ok else "FAILED"
        shown = ", ".join(map(repr, args))
        print(f"{i:>2}. {name}({shown}) -> {status}")
    print("balances:")
    for name, bal in sorted(accounts.items()):
        print(f"  {name:<6} {cents(bal):>12}")
    failures = 0
    for _, _, ok in history:
        if not ok:
            failures += 1
    print(f"fees collected: {cents(fee_total)}")
    print(f"{failures} of {len(history)} operations failed")
    print(f"total holdings: {cents(sum(accounts.values()) + fee_total)}")
    print(deposit.__name__, cents(-1234567))


main()
