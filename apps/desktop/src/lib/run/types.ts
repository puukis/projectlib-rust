import type { RunStatus } from "@projectlib/db";

export type RunLifecycleStatus = RunStatus["status"];

export interface RunState extends RunStatus {
  tabId: string | null;
}

export interface RunCommand {
  command: string;
  args: string[];
  env: Record<string, string>;
  cwd: string;
  runId: string | null;
}

export interface RunOverrides {
  command?: string;
  args?: string[];
  env?: Record<string, string>;
  cwd?: string;
  remember?: boolean;
  runId?: string | null;
}

export interface RunProcess {
  tabId: string;
  stopRequested: boolean;
  type: "pty" | "shell";
  kill(): Promise<void> | void;
}

export interface RunToastMessage {
  projectId: string;
  message: string;
  actionLabel?: string;
  tabId?: string;
}
