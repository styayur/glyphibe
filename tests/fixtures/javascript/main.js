// SPDX-License-Identifier: MPL-2.0
import { twice as double } from './helper.js';
import * as h from './helper.js';
import fallback from './helper.js';
export const calculate = (value) => {
  if (value > 0) return double(value);
  return 0;
};
export function indirect(callback) { return callback(2); }
export function pipeline(value) {
  const base = calculate(value);
  return h.floor(h.identity(h.add(base, 1)));
}
export function factorial(value) { return value <= 1 ? 1 : value * factorial(value - 1); }
export function aggregate(values) {
  let result = 0;
  for (const item of values) result = h.add(result, double(item));
  return result;
}
export class Counter {
  value() { return 2; }
  doubled() { return this.value() * 2; }
  report() { return this.value() + this.doubled(); }
}
export function recover(value) { try { return fallback(value); } catch (error) { return 0; } }
