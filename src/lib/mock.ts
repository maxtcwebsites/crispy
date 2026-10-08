// In-browser mock of the Crispy backend, used whenever the UI runs outside Tauri.
//
// A small but believable desk: this Mac, a connected Windows tower with two displays, an offline
// iMac and a Linux laptop that is nearby but not paired yet. The cursor glides from the Mac onto
// the PC and back, a transfer progresses, pairing works with the code 482913.
//
// URL parameters (handy for screenshots):
//   ?os=windows|linux     which computer "this" is (switches window chrome)
//   ?theme=dark|light     force a theme        ?flavor=bbq …   force a flavor
//   ?onboarding=1         first-run onboarding
//   ?page=settings&section=keyboard            open a page / settings section
//   ?pairing=enterCode|showCode|verifying|success|failed
//   ?perm=denied          macOS Accessibility not granted
//   ?warning=1            show a sample warning banner
//   ?empty=1              no paired devices yet
//   ?send=1               simulate files handed over by the OS "Send To" menu
//   ?effects=1            pretend native window material is available
//   ?still=1              freeze the simulation (stable screenshots)
//   ?stats=1              show live statistics

import { MOCK_KEY_TABLE } from './mock-keys';
import type {
  AppInfo,
  AppState,
  DeviceInfo,
  Os,
  Pairing,
  Peer,
  Screen,
  Settings,
  Toast,
  Transfer,
} from './types';

const params = new URLSearchParams(typeof location !== 'undefined' ? location.search : '');
const flag = (k: string) => params.has(k) && params.get(k) !== '0';
const STILL = flag('still');

// ---- events ------------------------------------------------------------------------------------

type Listener = (payload: unknown) => void;
const listeners = new Map<string, Set<Listener>>();

function emit(event: string, payload: unknown) {
  const set = listeners.get(event);
  if (!set) return;
  const copy = typeof structuredClone === 'function' ? structuredClone(payload) : JSON.parse(JSON.stringify(payload));
  set.forEach((l) => l(copy));
}

export function mockListen(event: string, cb: Listener): () => void {
  if (!listeners.has(event)) listeners.set(event, new Set());
  listeners.get(event)!.add(cb);
  return () => listeners.get(event)?.delete(cb);
}

const pushState = () => emit('crispy://state', state);
const toast = (t: Toast) => emit('crispy://toast', t);

// ---- the desk ----------------------------------------------------------------------------------

const scr = (id: string, name: string, x: number, y: number, width: number, height: number, scale: number, primary: boolean): Screen => ({
  id,
  name,
  x,
  y,
  width,
  height,
  scale,
  primary,
});

interface Catalog {
  info: DeviceInfo;
  address: string;
}

const MAC: Catalog = {
  info: {
    id: 'd-7f3a91c2',
    name: "Maya's MacBook Pro",
    os: 'macos',
    version: '1.0.0',
    fingerprint: '7F3A-91C2-0B44-E81D',
    screens: [scr('1', 'Built-in Retina Display', 0, 0, 1512, 982, 2, true)],
    addresses: ['192.168.1.20:24727', '[fe80::1c2b:9aff:fe31:7d02]:24727'],
  },
  address: '192.168.1.20:24727',
};
const STUDIO: Catalog = {
  info: {
    id: 'd-4c1e08b7',
    name: 'Studio PC',
    os: 'windows',
    version: '1.0.0',
    fingerprint: '4C1E-08B7-93AF-2D65',
    screens: [
      scr('\\\\.\\DISPLAY1', 'DELL U2723QE', 0, 0, 2560, 1440, 1, true),
      scr('\\\\.\\DISPLAY2', 'LG 24GN650', 2560, 360, 1920, 1080, 1, false),
    ],
    addresses: ['192.168.1.34:24727'],
  },
  address: '192.168.1.34:24727',
};
const IMAC: Catalog = {
  info: {
    id: 'd-b2d95e10',
    name: 'Living Room iMac',
    os: 'macos',
    version: '0.9.4',
    fingerprint: 'B2D9-5E10-6A7C-F431',
    screens: [scr('1', 'iMac Retina 5K', 0, 0, 2560, 1440, 2, true)],
    addresses: ['192.168.1.41:24727'],
  },
  address: '192.168.1.41:24727',
};
const THINKPAD: Catalog = {
  info: {
    id: 'd-e83f2a6d',
    name: 'ThinkPad X1',
    os: 'linux',
    version: '1.0.0',
    fingerprint: 'E83F-2A6D-11B9-C0F2',
    screens: [scr('eDP-1', 'Built-in display', 0, 0, 1920, 1200, 1.25, true)],
    addresses: ['192.168.1.57:24727'],
  },
  address: '192.168.1.57:24727',
};

