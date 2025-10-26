import {
  listRunStatuses,
  listRuns,
  saveRunConfig,
  saveRunStatus,
  type Project,
  type RunConfig,
  type RunStatus,
  updateRunOutcome,
} from "@projectlib/db";
import { platform } from "@tauri-apps/api/os";
import { Command, type Child } from "@tauri-apps/plugin-shell";
import { Terminal } from "@xterm/xterm";
import { get, writable, type Readable } from "svelte/store";
import type { TerminalTab } from "../terminal";
import { terminalService } from "../terminal";
import { MissingRunConfigurationError, RunAlreadyInProgressError } from "./errors";
import type { RunCommand, RunOverrides, RunProcess, RunState, RunToastMessage } from "./types";

function createIdleState(projectId: string): RunState {
  const now = Date.now();
  return {
    projectId,
    status: "idle",
    lastRunId: null,
    lastCommand: null,
    lastArgs: [],
    lastEnv: {},
    lastCwd: null,
    lastExitCode: null,
    startedAt: null,
    finishedAt: null,
    updatedAt: now,
    tabId: null,
  };
}

function cloneState(state: RunState): RunState {
  return {
    ...state,
    lastArgs: [...state.lastArgs],
    lastEnv: { ...state.lastEnv },
  };
}

class ShellAdapter {
  private readonly dataListeners = new Set<(data: string) => void>();
  private readonly exitListeners = new Set<(event: { exitCode: number; signal?: number }) => void>();
  private readonly stdoutHandler = (data: string) => {
    for (const listener of this.dataListeners) {
      listener(data);
    }
  };
  private readonly stderrHandler = (data: string) => {
    for (const listener of this.dataListeners) {
      listener(data);
    }
  };
  private readonly closeHandler = (payload: { code: number | null; signal: number | null }) => {
    const code = typeof payload.code === "number" ? payload.code : 0;
    for (const listener of this.exitListeners) {
      listener({ exitCode: code, signal: payload.signal ?? undefined });
    }
  };
  private cols: number;
  private rows: number;

  constructor(
    private readonly program: string,
    private readonly command: Command<string>,
    private readonly child: Child,
    columns: number,
    rows: number,
  ) {
    this.cols = columns;
    this.rows = rows;

    this.command.stdout.on("data", this.stdoutHandler);
    this.command.stderr.on("data", this.stderrHandler);
    this.command.on("close", this.closeHandler);
  }

  get adapter(): import("tauri-pty").IPty {
    const self = this;
    return {
      pid: this.child.pid,
      cols: this.cols,
      rows: this.rows,
      process: this.program,
      handleFlowControl: false,
      onData(listener) {
        self.dataListeners.add(listener);
        return {
          dispose() {
            self.dataListeners.delete(listener);
          },
        };
      },
      onExit(listener) {
        self.exitListeners.add(listener);
        return {
          dispose() {
            self.exitListeners.delete(listener);
          },
        };
      },
      resize(columns: number, rows: number) {
        self.cols = columns;
        self.rows = rows;
      },
      clear() {
        // no-op
      },
      write(data: string) {
        void self.child.write(data);
      },
      kill() {
        void self.child.kill();
      },
      pause() {
        // no-op
      },
      resume() {
        // no-op
      },
    };
  }

  dispose(): void {
    this.command.stdout.off("data", this.stdoutHandler);
    this.command.stderr.off("data", this.stderrHandler);
    this.command.off("close", this.closeHandler);
    this.dataListeners.clear();
    this.exitListeners.clear();
  }
}

class RunService {
  private readonly states = writable<Map<string, RunState>>(new Map());
  private readonly toastStore = writable<RunToastMessage | null>(null);
  private readonly processes = new Map<string, RunProcess>();
  private platformPromise: Promise<string> | null = null;

  get runStates(): Readable<Map<string, RunState>> {
    return { subscribe: this.states.subscribe };
  }

  get toasts(): Readable<RunToastMessage | null> {
    return { subscribe: this.toastStore.subscribe };
  }

