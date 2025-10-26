import { readDir } from '@tauri-apps/api/fs';
import { safeExec } from './logging';

export async function scanProjects(root: string) {
  return safeExec('project:scan', async () => {
    await readDir(root, { recursive: true });
    const projects: Array<Record<string, unknown>> = [];
    return { root, count: projects.length, projects };
  }, { root });
}
