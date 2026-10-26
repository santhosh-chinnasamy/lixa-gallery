#!/usr/bin/env node
// Build the static site into site/dist.
//
//   node site/build.mjs
//
// Reads CHANGELOG.md, bakes the latest GitHub release into the download
// links, and writes index.html + changelog.html. If the GitHub API is
// unreachable, links fall back to /releases/latest and the page script
// tries the API again in the browser.

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseChangelog, renderChangelog } from './changelog.mjs';
import {
  FALLBACK_URL,
  LATEST_API,
  REPO,
  formatReleaseDate,
  mergeRelease,
  parseGithubRelease,
} from './js/downloads.js';

const siteDir = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(siteDir, '..');
const dist = path.join(siteDir, 'dist');

function read(name) {
  return fs.readFileSync(path.join(siteDir, name), 'utf8');
}

function safeJson(value) {
  return JSON.stringify(value).replace(/</g, '\\u003c');
}

async function fetchLatest() {
  const response = await fetch(LATEST_API, {
    headers: {
      Accept: 'application/vnd.github+json',
      'User-Agent': 'lixa-gallery-site-build',
    },
  });
  if (!response.ok) {
    throw new Error(`GitHub API ${response.status} for ${LATEST_API}`);
  }
  return parseGithubRelease(await response.json());
}

function fallbackRelease(changelogVersions) {
  const pkg = JSON.parse(fs.readFileSync(path.join(root, 'package.json'), 'utf8'));
  return {
    version: pkg.version || changelogVersions[0]?.version || '',
    date: changelogVersions[0]?.date || '',
    htmlUrl: FALLBACK_URL,
    fallbackUrl: FALLBACK_URL,
    assets: {},
  };
}

function applyTokens(html, release) {
  const assets = release.assets || {};
  const url = (key) => assets[key] || release.fallbackUrl || FALLBACK_URL;
  const map = {
    version: release.version || '',
    date: formatReleaseDate(release.date),
    'date-iso': release.date || '',
    fallback: release.fallbackUrl || FALLBACK_URL,
    'macos-arm': url('macos-arm'),
    'macos-intel': url('macos-intel'),
    'windows-exe': url('windows-exe'),
    'windows-msi': url('windows-msi'),
    'linux-deb': url('linux-deb'),
    'linux-rpm': url('linux-rpm'),
    'linux-appimage': url('linux-appimage'),
    'release-json': safeJson({
      version: release.version || '',
      date: release.date || '',
      htmlUrl: release.htmlUrl || '',
      fallbackUrl: release.fallbackUrl || FALLBACK_URL,
      assets: {
        'macos-arm': assets['macos-arm'] || '',
        'macos-intel': assets['macos-intel'] || '',
        'windows-exe': assets['windows-exe'] || '',
        'windows-msi': assets['windows-msi'] || '',
        'linux-deb': assets['linux-deb'] || '',
        'linux-rpm': assets['linux-rpm'] || '',
        'linux-appimage': assets['linux-appimage'] || '',
      },
    }),
  };
  const rendered = html.replace(/@@([a-z0-9-]+)@@/g, (match, key) => {
    if (!Object.prototype.hasOwnProperty.call(map, key)) {
      throw new Error(`Unknown token ${match}`);
    }
    return map[key];
  });
  const leftover = rendered.match(/@@[^@\s]{0,80}/);
  if (leftover) {
    throw new Error(`Unreplaced token ${leftover[0]}`);
  }
  return rendered;
}

function nav(home, current) {
  const href = (hash) => (home ? `${home}${hash}` : hash);
  const currentAttr = (id) => (current === id ? ' aria-current="page"' : '');
  return `<nav>
  <a class="brand" href="index.html"><img src="assets/logo.png" alt="" width="28" height="28">Lixa Gallery</a>
  <div class="links">
    <a href="${href('#how')}">How it works</a>
    <a href="${href('#notes')}">Features</a>
    <a href="${href('#shortcuts')}">Shortcuts</a>
    <a href="${href('#install')}">Download</a>
    <a href="changelog.html"${currentAttr('changelog')}>Changelog</a>
    <a href="https://github.com/${REPO}">GitHub</a>
  </div>
</nav>`;
}

function footer() {
  return `<footer>
  <span>Lixa Gallery · MIT · Santhosh C</span>
  <span>v<span data-release-version>@@version@@</span> · released <time data-release-date datetime="@@date-iso@@">@@date@@</time></span>
</footer>`;
}

function page({ title, description, current, home, content }) {
  return `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${title}</title>
<meta name="description" content="${description}">
<meta name="theme-color" content="#F3EFE6">
<link rel="icon" href="assets/favicon.png" type="image/png">
<link rel="stylesheet" href="css/site.css">
</head>
<body>
<div class="wrap">
<a class="skip" href="#content">Skip to content</a>
${nav(home, current)}
<main id="content">
${content}
</main>
${footer()}
</div>
<script id="release-data" type="application/json">@@release-json@@</script>
<script type="module" src="js/downloads.js"></script>
</body>
</html>
`;
}

function copyStatic() {
  fs.rmSync(dist, { recursive: true, force: true });
  fs.mkdirSync(dist, { recursive: true });
  for (const dir of ['css', 'js', 'fonts', 'assets']) {
    fs.cpSync(path.join(siteDir, dir), path.join(dist, dir), { recursive: true });
  }
}

async function main() {
  const changelogPath = path.join(root, 'CHANGELOG.md');
  const markdown = fs.readFileSync(changelogPath, 'utf8');
  const versions = parseChangelog(markdown);
  if (!versions.length) {
    throw new Error(`No releases parsed from ${changelogPath}`);
  }

  let release = fallbackRelease(versions);
  try {
    const latest = await fetchLatest();
    release = mergeRelease(release, latest);
    release.fallbackUrl = FALLBACK_URL;
    console.log(`Release v${release.version} (${release.date})`);
    for (const [key, url] of Object.entries(release.assets)) {
      console.log(`  ${key}: ${url || '(missing, fallback)'}`);
    }
  } catch (error) {
    console.warn(`Could not fetch ${LATEST_API}: ${error.message}`);
    console.warn('Download links will point at /releases/latest until the browser refresh succeeds.');
  }

  copyStatic();

  const home = page({
    title: 'Lixa Gallery — circle the keepers',
    description:
      'Lixa Gallery is a small desktop app for culling a photo shoot. Open a folder, mark the keepers, and copy or move them out. macOS, Windows, Linux, and Nix.',
    current: 'home',
    home: '',
    content: read('home.html').trim(),
  });
  const changelog = page({
    title: 'Changelog — Lixa Gallery',
    description: 'Release notes for Lixa Gallery, generated from CHANGELOG.md.',
    current: 'changelog',
    home: 'index.html',
    content: `<header class="page-head">
  <p class="kicker">Release notes</p>
  <h1>Changelog.</h1>
  <p class="lede">Cut from <a href="https://github.com/${REPO}/blob/main/CHANGELOG.md">CHANGELOG.md</a>. Notes people meet in the app are listed in the open. CI, Nix, tooling and refactors sit under Developer changes. Every line links to its commit. Installers for these versions are on the <a href="https://github.com/${REPO}/releases">release list</a>.</p>
</header>
<div class="timeline">
${renderChangelog(versions)}
</div>`,
  });

  fs.writeFileSync(path.join(dist, 'index.html'), applyTokens(home, release));
  fs.writeFileSync(path.join(dist, 'changelog.html'), applyTokens(changelog, release));
  console.log(`Wrote ${dist}`);
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
