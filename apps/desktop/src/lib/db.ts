import Database from '@tauri-apps/plugin-sql';
import { safeExec, logOk } from './logging';

export async function initDb(dbPath: string) {
  return safeExec('db:init', async () => {
    const db = await Database.load(`sqlite:${dbPath}`);
    await db.execute('PRAGMA foreign_keys = ON');
    await db.execute(`
      CREATE TABLE IF NOT EXISTS projects(
        id TEXT PRIMARY KEY, name TEXT, path TEXT UNIQUE, detected_lang TEXT,
        created_at INTEGER, updated_at INTEGER
      )
    `);
    await logOk('db:initialized', { path: dbPath });
    return db;
  }, { path: dbPath });
}

export async function applyMigration_001(db: any) {
  return safeExec('db:migrate:001', async () => {
    await db.execute(`CREATE TABLE IF NOT EXISTS runs(
      id TEXT PRIMARY KEY, project_id TEXT, command TEXT, args TEXT, env JSON, cwd TEXT,
      last_exit_code INTEGER, updated_at INTEGER
    )`);
  });
}
