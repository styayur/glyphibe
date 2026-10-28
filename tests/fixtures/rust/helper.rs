// SPDX-License-Identifier: MPL-2.0
pub fn twice(value: i32) -> i32 {
    value * 2
}
pub fn add(left: i32, right: i32) -> i32 { left + right }
pub fn identity(value: i32) -> i32 { value }
pub fn floor(value: i32) -> i32 { if value > 0 { value } else { 0 } }
