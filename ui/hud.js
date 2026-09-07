import { keyRects, boardUnits } from './geometry.mjs';
import { translateSlot, shiftLabel } from './translator.mjs';
import { LAYER_ACTIONS } from './layer-actions.mjs';
import { Heatmap, heatmapFill } from './heatmap.mjs';

const { invoke } = window.__TAURI__.core;
const { listen, emit } = window.__TAURI__.event;

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
// Some firmware paths can omit a key-up while reconnecting or when a tap is
// very fast. Macro recognition keeps its own short debounce window instead of
// treating the visual pressed-state set as the source of truth.
const macroDownAt = new Map();
let heatmapStorage;
try { heatmapStorage = window.localStorage; } catch (error) { console.warn('Heatmap storage unavailable:', error); }
const heatmap = new Heatmap(heatmapStorage);
const heatmapCounts = heatmap.counts;
window.addEventListener('pagehide', () => heatmap.flush());

// Cached inputs to the last successful render, so a window resize can
// re-render at the new scale without re-invoking the backend.
let lastLayout = null;
let lastConfig = null;
let toggleRecording = false;
let layoutVersion = 0;

function publishHeatmapStats() {
  const total = heatmapCounts.reduce((sum, count) => sum + count, 0);
  const keys = heatmapCounts.reduce((sum, count) => sum + (count > 0 ? 1 : 0), 0);
  emit('heatmap-stats', { total, keys }).catch(() => {});
}

function applyHeatmapKey(index) {
  const peak = Math.max(1, Number(lastConfig?.heatmap_peak ?? 20));
  const count = heatmapCounts[index] || 0;
  const strength = Math.min(1, count / peak);
  document.querySelectorAll(`.key[data-key-index="${index}"]`).forEach((el) => {
    el.classList.toggle('heatmap-active', strength > 0);
    el.style.setProperty('--heatmap-fill', heatmapFill(count, lastConfig?.heatmap_color, peak));
    const label = el.querySelector('.heatmap-count');
    if (label) {
      label.textContent = String(heatmapCounts[index] || 0);
    }
  });
}

function refreshHeatmap() {
  for (let i = 0; i < heatmapCounts.length; i += 1) applyHeatmapKey(i);
}

function recordHeatmap(index) {
  if (!heatmap.record(index)) return;
  applyHeatmapKey(index);
  publishHeatmapStats();
}

function decorateAction(element, slotName, slot, secondary = false) {
  const isLayer = slot?.layer !== null && slot?.layer !== undefined;
  if (!isLayer && (!secondary || !element.textContent)) return;
  const action = LAYER_ACTIONS[slotName];
  const kind = isLayer ? 'layer' : 'alternate';
  element.classList.add(`${kind}-action`);
  element.title = `${action.label}: ${isLayer ? `layer ${slot.layer}` : element.textContent}`;
  let layerLabel;
  if (isLayer) {
    layerLabel = document.createElement('span');
    layerLabel.className = 'layer-target-label';
    layerLabel.textContent = element.textContent;
    element.textContent = '';
  }
  const icon = document.createElement('span');
  icon.className = `${kind}-action-icon ${action.icon}`;
  icon.setAttribute('aria-hidden', 'true');
  if (isLayer) {
    element.classList.add('layer-reference');
    const targetIcon = document.createElement('span');
    targetIcon.className = 'layer-target-icon';
    targetIcon.setAttribute('aria-hidden', 'true');
    const layerIcon = document.createElement('span');
    layerIcon.className = 'layer-target-wrap';
    layerIcon.appendChild(targetIcon);
    const layerNumber = document.createElement('span');
    layerNumber.className = 'layer-target-number';
    layerNumber.textContent = String(slot.layer);
    layerIcon.appendChild(layerNumber);
    element.prepend(layerIcon);
    element.prepend(icon);
    if (layerLabel) element.appendChild(layerLabel);
    return;
  }
  element.prepend(icon);
}

function computeLayout(config) {
  const pad = config.padding ?? 10;
  const units = boardUnits(config.keyboard_halves_distance ?? 1.6);
  const availW = window.innerWidth - 2 * pad;
  const availH = window.innerHeight - 2 * pad;
  const unit = Math.max(8, Math.min(availW / units.w, availH / units.h));
  // Center the key grid on the board background.
  const offX = (window.innerWidth - units.w * unit) / 2;
  const offY = (window.innerHeight - units.h * unit) / 2;
  return { unit, offX, offY, units };
}

