# Getting Started

## Prerequisites

- [Rust toolchain](https://rustup.rs/) with the latest stable compiler and Cargo.
- [Node.js 18.20.8](https://nodejs.org/) with [pnpm](https://pnpm.io/) installed globally.

## Installation

```bash
pnpm install
```

## Running the Frontend

To develop the Svelte/Vite frontend in a browser, run the Vite dev server:

```bash
pnpm --filter apps/desktop dev
```

## Running the Backend

To develop the Rust/Tauri backend alongside the desktop shell, run the Tauri
development command (this will also build the frontend):

```bash
pnpm --filter apps/desktop tauri:dev
```
