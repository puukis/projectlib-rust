import * as monaco from "monaco-editor";
import EditorWorker from "monaco-editor/esm/vs/editor/editor.worker?worker";
import JsonWorker from "monaco-editor/esm/vs/language/json/json.worker?worker";
import CssWorker from "monaco-editor/esm/vs/language/css/css.worker?worker";
import HtmlWorker from "monaco-editor/esm/vs/language/html/html.worker?worker";
import TsWorker from "monaco-editor/esm/vs/language/typescript/ts.worker?worker";

type MonacoTheme = "vs" | "vs-dark";

declare const self: Window & typeof globalThis & {
  MonacoEnvironment?: {
    getWorker(moduleId: string, label: string): Worker;
  };
};

if (typeof self !== "undefined") {
  self.MonacoEnvironment = {
    getWorker(_moduleId, label) {
      if (label === "json") {
        return new JsonWorker();
      }
      if (label === "css" || label === "scss" || label === "less") {
        return new CssWorker();
      }
      if (label === "html" || label === "handlebars" || label === "razor") {
        return new HtmlWorker();
      }
      if (label === "typescript" || label === "javascript" || label === "ts") {
        return new TsWorker();
      }
      return new EditorWorker();
    },
  };
}

const modelCache = new Map<string, monaco.editor.ITextModel>();

export function getOrCreateModel(
  path: string,
  text: string,
  language: string,
): monaco.editor.ITextModel {
  const uri = monaco.Uri.file(path);
  const existing = modelCache.get(path);
  if (existing) {
    if (existing.getValue() !== text) {
      existing.setValue(text);
    }
    monaco.editor.setModelLanguage(existing, language);
    return existing;
  }

  const model = monaco.editor.createModel(text, language, uri);
  modelCache.set(path, model);
  return model;
}

export function disposeModel(path: string): void {
  const model = modelCache.get(path);
  if (model) {
    model.dispose();
    modelCache.delete(path);
  }
}

export function setTheme(theme: MonacoTheme): void {
  monaco.editor.setTheme(theme);
}

export function getModel(path: string): monaco.editor.ITextModel | undefined {
  return modelCache.get(path);
}

export function getMonaco(): typeof monaco {
  return monaco;
}
