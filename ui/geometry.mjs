const KEY = 1.0;          // key cell (includes gap)
const SPLIT_X = 7.6;      // right half x origin
// vertical offset per column, outer pinky -> inner column
const LEFT_STAGGER = [0.45, 0.35, 0.12, 0.0, 0.12, 0.28];
const RIGHT_STAGGER = [0.28, 0.12, 0.0, 0.12, 0.35, 0.45];

export function keyRects() {
  const rects = [];
  for (let row = 0; row < 4; row++)
    for (let col = 0; col < 6; col++)
      rects.push({ x: col * KEY, y: row * KEY + LEFT_STAGGER[col], w: 0.94, h: 0.94 });
  rects.push({ x: 4.35, y: 4.35, w: 0.94, h: 0.94 });  // 24: left thumb inner
  rects.push({ x: 5.41, y: 4.65, w: 0.94, h: 1.1 });   // 25: left thumb outer
  for (let row = 0; row < 4; row++)
    for (let col = 0; col < 6; col++)
      rects.push({ x: SPLIT_X + col * KEY, y: row * KEY + RIGHT_STAGGER[col], w: 0.94, h: 0.94 });
  rects.push({ x: SPLIT_X - 0.25, y: 4.65, w: 0.94, h: 1.1 }); // 50: right thumb outer
  rects.push({ x: SPLIT_X + 0.85, y: 4.35, w: 0.94, h: 0.94 }); // 51: right thumb inner
  return rects;
}

export const BOARD_UNITS = { w: SPLIT_X + 6, h: 6.0 };
