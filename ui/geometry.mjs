const KEY = 1.0;          // key cell (includes gap)
const HALF_WIDTH = 6.0;
const DEFAULT_HALF_DISTANCE = 1.6;
// vertical offset per column, outer pinky -> inner column
const LEFT_STAGGER = [0.45, 0.35, 0.12, 0.0, 0.12, 0.28];
const RIGHT_STAGGER = [0.28, 0.12, 0.0, 0.12, 0.35, 0.45];

export function keyRects(spacing = 0.06, halfDistance = DEFAULT_HALF_DISTANCE) {
  const splitX = HALF_WIDTH + halfDistance;
  const width = Math.max(0.5, 1 - spacing);
  const rects = [];
  for (let row = 0; row < 4; row++)
    for (let col = 0; col < 6; col++)
      rects.push({ x: col * KEY, y: row * KEY + LEFT_STAGGER[col], w: width, h: width });
  rects.push({ x: 4.35, y: 4.35, w: width, h: width });  // 24: left thumb inner
  rects.push({ x: 5.41, y: 4.65, w: width, h: 1.1 });   // 25: left thumb outer
  for (let row = 0; row < 4; row++)
    for (let col = 0; col < 6; col++)
      rects.push({ x: splitX + col * KEY, y: row * KEY + RIGHT_STAGGER[col], w: width, h: width });
  rects.push({ x: splitX - 0.25, y: 4.65, w: width, h: 1.1 }); // 50: right thumb outer
  rects.push({ x: splitX + 0.85, y: 4.35, w: width, h: width }); // 51: right thumb inner
  return rects;
}

export function boardUnits(halfDistance = DEFAULT_HALF_DISTANCE) {
  return { w: HALF_WIDTH * 2 + halfDistance, h: 6.0 };
}

export const BOARD_UNITS = boardUnits();
