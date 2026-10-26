import assert from 'node:assert/strict';
import test from 'node:test';
import {
  archFromSignals,
  detectOs,
  formatReleaseDate,
  highlightKeys,
  mergeRelease,
  parseGithubRelease,
  primaryChoice,
} from './js/downloads.js';

const release075 = {
  tag_name: 'lixa-gallery-v0.7.5',
  published_at: '2026-09-29T17:23:02Z',
  html_url: 'https://github.com/santhosh-chinnasamy/lixa-gallery/releases/tag/lixa-gallery-v0.7.5',
  assets: [
    'Lixa.Gallery-0.7.5-1.x86_64.rpm',
    'Lixa.Gallery_0.7.5_aarch64.dmg',
    'Lixa.Gallery_0.7.5_amd64.AppImage',
    'Lixa.Gallery_0.7.5_amd64.deb',
    'Lixa.Gallery_0.7.5_x64-setup.exe',
    'Lixa.Gallery_0.7.5_x64.dmg',
    'Lixa.Gallery_0.7.5_x64_en-US.msi',
    'Lixa.Gallery_aarch64.app.tar.gz',
    'Lixa.Gallery_x64.app.tar.gz',
  ].map((name) => ({
    name,
    browser_download_url: `https://github.com/santhosh-chinnasamy/lixa-gallery/releases/download/lixa-gallery-v0.7.5/${name}`,
  })),
};

test('maps the real 0.7.5 asset names and skips updater tarballs', () => {
  const parsed = parseGithubRelease(release075);
  assert.equal(parsed.version, '0.7.5');
  assert.equal(parsed.date, '2026-09-29');
  assert.match(parsed.assets['macos-arm'], /aarch64\.dmg$/);
  assert.match(parsed.assets['macos-intel'], /x64\.dmg$/);
  assert.match(parsed.assets['windows-exe'], /x64-setup\.exe$/);
  assert.match(parsed.assets['windows-msi'], /x64_en-US\.msi$/);
  assert.match(parsed.assets['linux-deb'], /amd64\.deb$/);
  assert.match(parsed.assets['linux-rpm'], /x86_64\.rpm$/);
  assert.match(parsed.assets['linux-appimage'], /amd64\.AppImage$/);
  for (const url of Object.values(parsed.assets)) {
    assert.doesNotMatch(url, /tar\.gz/);
  }
});

test('detects desktop OS and leaves phones alone', () => {
  assert.equal(detectOs('Mozilla/5.0 (Windows NT 10.0; Win64; x64)'), 'windows');
  assert.equal(
    detectOs('Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15'),
    'mac',
  );
  assert.equal(detectOs('Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36'), 'linux');
  assert.equal(detectOs('Mozilla/5.0 (Linux; Android 14)'), 'unknown');
});

test('tells Apple silicon from Intel, and admits when it cannot', () => {
  assert.equal(archFromSignals({ architecture: 'arm' }), 'arm');
  assert.equal(archFromSignals({ architecture: 'x86' }), 'intel');
  assert.equal(archFromSignals({ renderer: 'ANGLE (Apple, Apple M2, Unspecified)' }), 'arm');
  assert.equal(archFromSignals({ renderer: 'Intel Iris OpenGL Engine' }), 'intel');
  assert.equal(archFromSignals({ renderer: 'llvmpipe' }), null);
  assert.deepEqual(highlightKeys('mac', null), ['macos-arm', 'macos-intel']);
  assert.deepEqual(highlightKeys('mac', 'arm'), ['macos-arm']);
  assert.deepEqual(highlightKeys('windows', null), ['windows-exe']);
  assert.equal(primaryChoice('linux', null).key, 'linux-appimage');
  assert.equal(primaryChoice('mac', null).key, null);
  assert.equal(primaryChoice('mac', 'intel').label, 'Download for macOS');
});

test('a later release refreshes asset URLs without dropping ones it omitted', () => {
  const baked = {
    version: '0.7.4',
    date: '2026-09-29',
    fallbackUrl: 'https://github.com/santhosh-chinnasamy/lixa-gallery/releases/latest',
    assets: { 'macos-arm': 'https://example.com/old.dmg', 'linux-deb': 'https://example.com/old.deb' },
  };
  const merged = mergeRelease(baked, {
    version: '0.7.5',
    date: '2026-09-29',
    assets: { 'macos-arm': 'https://example.com/new.dmg' },
  });
  assert.equal(merged.version, '0.7.5');
  assert.equal(merged.assets['macos-arm'], 'https://example.com/new.dmg');
  assert.equal(merged.assets['linux-deb'], 'https://example.com/old.deb');
  assert.equal(formatReleaseDate('2026-09-29'), '29 Sep 2026');
});
