import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';
import { classifyVersion, isDeveloperChange, parseChangelog, renderChangelog } from './changelog.mjs';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const changelog = fs.readFileSync(path.join(root, 'CHANGELOG.md'), 'utf8');
const versions = parseChangelog(changelog);

function byVersion(version) {
  const found = versions.find((entry) => entry.version === version);
  assert.ok(found, `missing ${version}`);
  return classifyVersion(found);
}

function texts(items) {
  return items.map((item) => item.text);
}

test('parses every release-please version in CHANGELOG.md', () => {
  assert.deepEqual(
    versions.map((entry) => entry.version),
    ['0.7.5', '0.7.4', '0.7.3', '0.7.2', '0.7.1'],
  );
  assert.equal(versions[0].date, '2026-09-29');
  assert.match(versions[0].compareUrl, /lixa-gallery-v0\.7\.4\.\.\.lixa-gallery-v0\.7\.5/);
});

test('keeps product notes in front and folds CI, Nix and refactors', () => {
  const v075 = byVersion('0.7.5');
  assert.deepEqual(texts(v075.userSections[0].items), [
    'add monotonic wall clock for timer',
    'add slideshow support',
    'use lucide icons consistently across repo',
  ]);
  assert.equal(v075.devItems.length, 0);

  const v074 = byVersion('0.7.4');
  assert.equal(v074.userSections[0].items.length, 1);
  assert.match(v074.userSections[0].items[0].text, /folder grouping/);
  assert.equal(v074.userSections[0].items[0].scope, 'favourites');
  assert.equal(v074.devItems.length, 1);
  assert.match(v074.devItems[0].text, /CI workflows/);

  const v073 = byVersion('0.7.3');
  assert.equal(v073.userSections[0].items.length, 3);
  assert.equal(v073.devItems.length, 1);
  assert.match(v073.devItems[0].text, /Nix configuration/);

  const v072 = byVersion('0.7.2');
  assert.equal(v072.userSections.find((section) => section.title === 'Features').items.length, 1);
  assert.equal(v072.userSections.find((section) => section.title === 'Bug Fixes').items.length, 1);
  assert.equal(v072.devItems.length, 1);
  assert.match(v072.devItems[0].text, /refactor Nix/);

  const v071 = byVersion('0.7.1');
  assert.equal(v071.userSections[0].items.length, 3);
  assert.equal(v071.devItems.length, 3);
  assert.ok(v071.devItems.some((item) => item.scope === 'ci'));
  assert.ok(v071.devItems.some((item) => /benchmark/.test(item.text)));
  assert.ok(v071.devItems.some((item) => /gh actions/.test(item.text)));
  assert.ok(v071.userSections[0].items.some((item) => /zoom support/.test(item.text)));
});

test('scope alone is enough to file a note as developer-only', () => {
  assert.equal(isDeveloperChange({ scope: 'ci', text: 'ship the thing' }), true);
  assert.equal(isDeveloperChange({ scope: 'favourites', text: 'group by folder' }), false);
  assert.equal(isDeveloperChange({ scope: null, text: 'add slideshow support' }), false);
});

test('rendered changelog links commits and collapses developer notes', () => {
  const html = renderChangelog(versions);
  assert.match(html, /id="v0\.7\.5"/);
  assert.match(html, /29 Sep 2026/);
  assert.match(html, /href="https:\/\/github\.com\/santhosh-chinnasamy\/lixa-gallery\/commit\/0209c42/);
  assert.match(html, />0209c42</);

  const v074 = html.split('id="v0.7.4"')[1].split('id="v0.7.3"')[0];
  const dev = v074.split('<details class="dev">')[1];
  assert.ok(dev, 'expected a developer disclosure');
  assert.match(dev, /CI workflows/);
  assert.doesNotMatch(v074.split('<details')[0], /CI workflows/);
  assert.match(v074.split('<details')[0], /Favourites: add folder grouping/);

  const v075 = html.split('id="v0.7.5"')[1].split('id="v0.7.4"')[0];
  assert.doesNotMatch(v075, /<details class="dev">/);
  assert.match(v075, /Add slideshow support/);
});
