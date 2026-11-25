import assert from 'node:assert/strict';
import test from 'node:test';
import { previewVersion } from './preview-version.mjs';

test('builds a semver prerelease from the stable version', () => {
  assert.equal(
    previewVersion('0.7.5', '20261124', '12'),
    '0.7.5-preview.20261124.12',
  );
});

test('drops an existing prerelease and build metadata, including a v prefix', () => {
  assert.equal(
    previewVersion('v0.7.5-rc.1+abc1234', '20261124', '3'),
    '0.7.5-preview.20261124.3',
  );
  assert.equal(
    previewVersion('0.7.5-preview.20261124.3', '20261124', '3'),
    '0.7.5-preview.20261124.3',
  );
});

test('rejects dates and run numbers that are not plain semver identifiers', () => {
  assert.throws(() => previewVersion('0.7.5', '2026-11-24', '1'));
  assert.throws(() => previewVersion('0.7.5', '20261124', '0'));
  assert.throws(() => previewVersion('0.7.5', '20261124', '01'));
  assert.throws(() => previewVersion('not-a-version', '20261124', '1'));
});