  async loadPersistedStates(projects: Project[]): Promise<void> {
    const rows = await listRunStatuses();
    const map = new Map<string, RunState>();
    const projectIds = new Set(projects.map((project) => project.id));

    for (const row of rows) {
      if (!projectIds.has(row.projectId)) {
        continue;
      }
      map.set(row.projectId, { ...row, tabId: null });
    }

    for (const project of projects) {
      if (!map.has(project.id)) {
        map.set(project.id, createIdleState(project.id));
      }
    }

    this.states.set(map);
  }

  syncProjects(projects: Project[]): void {
    const map = new Map(get(this.states));
    const projectIds = new Set(projects.map((project) => project.id));

    for (const project of projects) {
      if (!map.has(project.id)) {
        map.set(project.id, createIdleState(project.id));
      }
    }

    for (const [projectId] of map) {
      if (!projectIds.has(projectId)) {
        map.delete(projectId);
      }
    }

    this.states.set(map);
  }

  getState(projectId: string): RunState {
    const current = get(this.states).get(projectId);
    if (current) {
      return current;
    }

    const next = createIdleState(projectId);
    const map = new Map(get(this.states));
    map.set(projectId, next);
    this.states.set(map);
    return next;
  }

  async start(project: Project, overrides: RunOverrides = {}): Promise<string> {
    const state = cloneState(this.getState(project.id));
    if (state.status === "starting" || state.status === "running") {
      throw new RunAlreadyInProgressError(project.id);
    }

    const command = await this.resolveCommand(project, overrides);
    if (!command) {
      throw new MissingRunConfigurationError(project.id);
    }

    const now = Date.now();
    const next: RunState = {
      ...state,
      status: "starting",
      lastRunId: command.runId ?? state.lastRunId,
      lastCommand: command.command,
      lastArgs: [...command.args],
      lastEnv: { ...command.env },
      lastCwd: command.cwd,
      lastExitCode: null,
      startedAt: now,
      finishedAt: null,
      updatedAt: now,
      tabId: null,
    };

    await saveRunStatus(this.toPersist(next));
    this.updateState(next);

    let tab: TerminalTab | null = null;
    let fallback = false;
    let shellAdapter: ShellAdapter | null = null;

    try {
      tab = await terminalService.createRunTab({
        projectId: project.id,
        title: `Run: ${project.name}`,
        command: command.command,
        args: command.args,
        cwd: command.cwd,
        env: command.env,
      });
    } catch (err) {
      console.warn("PTY run launch failed, attempting shell fallback", err);
      fallback = true;
      tab = await terminalService.createRunTab({
        projectId: project.id,
        title: `Run: ${project.name}`,
        command: command.command,
        args: command.args,
        cwd: command.cwd,
        env: command.env,
        spawnOverride: async (terminal: Terminal) => {
          const cmd = Command.create(command.command, command.args, {
            cwd: command.cwd,
            env: Object.keys(command.env).length > 0 ? command.env : undefined,
          });
          const child = await cmd.spawn();
          shellAdapter = new ShellAdapter(
            command.command,
            cmd,
            child,
            terminal.cols,
            terminal.rows,
          );
          return shellAdapter.adapter;
        },
      });
    }

    if (!tab) {
      throw new Error("Failed to create run terminal tab");
    }

    if (shellAdapter) {
      const adapterRef = shellAdapter;
      tab.disposables.push({
        dispose() {
          adapterRef.dispose();
        },
      });
    }

    const runningState: RunState = {
      ...next,
      status: "running",
      tabId: tab.id,
      updatedAt: Date.now(),
    };
    await saveRunStatus(this.toPersist(runningState));
    this.updateState(runningState);

    const runProcess: RunProcess = {
      tabId: tab.id,
      stopRequested: false,
      type: fallback ? "shell" : "pty",
      kill: async () => {
        const isWindows = await this.isWindows();
        try {
          if (!fallback && !isWindows) {
            tab?.pty.kill("SIGINT");
          } else {
            tab?.pty.kill();
          }
        } catch (err) {
          console.error("Failed to send kill signal", err);
          try {
            tab?.pty.kill();
          } catch (killErr) {
            console.error("Final kill failed", killErr);
          }
        }
      },
    };

    this.processes.set(project.id, runProcess);

    tab.pty.onExit(async ({ exitCode }) => {
      const processInfo = this.processes.get(project.id);
      const stopRequested = processInfo?.stopRequested ?? false;
      const finished = Date.now();
      const code = typeof exitCode === "number" ? exitCode : null;
      let status: RunStatus["status"] = "succeeded";
      if (stopRequested) {
        status = "stopped";
      } else if (code === null || code !== 0) {
        status = "failed";
      }

      const finalState: RunState = {
        ...this.getState(project.id),
        status,
        lastExitCode: code,
        finishedAt: finished,
        updatedAt: finished,
        tabId: tab?.id ?? null,
      };

      await saveRunStatus(this.toPersist(finalState));
      this.updateState(finalState);
      this.processes.delete(project.id);

      if (command.runId) {
        await updateRunOutcome(command.runId, code, finished);
      }

      if (status === "failed") {
        this.toastStore.set({
          projectId: project.id,
          message: `${project.name} run exited with code ${code ?? "?"}.`,
          actionLabel: "View logs",
          tabId: tab?.id,
        });
      }
    });

    return tab.id;
  }

