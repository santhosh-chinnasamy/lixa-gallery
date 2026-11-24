import { execFile, execFileSync, spawn } from 'node:child_process';
import { promisify } from 'node:util';
import fs from 'node:fs';
import path from 'node:path';
import { Builder, By, Key, until } from 'selenium-webdriver';

const exec = promisify(execFile);

export const SNAPSHOT_JS = `return (() => {
  const gridCells = [...document.querySelectorAll('[role="gridcell"]')];
  const main = document.querySelector('main');
  const mainImgs = main ? [...main.querySelectorAll('img')] : [];
  const cells = gridCells.length ? gridCells : mainImgs;
  const imgs = gridCells.length
    ? gridCells.flatMap((cell) => [...cell.querySelectorAll('img')])
    : mainImgs;
  const loaded = imgs.filter((img) => img.complete && img.naturalWidth > 0).length;
  const failNodes = (gridCells.length ? gridCells : (main ? [...main.querySelectorAll('div')] : []))
    .filter((el) => {
      return (el.innerText || '').trim() === 'Failed to load';
    });
  const failed = failNodes.length;
  const loading = (gridCells.length ? gridCells : (main ? [...main.querySelectorAll('div')] : []))
    .filter((el) => (el.innerText || '').trim() === 'Loading...').length;
  const text = document.body ? document.body.innerText : '';
  const countMatch = text.match(/(\\d+)\\s+picked\\s*\\/\\s*(\\d+)/);
  const failedNames = failNodes
    .map((el) => el.getAttribute('aria-label') || '')
    .filter(Boolean);
  return {
    cells: cells.length,
    loaded,
    failed,
    loading,
    failedNames,
    photoCount: countMatch ? Number(countMatch[2]) : null,
    picked: countMatch ? Number(countMatch[1]) : null,
    hasPickCounter: Boolean(countMatch),
    emptyFolder: text.includes('No photos in this folder') || text.includes("doesn't appear to contain any supported files"),
    ready: text.includes('Ready to Cull'),
    href: location.href,
    dark: document.documentElement.classList.contains('dark'),
    title: document.title,
  };
})()`;

const OBSERVER_JS = `return (() => {
  if (!window.__e2e) window.__e2e = { longtasks: [], frames: [] };
  if (!window.__e2e.obs) {
    try {
      const obs = new PerformanceObserver((list) => {
        for (const entry of list.getEntries()) {
          window.__e2e.longtasks.push({
            name: entry.name,
            duration: entry.duration,
            start: entry.startTime,
          });
        }
      });
      obs.observe({ entryTypes: ['longtask'] });
      window.__e2e.obs = true;
    } catch (err) {
      window.__e2e.obsError = String(err);
    }
  }
  if (!window.__e2e.raf) {
    let last = 0;
    const loop = (t) => {
      if (last) {
        window.__e2e.frames.push(t - last);
        if (window.__e2e.frames.length > 6000) window.__e2e.frames.splice(0, 1000);
      }
      last = t;
      window.__e2e.raf = requestAnimationFrame(loop);
    };
    window.__e2e.raf = requestAnimationFrame(loop);
  }
  return true;
})()`;

export function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

export function xpathLiteral(value) {
  if (!value.includes("'")) return `'${value}'`;
  return `concat('${value.split("'").join(`', "'", '`)}')`;
}

export async function dropCaches() {
  try {
    await exec('sudo', ['-n', 'sh', '-c', 'sync; echo 3 > /proc/sys/vm/drop_caches']);
    return true;
  } catch {
    return false;
  }
}

export function writeEnv(file, values) {
  const lines = Object.entries(values).map(([key, value]) => `${key}=${shellQuote(value)}`);
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, lines.join('\n') + '\n');
}

function shellQuote(value) {
  return `'${String(value).replaceAll("'", `'\\''`)}'`;
}

export async function connect(application) {
  const server = process.env.TAURI_DRIVER_URL || 'http://127.0.0.1:4444/';
  const driver = await new Builder()
    .usingServer(server)
    .withCapabilities({
      browserName: 'wry',
      'tauri:options': { application },
    })
    .build();
  await driver.manage().setTimeouts({ implicit: 500, pageLoad: 20000, script: 15000 });
  return driver;
}

export async function installObservers(driver) {
  try {
    await driver.executeScript(OBSERVER_JS);
  } catch {
    // Page may still be starting.
  }
}

export async function snapshot(driver) {
  return driver.executeScript(SNAPSHOT_JS);
}

export async function shot(driver, file) {
  fs.mkdirSync(path.dirname(file), { recursive: true });
  const png = await driver.takeScreenshot();
  fs.writeFileSync(file, Buffer.from(png, 'base64'));
  const display = process.env.DISPLAY;
  if (display) {
    const root = file.replace(/\.png$/, '-root.png');
    await exec('import', ['-window', 'root', root], {
      env: { ...process.env, DISPLAY: display },
    }).catch(() => {});
  }
}

export async function bodyText(driver) {
  return driver.executeScript('return document.body ? document.body.innerText.slice(0, 4000) : ""');
}

export async function clickText(driver, text) {
  const el = await driver.wait(
    until.elementLocated(By.xpath(`//button[contains(normalize-space(.), ${xpathLiteral(text)})] | //a[contains(normalize-space(.), ${xpathLiteral(text)})]`)),
    15000,
  );
  await el.click();
}

