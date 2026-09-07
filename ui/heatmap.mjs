export const HEATMAP_STORAGE_KEY = 'layer-hud-heatmap-v1';
const MAX_COUNT = 0xffffffff;

export function restoreCounts(saved, size = 52) {
  const counts = new Uint32Array(size);
  if (Array.isArray(saved)) saved.slice(0, size).forEach((count, index) => {
    if (Number.isSafeInteger(count) && count > 0) counts[index] = Math.min(MAX_COUNT, count);
  });
  return counts;
}

export function heatmapFill(count, color, peak) {
  const strength = Math.min(1, Math.max(0, count) / Math.max(1, peak));
  const safe = /^#[0-9a-f]{6}$/i.test(color) ? color : '#ff5c5c';
  const n = parseInt(safe.slice(1), 16);
  return `rgba(${(n >> 16) & 255},${(n >> 8) & 255},${n & 255},${count > 0 ? 0.08 + strength * 0.72 : 0})`;
}

export class Heatmap {
  constructor(storage, schedule = setTimeout, cancel = clearTimeout, onError = console.warn) {
    this.storage = storage;
    this.schedule = schedule;
    this.cancel = cancel;
    this.onError = onError;
    this.timer = null;
    let saved;
    try { saved = JSON.parse(storage?.getItem(HEATMAP_STORAGE_KEY) || '[]'); }
    catch (error) { onError('Could not load heatmap history', error); }
    this.counts = restoreCounts(saved);
  }

  record(index) {
    if (!Number.isInteger(index) || index < 0 || index >= this.counts.length) return false;
    this.counts[index] = Math.min(MAX_COUNT, this.counts[index] + 1);
    // Throttle, rather than debounce: continuous typing must still reach disk.
    if (this.timer === null) this.timer = this.schedule(() => this.flush(), 250);
    return true;
  }

  flush() {
    if (this.timer !== null) this.cancel(this.timer);
    this.timer = null;
    try { this.storage?.setItem(HEATMAP_STORAGE_KEY, JSON.stringify([...this.counts])); }
    catch (error) { this.onError('Could not save heatmap history', error); }
  }

  reset() {
    this.counts.fill(0);
    this.flush();
  }
}