const osParam = params.get('os');
const meOs: Os = osParam === 'windows' ? 'windows' : osParam === 'linux' ? 'linux' : 'macos';
const ME = meOs === 'windows' ? STUDIO : meOs === 'linux' ? THINKPAD : MAC;
const OTHER = meOs === 'windows' ? MAC : STUDIO;
const NEARBY = meOs === 'linux' ? MAC : THINKPAD;

function peerFrom(c: Catalog, patch: Partial<Peer>): Peer {
  return {
    id: c.info.id,
    name: c.info.name,
    os: c.info.os,
    paired: true,
    status: 'connected',
    address: c.address,
    rttMs: 3,
    screens: structuredClone(c.info.screens),
    fingerprint: c.info.fingerprint,
    lastSeen: Date.now(),
    enabled: true,
    version: c.info.version,
    controlling: false,
    controllingMe: false,
    ...patch,
  };
}

const now = Date.now();
const empty = flag('empty');

const peers: Peer[] = empty
  ? [peerFrom(NEARBY, { paired: false, status: 'nearby', rttMs: null })]
  : [
      peerFrom(OTHER, { rttMs: 3 }),
      peerFrom(IMAC, { status: 'offline', rttMs: null, lastSeen: now - 1000 * 60 * 60 * 2.2, address: null }),
      peerFrom(NEARBY, { paired: false, status: 'nearby', rttMs: null }),
    ];

// Mac sits to the left of the PC, bottom edges aligned. The iMac has no position yet.
const positions: Record<string, { x: number; y: number }> =
  meOs === 'windows' || meOs === 'macos'
    ? { [MAC.info.id]: { x: 0, y: 458 }, [STUDIO.info.id]: { x: 1512, y: 0 } }
    : { [THINKPAD.info.id]: { x: 0, y: 0 } };

const homeDir = meOs === 'windows' ? 'C:\\Users\\maya' : meOs === 'linux' ? '/home/maya' : '/Users/maya';
const sep = meOs === 'windows' ? '\\' : '/';
const downloads = `${homeDir}${sep}Downloads${sep}Crispy`;