export async function clickHref(driver, href) {
  const el = await driver.wait(
    until.elementLocated(By.css(`a[href="${href}"]`)),
    15000,
  );
  await el.click();
}

export async function goHome(driver) {
  const home = await driver.findElements(By.css('button[title="Go to Home/Landing"]'));
  if (home.length) {
    await home[0].click();
    await sleep(400);
    return;
  }
  await clickHref(driver, '/');
  await sleep(400);
}

async function xdo(args) {
  await exec('xdotool', args, { env: process.env });
}

async function pasteClipboard(text) {
  await new Promise((resolve, reject) => {
    const child = spawn('xclip', ['-selection', 'clipboard'], { env: process.env });
    child.on('error', reject);
    child.on('exit', (code) => (code === 0 ? resolve() : reject(new Error(`xclip ${code}`))));
    child.stdin.end(text);
  });
}

async function chooserWindow() {
  for (const name of ['Select Folder', 'Open']) {
    try {
      const found = await exec('xdotool', ['search', '--onlyvisible', '--name', name], {
        env: process.env,
      });
      const id = found.stdout.toString().trim().split('\n').filter(Boolean).pop();
      if (id) return id;
    } catch {
      // Window is not up yet.
    }
  }
  return '';
}

async function gtkChoose(dir) {
  let wid = '';
  for (let i = 0; i < 25 && !wid; i += 1) {
    wid = await chooserWindow();
    if (!wid) await sleep(120);
  }
  if (!wid) throw new Error('file chooser did not appear');
  const key = (...names) => exec('xdotool', ['key', '--window', wid, ...names], { env: process.env });
  await exec('xdotool', ['windowfocus', '--sync', wid], { env: process.env });
  await pasteClipboard(dir);
  await key('ctrl+l');
  await sleep(200);
  await key('ctrl+a', 'ctrl+v');
  await sleep(200);
  await key('Return');
  await sleep(500);
  if (await chooserWindow()) {
    await key('Return');
    await sleep(300);
  }
  if (await chooserWindow()) {
    const geo = await exec('xdotool', ['getwindowgeometry', '--shell', wid], { env: process.env });
    const vars = Object.fromEntries(
      geo.stdout
        .toString()
        .trim()
        .split('\n')
        .map((line) => line.split('=')),
    );
    const x = Number(vars.X) + Number(vars.WIDTH) - 50;
    const y = Number(vars.Y) + Number(vars.HEIGHT) - 28;
    await exec('xdotool', ['mousemove', '--sync', String(x), String(y), 'click', '1'], {
      env: process.env,
    });
    await sleep(400);
  }
}

export async function seedRecents(driver, folders) {
  await driver.executeScript(
    'localStorage.setItem("recent_folders", JSON.stringify(arguments[0]));',
    folders,
  );
  await driver.navigate().refresh();
  await sleep(800);
  await installObservers(driver);
}

export async function clickRecent(driver, folder) {
  const button = await driver.wait(
    until.elementLocated(By.xpath(`//button[@title=${xpathLiteral(folder)}]`)),
    10000,
  );
  await button.click();
}

/**
 * Open a folder through the real UI.
 * Prefers the Recent Folders button (same loadPath the app uses).
 * Falls back to the GTK directory dialog, then seeds recents and clicks.
 */