export function renderBoard(layoutJson, config) {
  const layers = layoutJson?.data?.layout?.revision?.layers;
  if (!Array.isArray(layers) || !layers.length || layers.some(layer => !Array.isArray(layer.keys) || layer.keys.length !== 52)) {
    throw new Error('Layout must contain 52-key Voyager layers');
  }
  lastLayout = layoutJson;
  lastConfig = config;
  const board = document.getElementById('board');
  board.innerHTML = '';
  const { unit, offX, offY, units } = computeLayout(config);
  board.style.setProperty('--key-unit', `${unit}px`);
  const rects = keyRects(config.key_spacing ?? 0.06, config.keyboard_halves_distance ?? 1.6);
  const badge = document.createElement('div');
  badge.id = 'badge';
  badge.style.left = `${config.layer_pill_horizontal ?? 50}%`;
  badge.style.top = `${config.layer_pill_vertical ?? 8}%`;
  board.appendChild(badge);
  const offline = document.createElement('div');
  offline.id = 'offline-indicator';
  offline.textContent = 'OFFLINE';
  offline.style.left = `${config.offline_pill_horizontal ?? 50}%`;
  offline.style.top = `${config.offline_pill_vertical ?? 50}%`;
  offline.hidden = !document.body.classList.contains('offline');
  board.appendChild(offline);
  for (const layer of layers) {
    const el = document.createElement('div');
    el.className = 'layer';
    el.dataset.layer = layer.position;
    el.dataset.name = layer.title || `Layer ${layer.position}`;
    const leftHalf = document.createElement('div');
    leftHalf.className = 'keyboard-half left-half';
    const rightHalf = document.createElement('div');
    rightHalf.className = 'keyboard-half right-half';
    el.append(leftHalf, rightHalf);
    layer.keys.forEach((key, i) => {
      const r = rects[i];
      const k = document.createElement('div');
      k.className = 'key';
      k.dataset.keyIndex = i;
      k.dataset.triggersLayers = Object.keys(LAYER_ACTIONS)
        .map(slot => key[slot]?.layer).filter(layer => layer !== null && layer !== undefined).join(',');
      if (pressedKeys.has(i)) k.classList.add('pressed');
      if (i === 24 || i === 25) k.classList.add('thumb-left');
      if (i === 50 || i === 51) k.classList.add('thumb-right');
      k.style.cssText = `left:${offX + r.x * unit}px;top:${offY + r.y * unit}px;width:${r.w * unit}px;height:${r.h * unit}px`;
      if (heatmapCounts[i]) {
        k.classList.add('heatmap-active');
        k.style.setProperty('--heatmap-fill', heatmapFill(heatmapCounts[i], config.heatmap_color, config.heatmap_peak ?? 20));
      }
      if (config.show_heatmap_counts) {
        const count = document.createElement('span');
        count.className = 'heatmap-count';
        count.textContent = String(heatmapCounts[i] || 0);
        k.appendChild(count);
      }
      if (config.use_oryx_colors && key.glowColor) k.style.setProperty('--oryx-fill', hexTint(key.glowColor));
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
          continue;
        }
        if (key[slot]) {
          const s = document.createElement('span');
          s.className = cls;
          s.textContent = translateSlot(key[slot]);
          decorateAction(s, slot, key[slot], true);
          k.appendChild(s);
        }
      }
      (i < 26 ? leftHalf : rightHalf).appendChild(k);
    });
    board.appendChild(el);
  }
}

function hexToRgba(hex, alpha) {
  const safe = /^#[0-9a-f]{6}$/i.test(hex) ? hex : '#ffffff';
  const n = parseInt(safe.slice(1), 16);
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
  const layerIndicator = ['none', 'textual', 'icon'].includes(config.layer_indicator) ? config.layer_indicator : 'icon';
  document.body.classList.toggle('show-layer-action-icons', layerIndicator !== 'none' && (config.show_layer_action_icons ?? true));
  document.body.classList.toggle('show-layer-target-icons', layerIndicator === 'icon');
  document.body.classList.toggle('layer-indicator-none', layerIndicator === 'none');
  document.body.classList.toggle('show-shift-icons', config.show_shift_icons ?? true);
  document.body.classList.toggle('show-alternate-action-icons', config.show_alternate_action_icons ?? true);
  document.body.classList.toggle('show-heatmap', config.show_heatmap ?? false);
  document.body.classList.toggle('show-heatmap-counts', config.show_heatmap_counts ?? false);
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
  st.setProperty('--halves-rotation', `${config.keyboard_halves_rotation ?? 0}deg`);
  st.setProperty('--border-color', hexToRgba(config.border_color, config.border_opacity));
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
  refreshHeatmap();
}

