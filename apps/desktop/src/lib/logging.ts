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

function toErrorMetadata(err: unknown): {
  message?: string;
  name?: string;
  stack?: string;
  cause?: unknown;
  raw?: unknown;
} {
  if (err instanceof Error) {
    return {
      message: err.message,
      name: err.name,
      stack: err.stack,
      cause: (err as { cause?: unknown }).cause,
    };
  }

  if (typeof err === 'string') {
    return { message: err };
  }

  if (typeof err === 'object' && err !== null) {
    const maybeMessage =
      'message' in err && typeof (err as { message?: unknown }).message === 'string'
        ? ((err as { message?: string }).message as string)
        : undefined;
    const maybeName =
      'name' in err && typeof (err as { name?: unknown }).name === 'string'
        ? ((err as { name?: string }).name as string)
        : undefined;
    const maybeStack =
      'stack' in err && typeof (err as { stack?: unknown }).stack === 'string'
        ? ((err as { stack?: string }).stack as string)
        : undefined;
    const maybeCause = 'cause' in err ? (err as { cause?: unknown }).cause : undefined;

    return {
      message: maybeMessage,
      name: maybeName,
      stack: maybeStack,
      cause: maybeCause,
      raw: err,
    };
  }

  return { raw: err };
}

function normalizeCause(cause: unknown): string | undefined {
  if (cause === undefined) return undefined;
  if (cause instanceof Error) return `${cause.name}: ${cause.message}`;
  if (typeof cause === 'string') return cause;

  try {
    return JSON.stringify(cause);
  } catch {
    return String(cause);
  }
}

function sanitizeRaw(raw: unknown): string | undefined {
  if (raw === undefined) return undefined;
  if (raw instanceof Error) return `${raw.name}: ${raw.message}`;
  if (typeof raw === 'string') return raw;

  try {
    return JSON.stringify(raw);
  } catch {
    return String(raw);
  }
}

async function sendLog(
  level: 'info' | 'warn' | 'error' | 'debug',
  fn: typeof info,
  msg: string,
  kv: KeyValues = {},
) {
  logToConsole(level === 'info' ? 'log' : level, msg, kv);
  if (!hasTauriBridge()) return;
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
    const metadata = toErrorMetadata(err);
    const message = metadata.message ?? e.message ?? String(err ?? 'Unknown error');

    void sendLog('error', error, 'ui:window:error', {
      msg: message,
      name: metadata.name,
      stack: metadata.stack,
      cause: normalizeCause(metadata.cause),
      raw: sanitizeRaw(metadata.raw),
      file: e.filename,
      line: e.lineno,
      column: e.colno,
      type: e.type,
      isTrusted: e.isTrusted,
      defaultPrevented: e.defaultPrevented,
      timeStamp: e.timeStamp,
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
