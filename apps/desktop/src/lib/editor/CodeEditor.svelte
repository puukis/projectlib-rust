<script lang="ts">
  import { createEventDispatcher, onDestroy, onMount } from "svelte";
  import type { editor } from "monaco-editor";
  import { getMonaco, setTheme } from "./monaco";

  export let model: editor.ITextModel | null = null;
  export let path: string | null = null;
  export let theme: "vs" | "vs-dark" = "vs-dark";
  export let viewState: editor.IStandaloneCodeEditorViewState | null = null;

  const dispatch = createEventDispatcher<{
    change: { path: string; value: string };
    save: { path: string };
    cursor: { path: string; viewState: editor.IStandaloneCodeEditorViewState | null };
  }>();

  let container: HTMLDivElement | null = null;
  let instance: editor.IStandaloneCodeEditor | null = null;
  let changeDisposable: editor.IDisposable | null = null;
  let cursorDisposable: editor.IDisposable | null = null;

  function bindModel(newModel: editor.ITextModel | null) {
    if (!instance || !newModel) {
      return;
    }
    instance.setModel(newModel);
    if (viewState) {
      instance.restoreViewState(viewState);
      instance.revealLineInCenter(viewState.cursorState?.[0]?.position?.lineNumber ?? 1);
    }
    if (changeDisposable) {
      changeDisposable.dispose();
    }
    changeDisposable = instance.onDidChangeModelContent(() => {
      const currentPath = path ?? newModel.uri.toString();
      dispatch("change", { path: currentPath, value: newModel.getValue() });
    });
    if (cursorDisposable) {
      cursorDisposable.dispose();
    }
    cursorDisposable = instance.onDidChangeCursorPosition(() => {
      const currentView = instance?.saveViewState() ?? null;
      if (path) {
        dispatch("cursor", { path, viewState: currentView });
      }
    });
  }

  onMount(() => {
    if (!container) {
      return;
    }
    const monaco = getMonaco();
    instance = monaco.editor.create(container, {
      automaticLayout: true,
      minimap: { enabled: true },
      fontSize: 14,
      tabSize: 2,
      scrollBeyondLastLine: false,
      smoothScrolling: true,
      bracketPairColorization: { enabled: true },
    });
    const saveCommand = instance.addCommand(
      monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS,
      () => {
        if (path) {
          dispatch("save", { path });
        }
      },
    );
    bindModel(model);

    return () => {
      saveCommand.dispose();
    };
  });

  onDestroy(() => {
    if (changeDisposable) {
      changeDisposable.dispose();
    }
    if (cursorDisposable) {
      cursorDisposable.dispose();
    }
    if (instance) {
      instance.dispose();
      instance = null;
    }
  });

  $: if (instance && model) {
    bindModel(model);
  }

  $: setTheme(theme);

  $: if (instance && viewState && model) {
    instance.restoreViewState(viewState);
  }
</script>

<div class="editor" bind:this={container}></div>

<style>
  .editor {
    position: relative;
    width: 100%;
    height: 100%;
  }
</style>