  async stop(projectId: string): Promise<void> {
    const process = this.processes.get(projectId);
    if (!process) {
      return;
    }

    process.stopRequested = true;
    await process.kill();
  }

  async rememberConfiguration(project: Project, config: RunConfig): Promise<void> {
    await saveRunConfig(config);
    const state = this.getState(project.id);
    const next: RunState = {
      ...state,
      lastRunId: config.id,
      lastCommand: config.command,
      lastArgs: [...config.args],
      lastEnv: { ...config.env },
      lastCwd: config.cwd ?? project.path,
      updatedAt: Date.now(),
    };
    await saveRunStatus(this.toPersist(next));
    this.updateState(next);
  }

  dismissToast(): void {
    this.toastStore.set(null);
  }

  private updateState(state: RunState): void {
    const map = new Map(get(this.states));
    map.set(state.projectId, cloneState(state));
    this.states.set(map);
  }

  private toPersist(state: RunState): RunStatus {
    return {
      projectId: state.projectId,
      status: state.status,
      lastRunId: state.lastRunId,
      lastCommand: state.lastCommand,
      lastArgs: state.lastArgs,
      lastEnv: state.lastEnv,
      lastCwd: state.lastCwd,
      lastExitCode: state.lastExitCode,
      startedAt: state.startedAt,
      finishedAt: state.finishedAt,
      updatedAt: state.updatedAt,
    };
  }

  private async resolveCommand(project: Project, overrides: RunOverrides): Promise<RunCommand | null> {
    if (overrides.command) {
      return {
        command: overrides.command,
        args: overrides.args ?? [],
        env: overrides.env ?? {},
        cwd: overrides.cwd ?? project.path,
        runId: overrides.runId ?? null,
      };
    }

    const current = this.getState(project.id);
    if (current.lastCommand) {
      return {
        command: current.lastCommand,
        args: overrides.args ?? current.lastArgs,
        env: overrides.env ?? current.lastEnv,
        cwd: overrides.cwd ?? current.lastCwd ?? project.path,
        runId: overrides.runId ?? current.lastRunId,
      };
    }

    const runs = await listRuns(project.id);
    const selected = runs[0];
    if (!selected) {
      return null;
    }

    return {
      command: selected.command,
      args: overrides.args ?? selected.args,
      env: overrides.env ?? selected.env,
      cwd: overrides.cwd ?? selected.cwd ?? project.path,
      runId: overrides.runId ?? selected.id,
    };
  }

  private async isWindows(): Promise<boolean> {
    if (!this.platformPromise) {
      this.platformPromise = platform();
    }

    try {
      const value = await this.platformPromise;
      return value.toLowerCase().startsWith("win");
    } catch (err) {
      console.error("Failed to detect platform", err);
      return false;
    }
  }
}

export const runService = new RunService();
