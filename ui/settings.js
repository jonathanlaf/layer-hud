const { invoke } = window.__TAURI__.core;

let cfg = await invoke('get_config');
const $ = (id) => document.getElementById(id);

const MOD_LABELS = { cmd: '⌘', alt: '⌥', ctrl: '⌃', shift: '⇧' };
const MOD_ORDER = ['cmd', 'alt', 'ctrl', 'shift'];
const comboText = (arr) => arr.map((m) => MOD_LABELS[m]).join('') || '—';

$('oryx').value = cfg.oryx_url;
$('opacity').value = cfg.opacity;
$('opacity-val').textContent = cfg.opacity;
$('key-opacity').value = cfg.key_opacity;
$('key-opacity-val').textContent = cfg.key_opacity;
$('bg-color').value = cfg.bg_color;
$('key-fill-color').value = cfg.key_fill_color;
$('key-fill-opacity').value = cfg.key_fill_opacity;
$('key-fill-opacity-val').textContent = cfg.key_fill_opacity;
$('padding').value = cfg.padding;
$('padding-val').textContent = cfg.padding;
$('text-color').value = cfg.text_color;
$('legend-color').value = cfg.legend_color;
$('border-color').value = cfg.border_color;
$('colors-toggle').checked = cfg.use_oryx_colors;
$('combo-display').textContent = comboText(cfg.grab_combo);

async function push() {
  // Never clobber a window rect saved by the overlay after this page loaded.
  const disk = await invoke('get_config');
  cfg.window = disk.window;
  await invoke('set_config', { config: cfg });
}

const bind = (id, field, parse = (v) => v, valId = null) => {
  $(id).addEventListener('input', async (e) => {
    cfg[field] = parse(e.target.value);
    if (valId) $(valId).textContent = cfg[field];
    await push();
  });
};
bind('opacity', 'opacity', Number, 'opacity-val');
bind('key-opacity', 'key_opacity', Number, 'key-opacity-val');
bind('bg-color', 'bg_color');
bind('key-fill-color', 'key_fill_color');
bind('key-fill-opacity', 'key_fill_opacity', Number, 'key-fill-opacity-val');
bind('padding', 'padding', Number, 'padding-val');
bind('text-color', 'text_color');
bind('legend-color', 'legend_color');
bind('border-color', 'border_color');

$('colors-toggle').addEventListener('change', async (e) => {
  cfg.use_oryx_colors = e.target.checked;
  await push();
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
