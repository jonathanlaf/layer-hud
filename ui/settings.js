const { invoke } = window.__TAURI__.core;

let cfg = await invoke('get_config');
const $ = (id) => document.getElementById(id);

$('oryx').value = cfg.oryx_url;
$('opacity').value = cfg.opacity;
$('opacity-val').textContent = cfg.opacity;
$('colors').checked = cfg.use_oryx_colors;
$('combo').value = cfg.grab_combo.join(',');

async function push() {
  await invoke('set_config', { config: cfg });
}

$('opacity').addEventListener('input', async (e) => {
  cfg.opacity = Number(e.target.value);
  $('opacity-val').textContent = cfg.opacity;
  await push();
});
$('colors').addEventListener('change', async (e) => {
  cfg.use_oryx_colors = e.target.checked;
  await push();
});
$('combo').addEventListener('change', async (e) => {
  cfg.grab_combo = e.target.value.split(',');
  await push();
});
$('apply-url').addEventListener('click', async () => {
  $('url-status').textContent = '…';
  try {
    const res = await invoke('refresh_layout', { url: $('oryx').value });
    cfg = await invoke('get_config');
    $('url-status').textContent = res.stale ? 'offline — using cache' : 'applied';
    $('url-status').className = res.stale ? 'error' : 'ok';
  } catch (err) {
    $('url-status').textContent = String(err);
    $('url-status').className = 'error';
  }
});