const transfers: Transfer[] = empty
  ? []
  : [
      {
        id: 't-1',
        peerId: OTHER.info.id,
        peerName: OTHER.info.name,
        direction: 'send',
        purpose: 'files',
        files: [
          { name: 'Brand assets.zip', size: 1_240_000_000 },
          { name: 'Hero film — final.mov', size: 860_000_000 },
          { name: 'Fonts.zip', size: 12_400_000 },
        ],
        fileCount: 5,
        totalBytes: 2_350_000_000,
        doneBytes: 2_350_000_000 * 0.42,
        bytesPerSec: 84_000_000,
        state: 'active',
        error: null,
        startedAt: now - 12_000,
        finishedAt: null,
        savePath: null,
        currentFile: 'Hero film — final.mov',
      },
      {
        id: 't-2',
        peerId: OTHER.info.id,
        peerName: OTHER.info.name,
        direction: 'receive',
        purpose: 'files',
        files: [
          { name: 'Lisbon — day 1', size: 160_000_000 },
          { name: 'Lisbon — day 2', size: 152_000_000 },
        ],
        fileCount: 48,
        totalBytes: 312_000_000,
        doneBytes: 0,
        bytesPerSec: 0,
        state: 'awaiting',
        error: null,
        startedAt: now - 4_000,
        finishedAt: null,
        savePath: null,
        currentFile: null,
      },
      {
        id: 't-3',
        peerId: OTHER.info.id,
        peerName: OTHER.info.name,
        direction: 'receive',
        purpose: 'files',
        files: [{ name: 'Invoice-0921.pdf', size: 248_000 }],
        fileCount: 1,
        totalBytes: 248_000,
        doneBytes: 248_000,
        bytesPerSec: 0,
        state: 'done',
        error: null,
        startedAt: now - 1000 * 60 * 6,
        finishedAt: now - 1000 * 60 * 6 + 900,
        savePath: downloads,
        currentFile: null,
      },
      {
        id: 't-4',
        peerId: OTHER.info.id,
        peerName: OTHER.info.name,
        direction: 'receive',
        purpose: 'clipboard',
        files: [{ name: 'Screenshot 2026-10-08 at 09.41.png', size: 2_140_000 }],
        fileCount: 1,
        totalBytes: 2_140_000,
        doneBytes: 2_140_000,
        bytesPerSec: 0,
        state: 'done',
        error: null,
        startedAt: now - 1000 * 60 * 22,
        finishedAt: now - 1000 * 60 * 22 + 300,
        savePath: null,
        currentFile: null,
      },
      {
        id: 't-5',
        peerId: IMAC.info.id,
        peerName: IMAC.info.name,
        direction: 'send',
        purpose: 'files',
        files: [{ name: 'Quarterly review.key', size: 96_000_000 }],
        fileCount: 1,
        totalBytes: 96_000_000,
        doneBytes: 41_000_000,
        bytesPerSec: 0,
        state: 'failed',
        error: 'Living Room iMac went offline',
        startedAt: now - 1000 * 60 * 64,
        finishedAt: now - 1000 * 60 * 63,
        savePath: null,
        currentFile: null,
      },
    ];

const pairingParam = params.get('pairing');
let pairing: Pairing | null = null;
if (pairingParam) {
  const stage = pairingParam as Pairing['stage'];
  pairing = {
    peerId: NEARBY.info.id,
    peerName: NEARBY.info.name,
    peerOs: NEARBY.info.os,
    role: stage === 'showCode' ? 'responder' : 'initiator',
    stage,
    code: stage === 'showCode' ? '482913' : null,
    error: stage === 'failed' ? "The code didn't match. Check the code shown on ThinkPad X1 and try again." : null,
  };
}

const state: AppState = {
  me: structuredClone(ME.info),
  peers,
  layout: { rev: 7, author: ME.info.id, positions },
  focus: { kind: 'local' },
  locked: false,
  paused: false,
  transfers,
  permissions: {
    accessibility: meOs !== 'macos' ? 'notRequired' : params.get('perm') === 'denied' ? 'denied' : 'granted',
    capture: params.get('perm') === 'denied' ? 'failed' : 'running',
    detail: params.get('perm') === 'denied' ? 'Accessibility access is off for Crispy.' : null,
  },
  pairing,
  warnings: flag('warning')
    ? [
        {
          id: 'w-version',
          level: 'warn',
          title: 'Living Room iMac runs an older Crispy',
          body: 'Update it to 1.0 so clipboard images and file transfers work in both directions.',
        },
      ]
    : [],
  stats: { eventsPerSec: 0, bytesSent: 18_400_000, bytesReceived: 6_200_000, uptimeSecs: 2 * 3600 + 14 * 60 },
};

// ---- settings ----------------------------------------------------------------------------------

const themeParam = params.get('theme');
const flavorParam = params.get('flavor');

