const { invoke } = window.__TAURI__.core;

let cfg = await invoke('get_config');
const $ = (id) => document.getElementById(id);

const MOD_LABELS = { cmd: '⌘', alt: '⌥', ctrl: '⌃', shift: '⇧' };
const MOD_ORDER = ['cmd', 'alt', 'ctrl', 'shift'];
const comboText = (arr) => arr.map((m) => MOD_LABELS[m]).join('') || '—';

$('oryx').value = cfg.oryx_url;
$('bg-color').value = cfg.bg_color;
$('key-fill-color').value = cfg.key_fill_color;
$('text-color').value = cfg.text_color;
$('legend-color').value = cfg.legend_color;
$('border-color').value = cfg.border_color;
$('base-outline-color').value = cfg.base_outline_color;
$('grab-outline-color').value = cfg.grab_outline_color;
$('combo-display').textContent = comboText(cfg.grab_combo);

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
bind('border-color', 'border_color');
bind('base-outline-color', 'base_outline_color');
bind('grab-outline-color', 'grab_outline_color');

const bindCheckbox = (id, field) => {
  $(id).checked = cfg[field];
  $(id).addEventListener('change', (e) => commit(field, e.target.checked));
};

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
  ['base-outline-opacity', 'base_outline_opacity'],
  ['base-outline-width', 'base_outline_width'],
  ['grab-outline-opacity', 'grab_outline_opacity'],
  ['grab-outline-width', 'grab_outline_width'],
  ['key-font-size', 'key_font_size'],
  ['legend-font-size', 'legend_font_size'],
  ['layer-name-font-size', 'layer_name_font_size'],
]) bindNumeric(id, field);

bindCheckbox('colors-toggle', 'use_oryx_colors');
bindCheckbox('base-outline-enabled', 'base_outline_enabled');
bindCheckbox('grab-outline-enabled', 'grab_outline_enabled');

const bindFontCheckbox = (id, field) => {
  $(id).checked = cfg[field];
  $(id).addEventListener('change', (e) => commit(field, e.target.checked));
};
const fontFamilies = await invoke('list_system_fonts');
for (const prefix of ['key', 'legend', 'layer-name']) {
  const fieldPrefix = prefix.replace(/-/g, '_');
  const select = $(`${prefix}-font-family`);
  for (const name of fontFamilies) {
    const option = document.createElement('option');
    option.value = name;
    option.textContent = name;
    select.appendChild(option);
  }
  const family = `${fieldPrefix}_font_family`;
  select.value = cfg[family];
  select.addEventListener('change', (e) => commit(family, e.target.value));
  bindFontCheckbox(`${prefix}-font-bold`, `${fieldPrefix}_font_bold`);
  bindFontCheckbox(`${prefix}-font-italic`, `${fieldPrefix}_font_italic`);
  bindFontCheckbox(`${prefix}-font-ligatures`, `${fieldPrefix}_font_ligatures`);
}

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

$('reset-position').addEventListener('click', async () => {
  await invoke('clear_window_position');
});

$('apply-url').addEventListener('click', async () => {
  $('url-status-row').hidden = false;
  $('url-status').textContent = '…';
  $('url-status').className = 'status';
  try {
    const res = await invoke('refresh_layout', { url: $('oryx').value });
    cfg = await invoke('get_config');
    $('url-status').textContent = res.stale ? 'offline — using cache' : 'applied';
    $('url-status').className = res.stale ? 'status error' : 'status ok';
  } catch (err) {
    $('url-status').textContent = String(err);
    $('url-status').className = 'status error';
  }
  $('url-status-row').hidden = false;
});
