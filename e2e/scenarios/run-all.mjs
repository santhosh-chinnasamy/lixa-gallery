/**
 * Drive a release build of Lixa Gallery through folder open, switch, scroll,
 * cull, favourites, export, viewer, and settings. One branch per process.
 *
 *   LIXA_BIN=/path/to/lixa-gallery LIXA_SETS=/path/to/sets \
 *   LIXA_OUT=/path/to/out/pr LIXA_BRANCH=pr node scenarios/run-all.mjs
 */
import fs from 'node:fs';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import {
  By,
  Key,
  appPid,
  clickHref,
  clickText,
  connect,
  dropCaches,
  frameStats,
  goHome,
  installObservers,
  logLine,
  longTasks,
  openFolder,
  perfRecord,
  pressKeys,
  resetFrames,
  setScenario,
  shot,
  sleep,
  snapshot,
  sweepFailures,
  waitForGrid,
  writeEnv,
} from './lib.mjs';

const BIN = process.env.LIXA_BIN;
const SETS = process.env.LIXA_SETS;
const OUT = process.env.LIXA_OUT;
const BRANCH = process.env.LIXA_BRANCH || 'app';
const RUNS = Number(process.env.LIXA_RUNS || '3');
const LAUNCHER = process.env.LIXA_LAUNCHER;
const ENV_FILE = process.env.LIXA_E2E_ENV || '/tmp/lixa-e2e-current.env';
const SCENARIO_FILE = path.join(OUT, 'scenario.txt');
const RESULTS = path.join(OUT, 'results.jsonl');

const folders = {
  A: path.join(SETS, 'A-flat-2000'),
  B: path.join(SETS, 'B-mixed-500'),
  C: path.join(SETS, 'C-nested'),
  C3: path.join(SETS, 'C-nested', 'shoot-03'),
  D: path.join(SETS, 'D-flat-1000'),
  E: path.join(SETS, 'E-export'),
};

function requireDir(dir) {
  if (!fs.existsSync(dir)) throw new Error(`missing folder set: ${dir}`);
}

async function main() {
  if (!BIN || !SETS || !OUT || !LAUNCHER) {
    throw new Error('LIXA_BIN, LIXA_SETS, LIXA_OUT, and LIXA_LAUNCHER are required');
  }
  fs.mkdirSync(OUT, { recursive: true });
  for (const dir of Object.values(folders)) requireDir(dir);
  const session = new AppSession();
  const failures = [];
  const only = new Set(
    (process.env.LIXA_ONLY || '')
      .split(',')
      .map((name) => name.trim())
      .filter(Boolean),
  );
  const steps = [
    ['load', () => firstLoads(session)],
    ['switch', () => folderSwitch(session)],
    ['scroll', () => scrolling(session)],
    ['cull', () => culling(session)],
    ['favourites', () => favourites(session)],
    ['viewer', () => viewer(session)],
    ['settings', () => settings(session)],
  ];
  for (const [name, fn] of steps) {
    if (only.size && !only.has(name)) continue;
    setScenario(SCENARIO_FILE, name);
    try {
      await fn();
    } catch (error) {
      const message = error instanceof Error ? error.stack || error.message : String(error);
      failures.push({ name, message });
      logLine(RESULTS, { branch: BRANCH, scenario: name, error: message.split('\n')[0] });
      if (session.driver) {
        await shot(session.driver, path.join(OUT, 'screenshots', `${name}-error.png`)).catch(() => {});
      }
    }
  }
  await session.stop();
  fs.writeFileSync(path.join(OUT, 'failures.json'), JSON.stringify(failures, null, 2));
  console.log(`DONE branch=${BRANCH} step_errors=${failures.length}`);
}

class AppSession {
  constructor() {
    this.driver = null;
    this.home = null;
    this.logFile = null;
    this.seq = 0;
  }

