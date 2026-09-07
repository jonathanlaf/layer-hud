// Base table: Oryx keycode -> what macOS Canadian-CSA actually outputs.
// Verified against the OS layout tables (UCKeyTranslate dump, 2026-09-01).
export const BASE_TABLE = {
  KC_NO: '', KC_TRANSPARENT: '',
  KC_ESCAPE: '⎋', KC_TAB: '⇥', KC_ENTER: '⏎', KC_SPACE: '␣', KC_BSPC: '⌫',
  KC_DELETE: '⌦', KC_CAPS_LOCK: '⇪',
  KC_LEFT_SHIFT: '⇧', KC_RIGHT_SHIFT: '⇧', KC_LEFT_CTRL: '⌃', KC_RIGHT_CTRL: '⌃',
  KC_LEFT_ALT: '⌥', KC_RIGHT_ALT: '⌥', KC_LEFT_GUI: '⌘', KC_RIGHT_GUI: '⌘',
  KC_LEFT: '←', KC_RIGHT: '→', KC_UP: '↑', KC_DOWN: '↓',
  KC_HOME: '⇱', KC_END: '⇲', KC_PAGE_UP: '⇞', KC_PGDN: '⇟',
  KC_MINUS: '-', KC_EQUAL: '=', KC_COMMA: ',', KC_DOT: '.',
  KC_COLN: ':', KC_SCLN: ';', KC_UNDS: '_',
  KC_LPRN: '(', KC_RPRN: ')',
  KC_KP_PLUS: '+', KC_KP_MINUS: '-', KC_KP_ASTERISK: '*', KC_KP_SLASH: '/',
  KC_KP_DOT: '.', KC_KP_EQUAL: '=',
  KC_MEDIA_PLAY_PAUSE: '⏯', KC_MEDIA_NEXT_TRACK: '⏭', KC_MEDIA_PREV_TRACK: '⏮',
  KC_MEDIA_STOP: '⏹', KC_AUDIO_VOL_UP: '🔊', KC_AUDIO_VOL_DOWN: '🔉', KC_AUDIO_MUTE: '🔇',
  MAC_SPOTLIGHT: '🔍',
  // CSA pack (Oryx CMS keycodes), values = macOS CSA output
  CSA_ECUT: 'é', CSA_EGRV: 'è', CSA_AGRV: 'à', CSA_CCED: 'ç', CSA_UGRV: 'ù',
  CSA_APOS: "'", CSA_DQOT: '"', CSA_QEST: '?',
  CSA_LGIL: '«', CSA_RGIL: '»', CSA_LESS: '<', CSA_GRTR: '>',
  CSA_LBRC: '[', CSA_RBRC: ']', CSA_LCBR: '{', CSA_RCBR: '}',
  CSA_DTLD: '~', CSA_BSLS: '\\', CSA_PIPE: '|', CSA_SLSH: '/',
};

const DEAD_MARK = '̲'; // combining low line under the char
const DEAD_TABLE = {
  CSA_DCRC: '^' + DEAD_MARK,
  CSA_DGRV: '`' + DEAD_MARK,
  CSA_DTRM: '¨' + DEAD_MARK,
};

// Combos verified on macOS Canadian-CSA (swap-immune set).
// Key = `${code}+${mods}` with mods letters in C,S,A,G order.
export const COMBO_TABLE = {
  'CSA_EGRV+A': '\\',
  'KC_MINUS+A': '|',
  'CSA_ECUT+A': '/',
  'CSA_AGRV+A': '`',
  'KC_6+S': '?',
  'KC_MINUS+S': '_',
  'KC_COMMA+S': "'",
  'KC_DOT+S': '"',
  'KC_EQUAL+S': '+',
};

// What Shift produces on macOS Canadian-CSA for keys whose shifted output is
// not obvious (letters and accented letters just uppercase — omitted).
const SHIFT_TABLE = {
  KC_1: '!', KC_2: '@', KC_3: '#', KC_4: '$', KC_5: '%',
  KC_6: '?', KC_7: '&', KC_8: '*', KC_9: '(', KC_0: ')',
  KC_MINUS: '_', KC_EQUAL: '+', KC_COMMA: "'", KC_DOT: '"',
  KC_SCLN: ':',
};

// The small shift legend for a tap slot, or '' when it would be noise:
// slots with modifiers baked in (Shift wouldn't produce the table value),
// layer refs, macros, custom labels, and keys not in SHIFT_TABLE.
export function shiftLabel(slot) {
  if (!slot || slot.customLabel) return '';
  if (slot.layer !== null && slot.layer !== undefined) return '';
  if (slot.macro || modString(slot.modifiers)) return '';
  return SHIFT_TABLE[slot.code] ?? '';
}

const MOD_SYMBOL = { C: '⌃', S: '⇧', A: '⌥', G: '⌘' };

function modString(modifiers) {
  if (!modifiers) return '';
  let s = '';
  if (modifiers.leftCtrl || modifiers.rightCtrl) s += 'C';
  if (modifiers.leftShift || modifiers.rightShift) s += 'S';
  if (modifiers.leftAlt || modifiers.rightAlt) s += 'A';
  if (modifiers.leftGui || modifiers.rightGui) s += 'G';
  return s;
}

function cleanCode(code) {
  return typeof code === 'string' ? code.replace(/^(KC|CSA|MAC)_/, '').replace(/_/g, ' ') : '';
}

export function translateSlot(slot) {
  if (!slot) return '';
  if (typeof slot.customLabel === 'string' && slot.customLabel) return slot.customLabel;
  if (slot.layer !== null && slot.layer !== undefined) return `L${slot.layer}`;
  if (slot.macro && Array.isArray(slot.macro.keys)) {
    const parts = slot.macro.keys
      .filter((k) => typeof k?.code === 'string' && k.code !== 'KC_TRANSPARENT')
      .map((k) => {
        const mods = modString(k.modifiers)
          .split('')
          .map((m) => MOD_SYMBOL[m])
          .join('');
        return mods + cleanCode(k.code);
      });
    return parts.length ? parts.join(' ') : 'MACRO';
  }
  if (typeof slot.code !== 'string') return '';
  const mods = modString(slot.modifiers);
  if (mods) {
    const combo = COMBO_TABLE[`${slot.code}+${mods}`];
    if (combo) return combo;
    const base = DEAD_TABLE[slot.code] ?? BASE_TABLE[slot.code] ?? cleanCode(slot.code);
    return mods.split('').map((m) => MOD_SYMBOL[m]).join('') + base;
  }
  if (Object.hasOwn(DEAD_TABLE, slot.code)) return DEAD_TABLE[slot.code];
  if (Object.hasOwn(BASE_TABLE, slot.code)) return BASE_TABLE[slot.code];
  const m = slot.code.match(/^KC_([A-Z0-9])$/);
  if (m) return m[1];
  if (/^KC_F\d+$/.test(slot.code)) return slot.code.slice(3);
  return cleanCode(slot.code);
}
