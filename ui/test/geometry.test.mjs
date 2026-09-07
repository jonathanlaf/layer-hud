import { test } from 'node:test';
import assert from 'node:assert/strict';
import { keyRects, boardUnits } from '../geometry.mjs';

test('52 keys', () => {
  assert.equal(keyRects().length, 52);
});

test('halves distance moves only the right half and updates the board bounds', () => {
  const near = keyRects(0.06, 1.6);
  const far = keyRects(0.06, 20);
  assert.deepEqual(near.slice(0, 26), far.slice(0, 26));
  for (let i = 26; i < 52; i++) assert.ok(Math.abs(far[i].x - near[i].x - 18.4) < 1e-10);
  assert.deepEqual(boardUnits(20), { w: 32, h: 6 });
});

test('key spacing changes key size without moving the key origins', () => {
  const tight = keyRects(0);
  const loose = keyRects(0.25);
  for (let i = 0; i < 52; i++) {
    assert.equal(tight[i].x, loose[i].x);
    assert.equal(tight[i].y, loose[i].y);
    assert.equal(tight[i].w - loose[i].w, 0.25);
  }
});

test('halves are separated by a split gap', () => {
  const r = keyRects();
  const leftMax = Math.max(...r.slice(0, 26).map((k) => k.x + k.w));
  const rightMin = Math.min(...r.slice(26, 52).map((k) => k.x));
  assert.ok(rightMin - leftMax >= 1.0, `gap ${rightMin - leftMax}`);
});

test('rows are top to bottom within a half', () => {
  const r = keyRects();
  assert.ok(r[0].y < r[6].y && r[6].y < r[12].y && r[12].y < r[18].y);
  assert.ok(r[24].y > r[18].y, 'thumbs below bottom row');
});

test('no overlapping keys', () => {
  const r = keyRects();
  for (let i = 0; i < r.length; i++)
    for (let j = i + 1; j < r.length; j++) {
      const a = r[i], b = r[j];
      const overlap = a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h;
      assert.ok(!overlap, `keys ${i} and ${j} overlap`);
    }
});
