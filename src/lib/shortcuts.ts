import { writable } from 'svelte/store';

export type ShortcutContext =
  | 'app'
  | 'grid'
  | 'viewer'
  | 'slideshow'
  | 'favourites';

export type ShortcutLayer = 'slideshow' | 'viewer' | 'favourites' | 'grid';

export interface Shortcut {
  id: string;
  label: string;
  detail?: string;
  /** Each entry is one way to press the shortcut. Each way is a row of keycaps. */
  chords: string[][];
  contexts: ShortcutContext[];
}

export const SHORTCUTS: readonly Shortcut[] = [
  {
    id: 'pick',
    label: 'Pick',
    detail: 'L and F do the same thing',
    chords: [['P'], ['L'], ['F']],
    contexts: ['grid', 'viewer', 'slideshow'],
  },
  {
    id: 'reject',
    label: 'Reject',
    chords: [['X']],
    contexts: ['grid', 'viewer', 'slideshow'],
  },
  {
    id: 'unflag',
    label: 'Unflag',
    chords: [['U']],
    contexts: ['grid', 'viewer', 'slideshow'],
  },
  {
    id: 'move-left',
    label: 'Previous column',
    chords: [['←']],
    contexts: ['grid'],
  },
  {
    id: 'move-right',
    label: 'Next column',
    chords: [['→']],
    contexts: ['grid'],
  },
  {
    id: 'move-up',
    label: 'Previous row',
    chords: [['↑']],
    contexts: ['grid'],
  },
  {
    id: 'move-down',
    label: 'Next row',
    chords: [['↓']],
    contexts: ['grid'],
  },
  {
    id: 'home',
    label: 'First photo',
    chords: [['Home']],
    contexts: ['grid'],
  },
  {
    id: 'end',
    label: 'Last photo',
    chords: [['End']],
    contexts: ['grid'],
  },
  {
    id: 'page-up',
    label: 'Page up',
    chords: [['PgUp']],
    contexts: ['grid'],
  },
  {
    id: 'page-down',
    label: 'Page down',
    chords: [['PgDn']],
    contexts: ['grid'],
  },
  {
    id: 'open-viewer',
    label: 'Open the frame',
    chords: [['Enter'], ['Space']],
    contexts: ['grid'],
  },
  {
    id: 'viewer-prev',
    label: 'Previous photo',
    chords: [['←'], ['[']],
    contexts: ['viewer'],
  },
  {
    id: 'viewer-next',
    label: 'Next photo',
    chords: [['→'], [']']],
    contexts: ['viewer'],
  },
  {
    id: 'zoom-in',
    label: 'Zoom in',
    detail: 'With or without Ctrl',
    chords: [['+']],
    contexts: ['viewer'],
  },
  {
    id: 'zoom-out',
    label: 'Zoom out',
    detail: 'With or without Ctrl',
    chords: [['−']],
    contexts: ['viewer'],
  },
  {
    id: 'zoom-reset',
    label: 'Reset zoom',
    detail: 'With or without Ctrl',
    chords: [['0']],
    contexts: ['viewer'],
  },
  {
    id: 'info',
    label: 'Photo details',
    chords: [['I']],
    contexts: ['viewer', 'slideshow'],
  },
  {
    id: 'start-slideshow',
    label: 'Start slideshow',
    chords: [['S']],
    contexts: ['grid', 'viewer'],
  },
  {
    id: 'play-pause',
    label: 'Play / pause',
    chords: [['Space']],
    contexts: ['slideshow'],
  },
  {
    id: 'slide-next',
    label: 'Next slide',
    chords: [['→'], ['PgDn']],
    contexts: ['slideshow'],
  },
  {
    id: 'slide-prev',
    label: 'Previous slide',
    chords: [['←'], ['PgUp']],
    contexts: ['slideshow'],
  },
  {
    id: 'shuffle',
    label: 'Shuffle',
    chords: [['S']],
    contexts: ['slideshow'],
  },
  {
    id: 'repeat',
    label: 'Repeat',
    chords: [['R']],
    contexts: ['slideshow'],
  },
  {
    id: 'open-folder',
    label: 'Open folder',
    chords: [['O']],
    contexts: ['app', 'grid'],
  },
  {
    id: 'export',
    label: 'Export picks',
    chords: [['E']],
    contexts: ['favourites'],
  },
  {
    id: 'sidebar',
    label: 'Toggle sidebar',
    chords: [
      ['Ctrl', 'B'],
      ['⌘', 'B'],
    ],
    contexts: ['app', 'grid', 'favourites'],
  },
  {
    id: 'fullscreen',
    label: 'Fullscreen',
    chords: [['F11']],
    contexts: ['app', 'grid', 'viewer', 'slideshow', 'favourites'],
  },
  {
    id: 'close',
    label: 'Close',
    chords: [['Esc']],
    contexts: ['viewer', 'slideshow'],
  },
  {
    id: 'help',
    label: 'Shortcut list',
    chords: [['?']],
    contexts: ['app', 'grid', 'viewer', 'slideshow', 'favourites'],
  },
];