  async start({ fresh, cold }) {
    await this.stop();
    if (fresh || !this.home) {
      this.seq += 1;
      this.home = path.join(OUT, 'xdg', `run-${this.seq}`);
    }
    this.logFile = path.join(OUT, 'logs', `run-${this.seq}.log`);
    fs.mkdirSync(this.home, { recursive: true });
    writeEnv(ENV_FILE, {
      LIXA_REAL_BIN: BIN,
      LIXA_APP_LOG: this.logFile,
      XDG_DATA_HOME: path.join(this.home, 'data'),
      XDG_CONFIG_HOME: path.join(this.home, 'config'),
      XDG_CACHE_HOME: path.join(this.home, 'cache'),
      XDG_STATE_HOME: path.join(this.home, 'state'),
    });
    let dropped = false;
    if (cold) dropped = await dropCaches();
    this.driver = await connect(LAUNCHER);
    await sleep(1200);
    await installObservers(this.driver);
    this.dropped = dropped;
    return this.driver;
  }

  async stop() {
    if (!this.driver) return;
    const driver = this.driver;
    this.driver = null;
    await Promise.race([driver.quit().catch(() => {}), sleep(8000)]);
    try {
      execFileSync('pkill', ['-x', 'lixa-gallery']);
    } catch {
      // Already gone.
    }
    await sleep(300);
  }
}

function record(row) {
  const full = { branch: BRANCH, ...row, signals: signals(row.logFile) };
  logLine(RESULTS, full);
  const brief = [
    full.scenario,
    full.folder || '',
    full.temperature || '',
    full.run ?? '',
    full.gridMs ?? '',
    full.firstThumbMs ?? '',
    full.visibleMs ?? '',
    full.failed ?? '',
    full.how || '',
    full.error || '',
  ].join(' ');
  console.log(brief);
}

function signals(logFile) {
  if (!logFile || !fs.existsSync(logFile)) return null;
  const text = fs.readFileSync(logFile, 'utf8');
  const keys = ['cancelled', '404', 'Scan folder', 'Lazy mode', 'Background work'];
  const counts = {};
  for (const key of keys) counts[key] = text.split(key).length - 1;
  return counts;
}

async function openTimed(session, folder, timeoutMs) {
  const opened = await openFolder(session.driver, folder);
  const started = Date.now();
  let timing = await waitForGrid(session.driver, Math.min(timeoutMs, 25000));
  if (timing.gridMs == null) {
    try {
      execFileSync('xdotool', ['key', 'Escape'], { env: process.env });
    } catch {
      // No dialog to dismiss.
    }
    await sleep(200);
    const retry = await openFolder(session.driver, folder);
    timing = await waitForGrid(session.driver, timeoutMs);
    return { ...timing, how: `${opened.how}->${retry.how}`, interactMs: opened.interactMs };
  }
  if (timing.visibleMs == null) {
    const elapsed = Date.now() - started;
    const rest = await waitForGrid(session.driver, Math.max(1000, timeoutMs - elapsed));
    const add = (value) => (value == null ? null : value + elapsed);
    timing = {
      gridMs: timing.gridMs,
      firstThumbMs: timing.firstThumbMs ?? add(rest.firstThumbMs),
      visibleMs: add(rest.visibleMs),
      snap: rest.snap,
    };
  }
  return { ...timing, how: opened.how, interactMs: opened.interactMs };
}

