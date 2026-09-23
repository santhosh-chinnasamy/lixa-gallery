#!/usr/bin/env node

import { readFileSync, writeFileSync, existsSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { execSync } from 'node:child_process';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const rootDir = resolve(__dirname, '..');

const FILES = {
  packageJson: resolve(rootDir, 'package.json'),
  tauriConf: resolve(rootDir, 'src-tauri/tauri.conf.json'),
  cargoToml: resolve(rootDir, 'src-tauri/Cargo.toml'),
  cargoLock: resolve(rootDir, 'src-tauri/Cargo.lock'),
};

function printUsage(currentVersion) {
  console.log(`
Current version: ${currentVersion}

Usage:
  yarn bump <patch|minor|major|version> [options]

Arguments:
  patch       Bump patch version (e.g. 0.7.0 -> 0.7.1)
  minor       Bump minor version (e.g. 0.7.0 -> 0.8.0)
  major       Bump major version (e.g. 0.7.0 -> 1.0.0)
  <version>   Set explicit semver (e.g. 0.8.0, 1.0.0-rc.1)

Options:
  --dry-run       Preview changes without writing to files
  --commit, -c    Create git commit automatically
  --tag, -t       Create git tag automatically (implies --commit)
  --help, -h      Show this help message

Examples:
  yarn bump patch
  yarn bump minor --commit --tag
  yarn bump 0.8.0
`);
}

function parseSemver(version) {
  const clean = version.trim().replace(/^v/, '');
  const match = clean.match(/^(\d+)\.(\d+)\.(\d+)(?:-([0-9A-Za-z.-]+))?(?:\+([0-9A-Za-z.-]+))?$/);
  if (!match) {
    throw new Error(`Invalid semantic version: "${version}"`);
  }
  return {
    major: parseInt(match[1], 10),
    minor: parseInt(match[2], 10),
    patch: parseInt(match[3], 10),
    prerelease: match[4] || '',
    build: match[5] || '',
    raw: clean,
  };
}

function bumpVersion(currentVersion, bumpType) {
  const parsed = parseSemver(currentVersion);
  switch (bumpType) {
    case 'major':
      return `${parsed.major + 1}.0.0`;
    case 'minor':
      return `${parsed.major}.${parsed.minor + 1}.0`;
    case 'patch':
      return `${parsed.major}.${parsed.minor}.${parsed.patch + 1}`;
    default:
      return parseSemver(bumpType).raw;
  }
}

function updateCargoToml(content, newVersion) {
  let inPackage = false;
  let inWorkspacePackage = false;
  let updatedPackage = false;
  let updatedWorkspace = false;

  const lines = content.split('\n');
  const updatedLines = lines.map((line) => {
    const trimmed = line.trim();
    if (trimmed.startsWith('[')) {
      inPackage = trimmed === '[package]';
      inWorkspacePackage = trimmed === '[workspace.package]';
      return line;
    }
    if (inPackage && /^version\s*=/.test(trimmed)) {
      updatedPackage = true;
      return line.replace(/version\s*=\s*".*?"/, `version = "${newVersion}"`);
    }
    if (inWorkspacePackage && /^version\s*=/.test(trimmed)) {
      updatedWorkspace = true;
      return line.replace(/version\s*=\s*".*?"/, `version = "${newVersion}"`);
    }
    return line;
  });

  if (!updatedPackage) {
    throw new Error('Could not find [package] version in src-tauri/Cargo.toml');
  }
  if (!updatedWorkspace) {
    throw new Error('Could not find [workspace.package] version in src-tauri/Cargo.toml');
  }

  return updatedLines.join('\n');
}

function extractWorkspaceCrates(cargoTomlContent) {
  const crates = new Set();
  const pkgMatch = cargoTomlContent.match(/\[package\][\s\S]*?name\s*=\s*"([^"]+)"/);
  if (pkgMatch) {
    crates.add(pkgMatch[1]);
  }
  const membersMatch = cargoTomlContent.match(/members\s*=\s*\[([\s\S]*?)\]/);
  if (membersMatch) {
    const members = membersMatch[1]
      .split('\n')
      .map((l) => l.trim().replace(/[",]/g, ''))
      .filter(Boolean);
    for (const member of members) {
      crates.add(member);
    }
  }
  return crates;
}

function updateCargoLock(lockContent, workspaceCrates, newVersion) {
  const lines = lockContent.split('\n');
  let currentPkg = null;
  let updatedCount = 0;

  const updatedLines = lines.map((line) => {
    const trimmed = line.trim();
    if (trimmed === '[[package]]') {
      currentPkg = null;
      return line;
    }
    const nameMatch = trimmed.match(/^name\s*=\s*"([^"]+)"/);
    if (nameMatch) {
      currentPkg = nameMatch[1];
      return line;
    }
    if (currentPkg && workspaceCrates.has(currentPkg) && /^version\s*=/.test(trimmed)) {
      updatedCount++;
      return line.replace(/version\s*=\s*".*?"/, `version = "${newVersion}"`);
    }
    return line;
  });

  return { content: updatedLines.join('\n'), updatedCount };
}

function main() {
  const args = process.argv.slice(2);
  const flags = new Set(args.filter((arg) => arg.startsWith('-')));
  const positionalArgs = args.filter((arg) => !arg.startsWith('-'));

  const isDryRun = flags.has('--dry-run');
  const shouldTag = flags.has('--tag') || flags.has('-t');
  const shouldCommit = shouldTag || flags.has('--commit') || flags.has('-c');
  const isHelp = flags.has('--help') || flags.has('-h');

  // Read current version from package.json
  const pkgContent = readFileSync(FILES.packageJson, 'utf-8');
  const pkgJson = JSON.parse(pkgContent);
  const currentVersion = pkgJson.version;

  if (isHelp || positionalArgs.length === 0) {
    printUsage(currentVersion);
    process.exit(isHelp ? 0 : 1);
  }

  const bumpTarget = positionalArgs[0];
  const newVersion = bumpVersion(currentVersion, bumpTarget);

  console.log(`\n📦 Bumping version: ${currentVersion} -> ${newVersion}`);
  if (isDryRun) {
    console.log('🔍 [DRY RUN] No files will be modified.\n');
  }

  // 1. Update package.json
  pkgJson.version = newVersion;
  const newPkgContent = JSON.stringify(pkgJson, null, 2) + '\n';
  if (!isDryRun) {
    writeFileSync(FILES.packageJson, newPkgContent, 'utf-8');
  }
  console.log(`  ✓ package.json -> ${newVersion}`);

  // 2. Update src-tauri/tauri.conf.json
  const tauriContent = readFileSync(FILES.tauriConf, 'utf-8');
  const tauriJson = JSON.parse(tauriContent);
  tauriJson.version = newVersion;
  const newTauriContent = JSON.stringify(tauriJson, null, 2) + '\n';
  if (!isDryRun) {
    writeFileSync(FILES.tauriConf, newTauriContent, 'utf-8');
  }
  console.log(`  ✓ src-tauri/tauri.conf.json -> ${newVersion}`);

  // 3. Update src-tauri/Cargo.toml
  const cargoContent = readFileSync(FILES.cargoToml, 'utf-8');
  const newCargoContent = updateCargoToml(cargoContent, newVersion);
  if (!isDryRun) {
    writeFileSync(FILES.cargoToml, newCargoContent, 'utf-8');
  }
  console.log(`  ✓ src-tauri/Cargo.toml ([package] & [workspace.package]) -> ${newVersion}`);

  // 4. Update src-tauri/Cargo.lock directly (instant, safe, no build locks)
  const changedFiles = [
    'package.json',
    'src-tauri/tauri.conf.json',
    'src-tauri/Cargo.toml',
  ];

  if (existsSync(FILES.cargoLock)) {
    const lockContent = readFileSync(FILES.cargoLock, 'utf-8');
    const workspaceCrates = extractWorkspaceCrates(cargoContent);
    const { content: newLockContent, updatedCount } = updateCargoLock(lockContent, workspaceCrates, newVersion);
    if (!isDryRun) {
      writeFileSync(FILES.cargoLock, newLockContent, 'utf-8');
    }
    console.log(`  ✓ src-tauri/Cargo.lock (${updatedCount} workspace packages) -> ${newVersion}`);
    changedFiles.push('src-tauri/Cargo.lock');
  }

  // 5. Git commit and tag if requested
  if (shouldCommit && !isDryRun) {
    try {
      execSync(`git add ${changedFiles.join(' ')}`, {
        cwd: rootDir,
        stdio: ['ignore', 'inherit', 'inherit'],
      });
      execSync(`git commit -m "chore: release v${newVersion}"`, {
        cwd: rootDir,
        stdio: ['ignore', 'inherit', 'inherit'],
      });
      console.log(`\n✨ Created commit: "chore: release v${newVersion}"`);

      if (shouldTag) {
        execSync(`git tag -a "v${newVersion}" -m "Release v${newVersion}"`, {
          cwd: rootDir,
          stdio: ['ignore', 'inherit', 'inherit'],
        });
        console.log(`✨ Created tag: "v${newVersion}"`);
      }
    } catch (err) {
      console.error('Failed to git commit/tag:', err.message);
      process.exit(1);
    }
  }

  console.log('\n🎉 Successfully bumped version!');
  if (!shouldCommit) {
    console.log('\nNext steps:');
    console.log(`  git add ${changedFiles.join(' ')}`);
    console.log(`  git commit -m "chore: release v${newVersion}"`);
    console.log(`  git tag v${newVersion}`);
    console.log('  git push origin main --tags\n');
  }

  process.exit(0);
}

main();
