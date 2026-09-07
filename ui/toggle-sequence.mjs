// Matches ordered physical presses; this does not infer firmware tap/hold actions.
export class ToggleSequence {
  constructor() { this.configure([]); }
  configure(keys) {
    this.keys = Array.isArray(keys) ? keys.filter(k => Number.isInteger(k) && k >= 0 && k < 52).slice(0, 64) : [];
    this.reset();
  }
  reset() { this.buffer = []; this.lastAt = -Infinity; }
  press(key, now) {
    if (!this.keys.length) return false;
    if (now - this.lastAt > 1000) this.reset();
    this.lastAt = now;
    this.buffer.push(key);
    if (this.buffer.length > this.keys.length) this.buffer.shift();
    const matches = this.buffer.length === this.keys.length && this.buffer.every((k, i) => k === this.keys[i]);
    if (matches) this.reset();
    return matches;
  }
}
