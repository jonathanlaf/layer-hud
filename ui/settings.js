const { invoke } = window.__TAURI__.core;
const { listen, emit } = window.__TAURI__.event;

let cfg = await invoke('get_config');
const $ = (id) => document.getElementById(id);

async function resetAllSettings() {
  const status = $('settings-data-status');
  if (status) status.textContent = 'Resetting…';
  try {
    cfg = await invoke('reset_config');
    if (status) status.textContent = 'Settings reset to defaults.';
    setTimeout(() => window.location.reload(), 250);
  } catch (err) {
    if (status) status.textContent = `Reset failed: ${err}`;
  }
}
window.resetAllSettings = resetAllSettings;

const tabSections = new Map([
  ['layout', 'General'], ['appearance', 'Appearance'], ['fonts', 'Fonts'],
  ['interaction', 'Interaction'], ['position', 'Position'],
]);
const headings = [...document.querySelectorAll('h2')];
for (const [tab, title] of tabSections) {
  const heading = headings.find((node) => node.textContent.trim() === title);
  if (!heading) continue;
  heading.dataset.tabSection = tab;
  heading.nextElementSibling?.setAttribute('data-tab-section', tab);
}
function selectTab(tab) {
  for (const button of document.querySelectorAll('.tab-button')) {
    const active = button.dataset.tab === tab;
    button.classList.toggle('active', active);
    button.setAttribute('aria-selected', active);
  }
  for (const node of document.querySelectorAll('[data-tab-section]')) {
    node.hidden = node.dataset.tabSection !== tab;
  }
}
document.querySelectorAll('.tab-button').forEach((button) => {
  button.addEventListener('click', () => selectTab(button.dataset.tab));
});
selectTab('layout');

for (const prefix of ['key', 'legend', 'layer_name']) {
  const family = $(`${prefix.replace('_', '-')}-font-family`);
  const size = `${prefix}-font-size`;
  if (family) family.value = cfg[`${prefix}_font_family`] || '';
  if ($(size)) { $(size).value = cfg[`${prefix}_font_size`]; $(`${size}-val`).value = cfg[`${prefix}_font_size`]; }
}
$('font-ligatures').checked = cfg.font_ligatures !== false;
for (const button of document.querySelectorAll('[data-font-style]')) {
  const prefix = button.dataset.fontStyle;
  button.classList.toggle('active', button.dataset.style === 'bold' ? !!cfg[`${prefix}_font_bold`] : !!cfg[`${prefix}_font_italic`]);
  button.addEventListener('click', () => { const field = `${prefix}_font_${button.dataset.style}`; cfg[field] = !cfg[field]; button.classList.toggle('active', cfg[field]); push(); });
}
$('font-ligatures').addEventListener('change', (e) => commit('font_ligatures', e.target.checked));

const MOD_LABELS = { cmd: '⌘', alt: '⌥', ctrl: '⌃', shift: '⇧' };
const MOD_ORDER = ['cmd', 'alt', 'ctrl', 'shift'];
const comboText = (arr) => arr.map((m) => MOD_LABELS[m]).join('') || '—';

$('bg-color').value = cfg.bg_color;
$('key-fill-color').value = cfg.key_fill_color;
$('text-color').value = cfg.text_color;
$('legend-color').value = cfg.legend_color;
$('shift-color').value = cfg.shift_color;
$('alternate-color').value = cfg.alternate_color;
$('border-color').value = cfg.border_color;
$('pressed-key-color').value = cfg.pressed_key_color;
$('pressed-key-border-color').value = cfg.pressed_key_border_color;
$('key-shadow-color').value = cfg.key_shadow_color;
$('pressed-key-shadow-color').value = cfg.pressed_key_shadow_color;
$('heatmap-color').value = cfg.heatmap_color;
$('base-outline-color').value = cfg.base_outline_color;
$('grab-outline-color').value = cfg.grab_outline_color;
$('colors-toggle').checked = cfg.use_oryx_colors;
$('layer-action-icons').checked = cfg.show_layer_action_icons;
$('shift-icons').checked = cfg.show_shift_icons;
$('alternate-action-icons').checked = cfg.show_alternate_action_icons;
$('heatmap-toggle').checked = cfg.show_heatmap;
$('heatmap-counts-toggle').checked = cfg.show_heatmap_counts;
$('key-shadows').checked = cfg.show_key_shadows;
$('pressed-key-shadow').checked = cfg.show_pressed_key_shadow;
$('base-outline-enabled').checked = cfg.base_outline_enabled;
$('grab-outline-enabled').checked = cfg.grab_outline_enabled;
$('combo-display').textContent = comboText(cfg.grab_combo);
$('hide-side').value = cfg.hide_side;