const CONTEXT_LABEL: Record<ShortcutContext, string> = {
  app: 'Gallery',
  grid: 'Grid',
  viewer: 'Viewer',
  slideshow: 'Slideshow',
  favourites: 'Favourites',
};

export function contextLabel(context: ShortcutContext): string {
  return CONTEXT_LABEL[context];
}

export function shortcutsFor(context: ShortcutContext): Shortcut[] {
  if (context === 'favourites') {
    const grid = SHORTCUTS.filter(
      (shortcut) =>
        shortcut.contexts.includes('grid') ||
        shortcut.contexts.includes('favourites'),
    );
    return grid;
  }
  return SHORTCUTS.filter((shortcut) => shortcut.contexts.includes(context));
}

function plainKey(event: KeyboardEvent, ...keys: string[]): boolean {
  if (event.metaKey || event.ctrlKey || event.altKey) return false;
  const pressed = event.key.length === 1 ? event.key.toLowerCase() : event.key;
  return keys.includes(pressed);
}

const MATCHERS: Record<string, (event: KeyboardEvent) => boolean> = {
  pick: (event) => plainKey(event, 'p', 'l', 'f'),
  reject: (event) => plainKey(event, 'x'),
  unflag: (event) => plainKey(event, 'u'),
  'move-left': (event) => plainKey(event, 'ArrowLeft'),
  'move-right': (event) => plainKey(event, 'ArrowRight'),
  'move-up': (event) => plainKey(event, 'ArrowUp'),
  'move-down': (event) => plainKey(event, 'ArrowDown'),
  home: (event) => plainKey(event, 'Home'),
  end: (event) => plainKey(event, 'End'),
  'page-up': (event) => plainKey(event, 'PageUp'),
  'page-down': (event) => plainKey(event, 'PageDown'),
  'open-viewer': (event) => plainKey(event, 'Enter', ' '),
  'viewer-prev': (event) => plainKey(event, 'ArrowLeft', '['),
  'viewer-next': (event) => plainKey(event, 'ArrowRight', ']'),
  'zoom-in': (event) =>
    !event.altKey && (event.key === '+' || event.key === '='),
  'zoom-out': (event) =>
    !event.altKey && (event.key === '-' || event.key === '_'),
  'zoom-reset': (event) => !event.altKey && event.key === '0',
  info: (event) => plainKey(event, 'i'),
  'start-slideshow': (event) => plainKey(event, 's'),
  'play-pause': (event) => plainKey(event, ' '),
  'slide-next': (event) => plainKey(event, 'ArrowRight', 'PageDown'),
  'slide-prev': (event) => plainKey(event, 'ArrowLeft', 'PageUp'),
  shuffle: (event) => plainKey(event, 's'),
  repeat: (event) => plainKey(event, 'r'),
  'open-folder': (event) => plainKey(event, 'o'),
  export: (event) => plainKey(event, 'e'),
  fullscreen: (event) => event.key === 'F11',
  close: (event) => plainKey(event, 'Escape'),
  help: (event) => plainKey(event, '?'),
};

export function shortcutMatches(event: KeyboardEvent, id: string): boolean {
  return MATCHERS[id]?.(event) ?? false;
}

export function isTextEntry(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  const tag = target.tagName;
  return (
    tag === 'INPUT' ||
    tag === 'TEXTAREA' ||
    tag === 'SELECT' ||
    target.isContentEditable
  );
}

export const shortcutHelpOpen = writable(false);
export const shortcutContext = writable<ShortcutContext>('app');

const layerDepth: Record<ShortcutLayer, number> = {
  slideshow: 0,
  viewer: 0,
  favourites: 0,
  grid: 0,
};

export function setShortcutLayer(layer: ShortcutLayer, active: boolean) {
  layerDepth[layer] += active ? 1 : -1;
  if (layerDepth[layer] < 0) layerDepth[layer] = 0;

  if (layerDepth.slideshow > 0) shortcutContext.set('slideshow');
  else if (layerDepth.viewer > 0) shortcutContext.set('viewer');
  else if (layerDepth.favourites > 0) shortcutContext.set('favourites');
  else if (layerDepth.grid > 0) shortcutContext.set('grid');
  else shortcutContext.set('app');
}

export function toggleShortcutHelp() {
  shortcutHelpOpen.update((open) => !open);
}

export function closeShortcutHelp() {
  shortcutHelpOpen.set(false);
}