let settings: Settings = {
  general: {
    deviceName: '',
    launchAtLogin: true,
    startMinimized: false,
    closeToTray: true,
    notifications: true,
    sounds: false,
    theme: themeParam === 'light' || themeParam === 'dark' ? themeParam : 'system',
    flavor: (flavorParam as Settings['general']['flavor']) ?? 'classic',
    reduceMotion: false,
    translucentWindow: true,
    onboarded: !flag('onboarding'),
  },
  switching: {
    enabled: true,
    delayMs: 0,
    doubleTap: false,
    doubleTapWindowMs: 450,
    cornerSize: 24,
    corners: { topLeft: true, topRight: true, bottomLeft: false, bottomRight: false },
    blockWhileDragging: true,
    requiredModifier: 'none',
    blockFullscreen: true,
    edgeMapping: 'direct',
    localInputTakesOver: true,
  },
  mouse: {
    pointerSpeed: 1,
    scrollSpeed: 1,
    invertVertical: false,
    invertHorizontal: false,
    smoothScrolling: true,
    relativeMovement: false,
  },
  keyboard: {
    macToPc: { ctrl: 'meta', alt: 'alt', meta: 'ctrl' },
    pcToMac: { ctrl: 'meta', alt: 'alt', meta: 'ctrl' },
    forwardMediaKeys: true,
    syncCapsLock: true,
  },
  hotkeys: {
    lockToScreen: { ctrl: true, shift: true, alt: true, meta: false, key: 0x07000f },
    bringHome: { ctrl: true, shift: true, alt: true, meta: false, key: 0x07000b },
    nextDevice: { ctrl: true, shift: true, alt: true, meta: false, key: 0x070011 },
    pauseSharing: { ctrl: true, shift: true, alt: true, meta: false, key: 0x070013 },
  },
  clipboard: {
    enabled: true,
    mode: 'instant',
    text: true,
    richText: true,
    images: true,
    files: true,
    maxImageMb: 32,
    maxFilesMb: 256,
  },
  files: {
    saveDir: '',
    autoAccept: false,
    conflict: 'rename',
    revealWhenDone: false,
    preserveTimestamps: true,
    bandwidthLimitMbps: 0,
  },
  network: {
    discovery: true,
    port: 24727,
    manualPeers: ['192.168.1.41'],
    heartbeatTimeoutSecs: 6,
  },
  advanced: { logLevel: 'info', showStats: flag('stats') },
};

function sanitize(s: Settings): Settings {
  const c = structuredClone(s);
  const fin = (v: number, d: number) => (Number.isFinite(v) ? v : d);
  c.mouse.pointerSpeed = Math.min(5, Math.max(0.1, fin(c.mouse.pointerSpeed, 1)));
  c.mouse.scrollSpeed = Math.min(10, Math.max(0.1, fin(c.mouse.scrollSpeed, 1)));
  c.switching.delayMs = Math.min(5000, Math.max(0, Math.round(c.switching.delayMs)));
  c.switching.doubleTapWindowMs = Math.min(2000, Math.max(100, Math.round(c.switching.doubleTapWindowMs)));
  c.switching.cornerSize = Math.min(500, Math.max(0, Math.round(c.switching.cornerSize)));
  if (!(c.network.port >= 1024 && c.network.port <= 65535)) c.network.port = 24727;
  c.network.heartbeatTimeoutSecs = Math.min(60, Math.max(2, Math.round(c.network.heartbeatTimeoutSecs)));
  c.general.deviceName = c.general.deviceName.trim().slice(0, 64);
  c.network.manualPeers = c.network.manualPeers.map((p) => p.trim()).filter(Boolean);
  return c;
}

const info: AppInfo = {
  version: '1.0.0',
  os: meOs,
  windowEffects: flag('effects'),
  saveDir: downloads,
  logDir: meOs === 'windows' ? `${homeDir}\\AppData\\Roaming\\Crispy\\logs` : `${homeDir}/Library/Logs/Crispy`,
};