const macroLabel = (macro) => macro?.length ? macro.map((index) => `Key ${index}`).join(' → ') : 'Not configured';
$('toggle-macro-display').textContent = macroLabel(cfg.toggle_macro);

// Serialized so rapid-fire commits (e.g. fast typing, each one a separate
// read-modify-write) can never resolve out of order and let a stale value
// overwrite a newer one.
let pushChain = Promise.resolve();
function push() {
  // Swallow a prior link's rejection before chaining the next one — .then()
  // on an already-rejected promise never runs its callback, so without this
  // a single failed invoke() would silently stop every future commit from
  // persisting for the rest of the session.
  // set_config preserves whatever window rect is already on disk itself
  // (the overlay's drag/resize handler owns that field), so this doesn't
  // need to re-fetch it first — that used to be a client-side workaround
  // for a race the backend now closes with its own lock.
  const attempt = pushChain.catch(() => {}).then(() => invoke('set_config', { config: cfg }));
  pushChain = attempt;
  return attempt;
}

// Shared mutate-then-persist step used by every binding below, so there's
// one place that owns "a setting changed" instead of each binding re-deriving it.
async function commit(field, value) {
  cfg[field] = value;
  await push();
}

const bind = (id, field) => {
  $(id).addEventListener('input', (e) => commit(field, e.target.value));
};
bind('bg-color', 'bg_color');
bind('key-fill-color', 'key_fill_color');
bind('text-color', 'text_color');
bind('legend-color', 'legend_color');
bind('shift-color', 'shift_color');
bind('alternate-color', 'alternate_color');
bind('border-color', 'border_color');
bind('pressed-key-color', 'pressed_key_color');
bind('pressed-key-border-color', 'pressed_key_border_color');
bind('key-shadow-color', 'key_shadow_color');
bind('pressed-key-shadow-color', 'pressed_key_shadow_color');
bind('heatmap-color', 'heatmap_color');
bind('base-outline-color', 'base_outline_color');
bind('grab-outline-color', 'grab_outline_color');

// Numeric settings: slider + manual text entry, kept in sync both ways.
// Every keystroke commits immediately (like the slider) so a value typed
// then the window closed before blur isn't lost; the box's own text is only
// touched when a keystroke actually needed sanitizing, so the caret isn't
// forced to the end on ordinary typing.
const clampNum = (v, min, max) => Math.min(max, Math.max(min, v));
const bindNumeric = (id, field) => {
  const slider = $(id);
  const box = $(id + '-val');
  const min = Number(slider.min);
  const max = Number(slider.max);
  const step = Number(slider.step) || 1;
  const stepDecimals = (String(step).split('.')[1] || '').length;
  const roundToStep = (v) => Number((Math.round(v / step) * step).toFixed(stepDecimals));
  slider.value = cfg[field];
  box.value = cfg[field];
  // The one place that owns "this field's value changed": mutate, sync the
  // slider, persist. Both the slider and the box route through this instead
  // of each re-deriving the same three steps.
  const applyValue = (v) => {
    cfg[field] = v;
    slider.value = v;
    return push();
  };
  slider.addEventListener('input', (e) => {
    applyValue(Number(e.target.value));
    box.value = slider.value;
  });
  box.addEventListener('input', () => {
    const before = box.value;
    const digitsAndDot = before.replace(/[^0-9.]/g, '');
    const firstDot = digitsAndDot.indexOf('.');
    const sanitized = firstDot === -1
      ? digitsAndDot
      : digitsAndDot.slice(0, firstDot + 1) + digitsAndDot.slice(firstDot + 1).replace(/\./g, '');
    if (sanitized !== before) {
      const caret = box.selectionStart - (before.length - sanitized.length);
      box.value = sanitized;
      box.setSelectionRange(caret, caret);
    }
    const v = parseFloat(sanitized);
    if (!Number.isFinite(v)) return;
    const clamped = clampNum(roundToStep(v), min, max);
    // Out-of-range or off-step: show the corrected value immediately (so the
    // box never displays a number other than the one actually applied),
    // preserving the caret the same way the sanitize step above does.
    if (clamped !== v) {
      const corrected = String(clamped);
      const caret = box.selectionStart - (box.value.length - corrected.length);
      box.value = corrected;
      box.setSelectionRange(caret, caret);
    }
    applyValue(clamped);
  });
  box.addEventListener('change', () => {
    // Always renormalize on blur, even when the typed text parses to the
    // already-committed number (e.g. "3.", "007") — otherwise a malformed
    // but numerically-equal string can stay displayed indefinitely.
    const v = parseFloat(box.value);
    box.value = Number.isFinite(v) ? clampNum(roundToStep(v), min, max) : cfg[field];
  });
};
for (const [id, field] of [
  ['opacity', 'opacity'],
  ['char-opacity', 'char_opacity'],
  ['border-opacity', 'border_opacity'],
  ['key-fill-opacity', 'key_fill_opacity'],
  ['border-width', 'border_width'],
  ['padding', 'padding'],
  ['shift-icon-scale', 'shift_icon_scale'],
  ['alternate-action-icon-scale', 'alternate_action_icon_scale'],
  ['heatmap-peak', 'heatmap_peak'],
  ['key-font-size', 'key_font_size'],
  ['legend-font-size', 'legend_font_size'],
  ['layer-name-font-size', 'layer_name_font_size'],
  ['pressed-key-fill-opacity', 'pressed_key_fill_opacity'],
  ['pressed-key-border-opacity', 'pressed_key_border_opacity'],
  ['pressed-key-border-width', 'pressed_key_border_width'],
  ['base-outline-opacity', 'base_outline_opacity'],
  ['base-outline-width', 'base_outline_width'],
  ['grab-outline-opacity', 'grab_outline_opacity'],
  ['grab-outline-width', 'grab_outline_width'],
  ['key-border-radius', 'key_border_radius'],
  ['pill-border-radius', 'pill_border_radius'],
  ['key-shadow-opacity', 'key_shadow_opacity'],
  ['pressed-key-shadow-opacity', 'pressed_key_shadow_opacity'],
  ['key-spacing', 'key_spacing'],
  ['keyboard-halves-distance', 'keyboard_halves_distance'],
  ['hide-reveal', 'hide_reveal'],
  ['hide-animation-ms', 'hide_animation_ms'],
]) bindNumeric(id, field);

