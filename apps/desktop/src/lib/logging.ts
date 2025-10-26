import { info, warn, error, debug, attachConsole } from '@tauri-apps/plugin-log';

export async function setupLogging() {
  if (import.meta.env.DEV) await attachConsole();
  window.addEventListener('error', (e) => {
    if (e?.error) {
      error('ui:window:error', {
        keyValues: { msg: String(e.error), file: e.filename, line: e.lineno },
      });
    }
  });
  window.addEventListener('unhandledrejection', (e: PromiseRejectionEvent | any) => {
    error('ui:promise:unhandled', {
      keyValues: { reason: String(e?.reason ?? e) },
    });
  });
}

export async function logOk(msg: string, kv: Record<string, any> = {}) {
  await info(msg, { keyValues: kv });
}

export async function logWarn(msg: string, kv: Record<string, any> = {}) {
  await warn(msg, { keyValues: kv });
}

export async function logErr(msg: string, kv: Record<string, any> = {}) {
  await error(msg, { keyValues: kv });
}

export async function logDbg(msg: string, kv: Record<string, any> = {}) {
  if (import.meta.env.DEV) await debug(msg, { keyValues: kv });
}

export async function safeExec<T>(
  label: string,
  fn: () => Promise<T>,
  ctx: Record<string, any> = {}
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
