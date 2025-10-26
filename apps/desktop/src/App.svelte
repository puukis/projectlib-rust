<script lang="ts">
  import { onDestroy, onMount, tick } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { Shell } from "@projectlib/ui";
  import {
    deleteRunConfig,
    listProjects,
    listRuns,
    saveRunConfig,
    upsertProject,
    loadEditorState,
    saveEditorState,
    type Project,
    type RunConfig,
    type EditorState,
    type EditorFileState
  } from "@projectlib/db";
  import TerminalTabs from "./components/TerminalTabs.svelte";
  import ProjectCard from "./components/ProjectCard.svelte";
  import RunButton from "./components/RunButton.svelte";
  import RunConfigModal from "./components/RunConfigModal.svelte";
  import RunToast from "./components/RunToast.svelte";
  import FileTree from "./lib/editor/FileTree.svelte";
  import EditorTabs, { type EditorTab as WorkspaceEditorTab } from "./lib/editor/EditorTabs.svelte";
  import CodeEditor from "./lib/editor/CodeEditor.svelte";
  import GitPanel from "./lib/editor/GitPanel.svelte";
  import { terminalService } from "./lib/terminal";
  import { logOk, logErr } from "./lib/logging";
  import {
    MissingRunConfigurationError,
    RunAlreadyInProgressError,
    runService,
    type RunState,
    type RunToastMessage,
  } from "./lib/run";
  import type { RunOverrides } from "./lib/run";
  import { open as openPath } from "@tauri-apps/plugin-shell";
  import { fileTreeService, type TreeNode } from "./lib/editor/file-tree-service";
  import { getOrCreateModel, disposeModel, setTheme, getMonaco } from "./lib/editor/monaco";
  import type { editor } from "monaco-editor";
  import { mkdir, writeTextFile, remove, rename, exists } from "@tauri-apps/plugin-fs";
  import { join } from "@tauri-apps/api/path";
  import {
    PingSchema,
    type Ping,
    GitPathInfoSchema,
    type GitPathInfo,
    GitRepositoryInfoSchema,
    type GitRepositoryInfo,
    GitStatusResponseSchema,
    type GitStatusResponse,
    RunDetectionListSchema,
    type RunDetection
  } from "@projectlib/shared";

  type RunDraft = {
    command: string;
    argsText: string;
    envText: string;
    cwd: string;
  };

  let ping: Ping = { message: "Pinging backend..." };
  let error: string | null = null;
  let gitInfo: GitPathInfo | null = null;
  let repoPath = "";
  let repoInfo: GitRepositoryInfo | null = null;
  let repoError: string | null = null;
  let status: GitStatusResponse | null = null;
  let statusError: string | null = null;
  let projects: Project[] = [];
  let projectError: string | null = null;
  let projectMessage: string | null = null;
  let selectedProjectId: string | null = null;
  let selectedProject: Project | null = null;
  let terminalError: string | null = null;
  let savingProject = false;

  let runConfigs: RunConfig[] = [];
  let runDrafts: Record<string, RunDraft> = {};
  let runViews: { run: RunConfig; draft: RunDraft }[] = [];
  let runDirty = new Set<string>();
  let localOnlyRuns = new Set<string>();
  let savingRuns: Record<string, boolean> = {};
  let deletingRuns: Record<string, boolean> = {};
  let detectingRuns = false;
  let runError: string | null = null;
  let runMessage: string | null = null;
  let loadingRunsFor: string | null = null;
  let lastRunsProjectId: string | null = null;

  type RunModalMode = "run" | "runWithArgs" | "edit";
  type RunModalState = {
    project: Project;
    initialCommand: string;
    initialArgs: string;
    initialEnv: string;
    initialCwd: string;
    remember: boolean;
    mode: RunModalMode;
  };

  let runStateMap = new Map<string, RunState>();
  let runToast: RunToastMessage | null = null;
  let focusTabId: string | null = null;
  let runControlError: string | null = null;
  let runModal: RunModalState | null = null;

  type EditorDocument = {
    path: string;
    name: string;
    language: string;
    model: editor.ITextModel;
    dirty: boolean;
    viewState: editor.IStandaloneCodeEditorViewState | null;
    savedValue: string;
  };

  let fileTreeNodes: TreeNode[] = [];
  let fileTreeError: string | null = null;
  let fileTreeLoading = false;
  let showFileTree = true;
  let documents: EditorDocument[] = [];
  let activeFilePath: string | null = null;
  let activeDocument: EditorDocument | null = null;
  let workspaceError: string | null = null;
  let fsUnsubscribe: (() => void) | null = null;
  let currentRootPath: string | null = null;
  let quickOpenVisible = false;
  let quickOpenQuery = "";
  let quickOpenMatches: { path: string; name: string }[] = [];
  let quickOpenIndex = 0;
  let quickOpenInput: HTMLInputElement | null = null;
  let showShortcutOverlay = false;
  let awaitingShortcutCombo = false;
  let awaitingTimeout: ReturnType<typeof setTimeout> | null = null;
  let persistTimeout: ReturnType<typeof setTimeout> | null = null;
  let gitPanelError: string | null = null;
  let lastWorkspaceProjectId: string | null = null;
  let flattenedFiles: { path: string; name: string }[] = [];

  const unsubscribeRunStates = runService.runStates.subscribe((value) => {
    runStateMap = value;
  });
  const unsubscribeToasts = runService.toasts.subscribe((value) => {
    runToast = value;
  });

  onMount(async () => {
    setTheme("vs-dark");
    window.addEventListener("keydown", handleGlobalKeydown);
    try {
      const response = await invoke<string>("ping", { message: "Hello from Svelte" });
      ping = PingSchema.parse({ message: response });
      const info = await invoke("git_path_info");
      gitInfo = GitPathInfoSchema.parse(info);
      await loadProjects();
      await runService.loadPersistedStates(projects);
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    }
  });

  onDestroy(() => {
    unsubscribeRunStates();
    unsubscribeToasts();
    window.removeEventListener("keydown", handleGlobalKeydown);
    if (fsUnsubscribe) {
      fsUnsubscribe();
      fsUnsubscribe = null;
    }
    if (currentRootPath) {
      void fileTreeService.unregisterRoot(currentRootPath);
      currentRootPath = null;
    }
    if (awaitingTimeout) {
      clearTimeout(awaitingTimeout);
      awaitingTimeout = null;
    }
    if (persistTimeout) {
      clearTimeout(persistTimeout);
      persistTimeout = null;
    }
  });

  async function loadProjects() {
    try {
      projectError = null;
      const loaded = await listProjects();
      projects = loaded;
      runService.syncProjects(projects);
      if (projects.length > 0) {
        const firstId = projects[0].id;
        if (
          !selectedProjectId ||
          !projects.some((project) => project.id === selectedProjectId)
        ) {
          selectedProjectId = firstId;
        }
      } else {
        selectedProjectId = null;
      }
    } catch (err) {
      projectError = err instanceof Error ? err.message : String(err);
    }
  }

  $: selectedProject = selectedProjectId
    ? projects.find((project) => project.id === selectedProjectId) ?? null
    : null;

  $: {
    const currentId = selectedProject?.id ?? null;
    if (currentId !== lastWorkspaceProjectId) {
      lastWorkspaceProjectId = currentId;
      void setupWorkspace(selectedProject ?? null);
    }
  }

  $: if (selectedProjectId && selectedProjectId !== lastRunsProjectId) {
    lastRunsProjectId = selectedProjectId;
    loadRunsForProject(selectedProjectId);
  } else if (!selectedProjectId && lastRunsProjectId) {
    lastRunsProjectId = null;
    runConfigs = [];
    runDrafts = {};
    runViews = [];
    runDirty = new Set();
    localOnlyRuns = new Set();
    savingRuns = {};
    deletingRuns = {};
    runError = null;
    runMessage = null;
  }

  $: runViews = runConfigs.map((run) => {
    let draft = runDrafts[run.id];
    if (!draft) {
      draft = createDraftFromRun(run);
      runDrafts = { ...runDrafts, [run.id]: draft };
    }

    return { run, draft };
  });

  $: flattenedFiles = flattenFileTree(fileTreeNodes, []);

  $: if (quickOpenVisible) {
    quickOpenMatches = computeQuickOpenMatches();
    if (quickOpenMatches.length === 0) {
      quickOpenIndex = 0;
    } else if (quickOpenIndex >= quickOpenMatches.length) {
      quickOpenIndex = quickOpenMatches.length - 1;
    }
  } else {
    quickOpenMatches = [];
    quickOpenIndex = 0;
  }

  $: activeDocument = activeFilePath ? getDocument(activeFilePath) ?? null : null;

  function createDraftFromRun(run: RunConfig): RunDraft {
    return {
      command: run.command,
      argsText: run.args.join("\n"),
      envText: formatEnv(run.env),
      cwd: run.cwd ?? ""
    };
  }

  function createEmptyDraft(): RunDraft {
    return {
      command: "",
      argsText: "",
      envText: "",
      cwd: selectedProject?.path ?? ""
    };
  }

  async function loadRunsForProject(projectId: string) {
    loadingRunsFor = projectId;
    try {
      runError = null;
      const runs = await listRuns(projectId);
      if (loadingRunsFor !== projectId) {
        return;
      }

      runConfigs = runs;
      const drafts: Record<string, RunDraft> = {};
      for (const run of runs) {
        drafts[run.id] = createDraftFromRun(run);
      }
      runDrafts = drafts;
      runDirty = new Set();
      localOnlyRuns = new Set();
      savingRuns = {};
      deletingRuns = {};
      runMessage = null;
    } catch (err) {
      runError = err instanceof Error ? err.message : String(err);
    } finally {
      if (loadingRunsFor === projectId) {
        loadingRunsFor = null;
      }
    }
  }

  function getRunState(projectId: string): RunState {
    return runStateMap.get(projectId) ?? runService.getState(projectId);
  }

  async function focusTerminal(tabId: string) {
    focusTabId = null;
    await tick();
    focusTabId = tabId;
  }

  function openRunModalForProject(
    project: Project,
    mode: RunModalMode,
    overrides: RunOverrides = {},
  ) {
    const state = getRunState(project.id);
    const command = overrides.command ?? state.lastCommand ?? "";
    const args = overrides.args ?? state.lastArgs ?? [];
    const env = overrides.env ?? state.lastEnv ?? {};
    const cwd = overrides.cwd ?? state.lastCwd ?? project.path;
    const rememberDefault =
      mode === "run" ? true : mode === "runWithArgs" ? overrides.remember ?? false : true;

    runModal = {
      project,
      initialCommand: command,
      initialArgs: args.join("\n"),
      initialEnv: formatEnv(env),
      initialCwd: cwd,
      remember: rememberDefault,
      mode,
    };
  }

  async function handleProjectRun(project: Project, overrides: RunOverrides = {}) {
    const state = getRunState(project.id);
    if (state.status === "running" || state.status === "starting") {
      const shouldStop = confirm(`Stop ${project.name}?`);
      if (shouldStop) {
        try {
          await runService.stop(project.id);
          await logOk("ui:stop:clicked", { projectId: project.id });
        } catch (err) {
          await logErr("ui:stop:err", { projectId: project.id, msg: String(err) });
          runControlError = err instanceof Error ? err.message : String(err);
        }
      }
      return;
    }

    runControlError = null;
    selectedProjectId = project.id;

    try {
      const tabId = await runService.start(project, overrides);
      await logOk("ui:run:clicked", { projectId: project.id });
      await focusTerminal(tabId);
    } catch (err) {
      await logErr("ui:run:err", { projectId: project.id, msg: String(err) });
      if (err instanceof MissingRunConfigurationError) {
        openRunModalForProject(project, "run", overrides);
      } else if (err instanceof RunAlreadyInProgressError) {
        // ignore; state guard above should prevent this
      } else {
        runControlError = err instanceof Error ? err.message : String(err);
      }
    }
  }

  async function handleRunModalSubmit(event: CustomEvent<{
    command: string;
    argsText: string;
    envText: string;
    cwd: string;
    remember: boolean;
  }>) {
    if (!runModal) {
      return;
    }

    const { project, mode } = runModal;
    runModal = null;

    const command = event.detail.command.trim();
    const args = parseArgs(event.detail.argsText);
    const { env, error: envError } = parseEnv(event.detail.envText);
    if (envError) {
      runControlError = envError;
      return;
    }

    const cwdInput = event.detail.cwd.trim();
    const cwd = cwdInput || project.path;

    const state = getRunState(project.id);
    let runId = state.lastRunId ?? null;
    const remember = mode === "edit" ? true : event.detail.remember;

    if (remember) {
      const now = Date.now();
      runId = runId ?? crypto.randomUUID();
      const runConfig: RunConfig = {
        id: runId,
        projectId: project.id,
        command,
        args,
        env,
        cwd,
        lastExitCode: null,
        updatedAt: now,
      };
      try {
        await runService.rememberConfiguration(project, runConfig);
        await loadRunsForProject(project.id);
      } catch (err) {
        runControlError = err instanceof Error ? err.message : String(err);
        return;
      }
    }

    if (mode !== "edit") {
      await handleProjectRun(project, { command, args, env, cwd, runId });
    }
  }

  function handleRunModalCancel() {
    runModal = null;
  }

  function handleRunWithArgs(project: Project) {
    runControlError = null;
    openRunModalForProject(project, "runWithArgs", { remember: false });
  }

  function handleEditRunConfig(project: Project) {
    runControlError = null;
    openRunModalForProject(project, "edit");
  }

  async function handleStopRun(project: Project) {
    try {
      await runService.stop(project.id);
      await logOk("ui:stop:clicked", { projectId: project.id });
    } catch (err) {
      await logErr("ui:stop:err", { projectId: project.id, msg: String(err) });
      runControlError = err instanceof Error ? err.message : String(err);
    }
  }

  async function handleOpenFolder(project: Project) {
    try {
      await openPath(project.path);
    } catch (err) {
      runControlError = err instanceof Error ? err.message : String(err);
    }
  }

  function handleToastAction() {
    if (!runToast) {
      return;
    }
    selectedProjectId = runToast.projectId;
    if (runToast.tabId) {
      focusTerminal(runToast.tabId);
    }
    runService.dismissToast();
  }

  function handleToastDismiss() {
    runService.dismissToast();
  }

  function schedulePersist() {
    if (persistTimeout) {
      clearTimeout(persistTimeout);
    }
    persistTimeout = setTimeout(() => {
      persistTimeout = null;
      void persistEditorStateSnapshot();
    }, 300);
  }

  async function persistEditorStateSnapshot() {
    const openFiles: EditorFileState[] = documents.map((doc) => ({
      path: doc.path,
      language: doc.language,
    }));

    const viewStateRecord: Record<string, unknown> = {};
    for (const doc of documents) {
      if (doc.viewState) {
        viewStateRecord[doc.path] = doc.viewState;
      }
    }

    const payload: EditorState = {
      id: "global",
      openFiles,
      activeFile: activeFilePath,
      viewState: viewStateRecord,
      updatedAt: Date.now(),
    };

    try {
      await saveEditorState(payload);
    } catch (error) {
      console.error("Failed to persist editor state", error);
    }
  }

  function getDocument(path: string): EditorDocument | undefined {
    return documents.find((doc) => doc.path === path);
  }

  async function openDocument(
    path: string,
    focus: boolean = true,
    languageOverride?: string,
    viewState?: editor.IStandaloneCodeEditorViewState | null,
  ) {
    let doc = getDocument(path);
    if (!doc) {
      try {
        const { text, language } = await fileTreeService.openFile(path);
        const name = path.split(/[/\\]/).pop() ?? path;
        const resolvedLanguage = languageOverride ?? language;
        const model = getOrCreateModel(path, text, resolvedLanguage);
        doc = {
          path,
          name,
          language: resolvedLanguage,
          model,
          dirty: false,
          viewState: viewState ?? null,
          savedValue: text,
        };
        documents = [...documents, doc];
        workspaceError = null;
      } catch (error) {
        workspaceError =
          error instanceof Error ? error.message : `Failed to open ${path}`;
        return;
      }
    } else if (languageOverride && languageOverride !== doc.language) {
      const monaco = getMonaco();
      monaco.editor.setModelLanguage(doc.model, languageOverride);
      doc.language = languageOverride;
      documents = [...documents];
    }

    if (viewState) {
      doc.viewState = viewState;
      documents = [...documents];
    }

    if (focus) {
      activeFilePath = path;
    }

    schedulePersist();
  }

  async function saveDocument(path: string) {
    const doc = getDocument(path);
    if (!doc) {
      return;
    }
    try {
      const value = doc.model.getValue();
      await fileTreeService.saveFile(doc.path, value);
      doc.savedValue = value;
      doc.dirty = false;
      documents = [...documents];
      schedulePersist();
      workspaceError = null;
      await logOk("editor:save:ok", { path: doc.path, bytes: value.length });
    } catch (error) {
      await logErr("editor:save:err", { path: doc.path, msg: String(error) });
      workspaceError =
        error instanceof Error ? error.message : `Failed to save ${doc.name}`;
    }
  }

  function closeDocument(path: string, force = false) {
    const doc = getDocument(path);
    if (!doc) {
      return;
    }
    if (!force && doc.dirty) {
      const shouldClose = confirm(`Discard unsaved changes in ${doc.name}?`);
      if (!shouldClose) {
        return;
      }
    }
    disposeModel(path);
    documents = documents.filter((item) => item.path !== path);
    if (activeFilePath === path) {
      activeFilePath = documents[0]?.path ?? null;
    }
    schedulePersist();
  }

  async function loadFileTree(rootPath: string) {
    fileTreeLoading = true;
    try {
      fileTreeNodes = await fileTreeService.scanProject(rootPath);
      fileTreeError = null;
    } catch (error) {
      fileTreeError = error instanceof Error ? error.message : String(error);
    } finally {
      fileTreeLoading = false;
    }
  }

  async function handleFsChange(event: { path: string }) {
    if (!currentRootPath) {
      return;
    }
    if (!event.path.startsWith(currentRootPath)) {
      return;
    }
    await loadFileTree(currentRootPath);
    const doc = getDocument(event.path);
    if (doc && !doc.dirty) {
      try {
        const { text } = await fileTreeService.openFile(doc.path);
        doc.savedValue = text;
        doc.model.setValue(text);
        doc.dirty = false;
        documents = [...documents];
      } catch (error) {
        console.error("Failed to refresh file", error);
      }
    }
  }

  function flattenFileTree(nodes: TreeNode[], acc: { path: string; name: string }[] = []) {
    for (const node of nodes) {
      if (node.type === "file") {
        acc.push({ path: node.path, name: node.name });
      }
      if (node.children) {
        flattenFileTree(node.children, acc);
      }
    }
    return acc;
  }

  function computeQuickOpenMatches() {
    const query = quickOpenQuery.trim().toLowerCase();
    if (!query) {
      return flattenedFiles.slice(0, 50);
    }
    return flattenedFiles
      .filter(
        (item) =>
          item.name.toLowerCase().includes(query) ||
          item.path.toLowerCase().includes(query),
      )
      .slice(0, 50);
  }

  async function openQuickOpen() {
    quickOpenVisible = true;
    quickOpenQuery = "";
    quickOpenIndex = 0;
    await tick();
    quickOpenInput?.focus();
  }

  function closeQuickOpen() {
    quickOpenVisible = false;
    quickOpenQuery = "";
    quickOpenIndex = 0;
  }

  function handleQuickOpenKey(event: KeyboardEvent) {
    if (!quickOpenVisible) {
      return;
    }
    if (event.key === "ArrowDown") {
      event.preventDefault();
      quickOpenIndex = Math.min(quickOpenIndex + 1, quickOpenMatches.length - 1);
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      quickOpenIndex = Math.max(quickOpenIndex - 1, 0);
    } else if (event.key === "Enter") {
      event.preventDefault();
      const match = quickOpenMatches[quickOpenIndex];
      if (match) {
        void openDocument(match.path);
      }
      closeQuickOpen();
    } else if (event.key === "Escape") {
      event.preventDefault();
      closeQuickOpen();
    }
  }

  async function setupWorkspace(project: Project | null) {
    if (persistTimeout) {
      clearTimeout(persistTimeout);
      persistTimeout = null;
    }
    if (fsUnsubscribe) {
      fsUnsubscribe();
      fsUnsubscribe = null;
    }
    if (currentRootPath && (!project || project.path !== currentRootPath)) {
      await fileTreeService.unregisterRoot(currentRootPath);
    }

    documents.forEach((doc) => disposeModel(doc.path));
    documents = [];
    activeFilePath = null;
    fileTreeNodes = [];
    flattenedFiles = [];

    if (!project) {
      currentRootPath = null;
      return;
    }

    currentRootPath = project.path;
    try {
      await fileTreeService.registerRoot(project.path);
      fsUnsubscribe = fileTreeService.subscribe(handleFsChange);
      await loadFileTree(project.path);
      await restoreEditorStateForProject(project.path);
      workspaceError = null;
    } catch (error) {
      workspaceError = error instanceof Error ? error.message : String(error);
    }
  }

  async function restoreEditorStateForProject(rootPath: string) {
    try {
      const persisted = await loadEditorState();
      if (!persisted) {
        return;
      }
      for (const file of persisted.openFiles) {
        if (file.path.startsWith(rootPath)) {
          const view =
            typeof persisted.viewState[file.path] === "object"
              ? (persisted.viewState[file.path] as editor.IStandaloneCodeEditorViewState)
              : null;
          await openDocument(file.path, false, file.language ?? undefined, view);
        }
      }
      if (persisted.activeFile && getDocument(persisted.activeFile)) {
        activeFilePath = persisted.activeFile;
      } else if (documents.length > 0) {
        activeFilePath = documents[0].path;
      }
    } catch (error) {
      console.error("Failed to restore editor state", error);
    }
  }

  async function handleCreateFile(directory: string) {
    const name = prompt("New file name");
    if (!name) {
      return;
    }
    const filePath = await join(directory, name);
    const existsAlready = await exists(filePath);
    if (existsAlready) {
      alert("A file with that name already exists.");
      return;
    }
    await writeTextFile(filePath, "");
    await loadFileTree(currentRootPath ?? directory);
    await openDocument(filePath);
  }

  async function handleCreateFolder(directory: string) {
    const name = prompt("New folder name");
    if (!name) {
      return;
    }
    const folderPath = await join(directory, name);
    const existsAlready = await exists(folderPath);
    if (existsAlready) {
      alert("A folder with that name already exists.");
      return;
    }
    await mkdir(folderPath, { recursive: true });
    await loadFileTree(currentRootPath ?? directory);
  }

  async function handleRename(path: string) {
    const segments = path.split(/[/\\]/);
    const currentName = segments.pop() ?? path;
    const parent = segments.join("/");
    const nextName = prompt("Rename to", currentName);
    if (!nextName || nextName === currentName) {
      return;
    }
    const nextPath = await join(parent, nextName);
    await rename(path, nextPath);
    await loadFileTree(currentRootPath ?? parent);
    const doc = getDocument(path);
    if (doc) {
      disposeModel(path);
      documents = documents.filter((item) => item.path !== path);
      await openDocument(nextPath, activeFilePath === path);
    }
  }

  async function handleDelete(path: string) {
    const confirmed = confirm(`Delete ${path}?`);
    if (!confirmed) {
      return;
    }
    const doc = getDocument(path);
    if (doc) {
      closeDocument(path, true);
    }
    try {
      await remove(path);
    } catch {
      await remove(path, { recursive: true });
    }
    await loadFileTree(currentRootPath ?? path);
  }

  async function handleReveal(path: string) {
    try {
      await openPath(path);
    } catch (error) {
      workspaceError = error instanceof Error ? error.message : String(error);
    }
  }

  async function handleTerminal(directory: string) {
    if (!selectedProject) {
      return;
    }
    try {
      const tab = await terminalService.createTab(selectedProject.id);
      tab.terminal.write(`cd "${directory}"\r`);
    } catch (error) {
      workspaceError = error instanceof Error ? error.message : String(error);
    }
  }

  function closeOtherDocuments(path: string) {
    const others = documents.filter((doc) => doc.path !== path);
    if (others.length === 0) {
      return;
    }
    const dirtyOthers = others.filter((doc) => doc.dirty);
    if (dirtyOthers.length > 0) {
      const shouldClose = confirm(
        `Discard unsaved changes in ${dirtyOthers.length} file${dirtyOthers.length === 1 ? "" : "s"}?`,
      );
      if (!shouldClose) {
        return;
      }
    }
    for (const doc of others) {
      closeDocument(doc.path, true);
    }
    activeFilePath = path;
    schedulePersist();
  }

  function handleGlobalKeydown(event: KeyboardEvent) {
    if (quickOpenVisible) {
      handleQuickOpenKey(event);
      if (event.defaultPrevented) {
        return;
      }
    }

    const isModifier = event.metaKey || event.ctrlKey;

    if (awaitingShortcutCombo && isModifier && event.key.toLowerCase() === "s") {
      event.preventDefault();
      awaitingShortcutCombo = false;
      if (awaitingTimeout) {
        clearTimeout(awaitingTimeout);
        awaitingTimeout = null;
      }
      showShortcutOverlay = true;
      return;
    }

    if (isModifier && event.key.toLowerCase() === "s") {
      event.preventDefault();
      if (activeFilePath) {
        void saveDocument(activeFilePath);
      }
      return;
    }

    if (isModifier && event.key.toLowerCase() === "k") {
      event.preventDefault();
      awaitingShortcutCombo = true;
      if (awaitingTimeout) {
        clearTimeout(awaitingTimeout);
      }
      awaitingTimeout = setTimeout(() => {
        awaitingShortcutCombo = false;
      }, 800);
      return;
    }

    if (isModifier && event.key.toLowerCase() === "p") {
      event.preventDefault();
      void openQuickOpen();
      return;
    }

    if (isModifier && event.key.toLowerCase() === "b") {
      event.preventDefault();
      showFileTree = !showFileTree;
      return;
    }

    if (event.key === "Escape") {
      if (quickOpenVisible) {
        event.preventDefault();
        closeQuickOpen();
      } else if (showShortcutOverlay) {
        event.preventDefault();
        showShortcutOverlay = false;
      }
    }
  }

  function updateDraftField(id: string, field: keyof RunDraft, value: string) {
    const base = runDrafts[id] ?? createEmptyDraft();
    const updated: RunDraft = { ...base, [field]: value };
    runDrafts = { ...runDrafts, [id]: updated };
    markDirty(id);
  }

  function markDirty(id: string) {
    const next = new Set(runDirty);
    next.add(id);
    runDirty = next;
  }

  function clearDirty(id: string) {
    if (!runDirty.has(id)) {
      return;
    }
    const next = new Set(runDirty);
    next.delete(id);
    runDirty = next;
  }

  function isDirty(id: string): boolean {
    return runDirty.has(id);
  }

  function addLocalOnly(id: string) {
    const next = new Set(localOnlyRuns);
    next.add(id);
    localOnlyRuns = next;
  }

  function removeLocalOnly(id: string) {
    if (!localOnlyRuns.has(id)) {
      return;
    }
    const next = new Set(localOnlyRuns);
    next.delete(id);
    localOnlyRuns = next;
  }

  function isLocalOnly(id: string): boolean {
    return localOnlyRuns.has(id);
  }

  function formatEnv(env: Record<string, string>): string {
    return Object.entries(env)
      .map(([key, value]) => `${key}=${value}`)
      .join("\n");
  }

  function parseArgs(text: string): string[] {
    return text
      .split(/\r?\n/)
      .map((line) => line.trim())
      .filter((line) => line.length > 0);
  }

  function parseEnv(text: string): { env: Record<string, string>; error: string | null } {
    const env: Record<string, string> = {};
    const lines = text
      .split(/\r?\n/)
      .map((line) => line.trim())
      .filter((line) => line.length > 0);

    for (const line of lines) {
      const separator = line.indexOf("=");
      if (separator === -1) {
        return {
          env: {},
          error: `Environment entries must be KEY=VALUE (got "${line}")`
        };
      }

      const key = line.slice(0, separator).trim();
      const value = line.slice(separator + 1).trim();
      if (!key) {
        return { env: {}, error: "Environment variable name cannot be empty." };
      }

      env[key] = value;
    }

    return { env, error: null };
  }

  function addRunConfig() {
    if (!selectedProject) {
      return;
    }

    const now = Date.now();
    const id = crypto.randomUUID();
    const run: RunConfig = {
      id,
      projectId: selectedProject.id,
      command: "",
      args: [],
      env: {},
      cwd: selectedProject.path,
      lastExitCode: null,
      updatedAt: now
    };

    runConfigs = [run, ...runConfigs];
    runDrafts = { ...runDrafts, [id]: createEmptyDraft() };
    markDirty(id);
    addLocalOnly(id);
    runMessage = null;
  }

  async function saveRun(id: string) {
    const run = runConfigs.find((item) => item.id === id);
    const draft = runDrafts[id];
    if (!run || !draft) {
      return;
    }

    runError = null;
    runMessage = null;

    const command = draft.command.trim();
    if (!command) {
      runError = "Command cannot be empty.";
      return;
    }

    const args = parseArgs(draft.argsText);
    const { env, error: envError } = parseEnv(draft.envText);
    if (envError) {
      runError = envError;
      return;
    }

    const cwd = draft.cwd.trim();

    savingRuns = { ...savingRuns, [id]: true };

    try {
      const updated: RunConfig = {
        ...run,
        command,
        args,
        env,
        cwd: cwd ? cwd : null,
        updatedAt: Date.now()
      };

      await saveRunConfig(updated);
      clearDirty(id);
      removeLocalOnly(id);
      await loadRunsForProject(updated.projectId);
      runMessage = "Run command saved.";
    } catch (err) {
      runError = err instanceof Error ? err.message : String(err);
    } finally {
      const { [id]: _removed, ...rest } = savingRuns;
      savingRuns = rest;
    }
  }

  async function removeRun(id: string) {
    if (isLocalOnly(id)) {
      runConfigs = runConfigs.filter((run) => run.id !== id);
      const { [id]: _removed, ...restDrafts } = runDrafts;
      runDrafts = restDrafts;
      clearDirty(id);
      removeLocalOnly(id);
      runMessage = "Run command removed.";
      return;
    }

    const run = runConfigs.find((item) => item.id === id);
    if (!run) {
      return;
    }

    deletingRuns = { ...deletingRuns, [id]: true };
    runError = null;
    runMessage = null;

    try {
      await deleteRunConfig(id);
      await loadRunsForProject(run.projectId);
      runMessage = "Run command removed.";
    } catch (err) {
      runError = err instanceof Error ? err.message : String(err);
    } finally {
      const { [id]: _removed, ...rest } = deletingRuns;
      deletingRuns = rest;
    }
  }

  async function detectRuns(projectOverride?: Project) {
    const project = projectOverride ?? selectedProject;
    if (!project) {
      return;
    }

    const isActive = !projectOverride || projectOverride.id === selectedProjectId;
    if (isActive) {
      runError = null;
      runMessage = null;
      detectingRuns = true;
    }

    try {
      const response = await invoke("detect_project_runs", {
        path: project.path
      });
      const suggestions = RunDetectionListSchema.parse(response) as RunDetection[];
      const existingRuns = isActive ? runConfigs : await listRuns(project.id);
      const existingKeys = new Set(
        existingRuns.map((run) => `${run.command}::${run.args.join("\u0000")}`)
      );

      const added: string[] = [];

      for (const suggestion of suggestions) {
        const key = `${suggestion.command}::${suggestion.args.join("\u0000")}`;
        if (existingKeys.has(key)) {
          continue;
        }

        const now = Date.now();
        const run: RunConfig = {
          id: crypto.randomUUID(),
          projectId: project.id,
          command: suggestion.command,
          args: suggestion.args,
          env: suggestion.env,
          cwd: suggestion.cwd ?? project.path,
          lastExitCode: null,
          updatedAt: now
        };

        await saveRunConfig(run);
        added.push(
          [run.command, ...run.args].filter((part) => part.length > 0).join(" ") ||
            run.command
        );
      }

      if (isActive) {
        if (added.length > 0) {
          await loadRunsForProject(project.id);
          runMessage = `Added ${added.length} run command${
            added.length === 1 ? "" : "s"
          }: ${added.join(", ")}`;
        } else {
          runMessage = "No new run commands detected.";
        }
      } else if (added.length > 0 && projectOverride) {
        if (projectOverride.id === selectedProjectId) {
          await loadRunsForProject(project.id);
        }
      }
    } catch (err) {
      if (isActive) {
        runError = err instanceof Error ? err.message : String(err);
      }
    } finally {
      if (isActive) {
        detectingRuns = false;
      }
    }
  }

  async function detectRepository() {
    repoError = null;
    repoInfo = null;
    projectMessage = null;
    projectError = null;
    if (!repoPath.trim()) {
      repoError = "Enter a folder path to inspect.";
      return;
    }

    try {
      const result = await invoke("git_detect_repository", {
        request: { repositoryPath: repoPath }
      });
      repoInfo = GitRepositoryInfoSchema.parse(result);
    } catch (err) {
      repoError = err instanceof Error ? err.message : String(err);
    }
  }

  async function loadStatus() {
    statusError = null;
    status = null;
    projectMessage = null;
    if (!repoPath.trim()) {
      statusError = "Enter a repository path first.";
      return;
    }

    try {
      const result = await invoke("git_status", { repositoryPath: repoPath });
      status = GitStatusResponseSchema.parse(result);
    } catch (err) {
      statusError = err instanceof Error ? err.message : String(err);
    }
  }

  async function saveRepositoryAsProject() {
    if (!repoInfo?.isRepository || !repoInfo.worktreeRoot) {
      projectMessage = "Detect a repository before saving it.";
      return;
    }

    projectMessage = null;
    savingProject = true;

    try {
      const existing = projects.find((project) => project.path === repoInfo.worktreeRoot);
      const now = Date.now();
      const projectId = existing?.id ?? crypto.randomUUID();
      const pathSegments = repoInfo.worktreeRoot.split(/[/\\]/).filter(Boolean);
      const name = existing?.name ?? pathSegments[pathSegments.length - 1] ?? "Repository";
      const projectRecord: Project = {
        id: projectId,
        name,
        path: repoInfo.worktreeRoot,
        detectedLang: existing?.detectedLang ?? null,
        createdAt: existing?.createdAt ?? now,
        updatedAt: now
      };

      await upsertProject(projectRecord);
      await loadProjects();
      selectedProjectId = projectId;
      await loadRunsForProject(projectId);
      await detectRuns(projectRecord);
      projectMessage = existing ? "Project entry updated." : "Project saved.";
    } catch (err) {
      projectError = err instanceof Error ? err.message : String(err);
    } finally {
      savingProject = false;
    }
  }

  async function handleCreateTerminal(projectId: string) {
    terminalError = null;
    try {
      await terminalService.createTab(projectId);
    } catch (err) {
      terminalError = err instanceof Error ? err.message : String(err);
    }
  }