function applyOverlayVisibility(payload) {
  const hidden = !!payload?.hidden;
  document.body.classList.toggle('overlay-hidden', hidden);
  const progress = Math.max(0, Math.min(1, Number(payload.progress ?? (hidden ? 1 : 0))));
  document.body.classList.toggle('overlay-fully-hidden', hidden && Number(payload.reveal) === 0 && progress >= 1);
  if (!hidden) {
    document.body.removeAttribute('data-hide-side');
    return;
  }
  const side = ['left', 'right', 'top', 'bottom'].includes(payload.side) ? payload.side : 'right';
  const baseReveal = Math.max(0, Math.min(1, Number(payload.reveal ?? 0.08)));
  // Native progress is 0 at the fully visible position and 1 at the hidden
  // edge. Convert that to the visible fraction used by the clip geometry.
  const reveal = baseReveal + (1 - baseReveal) * (1 - progress);
  const width = window.innerWidth;
  const height = window.innerHeight;
  const keyUnit = parseFloat(getComputedStyle(document.getElementById('board')).getPropertyValue('--key-unit')) || 32;
  const visibleX = reveal === 0 ? 0 : Math.min(width, Math.max(width * reveal, keyUnit * 0.94));
  const visibleY = reveal === 0 ? 0 : Math.min(height, Math.max(height * reveal, keyUnit * 0.94));
  const st = document.documentElement.style;
  st.setProperty('--hide-clip-left', side === 'right' ? `${width - visibleX}px` : '0px');
  st.setProperty('--hide-clip-right', side === 'left' ? `${width - visibleX}px` : '0px');
  st.setProperty('--hide-clip-top', side === 'bottom' ? `${height - visibleY}px` : '0px');
  st.setProperty('--hide-clip-bottom', side === 'top' ? `${height - visibleY}px` : '0px');
  st.setProperty('--hide-visible-x', `${visibleX}px`);
  st.setProperty('--hide-visible-y', `${visibleY}px`);
  st.setProperty('--hide-shift-x', side === 'left' ? `-${width - visibleX}px` : side === 'right' ? `${width - visibleX}px` : '0px');
  st.setProperty('--hide-shift-y', side === 'top' ? `-${height - visibleY}px` : side === 'bottom' ? `${height - visibleY}px` : '0px');
  document.body.dataset.hideSide = side;
}

function hexTint(hex) {
  return hexToRgba(hex, 0.25);
}