async function firstLoads(session) {
  const plan = [
    ['A', folders.A, 90000],
    ['D', folders.D, 90000],
    ['B', folders.B, 90000],
    ['C-root', folders.C, 45000],
    ['C-shoot-03', folders.C3, 60000],
  ];
  for (const [name, folder, timeoutMs] of plan) {
    for (let run = 1; run <= RUNS; run += 1) {
      setScenario(SCENARIO_FILE, `load-${name}-cold-${run}`);
      await session.start({ fresh: true, cold: true });
      const cold = await openTimed(session, folder, timeoutMs);
      const coldFailed = cold.snap?.failed ?? null;
      record({
        scenario: 'first-load',
        folder: name,
        temperature: 'cold',
        run,
        how: cold.how,
        interactMs: cold.interactMs,
        gridMs: cold.gridMs,
        firstThumbMs: cold.firstThumbMs,
        visibleMs: cold.visibleMs,
        photoCount: cold.snap?.photoCount ?? null,
        cells: cold.snap?.cells ?? null,
        loaded: cold.snap?.loaded ?? null,
        failed: coldFailed,
        droppedCaches: session.dropped,
        logFile: session.logFile,
      });
      if (run === 1) {
        await shot(session.driver, path.join(OUT, 'screenshots', `load-${name}-cold.png`));
      }
      if (coldFailed) {
        const names = await sweepFailures(session.driver, 6);
        record({
          scenario: 'first-load-failures',
          folder: name,
          temperature: 'cold',
          run,
          failedNames: names.length,
          logFile: session.logFile,
        });
      }
      setScenario(SCENARIO_FILE, `load-${name}-warm-${run}`);
      await goHome(session.driver);
      await sleep(300);
      const warm = await openTimed(session, folder, Math.min(timeoutMs, 45000));
      record({
        scenario: 'first-load',
        folder: name,
        temperature: 'warm',
        run,
        how: warm.how,
        interactMs: warm.interactMs,
        gridMs: warm.gridMs,
        firstThumbMs: warm.firstThumbMs,
        visibleMs: warm.visibleMs,
        photoCount: warm.snap?.photoCount ?? null,
        cells: warm.snap?.cells ?? null,
        loaded: warm.snap?.loaded ?? null,
        failed: warm.snap?.failed ?? null,
        logFile: session.logFile,
      });
    }
  }
}

async function folderSwitch(session) {
  const phases = [
    { name: 'early', waitMs: 2000, next: folders.D, nextName: 'D' },
    { name: 'mid', waitMs: 15000, next: folders.D, nextName: 'D' },
    { name: 'late', waitMs: 40000, next: folders.B, nextName: 'B' },
  ];
  for (let run = 1; run <= RUNS; run += 1) {
    const phase = phases[Math.min(run, phases.length) - 1];
    setScenario(SCENARIO_FILE, `switch-${phase.name}-${run}`);
    await session.start({ fresh: true, cold: true });
    const first = await openTimed(session, folders.A, 90000);
    await sleep(phase.waitMs);
    const midSnap = await snapshot(session.driver);
    const second = await openTimed(session, phase.next, 90000);
    const secondFailed = await sweepFailures(session.driver, 6);
    const back = await openTimed(session, folders.A, 60000);
    const backFailed = await sweepFailures(session.driver, 6);
    record({
      scenario: 'folder-switch',
      run,
      phase: phase.name,
      waitMs: phase.waitMs,
      firstGridMs: first.gridMs,
      firstVisibleMs: first.visibleMs,
      loadedBeforeSwitch: midSnap.loaded,
      failedBeforeSwitch: midSnap.failed,
      next: phase.nextName,
      nextGridMs: second.gridMs,
      nextVisibleMs: second.visibleMs,
      nextFailed: secondFailed.length,
      backGridMs: back.gridMs,
      backVisibleMs: back.visibleMs,
      backFailed: backFailed.length,
      how: `${first.how}|${second.how}|${back.how}`,
      logFile: session.logFile,
    });
    if (secondFailed.length || backFailed.length) {
      await shot(session.driver, path.join(OUT, 'screenshots', `switch-${phase.name}-failed.png`));
    }
  }
}

