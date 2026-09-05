import { LAYER_ACTIONS, layerTriggerList, triggerInstruction } from './layer-actions.mjs';

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const $ = (id) => document.getElementById(id);

function element(tag, text, className) {
  const node = document.createElement(tag);
  if (text !== undefined) node.textContent = text;
  if (className) node.className = className;
  return node;
}

function actionLabel(slot) {
  const action = LAYER_ACTIONS[slot];
  const label = element('span', undefined, 'action');
  const icon = element('span', undefined, 'action-icon');
  icon.style.setProperty('--action-icon', `url('icons/${action.icon}.svg')`);
  icon.setAttribute('aria-hidden', 'true');
  label.append(icon, element('span', action.name));
  return label;
}

for (const slot of ['tap', 'tapHold', 'hold', 'doubleTap']) $('action-legend').append(actionLabel(slot));
const shiftLegend = element('span', undefined, 'action');
const shiftIcon = element('span', undefined, 'action-icon');
shiftIcon.style.setProperty('--action-icon', "url('icons/shift.svg')");
shiftIcon.setAttribute('aria-hidden', 'true');
shiftLegend.append(shiftIcon, element('span', 'Alternate character (Shift)'));
$('action-legend').append(shiftLegend);

function render(layers) {
  const fragment = document.createDocumentFragment();
  for (const layer of layerTriggerList(layers)) {
    const item = element('li');
    item.append(element('strong', `Layer ${layer.position}: `));
    const instructions = layer.triggers.map(triggerInstruction);
    item.append(document.createTextNode(instructions.length
      ? instructions.join('; or ')
      : layer.position === 0 ? 'base layer — release a held layer key to return when applicable.'
        : 'no direct key shortcut configured.'));
    if (layer.title !== `Layer ${layer.position}`) item.append(element('small', layer.title));
    fragment.append(item);
  }
  $('layers').replaceChildren(fragment);
}

let loadVersion = 0;
async function reload() {
  const version = ++loadVersion;
  $('reload').disabled = true;
  $('status').textContent = 'Loading layout…';
  try {
    const layout = await invoke('load_layout');
    if (version !== loadVersion) return;
    const layers = layout?.data?.layout?.revision?.layers;
    if (!Array.isArray(layers)) throw new Error('Layout has no layer data.');
    render(layers);
    $('status').textContent = layers.length ? (layout.stale ? 'Using cached layout.' : '') : 'This layout has no layers.';
  } catch (error) {
    if (version !== loadVersion) return;
    $('layers').replaceChildren();
    $('status').textContent = `Could not load layers. Set an Oryx URL in Settings, then reload. ${error}`;
  } finally {
    if (version === loadVersion) $('reload').disabled = false;
  }
}

$('reload').addEventListener('click', reload);
// Install before loading, because the initial fetch can itself refresh the cache.
try {
  await listen('layout-refreshed', reload);
} catch (error) {
  console.error('Could not listen for layout refreshes:', error);
}
await reload();
