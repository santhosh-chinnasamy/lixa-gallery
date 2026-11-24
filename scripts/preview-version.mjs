import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const rootDir = resolve(dirname(fileURLToPath(import.meta.url)), '..');

// X.Y.Z-preview.YYYYMMDD.<run> is valid semver. The short git sha stays in
// the release notes: a "+sha" build suffix is rejected by WiX/MSI.
export function previewVersion(baseVersion, date, run) {
  const clean = String(baseVersion).trim().replace(/^v/, '');
  const match = clean.match(
    /^(\d+)\.(\d+)\.(\d+)(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/,
  );
  if (!match) {
    throw new Error(`Invalid semantic version: "${baseVersion}"`);
  }
  if (!/^\d{8}$/.test(String(date))) {
    throw new Error(`Date must be YYYYMMDD, got "${date}"`);
  }
  if (!/^[1-9]\d*$/.test(String(run))) {
    throw new Error(
      `Run number must be a positive integer without leading zeros, got "${run}"`,
    );
  }
  return `${match[1]}.${match[2]}.${match[3]}-preview.${date}.${run}`;
}

function readArg(name) {
  const index = process.argv.indexOf(name);
  const value = index === -1 ? undefined : process.argv[index + 1];
  if (!value || value.startsWith('--')) {
    throw new Error(`Missing ${name}`);
  }
  return value;
}

const isDirectRun =
  process.argv[1] &&
  import.meta.url === pathToFileURL(resolve(process.argv[1])).href;

if (isDirectRun) {
  const tauriConf = JSON.parse(
    readFileSync(resolve(rootDir, 'src-tauri/tauri.conf.json'), 'utf8'),
  );
  process.stdout.write(
    `${previewVersion(tauriConf.version, readArg('--date'), readArg('--run'))}\n`,
  );
}
