// Parse a release-please CHANGELOG.md and split user-facing notes
// from development-only ones.
//
// Heuristic (edit the two lists below when it mis-files a note):
// - A conventional-commit scope in DEV_SCOPES is development-only.
// - Otherwise the subject is development-only if it matches DEV_PATTERNS
//   (CI, Nix, Cachix, refactors, tooling, benchmarks, GitHub Actions).
// Everything else stays in its original Features / Bug Fixes group.
// release-please currently records CI and Nix work under "Features"
// when the commit was typed as feat, so the section title is not enough.

const DEV_SCOPES = new Set([
  'ci',
  'build',
  'chore',
  'deps',
  'dep',
  'dependencies',
  'nix',
  'dx',
  'test',
  'tests',
  'tooling',
  'tools',
  'refactor',
  'release',
  'meta',
  'github',
  'workflow',
  'workflows',
  'devops',
  'lint',
  'format',
  'style',
  'docs',
]);

const DEV_PATTERNS = [
  /\bci\b/i,
  /github actions?/i,
  /\bworkflows?\b/i,
  /\bnix(?:os)?\b/i,
  /\bcachix\b/i,
  /\brefactor/i,
  /\btooling\b/i,
  /\byarn\b/i,
  /\blockfile\b/i,
  /\bprettier\b/i,
  /\beslint\b/i,
  /\bcommitlint\b/i,
  /release-please/i,
  /cargo (?:fmt|clippy)/i,
  /dev(?:elopment)? environment/i,
  /\bbenchmark/i,
  /\bgh actions?\b/i,
];

const VERSION_RE = /^## \[([^\]]+)\]\(([^)]+)\) \(([0-9]{4}-[0-9]{2}-[0-9]{2})\)$/;
const SECTION_RE = /^### (.+)$/;
const COMMIT_RE = /\s+\(\[([0-9a-f]+)\]\(([^)]+)\)\)\s*$/;
const SCOPE_RE = /^\*\*([^*]+):\*\*\s*(.*)$/;

export function isDeveloperChange(item) {
  if (item.scope && DEV_SCOPES.has(item.scope.toLowerCase())) return true;
  const haystack = `${item.scope || ''} ${item.text || ''}`;
  return DEV_PATTERNS.some((pattern) => pattern.test(haystack));
}

function parseItem(raw) {
  let text = raw.trim();
  let sha = null;
  let commitUrl = null;
  const commit = COMMIT_RE.exec(text);
  if (commit) {
    sha = commit[1];
    commitUrl = commit[2];
    text = text.slice(0, commit.index).trim();
  }
  let scope = null;
  const scoped = SCOPE_RE.exec(text);
  if (scoped) {
    scope = scoped[1].trim().toLowerCase();
    text = scoped[2].trim();
  }
  return { scope, text, sha, commitUrl };
}

export function parseChangelog(markdown) {
  const versions = [];
  let current = null;
  let section = null;

  for (const line of markdown.split(/\r?\n/)) {
    const version = VERSION_RE.exec(line);
    if (version) {
      current = {
        version: version[1],
        compareUrl: version[2],
        date: version[3],
        sections: [],
      };
      versions.push(current);
      section = null;
      continue;
    }
    if (!current) continue;
    const heading = SECTION_RE.exec(line);
    if (heading) {
      section = { title: heading[1].trim(), items: [] };
      current.sections.push(section);
      continue;
    }
    if (section && line.startsWith('* ')) {
      section.items.push(parseItem(line.slice(2)));
    }
  }

  return versions;
}

export function classifyVersion(version) {
  const userSections = [];
  const devItems = [];
  for (const section of version.sections) {
    const userItems = [];
    for (const item of section.items) {
      if (isDeveloperChange(item)) {
        devItems.push({ ...item, section: section.title });
      } else {
        userItems.push(item);
      }
    }
    if (userItems.length) userSections.push({ title: section.title, items: userItems });
  }
  return { userSections, devItems };
}

function esc(value) {
  return String(value)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}

function scopeLabel(scope) {
  if (scope.length <= 3) return scope.toUpperCase();
  return scope.charAt(0).toUpperCase() + scope.slice(1);
}

function sentence(item) {
  const body = item.scope ? `${scopeLabel(item.scope)}: ${item.text}` : item.text;
  if (!body) return '';
  return body.charAt(0).toUpperCase() + body.slice(1);
}

function formatDate(iso) {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(iso || '');
  if (!match) return iso || '';
  const months = [
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
  return `${Number(match[3])} ${months[Number(match[2]) - 1]} ${match[1]}`;
}

function renderItem(item, kind) {
  const label = sentence(item);
  const kindHtml = kind ? `<i class="kind">${esc(kind)}</i> ` : '';
  const link = item.commitUrl
    ? `<a class="hash" href="${esc(item.commitUrl)}">${esc(item.sha)}</a>`
    : '';
  return `<li><span>${kindHtml}${esc(label)}</span>${link}</li>`;
}

function renderVersion(version) {
  const { userSections, devItems } = classifyVersion(version);
  const userHtml = userSections.length
    ? userSections
        .map(
          (section) =>
            `<h4>${esc(section.title)}</h4><ul>${section.items.map((item) => renderItem(item)).join('')}</ul>`,
        )
        .join('')
    : '<p class="quiet">No user-facing changes in this release.</p>';
  const devHtml = devItems.length
    ? `<details class="dev"><summary>Developer changes <span class="count">${devItems.length}</span></summary><ul>${devItems
        .map((item) => renderItem(item, item.section))
        .join('')}</ul></details>`
    : '';
  const id = `v${version.version.replace(/[^0-9A-Za-z.-]/g, '')}`;
  return `<article class="release" id="${esc(id)}">
  <div class="release-mark">
    <h3><a href="${esc(version.compareUrl)}">${esc(version.version)}</a></h3>
    <time datetime="${esc(version.date)}">${esc(formatDate(version.date))}</time>
  </div>
  <div class="changes">${userHtml}${devHtml}</div>
</article>`;
}

export function renderChangelog(versions) {
  if (!versions.length) {
    return '<p class="quiet">No releases in CHANGELOG.md yet.</p>';
  }
  return versions.map(renderVersion).join('\n');
}
