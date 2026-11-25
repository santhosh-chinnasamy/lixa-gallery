import fs from 'node:fs';
import path from 'node:path';
import { connect, installObservers, openFolder, shot, sleep, snapshot, waitForGrid, writeEnv } from './lib.mjs';

const out = process.env.LIXA_OUT;
const folder = process.env.LIXA_FOLDER;
fs.mkdirSync(out, { recursive: true });
const logFile = path.join(out, 'smoke.log');
writeEnv(process.env.LIXA_E2E_ENV, {
  LIXA_REAL_BIN: process.env.LIXA_BIN,
  LIXA_APP_LOG: logFile,
  XDG_DATA_HOME: path.join(out, 'xdg', 'data'),
  XDG_CONFIG_HOME: path.join(out, 'xdg', 'config'),
  XDG_CACHE_HOME: path.join(out, 'xdg', 'cache'),
  XDG_STATE_HOME: path.join(out, 'xdg', 'state'),
});
const driver = await connect(process.env.LIXA_LAUNCHER);
try {
  await sleep(1500);
  await installObservers(driver);
  const before = await snapshot(driver);
  console.log('BEFORE', JSON.stringify({ ready: before.ready, href: before.href, photoCount: before.photoCount }));
  await shot(driver, path.join(out, 'smoke-landing.png'));
  const opened = await openFolder(driver, folder);
  const timing = await waitForGrid(driver, 60000);
  console.log('OPEN', JSON.stringify({ how: opened.how, interactMs: opened.interactMs, gridMs: timing.gridMs, firstThumbMs: timing.firstThumbMs, visibleMs: timing.visibleMs, snap: timing.snap }));
  await shot(driver, path.join(out, 'smoke-grid.png'));
} finally {
  await Promise.race([driver.quit().catch(() => {}), sleep(8000)]);
  try {
    const { execFileSync } = await import('node:child_process');
    execFileSync('pkill', ['-x', 'lixa-gallery']);
  } catch {
    // already exited
  }
}
