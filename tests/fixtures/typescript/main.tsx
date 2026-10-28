// SPDX-License-Identifier: MPL-2.0
import { twice as double } from './helper';
import * as h from './helper';
import fallback from './helper';
export function calculate(value: number): number {
  if (value > 0) return double(value);
  return 0;
}
export function View() { return <output>{calculate(2)}</output>; }
export function indirect(callback: (value: number) => number) { return callback(2); }
export function pipeline(value: number): number {
  const base = calculate(value);
  return h.floor(h.identity(h.add(base, 1)));
}
export function factorial(value: number): number { return value <= 1 ? 1 : value * factorial(value - 1); }
export function aggregate(values: number[]): number {
  let result = 0;
  for (const item of values) result = h.add(result, double(item));
  return result;
}
export class Counter {
  value(): number { return 2; }
  doubled(): number { return this.value() * 2; }
  report(): number { return this.value() + this.doubled(); }
}
export function recover(value: number): number { try { return fallback(value); } catch (error) { return 0; } }
