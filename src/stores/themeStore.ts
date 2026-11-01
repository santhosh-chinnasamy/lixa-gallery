import { derived, get, writable } from 'svelte/store';

export type ThemePreference = 'system' | 'light' | 'dark';

const STORAGE_KEY = 'theme';

function readPreference(): ThemePreference {
  if (typeof localStorage === 'undefined') return 'system';
  const stored = localStorage.getItem(STORAGE_KEY);
  if (stored === 'light' || stored === 'dark' || stored === 'system') {
    return stored;
  }
  return 'system';
}

function systemIsDark(): boolean {
  if (typeof window === 'undefined') return false;
  return window.matchMedia('(prefers-color-scheme: dark)').matches;
}

const preference = writable<ThemePreference>(readPreference());
const systemDark = writable(systemIsDark());

export const themePreference = {
  subscribe: preference.subscribe,
  set(value: ThemePreference) {
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem(STORAGE_KEY, value);
    }
    preference.set(value);
    applyTheme(value);
  },
};

export const themeIsDark = derived(
  [preference, systemDark],
  ([$preference, $systemDark]) =>
    $preference === 'dark' || ($preference === 'system' && $systemDark),
);

export function applyTheme(value: ThemePreference = get(preference)) {
  if (typeof document === 'undefined') return;
  const dark = value === 'dark' || (value === 'system' && systemIsDark());
  document.documentElement.classList.toggle('dark', dark);
  document.documentElement.style.colorScheme = dark ? 'dark' : 'light';
}

export function initTheme(): () => void {
  if (typeof window === 'undefined') return () => {};
  systemDark.set(systemIsDark());
  applyTheme(get(preference));

  const media = window.matchMedia('(prefers-color-scheme: dark)');
  const onChange = () => {
    systemDark.set(media.matches);
    if (get(preference) === 'system') applyTheme('system');
  };
  media.addEventListener('change', onChange);
  return () => media.removeEventListener('change', onChange);
}
