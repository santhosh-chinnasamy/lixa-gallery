# Contributing to Lixa Gallery

🎉 First off, thank you for taking the time to contribute to **Lixa Gallery** – your support means a lot!

Lixa Gallery is an open-source desktop app built with **Tauri** and **SvelteKit** to manage and view image galleries. We welcome contributions of all kinds — bug reports, feature requests, code improvements, and documentation help.

---

## 🚀 Getting Started

### Prerequisites

#### Standard (Manual)
- [Node.js](https://nodejs.org/) (v20+)
- [Rust](https://www.rust-lang.org/tools/install)
- [Tauri CLI](https://tauri.app/start/prerequisites/)
- [yarn](https://yarnpkg.com/)

#### Nix (NixOS / Linux)
If you use Nix or NixOS, a `flake.nix` is provided. You don't need to install the above manually.
- [Nix](https://nixos.org/download.html) with Flakes enabled.

---

## 🚀 Getting Started

### Standard Workflow
```bash
git clone https://github.com/santhosh-chinnasamy/lixa-gallery.git
cd lixa-gallery
yarn
yarn tauri dev
```

### Nix Workflow
```bash
git clone https://github.com/santhosh-chinnasamy/lixa-gallery.git
cd lixa-gallery
# Enter development shell (automatically installs all tools and libraries)
nix develop
yarn install
yarn tauri dev
```

## Table of Contents

- [How Can I Contribute?](#how-can-i-contribute)
  - [Reporting Bugs](#reporting-bugs)
  - [Suggesting Features](#suggesting-features)
  - [Improving Documentation](#improving-documentation)
  - [Contributing Code](#contributing-code)
- [Setting Up the Development Environment](#setting-up-the-development-environment)
- [Testing](#testing)
- [How to create Migrations](#how-to-create-migrations)
- [Creating a Pull Request](#creating-a-pull-request)
- [License](#license)

## How Can I Contribute?

### Reporting Bugs

If you find a bug, please report it by creating a new issue. Be sure to include:

- A clear description of the bug.
- Steps to reproduce the bug.
- Expected and actual behavior.
- Any relevant screenshots or error messages.
- Your environment (OS, Rust version, etc.).

### Suggesting Features

We’re open to new ideas! To suggest a feature:

- Check if the feature is already being discussed in the [issues](https://github.com/santhosh-chinnasamylixa-gallery/issues).
- If not, open a new issue with a clear and detailed description of the feature.
- Discuss how the feature would benefit the project and its users.

### Improving Documentation

Contributions to the documentation are always welcome. If you spot an area for improvement, feel free to:

- Suggest changes by creating an issue.
- Submit a pull request with your changes.

### Contributing Code

#### Before You Start

- Check the [open issues](https://github.com/santhosh-chinnasamylixa-gallery/issues) to see if someone else is already working on a similar fix or feature.
- If not, create a new issue or comment on an existing one to express your interest in working on it.
- Discuss the changes you want to make with the maintainers by opening an issue or commenting on the existing one.

#### Guidelines

- Keep your code clean and readable.
- Follow Rust and JS/TS best practices and ensure the code compiles without errors.
- Make sure to include tests for your changes, when applicable.

## Setting Up the Development Environment

**Clone the repository:**

```bash
git clone https://github.com/santhosh-chinnasamy/lixa-gallery.git
cd lixa-gallery
```

### NixOS / Flake Users

The project includes a `flake.nix` that provides both a development shell and a production-wrapped package.

#### 1. Development
Use `nix develop` to enter a shell with all system dependencies (GTK, WebKit, Rust, Node, etc.) pre-configured.
```bash
nix develop
yarn install
yarn tauri dev
```

#### 2. Building & Running (Option B)
You can build a self-contained, wrapped version of the app using Nix.
```bash
nix run .
```

> **Note on Dependency Hashes**: The `flake.nix` uses a fixed-output derivation for the frontend. If you update `package.json` or `yarn.lock`, the build will fail with a hash mismatch. To fix:
> 1. Copy the "Actual" hash from the error message.
> 2. Update the `outputHash` in `flake.nix`.


## Creating a Pull Request

- Fork the repository.
- Create a new branch:
  ```bash
  git checkout -b your-feature-branch
  ```
- Make your changes, ensuring that your code is properly tested and formatted.
- Update the README.md with details of changes to the interface, if applicable.
- Update the CHANGELOG.md with notes on your changes.
- Commit your changes:
  ```bash
  git commit -m "Add description of the changes"
  ```
- Push the branch:
  ```bash
  git push origin your-feature-branch
  ```
- Open a pull request from your fork’s branch to the `main` branch of the Lixa Gallery repository.
- Describe your changes clearly in the pull request and link any related issues.
- The PR will be merged once you have the sign-off of at least one other developer/maintainer.

## Testing

We use a multi-layered testing strategy to ensure the architectural correctness and reliability of the application.

### Running All Tests

To run all Rust tests across the entire workspace:

```bash
cd src-tauri
cargo test --workspace
```

### Running Crate-Specific Tests

- **Domain Logic**: `cargo test -p gallery-core`
- **Application Services**: `cargo test -p services`
- **Infrastructure**: `cargo test -p infra`

### Test Layers

1. **`gallery-core` (Unit Tests)**: Pure tests for domain models and data structures.
2. **`services` (Mocked Tests)**: Business logic tests using "fake" implementations of repositories and file systems to avoid side effects.
3. **`infra` (Integration Tests)**: Real-world validation of SQLite repositories (using in-memory DB) and Local File System operations (using temporary directories).

---

## How to create Migrations

```bash
cd src-tauri && sqlx migrate add <migration file name> && cd -
```

## License

By contributing to Lixa Gallery, you agree that your contributions will be licensed under the [MIT License](./LICENSE).