// ---- simulation --------------------------------------------------------------------------------

function deviceScreens(id: string): Screen[] {
  if (id === state.me.id) return state.me.screens;
  return state.peers.find((p) => p.id === id)?.screens ?? [];
}

function worldRects(id: string) {
  const pos = state.layout.positions[id];
  const screens = deviceScreens(id);
  if (!pos || !screens.length) return [];
  const minX = Math.min(...screens.map((s) => s.x));
  const minY = Math.min(...screens.map((s) => s.y));
  return screens.map((s) => ({ x: pos.x + s.x - minX, y: pos.y + s.y - minY, w: s.width, h: s.height, minX, minY, pos }));
}

function deviceAt(x: number, y: number): string | null {
  for (const id of Object.keys(state.layout.positions)) {
    for (const r of worldRects(id)) if (x >= r.x && x < r.x + r.w && y >= r.y && y < r.y + r.h) return id;
  }
  return null;
}

function setFocus(id: string) {
  const local = id === state.me.id;
  const current = state.focus.kind === 'local' ? state.me.id : state.focus.peerId;
  if (current === id) return;
  state.focus = local ? { kind: 'local' } : { kind: 'remote', peerId: id };
  for (const p of state.peers) p.controlling = !local && p.id === id;
  pushState();
}

let simT = 0.18;
function pointerTick(dt: number) {
  if (!(ME === MAC || ME === STUDIO) || empty) return;
  const other = state.peers.find((p) => p.id === OTHER.info.id);
  if (!other || other.status !== 'connected' || !other.enabled) return;
  simT = (simT + dt / 15) % 1;
  // Ease between the Mac (left) and the PC's second display (right), lingering at both ends.
  const k = 0.5 - 0.5 * Math.cos(2 * Math.PI * simT);
  const eased = k * k * (3 - 2 * k);
  let wx = 520 + eased * (5100 - 520);
  const wy = 860 + 300 * Math.sin(2 * Math.PI * simT * 2 + 0.6) + 80 * Math.sin(2 * Math.PI * simT * 5);
  let target = deviceAt(wx, wy);
  const current = state.focus.kind === 'local' ? state.me.id : state.focus.peerId;
  if ((state.locked || state.paused || !settings.switching.enabled) && target !== current) {
    // Clamp to the computer that has the cursor.
    const rects = worldRects(current);
    const minX = Math.min(...rects.map((r) => r.x));
    const maxX = Math.max(...rects.map((r) => r.x + r.w)) - 1;
    wx = Math.min(maxX, Math.max(minX, wx));
    target = current;
  }
  if (!target) return;
  setFocus(target);
  const r = worldRects(target)[0];
  emit('crispy://pointer', { device: target, x: wx - r.pos.x + r.minX, y: wy - r.pos.y + r.minY });
}

let last = performance.now();
let acc = 0;
let statsAcc = 0;
function tick() {
  const t = performance.now();
  const dt = Math.min(0.1, (t - last) / 1000);
  last = t;
  pointerTick(dt);
  acc += dt;
  statsAcc += dt;
  if (acc >= 0.25) {
    transfersTick(acc);
    acc = 0;
  }
  if (statsAcc >= 1) {
    statsAcc = 0;
    const moving = state.focus.kind === 'remote';
    state.stats.eventsPerSec = moving ? 110 + Math.round(Math.random() * 40) : Math.round(Math.random() * 6);
    state.stats.uptimeSecs += 1;
    state.stats.bytesSent += moving ? 9_000 + Math.random() * 4_000 : 400;
    state.stats.bytesReceived += 1_200 + Math.random() * 600;
    const other = state.peers.find((p) => p.id === OTHER.info.id && p.status === 'connected');
    if (other) other.rttMs = 2 + Math.round(Math.random() * 2);
    pushState();
  }
}

