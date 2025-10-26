import { info, warn, error, debug, attachConsole } from '@tauri-apps/plugin-log';

type KeyValues = Record<string, unknown>;

type TauriWindow = Window & {
  __TAURI_INTERNALS__?: {
    transformCallback?: unknown;
  };
};

function hasTauriBridge() {
  if (typeof window === 'undefined') return false;
  const tauriWindow = window as TauriWindow;
  return typeof tauriWindow.__TAURI_INTERNALS__?.transformCallback === 'function';
}

function logToConsole(level: 'log' | 'warn' | 'error' | 'debug', msg: string, kv: KeyValues) {
  if (!import.meta.env.DEV) return;
  const serialized = serializeKeyValues(kv);
  const parts = Object.entries(serialized).map(([key, value]) => `${key}=${value}`);
  const suffix = parts.length > 0 ? ` ${parts.join(' ')}` : '';
  const text = `[${msg}]${suffix}`;
  if (level === 'warn') console.warn(text);
  else if (level === 'error') console.error(text);
  else if (level === 'debug') console.debug(text);
  else console.log(text);
}

async function sendLog(
  level: 'info' | 'warn' | 'error' | 'debug',
  fn: typeof info,
  msg: string,
  kv: KeyValues = {},
) {
  if (!hasTauriBridge()) {
    logToConsole(level === 'info' ? 'log' : level, msg, kv);
    return;
  }
  try {
    await fn(msg, { keyValues: serializeKeyValues(kv) });
  } catch (err) {
    logToConsole('warn', 'ui:log:failed', { level, msg, err: err instanceof Error ? err.message : String(err) });
  }
}

function canAttachConsole() {
  return hasTauriBridge();
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
    const err = e?.error;
    const message =
      err instanceof Error
        ? err.message
        : typeof err === 'string'
          ? err
          : e.message ?? String(err ?? 'Unknown error');

    void sendLog('error', error, 'ui:window:error', {
      msg: message,
      file: e.filename,
      line: e.lineno,
      column: e.colno,
      stack: err instanceof Error ? err.stack : undefined,
    });
  });
  window.addEventListener('unhandledrejection', (e: PromiseRejectionEvent | any) => {
    void sendLog('error', error, 'ui:promise:unhandled', { reason: String(e?.reason ?? e) });
  });
}

export async function logOk(msg: string, kv: KeyValues = {}) {
  await sendLog('info', info, msg, kv);
}

export async function logWarn(msg: string, kv: KeyValues = {}) {
  await sendLog('warn', warn, msg, kv);
}

export async function logErr(msg: string, kv: KeyValues = {}) {
  await sendLog('error', error, msg, kv);
}

export async function logDbg(msg: string, kv: KeyValues = {}) {
  if (!import.meta.env.DEV) return;
  await sendLog('debug', debug, msg, kv);
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
