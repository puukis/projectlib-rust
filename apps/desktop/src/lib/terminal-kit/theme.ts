import type { ITheme } from "@xterm/xterm";

type ColorMode = "light" | "dark";

const LIGHT_THEME: ITheme = {
  background: "#f8fafc",
  foreground: "#0f172a",
  cursor: "#2563eb",
  selectionBackground: "rgba(37, 99, 235, 0.2)",
  black: "#1f2933",
  red: "#d92d20",
  green: "#12b76a",
  yellow: "#f79009",
  blue: "#2563eb",
  magenta: "#8b5cf6",
  cyan: "#06aed4",
  white: "#f8fafc",
  brightBlack: "#334155",
  brightRed: "#f04438",
  brightGreen: "#22c55e",
  brightYellow: "#facc15",
  brightBlue: "#1d4ed8",
  brightMagenta: "#7c3aed",
  brightCyan: "#0891b2",
  brightWhite: "#ffffff",
};

const DARK_THEME: ITheme = {
  background: "#0b1120",
  foreground: "#e2e8f0",
  cursor: "#60a5fa",
  selectionBackground: "rgba(96, 165, 250, 0.35)",
  black: "#020617",
  red: "#f97066",
  green: "#34d399",
  yellow: "#fbbf24",
  blue: "#60a5fa",
  magenta: "#c084fc",
  cyan: "#22d3ee",
  white: "#f8fafc",
  brightBlack: "#1e293b",
  brightRed: "#fb7185",
  brightGreen: "#4ade80",
  brightYellow: "#fde047",
  brightBlue: "#3b82f6",
  brightMagenta: "#a855f7",
  brightCyan: "#38bdf8",
  brightWhite: "#f1f5f9",
};

function readCssVariable(name: string): string | null {
  if (typeof window === "undefined") {
    return null;
  }

  const value = getComputedStyle(document.documentElement).getPropertyValue(name);
  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : null;
}

function withCssOverrides(base: ITheme): ITheme {
  const background = readCssVariable("--terminal-bg");
  const foreground = readCssVariable("--terminal-fg");
  const cursor = readCssVariable("--terminal-cursor");
  const selection = readCssVariable("--terminal-selection");

  return {
    ...base,
    background: background ?? base.background,
    foreground: foreground ?? base.foreground,
    cursor: cursor ?? base.cursor,
    selectionBackground: selection ?? base.selectionBackground,
  };
}

export function resolveTerminalTheme(mode: ColorMode): ITheme {
  return withCssOverrides(mode === "dark" ? DARK_THEME : LIGHT_THEME);
}
