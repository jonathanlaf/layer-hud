import { keyRects, BOARD_UNITS } from './geometry.mjs';
import { translateSlot } from './translator.mjs';

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

export function renderBoard(layoutJson, config) {
  const layers = layoutJson.data.layout.revision.layers;
  const board = document.getElementById('board');
  board.innerHTML = '';
  const rects = keyRects();
  const badge = document.createElement('div');
  badge.id = 'badge';
  board.appendChild(badge);
  for (const layer of layers) {
    const el = document.createElement('div');
    el.className = 'layer';
    el.dataset.layer = layer.position;
    el.dataset.name = layer.title || `Layer ${layer.position}`;
    layer.keys.forEach((key, i) => {
      const r = rects[i];
      const k = document.createElement('div');
      k.className = 'key';
      k.style.cssText = `left:${r.x * 62}px;top:${r.y * 62}px;width:${r.w * 62}px;height:${r.h * 62}px`;
      if (config.use_oryx_colors && key.glowColor) k.style.background = hexTint(key.glowColor);
      const custom = key.customLabel;
      const tap = document.createElement('span');
      tap.className = 'tap';
      tap.textContent = translateSlot(key.tap ? { ...key.tap, customLabel: custom } : key.tap);
      k.appendChild(tap);
      for (const [slot, cls] of [['hold', 'hold'], ['doubleTap', 'dtap'], ['tapHold', 'thold']]) {
        if (key[slot]) {
          const s = document.createElement('span');
          s.className = cls;
          s.textContent = translateSlot(key[slot]);
          k.appendChild(s);
          if (key[slot].layer !== null && key[slot].layer !== undefined)
            k.dataset.triggersLayer = key[slot].layer;
        }
      }
      el.appendChild(k);
    });
    board.appendChild(el);
  }
  setActiveLayer(0);
}

function hexTint(hex) {
  const n = parseInt(hex.replace('#', ''), 16);
  const [r, g, b] = [(n >> 16) & 255, (n >> 8) & 255, n & 255];
  return `rgba(${r},${g},${b},0.25)`;
}

export function setActiveLayer(n) {
  document.querySelectorAll('.layer').forEach((el) => {
    el.classList.toggle('active', Number(el.dataset.layer) === n);
  });
  document.querySelectorAll(`[data-triggers-layer]`).forEach((el) => {
    el.classList.toggle('trigger-active', Number(el.dataset.triggersLayer) === n);
  });
  const active = document.querySelector('.layer.active');
  document.getElementById('badge').textContent = active ? active.dataset.name : `Layer ${n}`;
  document.body.dataset.base = n === 0 ? '1' : '0';
}

export function setOffline(off) {
  document.body.classList.toggle('offline', off);
}

async function main() {
  const config = await invoke('get_config');
  document.documentElement.style.setProperty('--hud-opacity', config.opacity);
  const layout = await invoke('load_layout');
  renderBoard(layout, config);
  if (layout.stale) document.getElementById('badge').textContent += ' (cached)';
  await listen('layer-changed', (e) => setActiveLayer(e.payload.layer));
  await listen('keymapp-offline', () => setOffline(true));
  await listen('keymapp-online', () => setOffline(false));
  await listen('grab-mode', (e) => document.body.classList.toggle('grab', e.payload.on));
  await listen('config-changed', async (e) => {
    document.documentElement.style.setProperty('--hud-opacity', e.payload.opacity);
    renderBoard(await invoke('load_layout'), e.payload);
  });
  await listen('layout-refreshed', async () => {
    renderBoard(await invoke('load_layout'), await invoke('get_config'));
  });
  document.getElementById('board').addEventListener('mousedown', (e) => {
    if (document.body.classList.contains('grab')) {
      window.__TAURI__.window.getCurrentWindow().startDragging();
      e.preventDefault();
    }
  });
}
main();