async function scrolling(session) {
  for (let run = 1; run <= RUNS; run += 1) {
    setScenario(SCENARIO_FILE, `scroll-${run}`);
    await session.start({ fresh: true, cold: run === 1 });
    await openTimed(session, folders.A, 90000);
    await resetFrames(session.driver);
    // Slow continuous scroll while indexing is still running.
    const slowStart = Date.now();
    for (let i = 0; i < 40; i += 1) {
      await session.driver.executeScript(
        `const main = document.querySelector('main'); if (main) main.scrollTop += 36;`,
      );
      await sleep(100);
    }
    const slowMs = Date.now() - slowStart;
    const afterSlow = await frameStats(session.driver);
    await resetFrames(session.driver);
    // Fast flick to the bottom, then keyboard end, then middle, then back up.
    await session.driver.executeScript(
      `const main = document.querySelector('main'); if (main) main.scrollTop = main.scrollHeight;`,
    );
    await sleep(700);
    await session.driver.actions().sendKeys(Key.END).perform();
    await sleep(500);
    await session.driver.executeScript(
      `const main = document.querySelector('main'); if (main) main.scrollTop = main.scrollHeight / 2;`,
    );
    await sleep(500);
    await session.driver.actions().sendKeys(Key.HOME).perform();
    await sleep(400);
    await session.driver.executeScript(
      `const main = document.querySelector('main'); if (main) main.scrollTop = 0;`,
    );
    const flick = await frameStats(session.driver);
    const tasks = await longTasks(session.driver, 50);
    const stuck = await sweepFailures(session.driver, 10);
    let perf = false;
    if (run === 1) {
      const pid = appPid();
      const dataFile = path.join(OUT, 'perf.data');
      setScenario(SCENARIO_FILE, 'scroll-perf');
      await session.driver.executeScript(
        `const main = document.querySelector('main');
         if (!main) return;
         let y = 0;
         const timer = setInterval(() => {
           y += 800;
           main.scrollTop = y % (main.scrollHeight || 1);
         }, 50);
         setTimeout(() => clearInterval(timer), 12000);`,
      );
      perf = await perfRecord(pid, 12, dataFile);
      if (perf) {
        try {
          const script = execFileSync('perf', ['script', '-i', dataFile], {
            encoding: 'utf8',
            maxBuffer: 128 * 1024 * 1024,
          });
          fs.writeFileSync(path.join(OUT, 'perf.script'), script);
        } catch {
          perf = false;
        }
      }
    }
    // Scroll while marking, after indexing has had time to run.
    await resetFrames(session.driver);
    for (let i = 0; i < 15; i += 1) {
      await session.driver.actions().sendKeys('p').perform();
      await session.driver.executeScript(
        `const main = document.querySelector('main'); if (main) main.scrollTop += 200;`,
      );
    }
    const duringCull = await frameStats(session.driver);
    const snap = await snapshot(session.driver);
    record({
      scenario: 'scroll',
      run,
      slowMs,
      slowFrame: afterSlow,
      flickFrame: flick,
      longTasks: tasks.length,
      longTaskMax: tasks.reduce((max, task) => Math.max(max, task.duration), 0),
      stuckOrFailed: stuck.length,
      duringCullFrame: duringCull,
      picked: snap.picked,
      loaded: snap.loaded,
      failed: snap.failed,
      perfCaptured: perf,
      logFile: session.logFile,
    });
    if (run === 1) {
      await shot(session.driver, path.join(OUT, 'screenshots', 'scroll.png'));
    }
  }
}