for (const prefix of ['key', 'legend', 'layer_name']) {
  const id = `${prefix.replace('_', '-')}-font-family`;
  $(id).addEventListener('change', (e) => commit(`${prefix}_font_family`, e.target.value));
}

$('colors-toggle').addEventListener('change', async (e) => {
  cfg.use_oryx_colors = e.target.checked;
  await push();
});

$('layer-action-icons').addEventListener('change', (e) => {
  commit('show_layer_action_icons', e.target.checked);
});

$('shift-icons').addEventListener('change', (e) => {
  commit('show_shift_icons', e.target.checked);
});

$('alternate-action-icons').addEventListener('change', (e) => {
  commit('show_alternate_action_icons', e.target.checked);
});
const syncHeatmapControls = (enabled) => {
  $('heatmap-peak').disabled = !enabled;
  $('heatmap-peak-val').disabled = !enabled;
};
syncHeatmapControls(cfg.show_heatmap);
$('heatmap-toggle').addEventListener('change', (e) => {
  syncHeatmapControls(e.target.checked);
  commit('show_heatmap', e.target.checked);
});
$('heatmap-counts-toggle').addEventListener('change', (e) => commit('show_heatmap_counts', e.target.checked));
$('heatmap-reset').addEventListener('click', async () => {
  try {
    await emit('heatmap-reset');
    $('heatmap-reset').textContent = 'Reset';
  } catch (err) {
    $('heatmap-reset').textContent = 'Failed';
    setTimeout(() => { $('heatmap-reset').textContent = 'Reset'; }, 1200);
  }
});
await listen('heatmap-stats', (event) => {
  const total = Number(event.payload?.total ?? 0);
  const keys = Number(event.payload?.keys ?? 0);
  $('heatmap-count').textContent = `${total.toLocaleString()} presses · ${keys} keys`;
});
try { await emit('heatmap-request'); } catch {}
$('key-shadows').addEventListener('change', (e) => commit('show_key_shadows', e.target.checked));
$('pressed-key-shadow').addEventListener('change', (e) => commit('show_pressed_key_shadow', e.target.checked));
$('base-outline-enabled').addEventListener('change', (e) => commit('base_outline_enabled', e.target.checked));
$('grab-outline-enabled').addEventListener('change', (e) => commit('grab_outline_enabled', e.target.checked));
$('hide-side').addEventListener('change', (e) => commit('hide_side', e.target.value));

