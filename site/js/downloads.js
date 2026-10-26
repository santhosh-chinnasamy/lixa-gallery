// Release metadata and OS detection for the landing page.
// Imported by the build script and by the browser (type="module").

export const REPO = 'santhosh-chinnasamy/lixa-gallery';
export const FALLBACK_URL = `https://github.com/${REPO}/releases/latest`;
export const LATEST_API = `https://api.github.com/repos/${REPO}/releases/latest`;

const MONTHS = [
  'Jan',
  'Feb',
  'Mar',
  'Apr',
  'May',
  'Jun',
  'Jul',
  'Aug',
  'Sep',
  'Oct',
  'Nov',
  'Dec',
];

export function formatReleaseDate(iso) {
  if (!iso) return '';
  const match = /^(\d{4})-(\d{2})-(\d{2})/.exec(iso);
  if (!match) return iso;
  const day = Number(match[3]);
  const month = MONTHS[Number(match[2]) - 1];
  if (!month || !day) return iso;
  return `${day} ${month} ${match[1]}`;
}

function pick(assets, pred) {
  return assets.find(pred)?.browser_download_url || null;
}

/**
 * Map a GitHub release payload to the installers this site links.
 * Updater bundles (*.app.tar.gz) are intentionally left out.
 */
export function parseGithubRelease(data) {
  const assets = Array.isArray(data?.assets) ? data.assets : [];
  const named = assets.filter((asset) => asset && asset.name && asset.browser_download_url);
  const exes = named.filter((asset) => /\.exe$/i.test(asset.name));
  const setup = exes.find((asset) => /setup/i.test(asset.name)) || exes[0];
  const tag = data?.tag_name || '';
  const versionMatch = tag.match(/(\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?)/);

  return {
    version: versionMatch ? versionMatch[1] : '',
    date: (data?.published_at || '').slice(0, 10),
    htmlUrl: data?.html_url || '',
    assets: {
      'macos-arm': pick(
        named,
        (asset) => /\.dmg$/i.test(asset.name) && /aarch64|arm64/i.test(asset.name),
      ),
      'macos-intel': pick(
        named,
        (asset) =>
          /\.dmg$/i.test(asset.name) &&
          /(x64|x86_64|amd64)/i.test(asset.name) &&
          !/aarch64|arm64/i.test(asset.name),
      ),
      'windows-exe': setup?.browser_download_url || null,
      'windows-msi': pick(named, (asset) => /\.msi$/i.test(asset.name)),
      'linux-deb': pick(named, (asset) => /\.deb$/i.test(asset.name)),
      'linux-rpm': pick(named, (asset) => /\.rpm$/i.test(asset.name)),
      'linux-appimage': pick(named, (asset) => /\.appimage$/i.test(asset.name)),
    },
  };
}

export function mergeRelease(base, fresh) {
  const assets = { ...(base?.assets || {}) };
  for (const [key, url] of Object.entries(fresh?.assets || {})) {
    if (url) assets[key] = url;
  }
  return {
    version: fresh?.version || base?.version || '',
    date: fresh?.date || base?.date || '',
    htmlUrl: fresh?.htmlUrl || base?.htmlUrl || '',
    fallbackUrl: base?.fallbackUrl || FALLBACK_URL,
    assets,
  };
}

export function detectOs(ua) {
  const text = ua || '';
  if (/Windows/i.test(text)) return 'windows';
  if (/Mac OS X|Macintosh/i.test(text)) return 'mac';
  if (/Android/i.test(text)) return 'unknown';
  if (/Linux/i.test(text)) return 'linux';
  return 'unknown';
}

/**
 * Apple silicon vs Intel. `architecture` comes from userAgentData;
 * `renderer` is the WebGL unmasked renderer, used when the high-entropy
 * hint is missing (Safari). Null means we could not tell, so both Mac
 * disk images should be offered.
 */
export function archFromSignals({ architecture = '', renderer = '' } = {}) {
  if (/arm/i.test(architecture)) return 'arm';
  if (/x86|x64/i.test(architecture)) return 'intel';
  if (/Apple M\d|Apple GPU/i.test(renderer)) return 'arm';
  if (/Intel/i.test(renderer)) return 'intel';
  return null;
}

