export class MissingRunConfigurationError extends Error {
  readonly projectId: string;

  constructor(projectId: string, message = "Run configuration required") {
    super(message);
    this.name = "MissingRunConfigurationError";
    this.projectId = projectId;
  }
}

export class RunAlreadyInProgressError extends Error {
  readonly projectId: string;

  constructor(projectId: string, message = "Run already in progress") {
    super(message);
    this.name = "RunAlreadyInProgressError";
    this.projectId = projectId;
  }
}
