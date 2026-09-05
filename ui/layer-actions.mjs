import { translateSlot } from './translator.mjs';

export const LAYER_ACTIONS = {
  tap: { icon: 'tapped', label: 'Tap', name: 'Tapped', instruction: 'tap' },
  hold: { icon: 'held', label: 'Hold', name: 'Held', instruction: 'hold' },
  doubleTap: { icon: 'double-tapped', label: 'Double-tap', name: 'Double Tapped', instruction: 'double tap' },
  tapHold: { icon: 'tapped-held', label: 'Tap then hold', name: 'Tapped and Held', instruction: 'tap and hold' },
};

const KEY_NAMES = {
  KC_ENT: 'ENTER', KC_ENTER: 'ENTER', KC_ESC: 'ESCAPE', KC_ESCAPE: 'ESCAPE',
  KC_SPC: 'SPACE', KC_SPACE: 'SPACE', KC_BSPC: 'BACKSPACE', KC_BSPACE: 'BACKSPACE',
  KC_DEL: 'DELETE', KC_PGDN: 'PAGE DOWN', KC_PGUP: 'PAGE UP',
};
const SYMBOL_NAMES = { '⏎': 'ENTER', '␣': 'SPACE', '⌫': 'BACKSPACE', '⌦': 'DELETE', '⇥': 'TAB', '⎋': 'ESCAPE' };

function textualKeyName(key, keyIndex) {
  if (key.customLabel) return SYMBOL_NAMES[key.customLabel] || key.customLabel;
  // Name the key by its main label, not by the layer action in another slot.
  const slot = key.tap || key.hold;
  if (!slot) return keyLocation(keyIndex);
  if (slot.layer !== null && slot.layer !== undefined) return `Layer ${slot.layer} key`;
  if (!slot.code || ['KC_NO', 'KC_TRANSPARENT', 'KC_TRNS'].includes(slot.code)) return keyLocation(keyIndex);
  if (slot.code in KEY_NAMES) return KEY_NAMES[slot.code];
  if (slot.code.startsWith('KC_')) return slot.code.slice(3).replace(/_/g, ' ');
  const label = translateSlot(slot);
  return SYMBOL_NAMES[label] || label || keyLocation(keyIndex);
}

export function triggerInstruction(trigger) {
  const source = trigger.sourcePosition === 0 ? '' : ` (from Layer ${trigger.sourcePosition})`;
  return `${LAYER_ACTIONS[trigger.slot].instruction} ${trigger.keyLabel}${source}`;
}

// All configured routes, including multiple actions on one key; not live gestures.
export function layerTriggerList(layers) {
  const result = layers.map((layer) => ({
    position: Number(layer.position),
    title: layer.title || `Layer ${layer.position}`,
    triggers: [],
  })).sort((a, b) => a.position - b.position);
  const byPosition = new Map(result.map((layer) => [layer.position, layer]));
  for (const source of layers) {
    for (const [keyIndex, key] of (source.keys || []).entries()) {
      for (const slot of Object.keys(LAYER_ACTIONS)) {
        const target = key[slot]?.layer;
        if (target === null || target === undefined || target === '') continue;
        byPosition.get(Number(target))?.triggers.push({
          slot, keyIndex, keyLabel: textualKeyName(key, keyIndex),
          sourcePosition: Number(source.position),
          sourceTitle: source.title || `Layer ${source.position}`,
        });
      }
    }
  }
  return result;
}

export function keyLocation(index) {
  const side = index < 26 ? 'Left' : 'Right';
  const local = index % 26;
  return local < 24
    ? `${side} · row ${Math.floor(local / 6) + 1}, column ${local % 6 + 1}`
    : `${side} · thumb ${local - 23}`;
}
