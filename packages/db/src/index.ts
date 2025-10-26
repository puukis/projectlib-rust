import { invoke } from "@tauri-apps/api/core";
import Database from "@tauri-apps/plugin-sql";
import { z } from "zod";

const projectRowSchema = z.object({
  id: z.string(),
  name: z.string(),
  path: z.string(),
  detected_lang: z.string().nullable(),
  created_at: z.number().int(),
  updated_at: z.number().int(),
});

const projectSchema = z.object({
  id: z.string(),
  name: z.string(),
  path: z.string(),
  detectedLang: z.string().nullable(),
  createdAt: z.number().int(),
  updatedAt: z.number().int(),
});

const runRowSchema = z.object({
  id: z.string(),
  project_id: z.string(),
  command: z.string(),
  args: z.string().nullable(),
  env: z.string().nullable(),
  cwd: z.string().nullable(),
  last_exit_code: z.number().int().nullable(),
  updated_at: z.number().int(),
});

const runSchema = z.object({
  id: z.string(),
  projectId: z.string(),
  command: z.string(),
  args: z.array(z.string()).default([]),
  env: z.record(z.string(), z.string()).default({}),
  cwd: z.string().nullable(),
  lastExitCode: z.number().int().nullable(),
  updatedAt: z.number().int(),
});

const runStatusRowSchema = z.object({
  project_id: z.string(),
  status: z.string(),
  last_run_id: z.string().nullable(),
  last_command: z.string().nullable(),
  last_args: z.string().nullable(),
  last_env: z.string().nullable(),
  last_cwd: z.string().nullable(),
  last_exit_code: z.number().int().nullable(),
  started_at: z.number().int().nullable(),
  finished_at: z.number().int().nullable(),
  updated_at: z.number().int(),
});

const runStatusSchema = z.object({
  projectId: z.string(),
  status: z.enum(["idle", "starting", "running", "succeeded", "failed", "stopped"]),
  lastRunId: z.string().nullable(),
  lastCommand: z.string().nullable(),
  lastArgs: z.array(z.string()).default([]),
  lastEnv: z.record(z.string(), z.string()).default({}),
  lastCwd: z.string().nullable(),
  lastExitCode: z.number().int().nullable(),
  startedAt: z.number().int().nullable(),
  finishedAt: z.number().int().nullable(),
  updatedAt: z.number().int(),
});

const stringArraySchema = z.array(z.string());
const stringRecordSchema = z.record(z.string(), z.string());

const terminalSchema = z.object({
  id: z.string(),
  projectId: z.string(),
  shell: z.string(),
  createdAt: z.number().int(),
});

const gitRemoteRowSchema = z.object({
  id: z.string(),
  project_id: z.string(),
  name: z.string(),
  url: z.string(),
});

const gitRemoteSchema = z.object({
  id: z.string(),
  projectId: z.string(),
  name: z.string(),
  url: z.string(),
});

const settingsSchema = z.object({
  id: z.string().default("global"),
  theme: z.string().nullable(),
  telemetryEnabled: z.boolean().nullable(),
  gitPath: z.string().nullable(),
});

const editorFileStateSchema = z.object({
  path: z.string(),
  language: z.string().nullable(),
});

const editorStateRowSchema = z.object({
  id: z.string(),
  open_files: z.string().nullable(),
  active_file: z.string().nullable(),
  view_state: z.string().nullable(),
  updated_at: z.number().int(),
});

const editorStateSchema = z.object({
  id: z.string().default("global"),
  openFiles: z.array(editorFileStateSchema),
  activeFile: z.string().nullable(),
  viewState: z.record(z.string(), z.unknown()).default({}),
  updatedAt: z.number().int(),
});

type ProjectRow = z.infer<typeof projectRowSchema>;
export type Project = z.infer<typeof projectSchema>;
type RunRow = z.infer<typeof runRowSchema>;
export type RunConfig = z.infer<typeof runSchema>;
type RunStatusRow = z.infer<typeof runStatusRowSchema>;
export type RunStatus = z.infer<typeof runStatusSchema>;
export type Terminal = z.infer<typeof terminalSchema>;
type GitRemoteRow = z.infer<typeof gitRemoteRowSchema>;
export type GitRemote = z.infer<typeof gitRemoteSchema>;
export type Settings = z.infer<typeof settingsSchema>;
type EditorStateRow = z.infer<typeof editorStateRowSchema>;
export type EditorFileState = z.infer<typeof editorFileStateSchema>;
export type EditorState = z.infer<typeof editorStateSchema>;

let databasePromise: Promise<Database> | null = null;

async function getDatabase(): Promise<Database> {
  if (!databasePromise) {
    databasePromise = (async () => {
      const url = await invoke<string>("resolve_database_url");
      const instance = await Database.load(url);
      await instance.execute("PRAGMA foreign_keys = ON");
      return instance;
    })();
  }

  return databasePromise;
}

function fromProjectRow(row: ProjectRow): Project {
  const parsed = projectRowSchema.parse(row);
  return projectSchema.parse({
    id: parsed.id,
    name: parsed.name,
    path: parsed.path,
    detectedLang: parsed.detected_lang,
    createdAt: parsed.created_at,
    updatedAt: parsed.updated_at,
  });
}