export function setActiveLayer(n) {
  document.querySelectorAll('.layer').forEach((el) => {
    el.classList.toggle('active', Number(el.dataset.layer) === n);
  });
  document.querySelectorAll('[data-triggers-layers]').forEach((el) => {
    el.classList.toggle('trigger-active', el.dataset.triggersLayers.split(',').filter(Boolean).map(Number).includes(n));
  });
  const active = document.querySelector('.layer.active');
  const badge = document.getElementById('badge');
  if (badge) badge.textContent = active ? active.dataset.name : `Layer ${n}`;
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

export function setOffline(off) {
  document.body.classList.toggle('offline', off);
  if (off) {
    for (const key of [...pressedKeys]) setKeyPressed(key, false);
    macroDownAt.clear();
    lastLayer = 0;
    setActiveLayer(0);
  }
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
    const { index, pressed } = e.payload;
    if (!Number.isInteger(index) || index < 0 || index >= 52) return;
    const now = performance.now();
    const lastMacroDown = macroDownAt.get(index) ?? -Infinity;
    const freshPress = pressed && now - lastMacroDown >= 160;
    if (pressed) macroDownAt.set(index, now);
    else macroDownAt.delete(index);
    setKeyPressed(index, pressed);
    if (freshPress) {
      recordHeatmap(index);
      // The Rust HID watcher owns macro matching so a missed WebView event
      // cannot make a physical sequence ineffective.
    }
  });
  await listen('macro-recording', e => { toggleRecording = !!e.payload; });
  await listen('heatmap-reset', () => {
    heatmap.reset();
    refreshHeatmap();
    publishHeatmapStats();
  });
  await listen('heatmap-request', publishHeatmapStats);
  await listen('keyboard-offline', () => setOffline(true));
  await listen('keyboard-online', () => setOffline(false));
  const setGrabCue = (on) => {
    document.body.classList.toggle('grab', on);
    document.querySelectorAll('.resize-handle').forEach((handle) => {
      handle.style.display = on ? 'block' : '';
    });
  };
  await listen('grab-mode', (e) => setGrabCue(!!e.payload.on));
  await listen('overlay-visibility', (e) => applyOverlayVisibility(e.payload));
  await listen('overlay-toggle-error', e => {
    console.warn('layer-hud: overlay toggle failed:', e.payload);
    const board = document.getElementById('board');
    if (board && !document.body.classList.contains('overlay-hidden')) {
      board.dataset.toggleError = String(e.payload || 'toggle failed');
      setTimeout(() => delete board.dataset.toggleError, 3500);
    }
  });
  try {
    setGrabCue(await invoke('is_overlay_pinned'));
  } catch {}
  await listen('config-changed', (e) => {
    const previous = lastConfig;
    lastConfig = e.payload;
    applyTheme(lastConfig);
    const distanceChanged = previous?.keyboard_halves_distance !== lastConfig.keyboard_halves_distance;
    const needsRender = !previous || ['use_oryx_colors', 'padding', 'key_spacing', 'keyboard_halves_distance', 'show_heatmap_counts',
      'layer_pill_horizontal', 'layer_pill_vertical', 'offline_pill_horizontal', 'offline_pill_vertical', 'layer_indicator']
      .some(field => previous[field] !== lastConfig[field]);
    if (needsRender && lastLayout) {
      renderBoard(lastLayout, lastConfig);
      setActiveLayer(lastLayer);
    }
    if (distanceChanged) invoke('recalculate_window_geometry')
      .catch(err => console.warn('Could not recalculate window geometry:', err));
  });
  await listen('layout-refreshed', async (e) => {
    const version = ++layoutVersion;
    const config = lastConfig ?? await invoke('get_config');
    if (version !== layoutVersion) return;
    applyTheme(config);
    renderBoard(e.payload, config);
    setActiveLayer(lastLayer);
  });
  await listen('layout-loading', e => {
    const previous = lastLayout?._layer_hud;
    if (!previous || previous.layout !== e.payload?.layout || previous.revision !== e.payload?.revision) {
      ++layoutVersion;
      lastLayout = null;
      showStartupError('loading the keyboard’s flashed layout…');
    }
  });
  await listen('layout-error', e => {
    console.warn('Layout refresh failed:', e.payload);
    if (!lastLayout) showStartupError(e.payload);
  });
  try {
    const status = await invoke('get_keyboard_status');
    lastLayer = status.layer;
    setOffline(!status.online);
    setActiveLayer(lastLayer);
  } catch (error) { console.warn('Could not read keyboard status:', error); }
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
      if (document.body.classList.contains('overlay-hidden')) {
        applyOverlayVisibility({ hidden: true, side: document.body.dataset.hideSide, reveal: lastConfig?.hide_reveal });
      }
    }, 100);
  });

  for (const dir of ['NorthWest', 'NorthEast', 'SouthWest', 'SouthEast']) {
    const h = document.createElement('div');
    h.className = `resize-handle ${dir.toLowerCase()}`;
    h.style.display = document.body.classList.contains('grab') ? 'block' : '';
    h.addEventListener('mousedown', (e) => {
      if (!document.body.classList.contains('grab')) return;
      e.stopPropagation();
      e.preventDefault();
      window.__TAURI__.window.getCurrentWindow().startResizeDragging(dir);
    });
    document.body.appendChild(h);
  }

  try {
    lastConfig = await invoke('get_config');
    applyTheme(lastConfig);
    const version = layoutVersion;
    const layout = await invoke('load_layout');
    if (version === layoutVersion) {
      renderBoard(layout, lastConfig);
      setActiveLayer(lastLayer);
    }
  } catch (err) {
    console.error('layer-hud: startup layout failed:', err);
    if (!lastLayout) showStartupError(err);
  }
}
main().catch((err) => {
  console.error('layer-hud startup failed:', err);
  showStartupError(err);
});
