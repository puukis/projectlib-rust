import { invoke } from '@tauri-apps/api/core';
import { logOk, logErr, safeExec } from './logging';

export type GitResult = { code: number; stdout: string; stderr: string };

export async function runGit(cwd: string, args: string[]): Promise<GitResult> {
  return safeExec('git:run', async () => {
    const [code, stdout, stderr] = await invoke<[number, string, string]>('git_run', { cwd, args });
    const result = { code, stdout, stderr };
    if (code === 0) {
      await logOk('git:ok', { cwd, cmd: args.join(' ') });
    } else {
      await logErr('git:exit', { cwd, cmd: args.join(' '), code });
    }
    return result;
  }, { cwd, cmd: args.join(' ') });
}

export const gitStatus = (cwd: string) => runGit(cwd, ['status', '--porcelain=v1', '-z']);
export const gitCommit = (cwd: string, msg: string) => runGit(cwd, ['commit', '-m', msg]);
export const gitFetch = (cwd: string) => runGit(cwd, ['fetch', '--all']);
export const gitPull = (cwd: string) => runGit(cwd, ['pull']);
export const gitPush = (cwd: string) => runGit(cwd, ['push']);
