import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const ui = new URL('../', import.meta.url);
const source = name => readFileSync(new URL(name, ui), 'utf8');

test('all shipped JavaScript parses, including the Tauri entry points', () => {
  for (const name of readdirSync(ui).filter(name => /\.(js|mjs)$/.test(name))) {
    const result = spawnSync(process.execPath, ['--check', fileURLToPath(new URL(name, ui))], { encoding: 'utf8' });
    assert.equal(result.status, 0, `${name}: ${result.stderr}`);
  }
});

test('every literal frontend command is registered in the backend', () => {
  const main = source('../src-tauri/src/main.rs');
  const handler = main.split('tauri::generate_handler![')[1].split('])')[0];
  const commands = new Set([...handler.matchAll(/\w+::(\w+)/g)].map(match => match[1]));
  for (const name of ['hud.js', 'legend.js', 'settings.js']) {
    for (const [, command] of source(name).matchAll(/invoke\('([^']+)'/g)) {
      if (command.startsWith('plugin:autostart|')) continue;
      assert.ok(commands.has(command), `${name}: unregistered command ${command}`);
    }
  }
});

test('settings controls have unique IDs and exist for every literal binding', () => {
  const ids = [...source('settings.html').matchAll(/\bid="([^"]+)"/g)].map(match => match[1]);
  const known = new Set(ids);
  assert.equal(known.size, ids.length, 'duplicate IDs');
  const script = source('settings.js');
  for (const [, id] of script.matchAll(/\$\('([^']+)'\)/g)) assert.ok(known.has(id), `missing #${id}`);
  for (const [, kind, id] of script.matchAll(/\b(bind|bindNumeric)\('([^']+)'/g)) {
    assert.ok(known.has(id), `missing #${id}`);
    if (kind === 'bindNumeric') assert.ok(known.has(`${id}-val`), `missing #${id}-val`);
  }
});

test('numeric Settings limits agree with backend validation', () => {
  const html = source('settings.html');
  const rust = source('../src-tauri/src/config.rs');
  for (const [, id, field] of source('settings.js').matchAll(/bindNumeric\('([^']+)', '([^']+)'\)/g)) {
    const tag = [...html.matchAll(/<input\b[^>]*>/g)].map(match => match[0]).find(tag => tag.includes(`id="${id}"`));
    const [, min] = tag.match(/min="([^"]+)"/);
    const [, max] = tag.match(/max="([^"]+)"/);
    const clamp = rust.match(new RegExp(`self\\.${field}\\.clamp\\((-?[\\d.]+),\\s*(-?[\\d.]+)\\)`));
    assert.ok(clamp, `missing backend clamp for ${field}`);
    assert.deepEqual(clamp.slice(1).map(Number), [Number(min), Number(max)], field);
  }
});
