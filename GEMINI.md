# Project Overview: lixa-gallery

`lixa-gallery` is a high-performance photo gallery application built with **SvelteKit** and **Tauri v2**. It uses a **Rust** backend for heavy lifting (image processing, database operations) and a modern **Svelte 5** frontend for a smooth user experience.

## Development Mandates

### Frontend (SvelteKit + Svelte 5)

- **Svelte 5 Runes:** ALWAYS use Svelte 5 Runes (`$state`, `$derived`, `$effect`, `$props()`). DO NOT use legacy Svelte 4 reactivity (e.g., `let count = 0; $: double = count * 2;`).
- **SSG (Static Site Generation):** The project is configured with `@sveltejs/adapter-static` and `fallback: "index.html"`. Do not attempt to use SSR or server-side SvelteKit features.
- **Styling:** Use **Tailwind CSS v4**. Avoid inline styles. Prefer the UI components in `src/lib/components/ui` (Radix-like primitives via `bits-ui`).
- **Icons:** Use `@lucide/svelte` for icons.
- **TypeScript:** Strict typing is required. Use the `@/` alias for `src/` imports.

### Backend (Rust Workspace)

The backend follows a clean architecture pattern within a Cargo workspace:

- **Workspace Crates:**
  - `gallery-core`: Domain models, traits, and core result/error types.
  - `infra`: Infrastructure implementations (SQLite/sqlx, `image` crate for processing, local FS).
  - `services`: Business logic and use cases, coordinating core and infra.
  - `src-tauri`: The main Tauri application crate, handling commands, events, and app state.

- **Coding Standards:**
  - **Error Handling:** Use `thiserror` for library crates (`gallery-core`, `infra`, `services`) to define domain errors. Use `anyhow` ONLY in the `src-tauri` app crate for top-level command orchestration.
  - **Async:** Use `tokio` for async execution.
  - **Interfaces:** Use `async-trait` for repository and service traits defined in `gallery-core`.
  - **Database:** Use `sqlx` with SQLite. Migrations are located in `src-tauri/infra/migrations`.

### Cross-Cutting Concerns

- **Tauri Commands:** New features should follow the pattern: UI Action -> Tauri Command (in `src-tauri/src/tauri_api/commands.rs`) -> Service Call -> Infrastructure/Database update.
- **Events:** Use the event system in `src-tauri/src/tauri_api/events.rs` to communicate background task progress (like image indexing) to the frontend.
- **Performance & Loading:**
  - **Progressive Loading:** Prefer "Lazy" loading for large datasets. Metadata should be returned immediately while thumbnails are generated in the background.
  - **Custom Protocols:** Use `lixa-thumbnail://localhost/{path}` for on-demand, just-in-time thumbnail generation via the custom Rust protocol handler.
  - **Scaling:** ALWAYS use `DynamicImage::thumbnail()` instead of `resize()` for generating previews to minimize CPU/Memory overhead.
  - **Butter Smooth Indexing:** Background indexing must be strictly sequential and include a micro-sleep (e.g., 50ms) between images. This ensures the CPU is never saturated and the UI remains perfectly responsive ("butter smooth") even on older hardware.
  - **Concurrency:** Limit foreground (Sync) processing to a maximum of 2 concurrent threads to maintain system stability.
