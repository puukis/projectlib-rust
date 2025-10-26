import { readTextFile, writeTextFile } from '@tauri-apps/plugin-fs';
import { safeExec } from './logging';

export async function readFileSafe(path: string) {
  return safeExec('fs:read', async () => {
    const text = await readTextFile(path);
    return { path, text, bytes: text.length };
  }, { path });
}

export async function writeFileSafe(path: string, text: string) {
  return safeExec('fs:write', async () => {
    await writeTextFile(path, text);
    return { path, bytes: text.length };
  }, { path, bytes: text.length });
}
