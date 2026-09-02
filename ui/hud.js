import { keyRects, BOARD_UNITS } from './geometry.mjs';
import { translateSlot, shiftLabel } from './translator.mjs';

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

// Layer the HUD should currently show. Updated by the layer-changed listener
// and re-applied after every renderBoard() so config-only re-renders (e.g. an
// opacity tick) don't snap the HUD back to layer 0.
let lastLayer = 0;

// Cached inputs to the last successful render, so a window resize can
// re-render at the new scale without re-invoking the backend.
let lastLayout = null;
let lastConfig = null;

function computeLayout(config) {
  const pad = config.padding ?? 10;
  const availW = window.innerWidth - 2 * pad;
  const availH = window.innerHeight - 2 * pad;
  const unit = Math.max(8, Math.min(availW / BOARD_UNITS.w, availH / BOARD_UNITS.h));
  // Center the key grid on the board background.
  const offX = (window.innerWidth - BOARD_UNITS.w * unit) / 2;
  const offY = (window.innerHeight - BOARD_UNITS.h * unit) / 2;
  return { unit, offX, offY };
}

export function renderBoard(layoutJson, config) {
  lastLayout = layoutJson;
  lastConfig = config;
  const layers = layoutJson.data.layout.revision.layers;
  const board = document.getElementById('board');
  board.innerHTML = '';
  const { unit, offX, offY } = computeLayout(config);
  board.style.setProperty('--key-unit', `${unit}px`);
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
      k.style.cssText = `left:${offX + r.x * unit}px;top:${offY + r.y * unit}px;width:${r.w * unit}px;height:${r.h * unit}px`;
      if (config.use_oryx_colors && key.glowColor) k.style.background = hexTint(key.glowColor);
      const custom = key.customLabel;
      const tap = document.createElement('span');
      tap.className = 'tap';
      // A hold-only key (e.g. a bare Ctrl/⌘ home-row modifier) has no tap:
      // its hold action IS the key, so promote it to the main label.
      let tapText = key.tap ? translateSlot({ ...key.tap, customLabel: custom }) : (custom || '');
      let holdPromoted = false;
      if (!tapText && key.hold) {
        tapText = translateSlot(key.hold);
        holdPromoted = true;
      }
      tap.textContent = tapText;
      k.appendChild(tap);
      const shifted = shiftLabel(key.tap ? { ...key.tap, customLabel: custom } : key.tap);
      if (shifted) {
        const s = document.createElement('span');
        s.className = 'shift';
        s.textContent = shifted;
        k.appendChild(s);
      }
      for (const [slot, cls] of [['hold', 'hold'], ['doubleTap', 'dtap'], ['tapHold', 'thold']]) {
        if (slot === 'hold' && holdPromoted) {
          if (key.hold.layer !== null && key.hold.layer !== undefined)
            k.dataset.triggersLayer = key.hold.layer;
          continue;
        }
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
}

function hexToRgba(hex, alpha) {
  const n = parseInt(hex.replace('#', ''), 16);
  return `rgba(${(n >> 16) & 255},${(n >> 8) & 255},${n & 255},${alpha})`;
}

function applyTheme(config) {
  const st = document.documentElement.style;
  st.setProperty('--board-bg', hexToRgba(config.bg_color, config.opacity));
  st.setProperty('--key-opacity', config.key_opacity);
  st.setProperty('--text-color', config.text_color);
  st.setProperty('--legend-color', config.legend_color);
  st.setProperty('--border-color', hexToRgba(config.border_color, 0.35));
  st.setProperty('--key-fill', hexToRgba(config.key_fill_color, config.key_fill_opacity));
}

function hexTint(hex) {
  return hexToRgba(hex, 0.25);
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

function showStartupError() {
  const board = document.getElementById('board');
  board.innerHTML = '';
  const msg = document.createElement('div');
  msg.id = 'startup-error';
  msg.textContent = 'No layout — set Oryx URL in Settings';
  board.appendChild(msg);
}

async function main() {
  // Register every listener before doing any startup work that can fail, so
  // a rejected first load (e.g. offline on first launch, no cached layout
  // yet) still leaves a live overlay that can recover later.
  await listen('layer-changed', (e) => {
    lastLayer = e.payload.layer;
    setActiveLayer(lastLayer);
  });
  await listen('keymapp-offline', () => setOffline(true));
  await listen('keymapp-online', () => setOffline(false));
  await listen('grab-mode', (e) => document.body.classList.toggle('grab', e.payload.on));
  await listen('config-changed', async (e) => {
    applyTheme(e.payload);
    // Only a use_oryx_colors flip changes the DOM (glowColor tints are baked
    // in at render time); everything else is covered by the CSS vars above.
    const needsRender = !lastConfig
      || e.payload.use_oryx_colors !== lastConfig.use_oryx_colors
      || e.payload.padding !== lastConfig.padding;
    if (needsRender) {
      renderBoard(await invoke('load_layout'), e.payload);
      setActiveLayer(lastLayer);
    } else {
      lastConfig = e.payload;
    }
  });
  await listen('layout-refreshed', async () => {
    renderBoard(await invoke('load_layout'), await invoke('get_config'));
    setActiveLayer(lastLayer);
  });
  document.getElementById('board').addEventListener('mousedown', (e) => {
    if (document.body.classList.contains('grab')) {
      window.__TAURI__.window.getCurrentWindow().startDragging();
      e.preventDefault();
    }
  });

  let resizeTimer;
  window.addEventListener('resize', () => {
    clearTimeout(resizeTimer);
    resizeTimer = setTimeout(() => {
      if (lastLayout && lastConfig) {
        renderBoard(lastLayout, lastConfig);
        setActiveLayer(lastLayer);
      }
    }, 100);
  });

  for (const dir of ['NorthWest', 'NorthEast', 'SouthWest', 'SouthEast']) {
    const h = document.createElement('div');
    h.className = `resize-handle ${dir.toLowerCase()}`;
    h.addEventListener('mousedown', (e) => {
      if (!document.body.classList.contains('grab')) return;
      e.stopPropagation();
      e.preventDefault();
      window.__TAURI__.window.getCurrentWindow().startResizeDragging(dir);
    });
    document.body.appendChild(h);
  }

  try {
    const config = await invoke('get_config');
    applyTheme(config);
    const layout = await invoke('load_layout');
    renderBoard(layout, config);
    setActiveLayer(lastLayer);
    if (layout.stale) document.getElementById('badge').textContent += ' (cached)';
  } catch (err) {
    showStartupError();
  }
}
main().catch((err) => {
  console.error('voyager-hud startup failed:', err);
  showStartupError();
});
