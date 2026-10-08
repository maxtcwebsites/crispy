import type { Hotkey, KeyInfo, ModKey, Os, Peer, Screen, Transfer } from './types';

// ---- numbers ------------------------------------------------------------------------------------

/** Decimal units, like Finder and Explorer's "size on disk" columns. */
export function formatBytes(n: number, digits?: number): string {
  if (!Number.isFinite(n) || n <= 0) return '0 KB';
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  let i = 0;
  let v = n;
  while (v >= 1000 && i < units.length - 1) {
    v /= 1000;
    i++;
  }
  if (i === 0) return `${Math.round(v)} B`;
  const d = digits ?? (v >= 100 ? 0 : v >= 10 ? 1 : 1);
  return `${v.toFixed(d).replace(/\.0$/, '')} ${units[i]}`;
}

export function formatSpeed(bytesPerSec: number): string {
  if (!bytesPerSec || bytesPerSec < 1) return '—';
  return `${formatBytes(bytesPerSec)}/s`;
}

export function formatDuration(secs: number): string {
  if (!Number.isFinite(secs) || secs < 0) return '—';
  if (secs < 1) return 'less than a second';
  if (secs < 60) return `${Math.ceil(secs)} s`;
  const m = Math.floor(secs / 60);
  if (m < 60) return `${m} min`;
  const h = Math.floor(m / 60);
  return `${h} h ${m % 60} min`;
}

export function formatUptime(secs: number): string {
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  if (h > 0) return `${h}h ${m.toString().padStart(2, '0')}m`;
  const s = Math.floor(secs % 60);
  return `${m}m ${s.toString().padStart(2, '0')}s`;
}

export function timeAgo(ms: number | null, now = Date.now()): string {
  if (!ms) return 'never';
  const s = Math.max(0, (now - ms) / 1000);
  if (s < 45) return 'just now';
  if (s < 90) return 'a minute ago';
  const m = Math.round(s / 60);
  if (m < 60) return `${m} min ago`;
  const h = Math.round(m / 60);
  if (h < 24) return `${h} h ago`;
  const d = Math.round(h / 24);
  if (d === 1) return 'yesterday';
  if (d < 30) return `${d} days ago`;
  return new Date(ms).toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
}

export function clamp(v: number, lo: number, hi: number): number {
  return Math.min(hi, Math.max(lo, v));
}

export function plural(n: number, one: string, many = `${one}s`): string {
  return `${n} ${n === 1 ? one : many}`;
}

// ---- OS ---------------------------------------------------------------------------------------

export function osName(os: Os): string {
  return os === 'macos' ? 'macOS' : os === 'windows' ? 'Windows' : 'Linux';
}

/** "This Mac" / "This PC" / "This computer". */
export function thisDeviceLabel(os: Os): string {
  return os === 'macos' ? 'This Mac' : os === 'windows' ? 'This PC' : 'This computer';
}

// ---- screens ----------------------------------------------------------------------------------

export interface Bounds {
  x: number;
  y: number;
  w: number;
  h: number;
}

export function screenBounds(screens: Screen[]): Bounds {
  if (!screens.length) return { x: 0, y: 0, w: 0, h: 0 };
  const minX = Math.min(...screens.map((s) => s.x));
  const minY = Math.min(...screens.map((s) => s.y));
  const maxX = Math.max(...screens.map((s) => s.x + s.width));
  const maxY = Math.max(...screens.map((s) => s.y + s.height));
  return { x: minX, y: minY, w: maxX - minX, h: maxY - minY };
}

export function displaysSummary(screens: Screen[]): string {
  if (!screens.length) return 'No displays';
  const primary = screens.find((s) => s.primary) ?? screens[0];
  const res = `${primary.width}×${primary.height}`;
  return screens.length === 1 ? `1 display · ${res}` : `${screens.length} displays · ${res}`;
}

// ---- keyboard ---------------------------------------------------------------------------------

export const MOD_ORDER: Record<Os, ModKey[]> = {
  macos: ['ctrl', 'alt', 'shift', 'meta'],
  windows: ['ctrl', 'alt', 'shift', 'meta'],
  linux: ['ctrl', 'alt', 'shift', 'meta'],
};

