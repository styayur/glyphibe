// SPDX-License-Identifier: MPL-2.0
export function twice(value) {
  return value * 2;
}
export function add(left, right) { return left + right; }
export function identity(value) { return value; }
export function floor(value) { return value > 0 ? value : 0; }
export default function fallback(value) { return value; }
