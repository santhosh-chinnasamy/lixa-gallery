/** Switch folders while the first thumbs are still decoding. Targets coalesce-on-stale-job. */
import fs from 'node:fs';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import {
  connect,
  dropCaches,
  installObservers,
  logLine,
  openFolder,
  shot,
  sleep,
  snapshot,
  sweepFailures,
  waitForGrid,
  writeEnv,
} from './lib.mjs';

const OUT = process.env.LIXA_OUT;
const BIN = process.env.LIXA_BIN;
const FOLDER_A = process.env.LIXA_FOLDER_A;
const FOLDER_D = process.env.LIXA_FOLDER_D;
const ENV_FILE = process.env.LIXA_E2E_ENV;
const LAUNCHER = process.env.LIXA_LAUNCHER;
const RUNS = Number(process.env.LIXA_RUNS || '3');
const RESULTS = path.join(OUT, 'decode-switch.jsonl');

async function session(home) {
  writeEnv(ENV_FILE, {
    LIXA_REAL_BIN: BIN,
    LIXA_APP_LOG: path.join(OUT, 'logs', `decode-switch-${path.basename(home)}.log`),
    XDG_DATA_HOME: path.join(home, 'data'),
    XDG_CONFIG_HOME: path.join(home, 'config'),
    XDG_CACHE_HOME: path.join(home, 'cache'),
    XDG_STATE_HOME: path.join(home, 'state'),
  });
  const dropped = await dropCaches();
  const driver = await connect(LAUNCHER);
  await sleep(1000);
  await installObservers(driver);
  return { driver, dropped };
}

async function stop(driver) {
  await Promise.race([driver.quit().catch(() => {}), sleep(8000)]);
  try {
    execFileSync('pkill', ['-x', path.basename(BIN)]);
  } catch {
    // already exited
  }
}

for (let run = 1; run <= RUNS; run += 1) {
  const home = path.join(OUT, 'xdg', `decode-${run}`);
  fs.mkdirSync(home, { recursive: true });
  const { driver, dropped } = await session(home);
  try {
    const opened = await openFolder(driver, FOLDER_A);
    const started = Date.now();
    let snap = null;
    while (Date.now() - started < 4000) {
      snap = await snapshot(driver);
      if (snap.photoCount && snap.loaded > 0 && snap.loaded < (snap.cells || 1)) break;
      if (snap.photoCount && Date.now() - started > 1500) break;
      await sleep(100);
    }
    const mid = snap;
    await openFolder(driver, FOLDER_D);
    const dStart = Date.now();
    let dSnap = null;
    while (Date.now() - dStart < 4000) {
      dSnap = await snapshot(driver);
      if (dSnap.photoCount) break;
      await sleep(100);
    }
    await openFolder(driver, FOLDER_A);
    const back = await waitForGrid(driver, 20000);
    const names = await sweepFailures(driver, 8);
    if (names.length) await shot(driver, path.join(OUT, 'screenshots', `decode-switch-${run}.png`));
    logLine(RESULTS, {
      branch: process.env.LIXA_BRANCH,
      run,
      dropped,
      how: opened.how,
      loadedWhenSwitched: mid?.loaded ?? null,
      cellsWhenSwitched: mid?.cells ?? null,
      failedWhenSwitched: mid?.failed ?? null,
      dPhoto: dSnap?.photoCount ?? null,
      dFailed: dSnap?.failed ?? null,
      backVisibleMs: back.visibleMs,
      backFailedVisible: back.snap?.failed ?? null,
      sweptFailed: names.length,
    });
    console.log('decode-switch', run, 'swept', names.length, 'backFail', back.snap?.failed);
  } finally {
    await stop(driver);
  }
}