function transfersTick(dt: number) {
  let changed = false;
  for (const tr of state.transfers) {
    if (tr.state !== 'active') continue;
    tr.bytesPerSec = tr.bytesPerSec * 0.8 + (tr.direction === 'send' ? 84e6 : 62e6) * (0.85 + Math.random() * 0.3) * 0.2;
    tr.doneBytes = Math.min(tr.totalBytes, tr.doneBytes + tr.bytesPerSec * dt);
    state.stats.bytesSent += tr.direction === 'send' ? tr.bytesPerSec * dt : 0;
    state.stats.bytesReceived += tr.direction === 'receive' ? tr.bytesPerSec * dt : 0;
    if (tr.doneBytes >= tr.totalBytes) {
      tr.state = 'done';
      tr.finishedAt = Date.now();
      tr.bytesPerSec = 0;
      tr.currentFile = null;
      if (tr.direction === 'receive') tr.savePath = info.saveDir;
      const n = tr.fileCount;
      toast({
        kind: 'success',
        title: tr.direction === 'send' ? `Sent to ${tr.peerName}` : `Received from ${tr.peerName}`,
        body: `${n} ${n === 1 ? 'file' : 'files'} · ${(tr.totalBytes / 1e9).toFixed(1)} GB`,
      });
    }
    changed = true;
  }
  if (changed) pushState();
}

let started = false;
function start() {
  if (started || STILL) return;
  started = true;
  const loop = () => {
    tick();
    setTimeout(loop, 33);
  };
  setTimeout(loop, 400);
  if (flag('send')) {
    setTimeout(
      () =>
        emit('crispy://send-request', {
          paths: [`${homeDir}/Desktop/Moodboard.fig`, `${homeDir}/Desktop/Palette.png`, `${homeDir}/Desktop/Notes.md`],
        }),
      900,
    );
  }
}

// When frozen, place the cursor on the PC so the "controlling" state shows up in screenshots.
if (STILL && !empty && (ME === MAC || ME === STUDIO)) {
  const otherId = OTHER.info.id;
  state.focus = { kind: 'remote', peerId: otherId };
  for (const p of state.peers) p.controlling = p.id === otherId;
  setTimeout(() => {
    const local = OTHER === STUDIO ? { x: 1880, y: 1010 } : { x: 1130, y: 720 };
    emit('crispy://pointer', { device: otherId, ...local });
  }, 300);
  if (flag('send')) {
    setTimeout(() => emit('crispy://send-request', { paths: ['/Users/maya/Desktop/Moodboard.fig', '/Users/maya/Desktop/Palette.png', '/Users/maya/Desktop/Notes.md'] }), 300);
  }
}

// ---- commands ----------------------------------------------------------------------------------

const wait = (ms: number) => new Promise((r) => setTimeout(r, ms));
let transferSeq = 10;

function findPeer(id: unknown): Peer {
  const p = state.peers.find((x) => x.id === id);
  if (!p) throw new Error(`Unknown device ${String(id)}`);
  return p;
}

