import { test } from 'node:test';
import assert from 'node:assert/strict';
import { layerTriggerList, keyLocation, triggerInstruction } from '../layer-actions.mjs';

test('retains every route on a multi-action key and includes routes back to layer zero', () => {
  const result = layerTriggerList([
    { position: 2, title: 'Symbols', keys: [{ tap: { layer: 0 } }] },
    { position: 0, title: 'Base', keys: [{
      tap: { layer: 2 }, hold: { layer: 2 },
      doubleTap: { layer: 2 }, tapHold: { layer: 2 },
    }] },
  ]);
  assert.deepEqual(result.map((layer) => layer.position), [0, 2]);
  assert.deepEqual(result[1].triggers.map((trigger) => trigger.slot), ['tap', 'hold', 'doubleTap', 'tapHold']);
  assert.equal(result[0].triggers[0].sourcePosition, 2);
});

test('missing actions are not mistaken for layer zero and unused layers remain listed', () => {
  const result = layerTriggerList([
    { position: 0, keys: [{ tap: { layer: null }, hold: {}, doubleTap: { layer: '' }, tapHold: { layer: 99 } }] },
    { position: 1, keys: [] },
  ]);
  assert.equal(result.length, 2);
  assert.ok(result.every((layer) => layer.triggers.length === 0));
});

test('key positions match the split keyboard geometry boundaries', () => {
  assert.equal(keyLocation(23), 'Left · row 4, column 6');
  assert.equal(keyLocation(24), 'Left · thumb 1');
  assert.equal(keyLocation(26), 'Right · row 1, column 1');
  assert.equal(keyLocation(51), 'Right · thumb 2');
});

test('tap-and-hold instructions name the ENTER key, not its target layer', () => {
  const layers = layerTriggerList([
    { position: 0, keys: [{ tap: { code: 'KC_ENTER' }, tapHold: { layer: 2 } }] },
    { position: 2, keys: [] },
  ]);
  assert.equal(triggerInstruction(layers[1].triggers[0]), 'tap and hold ENTER');
});

test('instructions preserve custom names and qualify non-base source layers', () => {
  const layers = layerTriggerList([
    { position: 1, keys: [{ customLabel: 'NAV', hold: { layer: 2 } }] },
    { position: 2, keys: [] },
  ]);
  assert.equal(triggerInstruction(layers[1].triggers[0]), 'hold NAV (from Layer 1)');
});
