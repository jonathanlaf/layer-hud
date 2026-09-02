import { test } from 'node:test';
import assert from 'node:assert/strict';
import { translateSlot } from '../translator.mjs';

test('custom label wins', () => {
  assert.equal(translateSlot({ code: 'CSA_EGRV', customLabel: 'BS' }), 'BS');
});

test('verified CSA modifier combos', () => {
  const alt = { rightAlt: true };
  assert.equal(translateSlot({ code: 'CSA_EGRV', modifiers: alt }), '\\');
  assert.equal(translateSlot({ code: 'KC_MINUS', modifiers: alt }), '|');
  assert.equal(translateSlot({ code: 'CSA_ECUT', modifiers: alt }), '/');
  assert.equal(translateSlot({ code: 'CSA_AGRV', modifiers: alt }), '`');
});

test('base CSA and plain codes', () => {
  assert.equal(translateSlot({ code: 'KC_A' }), 'A');
  assert.equal(translateSlot({ code: 'KC_COLN' }), ':');
  assert.equal(translateSlot({ code: 'CSA_QEST' }), '?');
  assert.equal(translateSlot({ code: 'CSA_ECUT' }), 'é');
  assert.equal(translateSlot({ code: 'CSA_CCED' }), 'ç');
  assert.equal(translateSlot({ code: 'KC_NO' }), '');
});

test('dead keys are marked', () => {
  assert.equal(translateSlot({ code: 'CSA_DCRC' }), '^̲'); // combining low line marks dead
  assert.equal(translateSlot({ code: 'CSA_DGRV' }), '`̲');
});

test('layer references', () => {
  assert.equal(translateSlot({ code: 'MO', layer: 2 }), 'L2');
  assert.equal(translateSlot({ code: 'TG', layer: 1 }), 'L1');
});

test('macros show a name', () => {
  assert.equal(
    translateSlot({ code: 'KC_TRANSPARENT', macro: { keys: [{ code: 'KC_B', modifiers: { leftCtrl: true } }, { code: 'KC_1' }] } }),
    '⌃B 1'
  );
});

test('unknown code falls back to cleaned name', () => {
  assert.equal(translateSlot({ code: 'KC_MEDIA_PLAY_PAUSE' }), '⏯');
  assert.equal(translateSlot({ code: 'KC_SOMETHING_NEW' }), 'SOMETHING NEW');
});
