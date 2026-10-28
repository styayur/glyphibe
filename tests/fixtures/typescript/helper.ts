// SPDX-License-Identifier: MPL-2.0
export function twice(value: number): number {
  return value * 2;
}
export function add(left: number, right: number): number { return left + right; }
export function identity(value: number): number { return value; }
export function floor(value: number): number { return value > 0 ? value : 0; }
export default function fallback(value: number): number { return value; }
