export type Migration = {
  version: number;
  description: string;
  sql: string;
};

export const migrations: Migration[] = [];

export const loadMigrations = () => migrations;
