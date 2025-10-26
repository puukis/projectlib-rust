import { info, warn, error, debug, attachConsole } from '@tauri-apps/plugin-log';

type KeyValues = Record<string, unknown>;

type TauriWindow = Window & {
  __TAURI_INTERNALS__?: {
    transformCallback?: unknown;
  };
};

function canAttachConsole() {
  const tauriWindow = window as TauriWindow;
  return typeof tauriWindow.__TAURI_INTERNALS__?.transformCallback === 'function';
}

function serializeKeyValues(values: KeyValues): Record<string, string> {
  return Object.fromEntries(
    Object.entries(values)
      .filter(([_, value]) => value !== undefined)
      .map(([key, value]) => {
        if (value instanceof Error) return [key, value.message];
        if (typeof value === 'string') return [key, value];
        if (typeof value === 'number' || typeof value === 'boolean' || value === null)
          return [key, String(value)];

        try {
          return [key, JSON.stringify(value)];
        } catch {
          return [key, String(value)];
        }
      }),
  );
}

export async function setupLogging() {
  if (import.meta.env.DEV && canAttachConsole()) await attachConsole();
  window.addEventListener('error', (e) => {
    if (e?.error) {
      error('ui:window:error', {
        keyValues: serializeKeyValues({
          msg: String(e.error),
          file: e.filename,
          line: e.lineno,
        }),
      });
    }
  });
  window.addEventListener('unhandledrejection', (e: PromiseRejectionEvent | any) => {
    error('ui:promise:unhandled', {
      keyValues: serializeKeyValues({ reason: String(e?.reason ?? e) }),
    });
  });
}

export async function logOk(msg: string, kv: KeyValues = {}) {
  await info(msg, { keyValues: serializeKeyValues(kv) });
}

export async function logWarn(msg: string, kv: KeyValues = {}) {
  await warn(msg, { keyValues: serializeKeyValues(kv) });
}

export async function logErr(msg: string, kv: KeyValues = {}) {
  await error(msg, { keyValues: serializeKeyValues(kv) });
}

export async function logDbg(msg: string, kv: KeyValues = {}) {
  if (import.meta.env.DEV) await debug(msg, { keyValues: serializeKeyValues(kv) });
}

export async function safeExec<T>(
  label: string,
  fn: () => Promise<T>,
  ctx: KeyValues = {}
): Promise<T> {
  const t0 = performance.now();
  try {
    const result = await fn();
    const ms = Math.round(performance.now() - t0);
    await logOk(`${label}:ok`, { ...ctx, ms });
    return result;
  } catch (err: any) {
    const ms = Math.round(performance.now() - t0);
    await logErr(`${label}:err`, { ...ctx, ms, msg: String(err) });
    throw err;
  }
}