function fromRunRow(row: RunRow): RunConfig {
  const parsed = runRowSchema.parse(row);
  const args = parsed.args
    ? stringArraySchema.parse(JSON.parse(parsed.args))
    : [];
  const env = parsed.env
    ? stringRecordSchema.parse(JSON.parse(parsed.env))
    : {};

  return runSchema.parse({
    id: parsed.id,
    projectId: parsed.project_id,
    command: parsed.command,
    args,
    env,
    cwd: parsed.cwd,
    lastExitCode: parsed.last_exit_code,
    updatedAt: parsed.updated_at,
  });
}

function fromRunStatusRow(row: RunStatusRow): RunStatus {
  const parsed = runStatusRowSchema.parse(row);
  const args = parsed.last_args
    ? stringArraySchema.parse(JSON.parse(parsed.last_args))
    : [];
  const env = parsed.last_env
    ? stringRecordSchema.parse(JSON.parse(parsed.last_env))
    : {};

  return runStatusSchema.parse({
    projectId: parsed.project_id,
    status: parsed.status as RunStatus["status"],
    lastRunId: parsed.last_run_id,
    lastCommand: parsed.last_command,
    lastArgs: args,
    lastEnv: env,
    lastCwd: parsed.last_cwd,
    lastExitCode: parsed.last_exit_code,
    startedAt: parsed.started_at,
    finishedAt: parsed.finished_at,
    updatedAt: parsed.updated_at,
  });
}

function fromGitRemoteRow(row: GitRemoteRow): GitRemote {
  const parsed = gitRemoteRowSchema.parse(row);
  return gitRemoteSchema.parse({
    id: parsed.id,
    projectId: parsed.project_id,
    name: parsed.name,
    url: parsed.url,
  });
}

function fromEditorStateRow(row: EditorStateRow): EditorState {
  const parsed = editorStateRowSchema.parse(row);
  const openFiles = parsed.open_files
    ? editorFileStateSchema.array().parse(JSON.parse(parsed.open_files))
    : [];
  const viewState = parsed.view_state
    ? z.record(z.string(), z.unknown()).parse(JSON.parse(parsed.view_state))
    : {};

  return editorStateSchema.parse({
    id: parsed.id,
    openFiles,
    activeFile: parsed.active_file,
    viewState,
    updatedAt: parsed.updated_at,
  });
}

export async function upsertProject(project: Project): Promise<void> {
  const db = await getDatabase();
  const parsed = projectSchema.parse(project);

  await db.execute(
    `INSERT INTO projects (id, name, path, detected_lang, created_at, updated_at)
     VALUES (?, ?, ?, ?, ?, ?)
     ON CONFLICT(id) DO UPDATE SET
       name = excluded.name,
       path = excluded.path,
       detected_lang = excluded.detected_lang,
       updated_at = excluded.updated_at`,
    [
      parsed.id,
      parsed.name,
      parsed.path,
      parsed.detectedLang,
      parsed.createdAt,
      parsed.updatedAt,
    ],
  );
}

export async function listProjects(): Promise<Project[]> {
  const db = await getDatabase();
  const rows = (await db.select(
    `SELECT id, name, path, detected_lang, created_at, updated_at FROM projects ORDER BY name`,
  )) as ProjectRow[];

  return rows.map(fromProjectRow);
}

export async function getProject(id: string): Promise<Project | null> {
  const db = await getDatabase();
  const rows = (await db.select(
    `SELECT id, name, path, detected_lang, created_at, updated_at FROM projects WHERE id = ? LIMIT 1`,
    [id],
  )) as ProjectRow[];

  const row = rows[0];
  return row ? fromProjectRow(row) : null;
}

export async function saveRunConfig(run: RunConfig): Promise<void> {
  const db = await getDatabase();
  const parsed = runSchema.parse(run);

  await db.execute(
    `INSERT INTO runs (id, project_id, command, args, env, cwd, last_exit_code, updated_at)
     VALUES (?, ?, ?, ?, ?, ?, ?, ?)
     ON CONFLICT(id) DO UPDATE SET
       project_id = excluded.project_id,
       command = excluded.command,
       args = excluded.args,
       env = excluded.env,
       cwd = excluded.cwd,
       last_exit_code = excluded.last_exit_code,
       updated_at = excluded.updated_at`,
    [
      parsed.id,
      parsed.projectId,
      parsed.command,
      JSON.stringify(parsed.args),
      JSON.stringify(parsed.env),
      parsed.cwd,
      parsed.lastExitCode,
      parsed.updatedAt,
    ],
  );
}

export async function listRuns(projectId: string): Promise<RunConfig[]> {
  const db = await getDatabase();
  const rows = (await db.select(
    `SELECT id, project_id, command, args, env, cwd, last_exit_code, updated_at
     FROM runs
     WHERE project_id = ?
     ORDER BY updated_at DESC`,
    [projectId],
  )) as RunRow[];

  return rows.map(fromRunRow);
}

export async function deleteRunConfig(id: string): Promise<void> {
  const db = await getDatabase();
  await db.execute(`DELETE FROM runs WHERE id = ?`, [id]);
}