export async function openFolder(driver, folder, { forceDialog = false } = {}) {
  const started = Date.now();
  let how = 'recent';
  if (!forceDialog) {
    const existing = await driver.findElements(By.xpath(`//button[@title=${xpathLiteral(folder)}]`));
    if (existing.length) {
      await existing[0].click();
      return { how, interactMs: Date.now() - started };
    }
  }
  if (forceDialog) {
    const onLanding = (await driver.findElements(By.xpath("//button[contains(., 'Open Folder')]"))).length > 0;
    if (!onLanding) await goHome(driver);
    const openBtn = await driver.findElements(By.xpath("//button[contains(., 'Open Folder')]"));
    if (openBtn.length) {
      try {
        await openBtn[0].click();
        await gtkChoose(folder);
        return { how: 'dialog', interactMs: Date.now() - started };
      } catch {
        try {
          await exec('xdotool', ['key', 'Escape'], { env: process.env });
        } catch {
          // Dialog was not focused.
        }
      }
    }
  }
  await seedRecents(driver, [folder]);
  await clickRecent(driver, folder);
  return { how: 'seeded-recent', interactMs: Date.now() - started };
}

export async function waitForGrid(driver, timeoutMs) {
  const start = Date.now();
  let gridMs = null;
  let firstThumbMs = null;
  let visibleMs = null;
  let last = null;
  while (Date.now() - start < timeoutMs) {
    try {
      last = await snapshot(driver);
    } catch (error) {
      last = { error: String(error) };
      await sleep(300);
      continue;
    }
    const elapsed = Date.now() - start;
    if (gridMs === null && ((last.photoCount && last.photoCount > 0) || last.emptyFolder || last.cells > 0)) gridMs = elapsed;
    if (firstThumbMs === null && last.loaded > 0) firstThumbMs = elapsed;
    const settled =
      last.emptyFolder ||
      (last.cells > 0 &&
        last.loading === 0 &&
        last.loaded + last.failed >= last.cells &&
        (last.loaded > 0 || last.failed === last.cells));
    if (gridMs !== null && settled) {
      visibleMs = elapsed;
      break;
    }
    await sleep(250);
  }
  return { gridMs, firstThumbMs, visibleMs, snap: last };
}

export async function sweepFailures(driver, steps = 8) {
  const names = new Set();
  for (let i = 0; i <= steps; i++) {
    await driver.executeScript(
      `const main = document.querySelector('main');
       if (main) main.scrollTop = (main.scrollHeight - main.clientHeight) * ${i / steps};`,
    );
    await sleep(350);
    const snap = await snapshot(driver);
    for (const name of snap.failedNames || []) names.add(name);
  }
  await driver.executeScript(`const main = document.querySelector('main'); if (main) main.scrollTop = 0;`);
  return [...names];
}

export async function frameStats(driver) {
  return driver.executeScript(`return (() => {
    const frames = (window.__e2e && window.__e2e.frames) || [];
    if (!frames.length) return { n: 0 };
    const sorted = [...frames].sort((a, b) => a - b);
    const pick = (p) => sorted[Math.min(sorted.length - 1, Math.floor(p * (sorted.length - 1)))];
    const over50 = frames.filter((v) => v > 50).length;
    return {
      n: frames.length,
      p50: Math.round(pick(0.5) * 10) / 10,
      p95: Math.round(pick(0.95) * 10) / 10,
      max: Math.round(sorted[sorted.length - 1] * 10) / 10,
      over50,
    };
  })()`);
}

export async function longTasks(driver, minMs = 50) {
  return driver.executeScript(
    `const rows = (window.__e2e && window.__e2e.longtasks) || [];
     return rows.filter((row) => row.duration >= arguments[0]).map((row) => ({
       duration: Math.round(row.duration),
       start: Math.round(row.start),
       name: row.name,
     }));`,
    minMs,
  );
}

export async function resetFrames(driver) {
  await driver.executeScript(
    'if (window.__e2e) { window.__e2e.frames = []; window.__e2e.longtasks = []; }',
  );
}

export async function pressKeys(driver, keys, times) {
  const started = Date.now();
  for (let i = 0; i < times; i += 1) {
    await driver.actions().sendKeys(keys).perform();
  }
  return Date.now() - started;
}

export { Key, By };

export function logLine(file, row) {
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.appendFileSync(file, JSON.stringify(row) + '\n');
}

export function setScenario(file, name) {
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, name + '\n');
}

export async function perfRecord(pid, seconds, dataFile) {
  if (!pid) return false;
  try {
    await exec(
      'perf',
      ['record', '-F', '99', '-g', '--call-graph', 'fp', '-p', String(pid), '-o', dataFile, '--', 'sleep', String(seconds)],
      { timeout: (seconds + 20) * 1000 },
    );
    return true;
  } catch {
    return false;
  }
}

export function appPid() {
  try {
    const out = execFileSync('pgrep', ['-n', '-x', 'lixa-gallery'], { encoding: 'utf8' });
    return Number(out.trim()) || null;
  } catch {
    return null;
  }
}
