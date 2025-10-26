import { getProject } from "@projectlib/db";
import { invoke } from "@tauri-apps/api/core";
import { Terminal } from "@xterm/xterm";
import { writable, type Readable } from "svelte/store";
import { spawn } from "tauri-pty";

import { resolveTerminalTheme } from "./theme";
import type { Disposable, ShellInfo, TerminalTab } from "./types";

type ColorMode = "light" | "dark";

function detectColorMode(): ColorMode {
  if (typeof window === "undefined" || !("matchMedia" in window)) {
    return "light";
  }

  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

export class TerminalService {
  private readonly tabMap = new Map<string, TerminalTab>();
  private readonly tabsStore = writable<TerminalTab[]>([]);
  private shellCache: ShellInfo | null = null;
  private colorMode: ColorMode = detectColorMode();
  private readonly mediaListener?: (event: MediaQueryListEvent) => void;

  constructor() {
    this.setColorMode(this.colorMode);
    if (typeof window !== "undefined" && "matchMedia" in window) {
      const query = window.matchMedia("(prefers-color-scheme: dark)");
      this.mediaListener = (event) => {
        this.setColorMode(event.matches ? "dark" : "light");
      };
      query.addEventListener("change", this.mediaListener);
      this.setColorMode(query.matches ? "dark" : "light");
    }
  }

  get tabs(): Readable<TerminalTab[]> {
    return { subscribe: this.tabsStore.subscribe };
  }

  private setColorMode(mode: ColorMode) {
    this.colorMode = mode;
    const theme = resolveTerminalTheme(mode);
    for (const tab of this.tabMap.values()) {
      tab.terminal.options.theme = theme;
      tab.terminal.refresh(0, tab.terminal.rows - 1);
    }
  }

  private emitTabs() {
    this.tabsStore.set(Array.from(this.tabMap.values()));
  }

  private countProjectTabs(projectId: string): number {
    let count = 0;
    for (const tab of this.tabMap.values()) {
      if (tab.projectId === projectId) {
        count += 1;
      }
    }
    return count;
  }

  private async resolveShell(): Promise<ShellInfo> {
    if (!this.shellCache) {
      this.shellCache = await invoke<ShellInfo>("terminal_default_shell");
    }

    return this.shellCache;
  }

  async createTab(projectId: string): Promise<TerminalTab> {
    const project = await getProject(projectId);
    if (!project) {
      throw new Error(`Project ${projectId} not found`);
    }

    const shell = await this.resolveShell();
    const terminal = new Terminal({
      allowProposedApi: true,
      convertEol: true,
      cursorBlink: true,
      theme: resolveTerminalTheme(this.colorMode),
      fontFamily: "Menlo, 'Fira Code', Consolas, monospace",
      fontSize: 14,
    });

    const tabId = crypto.randomUUID();
    const labelIndex = this.countProjectTabs(projectId) + 1;
    const tabTitle = `${project.name} #${labelIndex}`;

    const pty = spawn(shell.program, shell.args, {
      cols: terminal.cols,
      rows: terminal.rows,
      cwd: project.path,
    });

    const disposables: Disposable[] = [];

    disposables.push(
      pty.onData((data) => {
        terminal.write(data);
      }),
    );

    disposables.push(
      terminal.onData((data) => {
        pty.write(data);
      }),
    );

    disposables.push(
      pty.onExit(({ exitCode }) => {
        terminal.write(`\r\nProcess exited with code ${exitCode}\r\n`);
      }),
    );

    const tab: TerminalTab = {
      id: tabId,
      projectId,
      title: tabTitle,
      cwd: project.path,
      shell,
      pty,
      terminal,
      disposables,
    };

    this.tabMap.set(tabId, tab);
    this.emitTabs();
    return tab;
  }

  write(id: string, data: string): void {
    const tab = this.tabMap.get(id);
    if (!tab) {
      return;
    }

    tab.pty.write(data);
  }

  resize(id: string, columns: number, rows: number): void {
    const tab = this.tabMap.get(id);
    if (!tab) {
      return;
    }

    if (columns > 0 && rows > 0) {
      tab.pty.resize(columns, rows);
    }
  }

  dispose(id: string): void {
    const tab = this.tabMap.get(id);
    if (!tab) {
      return;
    }

    for (const disposable of tab.disposables) {
      disposable.dispose();
    }

    try {
      tab.pty.kill();
    } catch (err) {
      console.error("Failed to kill terminal", err);
    }

    tab.terminal.dispose();
    this.tabMap.delete(id);
    this.emitTabs();
  }
}

export const terminalService = new TerminalService();
