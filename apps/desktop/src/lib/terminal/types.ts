import type { Terminal } from "@xterm/xterm";
import type { IPty } from "tauri-pty";

export interface Disposable {
  dispose(): void;
}

export interface ShellInfo {
  program: string;
  args: string[];
}

export type TerminalTabKind = "shell" | "run";

export interface TerminalTab {
  id: string;
  projectId: string;
  title: string;
  cwd: string;
  shell: ShellInfo;
  pty: IPty;
  terminal: Terminal;
  disposables: Disposable[];
  kind: TerminalTabKind;
}