/** Short label for a modifier on a given platform's keyboard. */
export function modLabel(m: ModKey, os: Os, long = false): string {
  if (os === 'macos') {
    if (long) return { ctrl: '⌃ Control', alt: '⌥ Option', shift: '⇧ Shift', meta: '⌘ Command' }[m];
    return { ctrl: '⌃', alt: '⌥', shift: '⇧', meta: '⌘' }[m];
  }
  const meta = os === 'windows' ? 'Win' : 'Super';
  return { ctrl: 'Ctrl', alt: 'Alt', shift: 'Shift', meta }[m];
}

const MAC_KEY_SYMBOLS: Record<string, string> = {
  Return: '↩',
  Backspace: '⌫',
  Delete: '⌦',
  Tab: '⇥',
  Esc: '⎋',
  'Caps Lock': '⇪',
};

export function keyLabel(hid: number, keys: KeyInfo[], os: Os): string {
  if (!hid) return '';
  const label = keys.find((k) => k.hid === hid)?.label ?? `#${hid.toString(16)}`;
  if (os === 'macos' && MAC_KEY_SYMBOLS[label]) return MAC_KEY_SYMBOLS[label];
  return label;
}

/** Keycaps for a hotkey, in platform order. Empty array = not set. */
export function hotkeyParts(h: Hotkey | null | undefined, keys: KeyInfo[], os: Os): string[] {
  if (!h || !h.key) return [];
  const parts = MOD_ORDER[os].filter((m) => h[m]).map((m) => modLabel(m, os));
  parts.push(keyLabel(h.key, keys, os));
  return parts;
}

export function hotkeyText(h: Hotkey | null | undefined, keys: KeyInfo[], os: Os): string {
  const parts = hotkeyParts(h, keys, os);
  if (!parts.length) return 'Not set';
  return os === 'macos' ? parts.join('') : parts.join(' + ');
}

// ---- peers & transfers --------------------------------------------------------------------------

export function statusLabel(p: Peer): string {
  if (!p.enabled && p.paired) return 'Disabled';
  switch (p.status) {
    case 'connected':
      return 'Connected';
    case 'connecting':
      return 'Connecting…';
    case 'offline':
      return 'Offline';
    case 'nearby':
      return 'Nearby';
  }
}

export function transferTitle(t: Transfer): string {
  if (t.purpose === 'clipboard') return t.files[0]?.name ?? 'Clipboard';
  const names = t.files.slice(0, 2).map((f) => f.name);
  if (t.fileCount <= 1) return names[0] ?? '1 file';
  const rest = t.fileCount - names.length;
  if (rest <= 0) return names.join(', ');
  return `${names.join(', ')} and ${rest} more`;
}

export function transferEta(t: Transfer): number | null {
  if (t.state !== 'active' || !t.bytesPerSec) return null;
  return Math.max(0, (t.totalBytes - t.doneBytes) / t.bytesPerSec);
}

export const isFinished = (t: Transfer) =>
  t.state === 'done' || t.state === 'failed' || t.state === 'cancelled' || t.state === 'declined';

export function basename(path: string): string {
  const parts = path.split(/[\\/]/).filter(Boolean);
  return parts[parts.length - 1] ?? path;
}

/** Collapse the home folder to ~ for display. */
export function prettyPath(path: string): string {
  return path
    .replace(/^\/Users\/[^/]+/, '~')
    .replace(/^\/home\/[^/]+/, '~')
    .replace(/^[A-Z]:\\Users\\[^\\]+/i, '~');
}

export function sleep(ms: number) {
  return new Promise((r) => setTimeout(r, ms));
}

export function debounce<A extends unknown[]>(fn: (...args: A) => void, ms: number) {
  let t: ReturnType<typeof setTimeout> | undefined;
  const d = (...args: A) => {
    clearTimeout(t);
    t = setTimeout(() => fn(...args), ms);
  };
  d.cancel = () => clearTimeout(t);
  return d;
}