let toggleMacroRecording = false;
let recordedToggleMacro = [];
$('toggle-macro-record').addEventListener('click', () => {
  toggleMacroRecording = true;
  recordedToggleMacro = [];
  $('toggle-macro-display').textContent = 'Recording…';
  $('toggle-macro-record').disabled = true;
  $('toggle-macro-stop').disabled = false;
});
$('toggle-macro-stop').addEventListener('click', async () => {
  toggleMacroRecording = false;
  cfg.toggle_macro = recordedToggleMacro;
  await push();
  $('toggle-macro-display').textContent = macroLabel(cfg.toggle_macro);
  $('toggle-macro-record').disabled = false;
  $('toggle-macro-stop').disabled = true;
});
await listen('key-event', (event) => {
  if (!toggleMacroRecording || !event.payload?.pressed || event.payload.index == null) return;
  recordedToggleMacro.push(Number(event.payload.index));
  $('toggle-macro-display').textContent = macroLabel(recordedToggleMacro);
});

try {
  $('autostart').checked = await invoke('plugin:autostart|is_enabled');
} catch { $('autostart').disabled = true; }
$('autostart').addEventListener('change', async (e) => {
  try {
    await invoke(e.target.checked ? 'plugin:autostart|enable' : 'plugin:autostart|disable');
  } catch {
    e.target.checked = !e.target.checked;
  }
});

let recording = false;
let maxMods = new Set();

const heldMods = (e) => {
  const s = new Set();
  if (e.metaKey) s.add('cmd');
  if (e.altKey) s.add('alt');
  if (e.ctrlKey) s.add('ctrl');
  if (e.shiftKey) s.add('shift');
  return s;
};

$('record').addEventListener('click', () => {
  recording = true;
  maxMods = new Set();
  $('record-hint').textContent = 'Hold modifiers, release to save · Esc cancels';
  $('combo-display').textContent = '…';
});

for (const type of ['keydown', 'keyup']) {
  window.addEventListener(type, async (e) => {
    if (!recording) return;
    e.preventDefault();
    if (e.key === 'Escape') {
      recording = false;
      $('record-hint').textContent = 'Cancelled';
      $('combo-display').textContent = comboText(cfg.grab_combo);
      setTimeout(() => { $('record-hint').textContent = 'Hold to move and resize the overlay'; }, 1500);
      return;
    }
    const held = heldMods(e);
    held.forEach((m) => maxMods.add(m));
    $('combo-display').textContent = comboText(MOD_ORDER.filter((m) => maxMods.has(m)));
    if (type === 'keyup' && held.size === 0 && maxMods.size > 0) {
      recording = false;
      cfg.grab_combo = MOD_ORDER.filter((m) => maxMods.has(m));
      $('record-hint').textContent = 'Saved';
      $('combo-display').textContent = comboText(cfg.grab_combo);
      await push();
      setTimeout(() => { $('record-hint').textContent = 'Hold to move and resize the overlay'; }, 1500);
    }
  });
}

async function alignWindow(axis) {
  try {
    await invoke('align_window', { axis });
  } catch (err) {
    console.warn('layer-hud: could not align window:', err);
  }
}
$('center-horizontal').addEventListener('click', () => alignWindow('horizontal'));
$('center-vertical').addEventListener('click', () => alignWindow('vertical'));
$('center-both').addEventListener('click', () => alignWindow('both'));
$('reset-position').addEventListener('click', async () => {
  const status = $('settings-data-status');
  if (status) status.textContent = 'Resetting overlay positions…';
  try {
    await invoke('reset_window_positions');
    if (status) status.textContent = 'Overlay positions reset to center.';
  } catch (err) {
    if (status) status.textContent = `Position reset failed: ${err}`;
    console.warn('layer-hud: could not reset window positions:', err);
  }
});

$('export-settings').addEventListener('click', async () => {
  try {
    const blob = new Blob([await invoke('export_config')], { type: 'application/json' });
    const a = document.createElement('a');
    const stamp = new Date().toISOString().replace(/[-:]/g, '').replace(/\.\d{3}Z$/, 'Z');
    const filename = `layer-hud-settings-${stamp}.json`;
    a.href = URL.createObjectURL(blob); a.download = filename; a.click(); URL.revokeObjectURL(a.href);
    $('settings-data-status').textContent = `Exported to ~/Downloads/${filename}`;
  }
  catch (err) { $('settings-data-status').textContent = String(err); }
});
$('import-settings').addEventListener('click', async () => {
  $('import-file').value = '';
  $('import-file').click();
});
$('import-file').addEventListener('change', async (e) => {
  const file = e.target.files?.[0]; if (!file) return;
  try { await invoke('import_config', { contents: await file.text() }); window.location.reload(); }
  catch (err) { $('settings-data-status').textContent = String(err); }
});
document.addEventListener('click', (event) => {
  if (event.target.closest('#reset-settings') && !event.defaultPrevented) {
    event.preventDefault();
    if (confirm('Reset all settings to defaults?')) resetAllSettings();
  }
});