async function culling(session) {
  setScenario(SCENARIO_FILE, 'cull-fast');
  await session.start({ fresh: true, cold: true });
  await openTimed(session, folders.A, 90000);
  await resetFrames(session.driver);
  const cullMs = await pressKeys(session.driver, 'p', 200);
  await sleep(500);
  const after = await snapshot(session.driver);
  const tasks = await longTasks(session.driver, 50);
  record({
    scenario: 'cull-fast',
    folder: 'A',
    marks: 200,
    cullMs,
    picked: after.picked,
    photoCount: after.photoCount,
    longTasks: tasks.length,
    longTaskMax: tasks.reduce((max, task) => Math.max(max, task.duration), 0),
    logFile: session.logFile,
  });
  // RAW+JPEG pair folder, a few marks, while indexing.
  setScenario(SCENARIO_FILE, 'cull-pairs');
  await openTimed(session, folders.B, 90000);
  const labels = await session.driver.executeScript(
    `return [...document.querySelectorAll('[role=gridcell]')].slice(0, 12).map((el) => el.getAttribute('aria-label'));`,
  );
  await pressKeys(session.driver, 'p', 8);
  await sleep(300);
  const pairSnap = await snapshot(session.driver);
  record({
    scenario: 'cull-pairs',
    folder: 'B',
    sampleLabels: Array.isArray(labels) ? labels.length : 0,
    rawLabels: Array.isArray(labels) ? labels.filter((name) => /\.(cr2|cr3|nef|arw|raf|orf|rw2|dng)$/i.test(name || '')).length : 0,
    picked: pairSnap.picked,
    logFile: session.logFile,
  });
  const keptHome = session.home;
  const keptLog = session.logFile;
  await session.stop();
  // Relaunch against the same XDG dir and confirm picks survived.
  session.home = keptHome;
  await session.start({ fresh: false, cold: false });
  // start() with fresh:false still rotates log file via seq. Restore comparison using new log only for signals.
  const reA = await openTimed(session, folders.A, 90000);
  const reB = await openTimed(session, folders.B, 60000);
  record({
    scenario: 'cull-persist-reopen',
    pickedA: reA.snap?.picked ?? null,
    photoA: reA.snap?.photoCount ?? null,
    pickedB: reB.snap?.picked ?? null,
    expectedA: after.picked,
    expectedB: pairSnap.picked,
    logFile: session.logFile,
    previousLog: keptLog,
  });
  await shot(session.driver, path.join(OUT, 'screenshots', 'cull-reopen.png'));
}

async function favourites(session) {
  setScenario(SCENARIO_FILE, 'favourites');
  await session.start({ fresh: true, cold: false });
  await openTimed(session, folders.E, 60000);
  await pressKeys(session.driver, 'p', 8);
  await sleep(400);
  const before = await snapshot(session.driver);
  await clickHref(session.driver, '/favourites');
  await sleep(1000);
  const fav = await snapshot(session.driver);
  await session.driver.executeScript(
    `const main = document.querySelector('main'); if (main) main.scrollTop = main.scrollHeight / 2;`,
  );
  await sleep(300);
  // Unpick the active favourite from the keyboard.
  await session.driver.actions().sendKeys('u').perform();
  await sleep(400);
  const afterUnpick = await snapshot(session.driver);
  await shot(session.driver, path.join(OUT, 'screenshots', 'favourites.png'));
  const exportDir = path.join(OUT, 'export-copy');
  const moveDir = path.join(OUT, 'export-move');
  fs.mkdirSync(exportDir, { recursive: true });
  fs.mkdirSync(moveDir, { recursive: true });
  let copyFiles = null;
  let copyError = null;
  try {
    await clickText(session.driver, 'Export');
    await sleep(400);
    await clickText(session.driver, 'Copy to Folder');
    await sleep(700);
    await chooseExportDir(exportDir);
    await sleep(4000);
    copyFiles = countFiles(exportDir);
  } catch (error) {
    copyError = error instanceof Error ? error.message : String(error);
  }
  await shot(session.driver, path.join(OUT, 'screenshots', 'export-copy.png'));
  let moveFiles = null;
  let cancelText = null;
  try {
    const still = await session.driver.findElements(By.xpath("//button[contains(., 'Export')]"));
    if (still.length) await still[0].click();
    await sleep(400);
    await clickText(session.driver, 'Move to Folder');
    await sleep(700);
    await chooseExportDir(moveDir);
    await sleep(600);
    const stop = await session.driver.findElements(By.xpath("//button[contains(., 'Stop after current file') or contains(., 'Cancel')]"));
    if (stop.length) await stop[0].click();
    await sleep(800);
    cancelText = await session.driver.executeScript('return document.body.innerText.includes("Cancelled")');
    moveFiles = countFiles(moveDir);
  } catch (error) {
    cancelText = error instanceof Error ? error.message : String(error);
  }
  record({
    scenario: 'favourites-export',
    pickedBefore: before.picked,
    favCells: fav.cells,
    favLoaded: fav.loaded,
    favFailed: fav.failed,
    afterUnpickCells: afterUnpick.cells,
    copyFiles,
    copyError,
    moveFiles,
    cancelText,
    logFile: session.logFile,
  });
}

