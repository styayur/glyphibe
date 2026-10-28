// SPDX-License-Identifier: MPL-2.0
mod helper;
use crate::helper::twice as double;

pub fn calculate(value: i32) -> i32 {
    if value > 0 {
        double(value)
    } else {
        0
    }
}

pub fn indirect(callback: fn(i32) -> i32) -> i32 {
    callback(2)
}

pub fn pipeline(value: i32) -> i32 {
    let base = calculate(value);
    crate::helper::floor(helper::identity(helper::add(base, 1)))
}
pub fn factorial(value: i32) -> i32 {
    if value <= 1 { 1 } else { value * factorial(value - 1) }
}
pub fn aggregate(values: &[i32]) -> i32 {
    let mut result = 0;
    for item in values { result = helper::add(result, double(*item)); }
    result
}
pub struct Counter;
impl Counter {
    pub fn value(&self) -> i32 { 2 }
    pub fn doubled(&self) -> i32 { self.value() * 2 }
    pub fn report(&self) -> i32 { self.value() + self.doubled() }
}
