export const api = globalThis.browser ?? globalThis.chrome;

export const isFirefox = Boolean(api?.runtime?.getURL) && api.runtime.getURL('').startsWith('moz-extension:');

export const action = api?.action ?? api?.browserAction;

export function withTimeout(promise, ms, message) {
  let timer;
  const timeout = new Promise((_, reject) => {
    timer = setTimeout(() => reject(new Error(message || 'timeout')), ms);
  });
  return Promise.race([promise, timeout]).finally(() => clearTimeout(timer));
}
