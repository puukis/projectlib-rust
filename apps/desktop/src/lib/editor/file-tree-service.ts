import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { readDir, readTextFile, writeTextFile, type DirEntry } from "@tauri-apps/api/fs";
import { detectLanguage } from "./language";

export type TreeNode = {
  id: string;
  name: string;
  path: string;
  type: "file" | "directory";
  children?: TreeNode[];
  expanded?: boolean;
};

export type FileOpenResult = {
  text: string;
  language: string;
};

export type FsChangeEvent = {
  kind: string;
  path: string;
};

const FILTERED_DIRECTORIES = new Set([
  ".git",
  "node_modules",
  "target",
  "build",
  "dist",
  "out",
  "__pycache__",
]);

export class FileTreeService {
  private fsListener: Promise<UnlistenFn> | null = null;
  private changeHandlers = new Set<(event: FsChangeEvent) => void>();

  constructor() {
    this.ensureListener();
  }

  subscribe(handler: (event: FsChangeEvent) => void): () => void {
    this.changeHandlers.add(handler);
    this.ensureListener();
    return () => {
      this.changeHandlers.delete(handler);
    };
  }

  async registerRoot(path: string): Promise<void> {
    await invoke("register_project_root", { path });
  }

  async unregisterRoot(path: string): Promise<void> {
    try {
      await invoke("unregister_project_root", { path });
    } catch (error) {
      // ignore if scope already removed
      console.warn("Failed to unregister project root", error);
    }
  }

  async scanProject(root: string): Promise<TreeNode[]> {
    const entries = await readDir(root, { recursive: false });
    const children = await Promise.all(
      entries
        .filter((entry) => this.shouldInclude(entry))
        .map(async (entry) => this.entryToNode(entry)),
    );
    return this.sortNodes(children);
  }

  async openFile(path: string): Promise<FileOpenResult> {
    const text = await readTextFile(path);
    const language = detectLanguage(path);
    return { text, language };
  }

  async saveFile(path: string, text: string): Promise<void> {
    await writeTextFile(path, text);
  }

  private async entryToNode(entry: DirEntry): Promise<TreeNode> {
    if (entry.children) {
      const childNodes = await Promise.all(
        entry.children
          .filter((child) => this.shouldInclude(child))
          .map(async (child) => this.entryToNode(child)),
      );
      return {
        id: entry.path,
        name: entry.name ?? entry.path,
        path: entry.path,
        type: "directory",
        children: this.sortNodes(childNodes),
        expanded: false,
      };
    }

    if (entry.path) {
      try {
        const children = await readDir(entry.path, { recursive: false });
        const childNodes = await Promise.all(
          children
            .filter((child) => this.shouldInclude(child))
            .map(async (child) => this.entryToNode(child)),
        );
        return {
          id: entry.path,
          name: entry.name ?? entry.path,
          path: entry.path,
          type: entry.children || childNodes.length > 0 ? "directory" : "file",
          children: childNodes.length > 0 ? this.sortNodes(childNodes) : undefined,
          expanded: false,
        };
      } catch (error) {
        // entry is a file
      }
    }

    return {
      id: entry.path,
      name: entry.name ?? entry.path,
      path: entry.path,
      type: "file",
    };
  }

  private sortNodes(nodes: TreeNode[]): TreeNode[] {
    return nodes.sort((a, b) => {
      if (a.type !== b.type) {
        return a.type === "directory" ? -1 : 1;
      }
      return a.name.localeCompare(b.name);
    });
  }

  private shouldInclude(entry: DirEntry): boolean {
    const name = entry.name ?? "";
    if (FILTERED_DIRECTORIES.has(name)) {
      return false;
    }
    return true;
  }

  private ensureListener() {
    if (this.fsListener) {
      return;
    }
    this.fsListener = listen<FsChangeEvent>("fs:changed", (event) => {
      for (const handler of this.changeHandlers) {
        handler(event.payload);
      }
    });
  }
}

export const fileTreeService = new FileTreeService();