async function chooseExportDir(dir) {
  const { execFile } = await import('node:child_process');
  const { promisify } = await import('node:util');
  const exec = promisify(execFile);
  const helper = path.join(path.dirname(new URL(import.meta.url).pathname), '..', 'lib', 'choose_dir.py');
  await sleep(400);
  try {
    execFileSync('python3', [helper, dir], { env: process.env, timeout: 8000 });
    return;
  } catch {
    // AT-SPI can miss the dialog; fall back to a paste.
  }
  execFileSync('xclip', ['-selection', 'clipboard'], { input: dir, env: process.env });
  await exec('xdotool', ['key', 'ctrl+l'], { env: process.env }).catch(() => {});
  await sleep(150);
  await exec('xdotool', ['key', 'ctrl+v'], { env: process.env });
  await sleep(100);
  await exec('xdotool', ['key', 'Return'], { env: process.env });
}

function countFiles(dir) {
  if (!fs.existsSync(dir)) return 0;
  let n = 0;
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    if (entry.isDirectory()) n += countFiles(path.join(dir, entry.name));
    else n += 1;
  }
  return n;
}

async function viewer(session) {
  setScenario(SCENARIO_FILE, 'viewer');
  await session.start({ fresh: true, cold: false });
  await openTimed(session, folders.A, 90000);
  await session.driver.actions().sendKeys(Key.ENTER).perform();
  await sleep(600);
  const started = Date.now();
  for (let i = 0; i < 50; i += 1) {
    await session.driver.actions().sendKeys(Key.ARROW_RIGHT).perform();
  }
  const arrowMs = Date.now() - started;
  await sleep(400);
  const text = await session.driver.executeScript('return document.body.innerText.slice(0, 1500)');
  await session.driver.actions().sendKeys('+').perform();
  await sleep(200);
  await shot(session.driver, path.join(OUT, 'screenshots', 'viewer.png'));
  await session.driver.actions().sendKeys(Key.ESCAPE).perform();
  await sleep(300);
  await openTimed(session, folders.B, 60000);
  const openedRaw = await session.driver.executeScript(`
    const cells = [...document.querySelectorAll('[role=gridcell]')];
    const raw = cells.find((el) => /\\.(cr2|cr3|nef|arw|raf|orf|rw2|dng)$/i.test(el.getAttribute('aria-label') || ''));
    if (!raw) return false;
    raw.click();
    return raw.getAttribute('aria-label');
  `);
  await sleep(800);
  const rawText = await session.driver.executeScript('return (document.body.innerText || "").includes("Failed to load")');
  if (openedRaw) await shot(session.driver, path.join(OUT, 'screenshots', 'viewer-raw.png'));
  record({
    scenario: 'viewer',
    arrowMs,
    failedText: typeof text === 'string' && text.includes('Failed to load'),
    openedRaw: Boolean(openedRaw),
    rawFailed: rawText,
    logFile: session.logFile,
  });
}

async function settings(session) {
  setScenario(SCENARIO_FILE, 'settings');
  await session.start({ fresh: true, cold: false });
  await clickHref(session.driver, '/settings');
  await sleep(600);
  await clickText(session.driver, 'Dark');
  await sleep(300);
  const dark = await snapshot(session.driver);
  await shot(session.driver, path.join(OUT, 'screenshots', 'settings-dark.png'));
  record({
    scenario: 'settings',
    dark: dark.dark,
    href: dark.href,
    logFile: session.logFile,
  });
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : error);
  process.exit(1);
});