export async function updateRunOutcome(
  id: string,
  lastExitCode: number | null,
  updatedAt: number,
): Promise<void> {
  const db = await getDatabase();
  await db.execute(
    `UPDATE runs SET last_exit_code = ?, updated_at = ? WHERE id = ?`,
    [lastExitCode, updatedAt, id],
  );
}

export async function listRunStatuses(): Promise<RunStatus[]> {
  const db = await getDatabase();
  const rows = (await db.select(
    `SELECT project_id, status, last_run_id, last_command, last_args, last_env, last_cwd, last_exit_code, started_at, finished_at, updated_at
     FROM run_status`,
  )) as RunStatusRow[];

  return rows.map(fromRunStatusRow);
}

export async function getRunStatus(projectId: string): Promise<RunStatus | null> {
  const db = await getDatabase();
  const rows = (await db.select(
    `SELECT project_id, status, last_run_id, last_command, last_args, last_env, last_cwd, last_exit_code, started_at, finished_at, updated_at
     FROM run_status WHERE project_id = ? LIMIT 1`,
    [projectId],
  )) as RunStatusRow[];

  const row = rows[0];
  return row ? fromRunStatusRow(row) : null;
}

export async function saveRunStatus(status: RunStatus): Promise<void> {
  const db = await getDatabase();
  const parsed = runStatusSchema.parse(status);

  await db.execute(
    `INSERT INTO run_status (project_id, status, last_run_id, last_command, last_args, last_env, last_cwd, last_exit_code, started_at, finished_at, updated_at)
     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
     ON CONFLICT(project_id) DO UPDATE SET
       status = excluded.status,
       last_run_id = excluded.last_run_id,
       last_command = excluded.last_command,
       last_args = excluded.last_args,
       last_env = excluded.last_env,
       last_cwd = excluded.last_cwd,
       last_exit_code = excluded.last_exit_code,
       started_at = excluded.started_at,
       finished_at = excluded.finished_at,
       updated_at = excluded.updated_at`,
    [
      parsed.projectId,
      parsed.status,
      parsed.lastRunId,
      parsed.lastCommand,
      JSON.stringify(parsed.lastArgs ?? []),
      JSON.stringify(parsed.lastEnv ?? {}),
      parsed.lastCwd,
      parsed.lastExitCode,
      parsed.startedAt,
      parsed.finishedAt,
      parsed.updatedAt,
    ],
  );
}

export async function saveTerminal(terminal: Terminal): Promise<void> {
  const db = await getDatabase();
  const parsed = terminalSchema.parse(terminal);

  await db.execute(
    `INSERT INTO terminals (id, project_id, shell, created_at)
     VALUES (?, ?, ?, ?)
     ON CONFLICT(id) DO UPDATE SET
       project_id = excluded.project_id,
       shell = excluded.shell,
       created_at = excluded.created_at`,
    [parsed.id, parsed.projectId, parsed.shell, parsed.createdAt],
  );
}

export async function listRemotes(projectId: string): Promise<GitRemote[]> {
  const db = await getDatabase();
  const rows = (await db.select(
    `SELECT id, project_id, name, url FROM git_remotes WHERE project_id = ? ORDER BY name`,
    [projectId],
  )) as GitRemoteRow[];

  return rows.map(fromGitRemoteRow);
}

export async function saveSettings(settings: Settings): Promise<void> {
  const db = await getDatabase();
  const parsed = settingsSchema.parse(settings);
  const telemetryValue =
    parsed.telemetryEnabled === null ? null : parsed.telemetryEnabled ? 1 : 0;

  await db.execute(
    `INSERT INTO settings (id, theme, telemetry_enabled, git_path)
     VALUES (?, ?, ?, ?)
     ON CONFLICT(id) DO UPDATE SET
       theme = excluded.theme,
       telemetry_enabled = excluded.telemetry_enabled,
       git_path = excluded.git_path`,
    [parsed.id, parsed.theme, telemetryValue, parsed.gitPath],
  );
}

export async function loadEditorState(): Promise<EditorState | null> {
  const db = await getDatabase();
  const rows = (await db.select(
    `SELECT id, open_files, active_file, view_state, updated_at FROM editor_state WHERE id = 'global' LIMIT 1`,
  )) as EditorStateRow[];

  const row = rows[0];
  return row ? fromEditorStateRow(row) : null;
}

export async function saveEditorState(state: EditorState): Promise<void> {
  const db = await getDatabase();
  const parsed = editorStateSchema.parse(state);
  await db.execute(
    `INSERT INTO editor_state (id, open_files, active_file, view_state, updated_at)
     VALUES (?, ?, ?, ?, ?)
     ON CONFLICT(id) DO UPDATE SET
       open_files = excluded.open_files,
       active_file = excluded.active_file,
       view_state = excluded.view_state,
       updated_at = excluded.updated_at`,
    [
      parsed.id,
      JSON.stringify(parsed.openFiles ?? []),
      parsed.activeFile,
      JSON.stringify(parsed.viewState ?? {}),
      parsed.updatedAt,
    ],
  );
}