const handlers: Record<string, (args: Record<string, any>) => unknown> = {
  get_state: () => {
    start();
    return state;
  },
  get_settings: () => settings,
  set_settings: ({ settings: next }) => {
    settings = sanitize(next as Settings);
    const name = settings.general.deviceName || ME.info.name;
    if (state.me.name !== name) {
      state.me.name = name;
      pushState();
    }
    return settings;
  },
  get_app_info: () => info,
  key_table: () => MOCK_KEY_TABLE,

  pair_start: async ({ peerId }) => {
    const p = findPeer(peerId);
    state.pairing = { peerId: p.id, peerName: p.name, peerOs: p.os, role: 'initiator', stage: 'connecting', code: null, error: null };
    pushState();
    await wait(900);
    if (state.pairing?.peerId === p.id) {
      state.pairing.stage = 'enterCode';
      pushState();
    }
  },
  pair_submit: async ({ code }) => {
    if (!state.pairing) return;
    state.pairing.stage = 'verifying';
    pushState();
    await wait(1100);
    if (!state.pairing) return;
    if (String(code) === '482913') {
      state.pairing.stage = 'success';
      const p = state.peers.find((x) => x.id === state.pairing!.peerId);
      if (p) {
        p.paired = true;
        p.status = 'connected';
        p.rttMs = 4;
      }
      pushState();
      toast({ kind: 'success', title: `Paired with ${state.pairing.peerName}`, body: 'Move your cursor across the edge to switch.' });
    } else {
      state.pairing.stage = 'failed';
      state.pairing.error = `The code didn't match. Check the code shown on ${state.pairing.peerName} and try again.`;
      pushState();
    }
  },
  pair_cancel: () => {
    state.pairing = null;
    pushState();
  },

  unpair: ({ peerId }) => {
    const p = findPeer(peerId);
    if (p.status === 'connected' || p.status === 'connecting') {
      p.paired = false;
      p.status = 'nearby';
      p.controlling = false;
      p.controllingMe = false;
    } else {
      state.peers = state.peers.filter((x) => x.id !== p.id);
    }
    delete state.layout.positions[p.id];
    if (state.focus.kind === 'remote' && state.focus.peerId === p.id) state.focus = { kind: 'local' };
    pushState();
    toast({ kind: 'info', title: `Unpaired ${p.name}`, body: 'It will need a new code to connect again.' });
  },
  set_peer_enabled: ({ peerId, enabled }) => {
    const p = findPeer(peerId);
    p.enabled = !!enabled;
    if (!p.enabled) {
      p.controlling = false;
      if (state.focus.kind === 'remote' && state.focus.peerId === p.id) state.focus = { kind: 'local' };
    }
    pushState();
  },
  set_layout: ({ positions: next }) => {
    state.layout = { rev: state.layout.rev + 1, author: state.me.id, positions: structuredClone(next) };
    pushState();
  },

  send_files: ({ peerId, paths }) => {
    const p = findPeer(peerId);
    const list = (paths as string[]).map((path) => ({ name: path.split(/[\\/]/).pop() ?? path, size: 4_000_000 + Math.random() * 180_000_000 }));
    const total = list.reduce((a, f) => a + f.size, 0);
    const tr: Transfer = {
      id: `t-${++transferSeq}`,
      peerId: p.id,
      peerName: p.name,
      direction: 'send',
      purpose: 'files',
      files: list.slice(0, 3).map((f) => ({ name: f.name, size: Math.round(f.size) })),
      fileCount: list.length,
      totalBytes: Math.round(total),
      doneBytes: 0,
      bytesPerSec: 0,
      state: 'pending',
      error: null,
      startedAt: Date.now(),
      finishedAt: null,
      savePath: null,
      currentFile: list[0]?.name ?? null,
    };
    state.transfers.unshift(tr);
    pushState();
    setTimeout(() => {
      if (tr.state === 'pending') {
        tr.state = 'active';
        tr.bytesPerSec = 40e6;
        pushState();
      }
    }, 700);
  },
  cancel_transfer: ({ id }) => {
    const tr = state.transfers.find((t) => t.id === id);
    if (tr && !['done', 'failed', 'cancelled', 'declined'].includes(tr.state)) {
      tr.state = 'cancelled';
      tr.finishedAt = Date.now();
      tr.bytesPerSec = 0;
      pushState();
    }
  },
  answer_transfer: ({ id, accept }) => {
    const tr = state.transfers.find((t) => t.id === id);
    if (!tr || tr.state !== 'awaiting') return;
    if (accept) {
      tr.state = 'active';
      tr.bytesPerSec = 50e6;
      tr.currentFile = 'IMG_2041.HEIC';
    } else {
      tr.state = 'declined';
      tr.finishedAt = Date.now();
    }
    pushState();
  },
  clear_transfers: () => {
    state.transfers = state.transfers.filter((t) => !['done', 'failed', 'cancelled', 'declined'].includes(t.state));
    pushState();
  },
  reveal_path: ({ path }) => toast({ kind: 'info', title: 'Reveal in folder', body: String(path) }),
  open_save_dir: () => toast({ kind: 'info', title: 'Open folder', body: settings.files.saveDir || info.saveDir }),

  toggle_lock: () => {
    state.locked = !state.locked;
    pushState();
  },
  toggle_pause: () => {
    state.paused = !state.paused;
    pushState();
  },
  request_permissions: async () => {
    await wait(700);
    if (state.permissions.accessibility === 'denied') {
      state.permissions = { accessibility: 'granted', capture: 'running', detail: null };
      pushState();
      toast({ kind: 'success', title: 'Accessibility is on', body: 'Crispy can now share your keyboard and mouse.' });
    }
  },
  open_permission_settings: () =>
    toast({ kind: 'info', title: 'System Settings', body: 'Privacy & Security → Accessibility would open here.' }),
  connect_address: async ({ address }) => {
    const addr = String(address).trim();
    await wait(1200);
    if (!/^[\w.-]+(:\d+)?$|^\[[0-9a-f:]+\](:\d+)?$/i.test(addr)) throw new Error(`“${addr}” is not a valid address`);
    if (state.peers.some((p) => p.address?.startsWith(addr))) return;
    state.peers.push({
      id: `d-${Math.random().toString(16).slice(2, 10)}`,
      name: 'Kitchen NUC',
      os: 'linux',
      paired: false,
      status: 'nearby',
      address: addr.includes(':') ? addr : `${addr}:24727`,
      rttMs: null,
      screens: [scr('HDMI-1', 'HDMI-1', 0, 0, 1920, 1080, 1, true)],
      fingerprint: '91AC-3F02-7D6E-B8C1',
      lastSeen: Date.now(),
      enabled: true,
      version: '1.0.0',
      controlling: false,
      controllingMe: false,
    });
    pushState();
    toast({ kind: 'success', title: 'Found Kitchen NUC', body: `at ${addr} — pair it to start sharing.` });
  },
  regenerate_identity: () => {
    const hex = () => Math.random().toString(16).slice(2, 6).toUpperCase().padEnd(4, '0');
    state.me.fingerprint = `${hex()}-${hex()}-${hex()}-${hex()}`;
    for (const p of state.peers) {
      p.paired = false;
      p.status = p.status === 'offline' ? 'offline' : 'nearby';
      p.controlling = false;
    }
    state.peers = state.peers.filter((p) => p.status !== 'offline');
    state.focus = { kind: 'local' };
    pushState();
    toast({ kind: 'warn', title: 'New identity created', body: 'Pair your other computers again.' });
  },
  open_logs: () => toast({ kind: 'info', title: 'Open logs', body: info.logDir }),
  quit_app: () => toast({ kind: 'info', title: 'Quit', body: 'Crispy would quit now.' }),
};

export async function mockInvoke(cmd: string, args?: Record<string, unknown>): Promise<unknown> {
  const h = handlers[cmd];
  if (!h) throw new Error(`mock: unknown command ${cmd}`);
  await wait(cmd === 'get_state' || cmd === 'get_settings' ? 0 : 30);
  const res = await h(args ?? {});
  return res === undefined ? null : structuredClone(res);
}

export async function mockPickFiles(): Promise<string[] | null> {
  await wait(150);
  return [`${homeDir}/Desktop/Quarterly report.pdf`, `${homeDir}/Desktop/Moodboard.png`, `${homeDir}/Desktop/Interview.m4a`];
}

export async function mockPickFolder(): Promise<string | null> {
  await wait(150);
  return `${homeDir}${sep}Documents${sep}Crispy Inbox`;
}

/** Initial UI hints for the screenshot harness. */
export const mockHints = {
  page: params.get('page'),
  section: params.get('section'),
};
