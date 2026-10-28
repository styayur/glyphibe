"""Public MPL-2.0 fixture authored for Glyphibe. No dependencies."""
def twice(value: int) -> int:
    return value * 2

def isolated():
    return None

def add(left, right):
    return left + right

def identity(value):
    return value

def floor(value):
    return value if value > 0 else 0
