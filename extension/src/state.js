import { api } from './platform.js';

export function createPersisted(key) {
  const area = api?.storage?.session ?? null;
  const data = {};
  let timer = null;

  const ready = area
    ? area.get(key).then((stored) => {
        const saved = stored?.[key];
        if (saved && typeof saved === 'object') {
          for (const name of Object.keys(saved)) if (!(name in data)) data[name] = saved[name];
        }
      }).catch(() => {})
    : Promise.resolve();

  function touch() {
    if (!area) return;
    clearTimeout(timer);
    timer = setTimeout(() => {
      area.set({ [key]: data }).catch(() => {});
    }, 150);
  }

  return { data, ready, touch };
}
