import { z } from "zod";

export const GitPathInfoSchema = z.object({
  detectedPath: z.string().nullable(),
  configuredPath: z.string().nullable(),
  effectivePath: z.string().nullable(),
  usesWrapper: z.boolean()
});

export type GitPathInfo = z.infer<typeof GitPathInfoSchema>;

export const GitRepositoryInfoSchema = z.object({
  isRepository: z.boolean(),
  worktreeRoot: z.string().nullable(),
  gitDir: z.string().nullable()
});

export type GitRepositoryInfo = z.infer<typeof GitRepositoryInfoSchema>;

export const GitFileChangeSchema = z.object({
  path: z.string(),
  originalPath: z.string().nullable(),
  indexStatus: z.string().nullable(),
  worktreeStatus: z.string().nullable()
});

export type GitFileChange = z.infer<typeof GitFileChangeSchema>;

export const GitStatusResponseSchema = z.object({
  branch: z.string().nullable(),
  upstream: z.string().nullable(),
  ahead: z.number(),
  behind: z.number(),
  detached: z.boolean(),
  staged: z.array(GitFileChangeSchema),
  unstaged: z.array(GitFileChangeSchema),
  conflicts: z.array(GitFileChangeSchema),
  untracked: z.array(z.string()),
  isClean: z.boolean()
});

export type GitStatusResponse = z.infer<typeof GitStatusResponseSchema>;

export const GitLogEntrySchema = z.object({
  commit: z.string(),
  refs: z.array(z.string()),
  summary: z.string()
});

export type GitLogEntry = z.infer<typeof GitLogEntrySchema>;

export const GitLogResponseSchema = z.object({
  entries: z.array(GitLogEntrySchema)
});

export type GitLogResponse = z.infer<typeof GitLogResponseSchema>;

export const GitGraphEntrySchema = z.object({
  commit: z.string(),
  parents: z.array(z.string()),
  author: z.string(),
  date: z.string(),
  subject: z.string()
});

export type GitGraphEntry = z.infer<typeof GitGraphEntrySchema>;

export const GitGraphResponseSchema = z.object({
  entries: z.array(GitGraphEntrySchema)
});

export type GitGraphResponse = z.infer<typeof GitGraphResponseSchema>;

export const GitBranchesResponseSchema = z.object({
  current: z.string().nullable(),
  local: z.array(z.string()),
  remote: z.array(z.string())
});

export type GitBranchesResponse = z.infer<typeof GitBranchesResponseSchema>;

export const GitSwitchResponseSchema = z.object({
  branch: z.string()
});

export type GitSwitchResponse = z.infer<typeof GitSwitchResponseSchema>;

export const GitStashEntrySchema = z.object({
  name: z.string(),
  hash: z.string(),
  relativeTime: z.string(),
  message: z.string()
});

export type GitStashEntry = z.infer<typeof GitStashEntrySchema>;

export const GitStashListSchema = z.object({
  entries: z.array(GitStashEntrySchema)
});

export type GitStashList = z.infer<typeof GitStashListSchema>;

export const GitRemoteSchema = z.object({
  name: z.string(),
  url: z.string(),
  kind: z.string()
});

export type GitRemote = z.infer<typeof GitRemoteSchema>;

export const GitRemoteListSchema = z.object({
  remotes: z.array(GitRemoteSchema)
});

export type GitRemoteList = z.infer<typeof GitRemoteListSchema>;

export const GitCommandOutcomeSchema = z.object({
  exitCode: z.number().nullable(),
  success: z.boolean(),
  stdout: z.string(),
  stderr: z.string()
});

export type GitCommandOutcome = z.infer<typeof GitCommandOutcomeSchema>;

export const GitCommitFileChangeSchema = z.object({
  status: z.string(),
  path: z.string()
});

export type GitCommitFileChange = z.infer<typeof GitCommitFileChangeSchema>;

export const GitCommitDetailsSchema = z.object({
  commit: z.string(),
  author: z.string(),
  date: z.string(),
  message: z.string(),
  files: z.array(GitCommitFileChangeSchema)
});

export type GitCommitDetails = z.infer<typeof GitCommitDetailsSchema>;

export const GitCommandHandleSchema = z.object({
  commandId: z.string()
});

export type GitCommandHandle = z.infer<typeof GitCommandHandleSchema>;

export const GitStreamEventSchema = z.object({
  commandId: z.string(),
  kind: z.enum(["stdout", "stderr", "completed", "error"]),
  data: z.string().nullable(),
  exitCode: z.number().nullable(),
  success: z.boolean().nullable()
});

export type GitStreamEvent = z.infer<typeof GitStreamEventSchema>;
