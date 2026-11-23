import { api } from '../platform.js';
import * as store from '../options-store.js';

const chip = document.getElementById('chip');
const note = document.getElementById('note');
const boxes = [...document.querySelectorAll('input[data-key]')];
const startRow = document.getElementById('startRow');
let timer = null;

function paint(options) {
  for (const box of boxes) box.checked = Boolean(options[box.dataset.key]);
  const start = boxes.find((b) => b.dataset.key === 'silentStart');
  start.disabled = !options.silentAdd;
  startRow.setAttribute('aria-disabled', String(!options.silentAdd));
}

function setStatus(reply) {
  chip.className = 'chip';
  if (reply?.ok) {
    chip.classList.add('ok');
    chip.textContent = 'Connected';
    note.hidden = true;
    return;
  }
  chip.classList.add('bad');
  chip.textContent = 'Not running';
  note.textContent = reply?.error || 'Seamless is not running. Open it, or start a download and it opens by itself.';
  note.hidden = false;
}

// Right after the browser starts, the background worker and the helper can take a moment to
// answer. Keep showing "Checking" through a few quick retries before saying "Not running".
let quickTries = 3;

async function refresh() {
  let reply;
  try {
    reply = await api.runtime.sendMessage({ type: 'status' });
  } catch {
    reply = { ok: false };
  }
  if (!reply?.ok && quickTries > 0) {
    quickTries -= 1;
    setTimeout(refresh, 700);
    return;
  }
  quickTries = 0;
  setStatus(reply);
}

for (const box of boxes) {
  box.addEventListener('change', async () => {
    paint(await store.save({ [box.dataset.key]: box.checked }));
  });
}

document.getElementById('all').addEventListener('click', async () => {
  await api.runtime.openOptionsPage();
  window.close();
});

store.load().then(paint);
store.onChange(paint);
refresh();
timer = setInterval(refresh, 4000);
window.addEventListener('unload', () => clearInterval(timer));