</script>

<main>
  <Shell title="Projectlib Desktop">
    {#if runToast}
      <div class="toast-container">
        <RunToast
          message={runToast.message}
          actionLabel={runToast.actionLabel}
          on:action={handleToastAction}
          on:dismiss={handleToastDismiss}
        />
      </div>
    {/if}

    {#if error}
      <p>Failed to reach backend: {error}</p>
    {:else}
      <p>{ping.message}</p>
    {/if}

    {#if workspaceError}
      <div class="workspace-banner error" role="alert">{workspaceError}</div>
    {/if}
    {#if gitPanelError}
      <div class="workspace-banner error" role="alert">{gitPanelError}</div>
    {/if}

    <section class="workspace">
      <aside class:collapsed={!showFileTree}>
        <div class="project-selector">
          <label>
            Project
            <select bind:value={selectedProjectId}>
              {#each projects as project}
                <option value={project.id}>{project.name}</option>
              {/each}
            </select>
          </label>
        </div>
        {#if fileTreeLoading}
          <p class="empty">Loading files…</p>
        {:else if fileTreeError}
          <p class="error">{fileTreeError}</p>
        {:else if selectedProject}
          <FileTree
            rootPath={selectedProject.path}
            nodes={fileTreeNodes}
            selectedPath={activeFilePath}
            on:open={(event) => openDocument(event.detail.path)}
            on:refresh={() => selectedProject && loadFileTree(selectedProject.path)}
            on:createFile={(event) => handleCreateFile(event.detail.directory)}
            on:createFolder={(event) => handleCreateFolder(event.detail.directory)}
            on:rename={(event) => handleRename(event.detail.path)}
            on:delete={(event) => handleDelete(event.detail.path)}
            on:reveal={(event) => handleReveal(event.detail.path)}
            on:terminal={(event) => handleTerminal(event.detail.directory)}
          />
        {:else}
          <p class="empty">Select a project to browse files.</p>
        {/if}
      </aside>

      <section class="editor-section">
        <EditorTabs
          tabs={documents.map((doc) => ({ path: doc.path, name: doc.name, language: doc.language, dirty: doc.dirty })) as WorkspaceEditorTab[]}
          activePath={activeFilePath}
          on:select={(event) => {
            activeFilePath = event.detail.path;
            schedulePersist();
          }}
          on:close={(event) => closeDocument(event.detail.path)}
          on:closeOthers={(event) => closeOtherDocuments(event.detail.path)}
          on:reveal={(event) => handleReveal(event.detail.path)}
        />
        <div class="editor-container">
          {#if activeDocument}
            <CodeEditor
              path={activeDocument.path}
              model={activeDocument.model}
              viewState={activeDocument.viewState}
              on:change={(event) => {
                const doc = getDocument(event.detail.path);
                if (doc) {
                  doc.dirty = event.detail.value !== doc.savedValue;
                  documents = [...documents];
                }
              }}
              on:save={(event) => {
                void saveDocument(event.detail.path);
              }}
              on:cursor={(event) => {
                const doc = getDocument(event.detail.path);
                if (doc) {
                  doc.viewState = event.detail.viewState;
                  schedulePersist();
                }
              }}
            />
          {:else}
            <div class="empty-editor">Open a file to start editing.</div>
          {/if}
        </div>
      </section>

      <aside class="git-section">
        <GitPanel
          repositoryPath={selectedProject?.path ?? null}
          projectName={selectedProject?.name ?? null}
          on:error={(event) => (gitPanelError = event.detail.message)}
        />
      </aside>
    </section>

    <section class="panel">
      <h2>Git runtime</h2>
      {#if gitInfo}
        <dl>
          <div>
            <dt>Detected git</dt>
            <dd>{gitInfo.detectedPath ?? "Unknown"}</dd>
          </div>
          <div>
            <dt>Configured override</dt>
            <dd>{gitInfo.configuredPath ?? "None"}</dd>
          </div>
          <div>
            <dt>Effective command</dt>
            <dd>{gitInfo.effectivePath ?? "Unavailable"}</dd>
          </div>
          <div>
            <dt>Uses wrapper</dt>
            <dd>{gitInfo.usesWrapper ? "Yes" : "No"}</dd>
          </div>
        </dl>
      {:else}
        <p>Detecting git command…</p>
      {/if}
    </section>

    <section class="panel">
      <h2>Repository tools</h2>
      <label>
        Repository path
        <input bind:value={repoPath} placeholder="/path/to/repository" />
      </label>
      <div class="actions">
        <button type="button" on:click={detectRepository}>Check for .git</button>
        <button type="button" on:click={loadStatus}>Load status</button>
        <button
          type="button"
          class="primary"
          on:click={saveRepositoryAsProject}
          disabled={!repoInfo?.isRepository || !repoInfo?.worktreeRoot || savingProject}
        >
          {savingProject ? "Saving…" : "Save as project"}
        </button>
      </div>
      {#if repoError}
        <p class="error">{repoError}</p>
      {/if}
      {#if repoInfo}
        <div class="result">
          <p>
            {#if repoInfo.isRepository}
              Repository detected at {repoInfo.worktreeRoot}
            {:else}
              No repository found
            {/if}
          </p>
        </div>
      {/if}
      {#if statusError}
        <p class="error">{statusError}</p>
      {/if}
      {#if status}
        <div class="result">
          <p>
            Branch:
            {#if status.branch}{status.branch}{:else}detached{/if}
            {#if status.upstream}
              (tracking {status.upstream})
            {/if}
          </p>
          <p>Ahead {status.ahead} / Behind {status.behind}</p>
          <p>{status.isClean ? "Working tree clean" : "Changes present"}</p>
        </div>
      {/if}
      {#if projectMessage}
        <p class="note">{projectMessage}</p>
      {/if}
    </section>

    <section class="panel">
      <div class="section-header">
        <h2>Projects</h2>
        {#if runControlError}
          <p class="error inline">{runControlError}</p>
        {/if}
      </div>
      {#if projects.length === 0}
        <p>No projects saved yet.</p>
      {:else}
        <div class="project-grid">
          {#each projects as project (project.id)}
            <ProjectCard
              {project}
              selected={project.id === selectedProjectId}
              state={getRunState(project.id)}
              on:select={() => (selectedProjectId = project.id)}
              on:run={() => handleProjectRun(project)}
              on:runWithArgs={() => handleRunWithArgs(project)}
              on:editConfig={() => handleEditRunConfig(project)}
              on:openTerminal={() => handleCreateTerminal(project.id)}
              on:openFolder={() => handleOpenFolder(project)}
              on:stop={() => handleStopRun(project)}
            />
          {/each}
        </div>
      {/if}
    </section>

    <section class="panel">
      <h2>Project terminals</h2>
      {#if projectError}
        <p class="error">{projectError}</p>
      {:else}
        <TerminalTabs
          {projects}
          bind:selectedProjectId
          creatingError={terminalError}
          onCreateTab={handleCreateTerminal}
          focusTabId={focusTabId}
        />
      {/if}
    </section>

    <section class="panel">
      <div class="section-header">
        <h2>Run commands</h2>
        {#if selectedProject}
          <div class="run-header-control">
            <RunButton
              projectName={selectedProject.name}
              status={getRunState(selectedProject.id).status}
              on:click={() => handleProjectRun(selectedProject)}
            />
          </div>
        {/if}
      </div>
      {#if !selectedProject}
        <p>Select a project to configure run commands.</p>
      {:else}
        <div class="run-actions">
          <button type="button" on:click={addRunConfig}>Add run command</button>
          <button type="button" on:click={() => detectRuns()} disabled={detectingRuns}>
            {detectingRuns ? "Detecting…" : "Detect defaults"}
          </button>
        </div>
        {#if runError}
          <p class="error">{runError}</p>
        {/if}
        {#if runMessage}
          <p class="note">{runMessage}</p>
        {/if}
        {#if runViews.length === 0}
          <p>No run commands configured yet.</p>
        {:else}
          {#each runViews as { run, draft } (run.id)}
            <div class="run-card">
              <div class="field-grid">
                <label>
                  Command
                  <input
                    value={draft.command}
                    placeholder="pnpm"
                    on:input={(event) =>
                      updateDraftField(run.id, "command", event.currentTarget.value)
                    }
                  />
                </label>
                <label>
                  Working directory
                  <input
                    value={draft.cwd}
                    placeholder={selectedProject?.path ?? ""}
                    on:input={(event) =>
                      updateDraftField(run.id, "cwd", event.currentTarget.value)
                    }
                  />
                </label>
              </div>
              <label>
                Arguments (one per line)
                <textarea
                  rows="3"
                  value={draft.argsText}
                  placeholder={"dev"}
                  on:input={(event) =>
                    updateDraftField(run.id, "argsText", event.currentTarget.value)
                  }
                ></textarea>
              </label>
              <label>
                Environment (KEY=VALUE per line)
                <textarea
                  rows="3"
                  value={draft.envText}
                  placeholder={"PORT=3000"}
                  on:input={(event) =>
                    updateDraftField(run.id, "envText", event.currentTarget.value)
                  }
                ></textarea>
              </label>
              <div class="run-footer">
                <div class="run-meta">
                  <span>Updated {new Date(run.updatedAt).toLocaleString()}</span>
                  {#if run.lastExitCode !== null}
                    <span>Last exit code {run.lastExitCode}</span>
                  {/if}
                  {#if isLocalOnly(run.id)}
                    <span class="unsaved">Unsaved</span>
                  {/if}
                </div>
                <div class="run-buttons">
                  <button
                    type="button"
                    on:click={() => removeRun(run.id)}
                    disabled={Boolean(deletingRuns[run.id])}
                  >
                    {deletingRuns[run.id] ? "Removing…" : "Remove"}
                  </button>
                  <button
                    type="button"
                    class="primary"
                    on:click={() => saveRun(run.id)}
                    disabled={!isDirty(run.id) || Boolean(savingRuns[run.id])}
                  >
                    {savingRuns[run.id] ? "Saving…" : "Save"}
                  </button>
                </div>
              </div>
            </div>
          {/each}
        {/if}
      {/if}
    </section>

    {#if quickOpenVisible}
      <div class="quick-open" role="dialog" aria-modal="true">
        <input
          bind:this={quickOpenInput}
          bind:value={quickOpenQuery}
          placeholder="Search files..."
          on:keydown={handleQuickOpenKey}
        />
        <ul role="listbox" aria-label="Matching files">
          {#if quickOpenMatches.length === 0}
            <li class="empty">No matches</li>
          {:else}
            {#each quickOpenMatches as file, index}
              <li>
                <button
                  type="button"
                  class:selected={index === quickOpenIndex}
                  on:mouseenter={() => (quickOpenIndex = index)}
                  on:focus={() => (quickOpenIndex = index)}
                  on:mousedown={(event) => event.preventDefault()}
                  on:click={() => {
                    const match = quickOpenMatches[index];
                    if (match) {
                      void openDocument(match.path);
                    }
                    closeQuickOpen();
                  }}
                  role="option"
                  aria-selected={index === quickOpenIndex}
                >
                  <span class="name">{file.name}</span>
                  <span class="path">{file.path}</span>
                </button>
              </li>
            {/each}
          {/if}
        </ul>
      </div>
    {/if}

    {#if showShortcutOverlay}
      <div class="shortcut-overlay" role="dialog" aria-modal="true">
        <div class="overlay-card">
          <h3>Keyboard Shortcuts</h3>
          <ul>
            <li><span>Ctrl/Cmd + P</span><span>Quick open</span></li>
            <li><span>Ctrl/Cmd + B</span><span>Toggle file tree</span></li>
            <li><span>Ctrl/Cmd + K, Ctrl/Cmd + S</span><span>Shortcut overview</span></li>
            <li><span>Ctrl/Cmd + S</span><span>Save current file</span></li>
          </ul>
          <button type="button" on:click={() => (showShortcutOverlay = false)}>Close</button>
        </div>
      </div>
    {/if}
  </Shell>
</main>

{#if runModal}
  <RunConfigModal
    projectName={runModal.project.name}
    initialCommand={runModal.initialCommand}
    initialArgs={runModal.initialArgs}
    initialEnv={runModal.initialEnv}
    initialCwd={runModal.initialCwd}
    remember={runModal.remember}
    mode={runModal.mode === "edit" ? "edit" : "run"}
    on:submit={handleRunModalSubmit}
    on:cancel={handleRunModalCancel}
  />
{/if}

<style>
  p {
    margin: 0;
    font-size: 1.1rem;
  }

  .panel {
    display: flex;
    flex-direction: column;
    gap: 0.9rem;
    padding: 1.25rem;
    border: 1px solid color-mix(in srgb, currentColor 25%, transparent);
    border-radius: 0.75rem;
    background: color-mix(in srgb, var(--terminal-bg) 35%, transparent);
    backdrop-filter: blur(6px);
  }

  .workspace {
    display: grid;
    grid-template-columns: 280px 1fr 320px;
    gap: 1rem;
    margin-bottom: 2rem;
  }

  .workspace aside {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    border: 1px solid color-mix(in srgb, currentColor 18%, transparent);
    border-radius: 0.75rem;
    background: color-mix(in srgb, var(--terminal-bg) 45%, transparent);
    padding: 0.75rem;
  }

  .workspace aside.collapsed {
    display: none;
  }

  .project-selector label {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .project-selector select {
    padding: 0.45rem 0.6rem;
    border-radius: 0.5rem;
    border: 1px solid color-mix(in srgb, currentColor 20%, transparent);
    background: color-mix(in srgb, var(--terminal-bg) 75%, transparent);
    color: inherit;
  }

  .editor-section {
    display: flex;
    flex-direction: column;
    border: 1px solid color-mix(in srgb, currentColor 18%, transparent);
    border-radius: 0.75rem;
    background: color-mix(in srgb, var(--terminal-bg) 45%, transparent);
    padding: 0.75rem;
    gap: 0.75rem;
    min-height: 520px;
  }

  .editor-container {
    flex: 1;
    min-height: 420px;
    border-radius: 0.65rem;
    overflow: hidden;
  }

  .editor-container :global(.editor) {
    height: 100%;
  }

  .empty-editor {
    display: grid;
    place-items: center;
    height: 100%;
    border: 2px dashed color-mix(in srgb, currentColor 20%, transparent);
    border-radius: 0.65rem;
    color: rgba(255, 255, 255, 0.55);
  }

  .git-section {
    border: 1px solid color-mix(in srgb, currentColor 18%, transparent);
    border-radius: 0.75rem;
    background: color-mix(in srgb, var(--terminal-bg) 45%, transparent);
    padding: 0.75rem;
    max-height: 100%;
    overflow: auto;
  }

  .workspace-banner {
    margin: 0.5rem 0;
    padding: 0.6rem 0.75rem;
    border-radius: 0.6rem;
    background: rgba(220, 38, 38, 0.18);
    border: 1px solid rgba(248, 113, 113, 0.35);
    color: #fecaca;
  }

  .workspace-banner.error {
    color: #fecaca;
  }

  .quick-open {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.45);
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding-top: 12vh;
    z-index: 3000;
  }

  .quick-open input {
    width: min(32rem, 90vw);
    padding: 0.6rem 0.75rem;
    border-radius: 0.75rem 0.75rem 0 0;
    border: none;
    outline: none;
    background: rgba(17, 24, 39, 0.95);
    color: inherit;
    font-size: 1rem;
  }

  .quick-open ul {
    width: min(32rem, 90vw);
    max-height: 18rem;
    overflow: auto;
    margin: 0;
    padding: 0;
    list-style: none;
    background: rgba(17, 24, 39, 0.92);
    border-radius: 0 0 0.75rem 0.75rem;
    border: 1px solid rgba(255, 255, 255, 0.1);
  }

  .quick-open li {
    margin: 0;
  }

  .quick-open li button {
    width: 100%;
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    padding: 0.5rem 0.75rem;
    background: transparent;
    border: none;
    color: inherit;
    text-align: left;
    cursor: pointer;
  }

  .quick-open li button.selected {
    background: rgba(59, 130, 246, 0.18);
  }

  .quick-open li button .path {
    opacity: 0.6;
    font-size: 0.85rem;
  }

  .quick-open li.empty {
    text-align: center;
    opacity: 0.6;
    padding: 0.5rem 0.75rem;
  }

  .shortcut-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.45);
    display: grid;
    place-items: center;
    z-index: 3001;
  }

  .shortcut-overlay .overlay-card {
    background: rgba(15, 23, 42, 0.92);
    border-radius: 0.9rem;
    padding: 1.5rem;
    width: min(32rem, 90vw);
    border: 1px solid rgba(255, 255, 255, 0.1);
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .shortcut-overlay ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .shortcut-overlay li {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
  }

  dl {
    display: grid;
    grid-template-columns: minmax(0, 12rem) 1fr;
    gap: 0.5rem 1rem;
    margin: 0;
  }

  dt {
    font-weight: 600;
  }

  dd {
    margin: 0;
  }

  label {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  input,
  textarea {
    padding: 0.55rem 0.75rem;
    border: 1px solid color-mix(in srgb, currentColor 25%, transparent);
    border-radius: 0.5rem;
    font: inherit;
    background: color-mix(in srgb, var(--terminal-bg) 80%, transparent);
  }

  textarea {
    resize: vertical;
    min-height: 5.5rem;
  }

  .actions,
  .run-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
  }

  button {
    padding: 0.5rem 1rem;
    border-radius: 0.6rem;
    border: 1px solid color-mix(in srgb, currentColor 25%, transparent);
    background: transparent;
    cursor: pointer;
    font: inherit;
    transition: background 0.2s ease, border-color 0.2s ease, color 0.2s ease;
  }

  button:hover:not(:disabled) {
    background: color-mix(in srgb, var(--terminal-bg) 85%, transparent);
    border-color: color-mix(in srgb, currentColor 35%, transparent);
  }

  button:disabled {
    cursor: not-allowed;
    opacity: 0.6;
  }

  button.primary {
    background: color-mix(in srgb, var(--terminal-cursor) 80%, transparent);
    color: #fff;
    border-color: color-mix(in srgb, var(--terminal-cursor) 60%, transparent);
  }

  button.primary:hover:not(:disabled) {
    background: color-mix(in srgb, var(--terminal-cursor) 90%, transparent);
  }

  .run-card {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    padding: 1rem;
    border: 1px solid color-mix(in srgb, currentColor 18%, transparent);
    border-radius: 0.65rem;
    background: color-mix(in srgb, var(--terminal-bg) 55%, transparent);
  }

  .field-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(12rem, 1fr));
    gap: 0.75rem;
  }

  .run-footer {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
  }

  .run-meta {
    display: flex;
    flex-wrap: wrap;
    gap: 0.75rem;
    font-size: 0.95rem;
  }

  .run-buttons {
    display: flex;
    gap: 0.5rem;
  }

  .toast-container {
    position: fixed;
    top: 1rem;
    right: 1rem;
    z-index: 1100;
  }

  .project-grid {
    display: grid;
    gap: 1rem;
    grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
  }

  .section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
  }

  .section-header .error.inline {
    margin: 0;
  }

  .run-header-control {
    display: inline-flex;
  }

  .unsaved {
    color: color-mix(in srgb, var(--terminal-cursor) 80%, currentColor 20%);
    font-weight: 600;
  }

  .error {
    color: #f87171;
  }

  .note {
    color: color-mix(in srgb, var(--terminal-cursor) 80%, currentColor 20%);
    font-size: 0.95rem;
  }

  .result {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }
</style>
