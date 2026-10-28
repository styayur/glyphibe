from helpers import twice as double
import helpers as h

def calculate(value: int) -> int:
    if value > 0:
        return double(value)
    return 0

def indirect(callback):
    return callback(2)

class Counter:
    def value(self):
        return 2

    def doubled(self):
        return self.value() * 2

    def report(self):
        return self.value() + self.doubled()

def pipeline(value):
    base = calculate(value)
    return h.floor(h.identity(h.add(base, 1)))

def factorial(value):
    if value <= 1:
        return 1
    return value * factorial(value - 1)

def aggregate(values):
    result = 0
    for item in values:
        result = h.add(result, double(item))
    return result
