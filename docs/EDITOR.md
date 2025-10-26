# Integrated Editor & Git Panel

The desktop app bundles a Monaco-based editor, a project file browser, and a Git workbench. This page highlights the primary workflows and shortcuts.

## Keyboard shortcuts

- **Ctrl/Cmd + P** – open the quick search palette. Use ↑/↓ to move through matches; press Enter or click to open a file. The list reacts to mouse hover and closes automatically after selection.
- **Ctrl/Cmd + B** – toggle the file tree visibility.
- **Ctrl/Cmd + S** – save the active document (works even when the editor is not focused).
- **Ctrl/Cmd + K, Ctrl/Cmd + S** – open the shortcut reference overlay.
- **Esc** – close the quick-open dialog or the shortcut overlay.

## File tree and tabs

- The tree hides common build folders (`.git`, `node_modules`, `target`, etc.) and refreshes automatically when files change on disk.
- Right-click any node to create files/folders, rename, delete, reveal the item in the OS file manager, or open a terminal at that location.
- Tabs display an unsaved indicator. Closing a dirty tab or closing other tabs now prompts for confirmation to prevent data loss. The editor state (open files, cursor/scroll positions, active tab) is persisted in the app database.

## Editing experience

- Each file opens in its own Monaco model with language-aware syntax highlighting. TypeScript/JavaScript/JSON/CSS/HTML benefit from Monaco IntelliSense out of the box.
- Cursor position and scroll offsets are restored when reopening files.
- External edits trigger a safe reload (only non-dirty buffers refresh) and the file tree stays in sync.

## Git panel

- **Branch selector** – switch between local branches. Create new branches or delete non-current ones with the adjacent actions. Remote branches appear in their own list with “Track” buttons to create a local tracking branch.
- **Status** – stage/unstage individual files or whole groups (`Stage All`, `Unstage All`). Conflict entries are highlighted.
- **Commit** – enter a message and commit staged changes. The panel resets after a successful commit.
- **History graph** – renders up to the latest 200 commits with parallel lanes for diverging branches. Click a commit to view author, message, and file changes; close the detail card with the dedicated button.
- **Remote operations** – run Fetch, Pull, or Push. Streaming output is appended to the log (accessible via the collapsible panel) and summaries include the exit code and success/failure badge.

All file-system access honors the Tauri security scope, and Git commands reuse the existing shell plugin whitelist.
