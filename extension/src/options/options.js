import { api } from '../platform.js';
import * as store from '../options-store.js';

const $ = (id) => document.getElementById(id);
const boxes = [...document.querySelectorAll('input[data-key]')];
const include = $('include');
const exclude = $('exclude');
const sites = $('sites');
const min = $('min');
const unit = $('unit');
const bypass = $('bypass');
const port = $('port');
const key = $('key');
const chip = $('chip');
const statusNote = $('statusNote');
const portNote = $('portNote');
const result = $('result');
const startRow = $('startRow');

const saved = document.createElement('div');
saved.className = 'saved';
saved.setAttribute('role', 'status');
document.body.appendChild(saved);
let savedTimer = null;

let current = null;
let recording = false;

function flash(text) {
  saved.textContent = text;
  saved.classList.add('show');
  clearTimeout(savedTimer);
  savedTimer = setTimeout(() => saved.classList.remove('show'), 1400);
}

function pickUnit(kb) {
  if (kb > 0 && kb % 1048576 === 0) return 1048576;
  if (kb > 0 && kb % 1024 === 0) return 1024;
  return 1;
}

function paint(options) {
  current = options;
  for (const box of boxes) box.checked = Boolean(options[box.dataset.key]);
  const start = boxes.find((b) => b.dataset.key === 'silentStart');
  start.disabled = !options.silentAdd;
  startRow.setAttribute('aria-disabled', String(!options.silentAdd));
  if (document.activeElement !== include) include.value = options.includeTypes.join(' ');
  if (document.activeElement !== exclude) exclude.value = options.excludeTypes.join(' ');
  if (document.activeElement !== sites) sites.value = options.blockedSites.join('\n');
  if (document.activeElement !== min && document.activeElement !== unit) {
    const factor = pickUnit(options.minSizeKb);
    unit.value = String(factor);
    min.value = String(options.minSizeKb / factor);
  }
  if (!recording) bypass.textContent = options.bypassKey;
  if (document.activeElement !== port) port.value = options.manualPort ?? '';
  if (document.activeElement !== key) key.value = options.manualKey;
}

async function commit(patch) {
  paint(await store.save(patch));
  flash('Saved');
}

for (const box of boxes) {
  box.addEventListener('change', () => commit({ [box.dataset.key]: box.checked }));
}

include.addEventListener('change', () => commit({ includeTypes: store.parseTypes(include.value) }));
exclude.addEventListener('change', () => commit({ excludeTypes: store.parseTypes(exclude.value) }));
sites.addEventListener('change', () => commit({ blockedSites: store.parseLines(sites.value) }));

function commitMin() {
  const value = Number(min.value);
  const kb = Number.isFinite(value) && value > 0 ? Math.round(value * Number(unit.value)) : 0;
  return commit({ minSizeKb: kb });
}
min.addEventListener('change', commitMin);
unit.addEventListener('change', commitMin);

for (const link of document.querySelectorAll('[data-reset]')) {
  link.addEventListener('click', async () => {
    paint(await store.reset([link.dataset.reset]));
    flash('Back to the default');
  });
}

function stopRecording() {
  recording = false;
  bypass.classList.remove('recording');
  bypass.textContent = current.bypassKey;
}

bypass.addEventListener('click', () => {
  recording = true;
  bypass.classList.add('recording');
  bypass.textContent = 'Press a key';
});

bypass.addEventListener('keydown', (event) => {
  if (!recording) return;
  if (event.key === 'Tab') {
    stopRecording();
    return;
  }
  event.preventDefault();
  if (event.key === 'Escape') {
    stopRecording();
    return;
  }
  if (['Shift', 'Control', 'Alt', 'Meta'].includes(event.key)) return;
  recording = false;
  bypass.classList.remove('recording');
  commit({ bypassKey: event.key.length === 1 ? event.key.toUpperCase() : event.key });
});

bypass.addEventListener('blur', () => {
  if (recording) stopRecording();
});

port.addEventListener('change', async () => {
  const parsed = store.parsePort(port.value);
  if (parsed === undefined) {
    portNote.textContent = 'Use a port from 1024 to 65535, or leave it empty.';
    portNote.className = 'hint bad-text';
    return;
  }
  portNote.textContent = 'Leave empty to use the one Seamless reports.';
  portNote.className = 'hint';
  await commit({ manualPort: parsed });
  refreshStatus();
});

key.addEventListener('change', async () => {
  await commit({ manualKey: key.value });
  refreshStatus();
});

async function refreshStatus() {
  let reply;
  try {
    reply = await api.runtime.sendMessage({ type: 'status' });
  } catch {
    reply = null;
  }
  chip.className = 'chip';
  if (reply?.ok) {
    chip.classList.add('ok');
    chip.textContent = 'Connected';
    statusNote.textContent = 'Downloads you catch will go straight to Seamless.';
  } else {
    chip.classList.add('bad');
    chip.textContent = 'Not running';
    statusNote.textContent = reply?.error || 'Seamless is not running. Open it, or start a download and it opens by itself.';
  }
}

$('test').addEventListener('click', async () => {
  const button = $('test');
  const parsed = store.parsePort(port.value);
  if (parsed === undefined) {
    result.className = 'result bad';
    result.textContent = 'Use a port from 1024 to 65535, or leave it empty.';
    return;
  }
  button.disabled = true;
  result.className = 'result';
  result.textContent = 'Testing';
  try {
    const reply = await api.runtime.sendMessage({ type: 'test', port: parsed, key: key.value.trim() });
    result.className = `result ${reply?.ok ? 'ok' : 'bad'}`;
    result.textContent = reply?.message || 'No answer from the extension. Reload it and try again.';
  } catch {
    result.className = 'result bad';
    result.textContent = 'No answer from the extension. Reload it and try again.';
  } finally {
    button.disabled = false;
  }
  refreshStatus();
});

store.load().then(paint);
store.onChange(paint);
refreshStatus();
