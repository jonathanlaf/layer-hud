import { test } from 'node:test';
import assert from 'node:assert/strict';
import { ToggleSequence } from '../toggle-sequence.mjs';

test('triple tap matches once per complete sequence', () => {
  const sequence = new ToggleSequence();
  sequence.configure([3, 3, 3]);
  assert.deepEqual([0, 100, 200, 300, 400, 500].map(t => sequence.press(3, t)), [false, false, true, false, false, true]);
});

test('timeouts, unrelated presses, and explicit reset discard partial matches', () => {
  const sequence = new ToggleSequence();
  sequence.configure([1, 2]);
  assert.equal(sequence.press(1, 0), false);
  assert.equal(sequence.press(2, 1001), false);
  assert.equal(sequence.press(1, 1100), false);
  assert.equal(sequence.press(3, 1200), false);
  assert.equal(sequence.press(2, 1300), false);
  sequence.press(1, 1400);
  sequence.reset();
  assert.equal(sequence.press(2, 1500), false);
  assert.equal(sequence.press(1, 1600), false);
  assert.equal(sequence.press(2, 1700), true);
});

test('empty configuration disables matching and reconfiguration clears progress', () => {
  const sequence = new ToggleSequence();
  assert.equal(sequence.press(0, 0), false);
  sequence.configure([0, 1]);
  sequence.press(0, 0);
  sequence.configure([0, 1]);
  assert.equal(sequence.press(1, 100), false);
  sequence.configure([-1, 52, '3', ...Array(100).fill(2)]);
  assert.equal(sequence.keys.length, 64);
  assert.ok(sequence.keys.every(k => k === 2));
});
