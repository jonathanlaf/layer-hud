import { keyRects, BOARD_UNITS } from './geometry.mjs';
import { translateSlot, shiftLabel } from './translator.mjs';
import { LAYER_ACTIONS } from './layer-actions.mjs';

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

// A ctrl-click while dragging in grab mode is macOS's system convention for
// a secondary click, which would otherwise pop the webview's native context
// menu mid-drag. DevTools access lives in the tray menu (debug builds only)
// instead of this menu.
document.addEventListener('contextmenu', (e) => e.preventDefault());

// Layer the HUD should currently show. Updated by the layer-changed listener
// and re-applied after every renderBoard() so config-only re-renders (e.g. an
// opacity tick) don't snap the HUD back to layer 0.
let lastLayer = 0;
const pressedKeys = new Set();

// Cached inputs to the last successful render, so a window resize can
// re-render at the new scale without re-invoking the backend.
let lastLayout = null;
let lastConfig = null;

function decorateAction(element, slotName, slot, secondary = false) {
  const isLayer = slot?.layer !== null && slot?.layer !== undefined;
  if (!isLayer && (!secondary || !element.textContent)) return;
  const action = LAYER_ACTIONS[slotName];
  const kind = isLayer ? 'layer' : 'alternate';
  element.classList.add(`${kind}-action`);
  element.title = `${action.label}: ${isLayer ? `layer ${slot.layer}` : element.textContent}`;
  const icon = document.createElement('span');
  icon.className = `${kind}-action-icon ${action.icon}`;
  icon.setAttribute('aria-hidden', 'true');
  element.prepend(icon);
}

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
  const rects = keyRects(config.key_spacing ?? 0.06);
  const badge = document.createElement('div');
  badge.id = 'badge';
  badge.style.top = `${Math.max(4, offY)}px`;
  board.appendChild(badge);
  const offline = document.createElement('div');
  offline.id = 'offline-indicator';
  offline.textContent = 'OFFLINE';
  offline.style.top = `${offY + (BOARD_UNITS.h * unit) / 2}px`;
  offline.hidden = !document.body.classList.contains('offline');
  board.appendChild(offline);
  for (const layer of layers) {
    const el = document.createElement('div');
    el.className = 'layer';
    el.dataset.layer = layer.position;
    el.dataset.name = layer.title || `Layer ${layer.position}`;
    layer.keys.forEach((key, i) => {
      const r = rects[i];
      const k = document.createElement('div');
      k.className = 'key';
      k.dataset.keyIndex = i;
      if (pressedKeys.has(i)) k.classList.add('pressed');
      if (i === 24 || i === 25) k.classList.add('thumb-left');
      if (i === 50 || i === 51) k.classList.add('thumb-right');
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
      decorateAction(tap, holdPromoted ? 'hold' : 'tap', holdPromoted ? key.hold : key.tap);
      k.appendChild(tap);
      const shifted = shiftLabel(key.tap ? { ...key.tap, customLabel: custom } : key.tap);
      if (shifted) {
        const s = document.createElement('span');
        s.className = 'shift';
        s.textContent = shifted;
        s.title = `Shift: ${shifted}`;
        const icon = document.createElement('span');
        icon.className = 'shift-icon';
        icon.setAttribute('aria-hidden', 'true');
        s.prepend(icon);
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
          decorateAction(s, slot, key[slot], true);
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

function fontVars(style, prefix, config) {
  const family = config[`${prefix}_font_family`];
  const cssPrefix = prefix.replace(/_/g, '-');
  style.setProperty(`--${cssPrefix}-font-family`, family ? JSON.stringify(family) : '-apple-system');
  style.setProperty(`--${cssPrefix}-font-weight`, config[`${prefix}_font_bold`] ? '700' : '400');
  style.setProperty(`--${cssPrefix}-font-style`, config[`${prefix}_font_italic`] ? 'italic' : 'normal');
  style.setProperty(`--${cssPrefix}-font-ligatures`, config.font_ligatures === false ? 'none' : 'common-ligatures');
}

function applyTheme(config) {
  document.body.classList.toggle('show-layer-action-icons', config.show_layer_action_icons ?? true);
  document.body.classList.toggle('show-shift-icons', config.show_shift_icons ?? true);
  document.body.classList.toggle('show-alternate-action-icons', config.show_alternate_action_icons ?? true);
  const st = document.documentElement.style;
  st.setProperty('--board-bg', hexToRgba(config.bg_color, config.opacity));
  st.setProperty('--char-opacity', config.char_opacity);
  st.setProperty('--text-color', config.text_color);
  st.setProperty('--legend-color', config.legend_color);
  st.setProperty('--layer-name-color', config.text_color);
  st.setProperty('--layer-name-border', hexToRgba(config.border_color, config.border_opacity));
  st.setProperty('--layer-name-opacity', config.char_opacity);
  st.setProperty('--shift-color', config.shift_color ?? '#ffffff');
  st.setProperty('--alternate-color', config.alternate_color ?? '#ffffff');
  st.setProperty('--shift-icon-scale', config.shift_icon_scale ?? 1);
  st.setProperty('--alternate-action-icon-scale', config.alternate_action_icon_scale ?? 1);
  st.setProperty('--border-color', hexToRgba(config.border_color, config.border_opacity));
  st.setProperty('--pressed-key-color', config.pressed_key_color ?? '#7ad7ff');
  st.setProperty('--pressed-key-fill', hexToRgba(config.pressed_key_color ?? '#7ad7ff', config.pressed_key_fill_opacity ?? 0.45));
  st.setProperty('--pressed-key-border', hexToRgba(config.pressed_key_border_color ?? config.pressed_key_color ?? '#7ad7ff', config.pressed_key_border_opacity ?? 0.85));
  st.setProperty('--pressed-key-border-width', `${config.pressed_key_border_width ?? 1}px`);
  st.setProperty('--key-border-radius', `${config.key_border_radius ?? 7}px`);
  st.setProperty('--pill-border-radius', `${config.pill_border_radius ?? 999}px`);
  st.setProperty('--key-shadow', config.show_key_shadows ? `0 2px 5px ${hexToRgba(config.key_shadow_color ?? '#ffffff', config.key_shadow_opacity ?? 0.25)}` : 'none');
  st.setProperty('--pressed-key-shadow', config.show_pressed_key_shadow ? `0 0 10px ${hexToRgba(config.pressed_key_shadow_color ?? config.pressed_key_border_color ?? config.pressed_key_color ?? '#7ad7ff', config.pressed_key_shadow_opacity ?? 0.85)}` : 'none');
  st.setProperty('--border-width', `${config.border_width}px`);
  st.setProperty('--key-fill', hexToRgba(config.key_fill_color, config.key_fill_opacity));
  st.setProperty('--base-outline', hexToRgba(config.base_outline_color, config.base_outline_opacity));
  st.setProperty('--base-outline-width', `${config.base_outline_enabled ? config.base_outline_width : 0}px`);
  st.setProperty('--grab-outline', hexToRgba(config.grab_outline_color, config.grab_outline_opacity));
  st.setProperty('--grab-outline-width', `${config.grab_outline_enabled ? config.grab_outline_width : 0}px`);
  st.setProperty('--key-font-scale', config.key_font_size);
  st.setProperty('--legend-font-scale', config.legend_font_size);
  st.setProperty('--layer-name-font-size', `${config.layer_name_font_size}px`);
  fontVars(st, 'key', config);
  fontVars(st, 'legend', config);
  fontVars(st, 'layer_name', config);
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

export function setKeyPressed(index, pressed) {
  const key = Number(index);
  if (!Number.isInteger(key) || key < 0) return;
  if (pressed) pressedKeys.add(key);
  else pressedKeys.delete(key);
  document.querySelectorAll(`.key[data-key-index="${key}"]`).forEach((el) => {
    el.classList.toggle('pressed', pressed);
  });
}

// Voyager's Oryx events contain QMK matrix coordinates.  This mirrors the
// LAYOUT_voyager matrix map, including the staggered thumb positions, and
// converts them to the flat Oryx layer-key order used by the renderer.
function voyagerIndex(col, row) {
  const matrix = [
    [null, 0, 1, 2, 3, 4, 5],
    [null, 6, 7, 8, 9, 10, 11],
    [null, 12, 13, 14, 15, 16, 17],
    [null, 18, 19, 20, 21, 22, null],
    [null, null, null, null, 23, null, null],
    [24, 25, null, null, null, null, null],
    [26, 27, 28, 29, 30, 31, null],
    [32, 33, 34, 35, 36, 37, null],
    [38, 39, 40, 41, 42, 43, null],
    [null, 45, 46, 47, 48, 49, null],
    [null, null, 44, null, null, null, null],
    [null, null, null, null, null, 50, 51],
  ];
  return matrix[row]?.[col] ?? null;
}

export function setOffline(off) {
  document.body.classList.toggle('offline', off);
  const indicator = document.getElementById('offline-indicator');
  if (indicator) indicator.hidden = !off;
}

function showStartupError(error) {
  const board = document.getElementById('board');
  board.innerHTML = '';
  const msg = document.createElement('div');
  msg.id = 'startup-error';
  msg.textContent = error ? `No layout — ${error}` : 'No keyboard layout detected';
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
  await listen('key-event', (e) => {
    const index = voyagerIndex(Number(e.payload.col), Number(e.payload.row));
    if (index !== null) setKeyPressed(index, e.payload.pressed);
  });
  await listen('keyboard-layout', async (e) => {
    // The keyboard identifies its Oryx layout over HID.  Refreshing here
    // keeps layout selection automatic; the backend falls back to its cache
    // when Oryx is unreachable.
    if (e.payload?.layout) {
      try {
        await invoke('refresh_layout', { url: e.payload.layout });
      } catch (err) {
        console.warn('layer-hud: layout refresh failed:', err);
      }
    }
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
      || e.payload.padding !== lastConfig.padding
      || e.payload.key_spacing !== lastConfig.key_spacing;
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
  try { setOffline(!(await invoke('is_keymapp_online'))); } catch {}
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
    console.error('layer-hud: startup layout failed:', err);
    showStartupError(err);
  }
}
main().catch((err) => {
  console.error('layer-hud startup failed:', err);
  showStartupError(err);
});
