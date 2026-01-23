# AGENTS.md - Project Context & Guidelines

This document provides context, technical details, and guidelines for AI assistants (like Gemini, Copilot, Claude, etc.) working on the `lixa-gallery` project.

## Project Overview

`lixa-gallery` is a desktop application built with **Tauri 2** and **Svelte 5** designed for managing, favoriting, and exporting photos. It allows users to browse a folder, select images as favorites, and export them.

## Tech Stack

- **Framework**: [Svelte 5](https://svelte.dev/) (using Runes)
- **Desktop Runtime**: [Tauri 2](https://tauri.app/)
- **Build Tool**: [Vite](https://vitejs.dev/)
- **Styling**: [Tailwind CSS 4](https://tailwindcss.com/)
- **Language**: TypeScript (Frontend), Rust (Backend)
- **UI Components**: [Bits UI](https://bits-ui.com/), [Lucide Svelte](https://lucide.dev/guide/svelte)
- **Package Manager**: Yarn (modern)

## Project Structure

- `src/`: Svelte frontend source code.
- `src-tauri/`: Rust backend and Tauri configuration.
- `assets/`: UI assets, screenshots, and demo media.
- `static/`: Static files served by the web server.
- `components.json`: Configuration for UI components (likely shadcn/ui or similar).

## Development Workflows

- **Development**: `npm run dev` (starts Vite) or `npm run tauri dev` (starts Tauri dev environment).
- **Build**: `npm run build` (builds the frontend and Rust binary).
- **Linting & Formatting**: 
    - `npm run lint`: Checks formatting.
    - `npm run lint:fix`: Fixes formatting for both TS/Svelte and Rust.
    - `npm run format:rs`: Formats Rust code.
    - `npm run format:ts`: Formats TS and Svelte code.

## Guidelines for AI Contributions

### Svelte 5 (Runes)
- **ALWAYS** use Svelte 5 Runes (`$state`, `$derived`, `$props`, `$effect`) for reactivity. Avoid legacy Svelte 4 syntax unless modifying legacy files.
- Use `Snippet` for reusable UI fragments within components.

### Tauri 2
- Use the Tauri 2 API for filesystem access, dialogs, and window management.
- Backend logic should be implemented in Rust within `src-tauri/src/main.rs` (or modularized) and exposed via `#[tauri::command]`.

### Styling
- Use Tailwind CSS classes for styling.
- Follow the existing design system for consistency.

### File Operations
- Ensure all file operations are handled safely using Tauri's plugins (e.g., `plugin-fs`, `plugin-dialog`).
