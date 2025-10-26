import { spawn, type IPty } from 'tauri-pty';
import { logOk, logErr, safeExec } from './logging';

export async function startRun(projectId: string, cmd: string, args: string[], cwd: string): Promise<IPty> {
  return safeExec('run:start', async () => {
    const pty = spawn(cmd, args, { cwd, cols: 80, rows: 24 });
    await logOk('run:started', { projectId, cmd, cwd });
    pty.onExit(async ({ exitCode }) =>
      (exitCode === 0)
        ? logOk('run:exit:ok', { projectId, code: 0 })
        : logErr('run:exit:err', { projectId, code: exitCode ?? -1 })
    );
    pty.onError?.(async (e: any) => logErr('run:pty:error', { projectId, msg: String(e) }));
    return pty;
  }, { projectId, cmd, cwd });
}

export * from './terminal-kit';