export function highlightKeys(os, arch) {
  if (os === 'windows') return ['windows-exe'];
  if (os === 'linux') return ['linux-deb', 'linux-rpm', 'linux-appimage'];
  if (os === 'mac' && arch === 'arm') return ['macos-arm'];
  if (os === 'mac' && arch === 'intel') return ['macos-intel'];
  if (os === 'mac') return ['macos-arm', 'macos-intel'];
  return [];
}

export function hintText(highlighted) {
  if (highlighted.length === 1) return 'this machine';
  if (highlighted.length > 1) return 'your platform';
  return '';
}

export function primaryChoice(os, arch) {
  if (os === 'windows') return { key: 'windows-exe', label: 'Download for Windows' };
  if (os === 'linux') return { key: 'linux-appimage', label: 'Download for Linux' };
  if (os === 'mac' && arch === 'arm') return { key: 'macos-arm', label: 'Download for macOS' };
  if (os === 'mac' && arch === 'intel') return { key: 'macos-intel', label: 'Download for macOS' };
  if (os === 'mac') return { key: null, label: 'Download for macOS' };
  return { key: null, label: 'Download' };
}

function webglRenderer() {
  try {
    const canvas = document.createElement('canvas');
    const gl = canvas.getContext('webgl') || canvas.getContext('experimental-webgl');
    if (!gl) return '';
    const ext = gl.getExtension('WEBGL_debug_renderer_info');
    if (!ext) return '';
    return gl.getParameter(ext.UNMASKED_RENDERER_WEBGL) || '';
  } catch {
    return '';
  }
}

function readBaked() {
  const fallback = {
    version: '',
    date: '',
    htmlUrl: FALLBACK_URL,
    fallbackUrl: FALLBACK_URL,
    assets: {},
  };
  const el = document.getElementById('release-data');
  if (!el) return fallback;
  try {
    return mergeRelease(fallback, JSON.parse(el.textContent || '{}'));
  } catch {
    return fallback;
  }
}

export function applyDom(release, os, arch) {
  document.querySelectorAll('[data-release-version]').forEach((el) => {
    if (release.version) el.textContent = release.version;
  });
  const pretty = formatReleaseDate(release.date);
  document.querySelectorAll('[data-release-date]').forEach((el) => {
    if (pretty) el.textContent = pretty;
    if (release.date) el.setAttribute('datetime', release.date);
  });

  document.querySelectorAll('[data-dl]').forEach((el) => {
    const key = el.getAttribute('data-dl');
    el.setAttribute('href', release.assets[key] || release.fallbackUrl || FALLBACK_URL);
  });

  const highlighted = highlightKeys(os, arch);
  const hint = hintText(highlighted);
  document.querySelectorAll('[data-dl]').forEach((el) => {
    el.classList.toggle('is-yours', highlighted.includes(el.getAttribute('data-dl')));
  });
  document.querySelectorAll('[data-hint]').forEach((el) => {
    const on = highlighted.includes(el.getAttribute('data-hint'));
    el.textContent = on ? hint : '';
    el.hidden = !on;
  });

  const primary = document.getElementById('primary-download');
  if (primary) {
    const choice = primaryChoice(os, arch);
    primary.textContent = choice.label;
    primary.href =
      choice.key && release.assets[choice.key] ? release.assets[choice.key] : '#install';
  }
}

async function detectMacArch() {
  let architecture = '';
  try {
    if (navigator.userAgentData?.getHighEntropyValues) {
      const values = await navigator.userAgentData.getHighEntropyValues(['architecture']);
      architecture = values?.architecture || '';
    }
  } catch {
    architecture = '';
  }
  return archFromSignals({ architecture, renderer: webglRenderer() });
}

async function boot() {
  const baked = readBaked();
  const os = detectOs(navigator.userAgent);
  const arch = os === 'mac' ? await detectMacArch() : null;
  applyDom(baked, os, arch);

  try {
    const response = await fetch(LATEST_API, {
      headers: { Accept: 'application/vnd.github+json' },
    });
    if (!response.ok) return;
    const fresh = parseGithubRelease(await response.json());
    applyDom(mergeRelease(baked, fresh), os, arch);
  } catch {
    // Baked asset URLs stay. Missing ones already point at /releases/latest.
  }
}

if (typeof document !== 'undefined') {
  boot();
}
