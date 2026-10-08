// The contract between the Rust engine and the UI. Mirrors crates/crispy-core/src/state.rs and
// config.rs (serde camelCase). Keep the two in sync.

export type Os = 'macos' | 'windows' | 'linux';

export interface Screen {
  id: string;
  name: string;
  x: number;
  y: number;
  width: number;
  height: number;
  scale: number;
  primary: boolean;
}

export interface DeviceInfo {
  id: string;
  name: string;
  os: Os;
  version: string;
  /** e.g. "7F3A-91C2-0B44-E81D" — shown so users can verify devices. */
  fingerprint: string;
  screens: Screen[];
  /** LAN addresses this device listens on, e.g. ["192.168.1.20:24727"]. */
  addresses: string[];
}

export type PeerStatus = 'connected' | 'connecting' | 'offline' | 'nearby';

export interface Peer {
  id: string;
  name: string;
  os: Os;
  /** Trusted (paired). `nearby` peers are discovered but not paired. */
  paired: boolean;
  status: PeerStatus;
  address: string | null;
  rttMs: number | null;
  screens: Screen[];
  fingerprint: string;
  lastSeen: number | null; // unix ms
  enabled: boolean;
  version: string | null;
  /** Our keyboard/mouse is currently driving this peer. */
  controlling: boolean;
  /** This peer's keyboard/mouse is currently driving us. */
  controllingMe: boolean;
}

export interface Pos {
  x: number;
  y: number;
}

export interface Layout {
  rev: number;
  author: string;
  positions: Record<string, Pos>;
}

export type Focus = { kind: 'local' } | { kind: 'remote'; peerId: string };

export type TransferState = 'pending' | 'awaiting' | 'active' | 'done' | 'failed' | 'cancelled' | 'declined';

export interface Transfer {
  id: string;
  peerId: string;
  peerName: string;
  direction: 'send' | 'receive';
  purpose: 'files' | 'clipboard';
  /** First few entries, for display. */
  files: { name: string; size: number }[];
  fileCount: number;
  totalBytes: number;
  doneBytes: number;
  bytesPerSec: number;
  state: TransferState;
  error: string | null;
  startedAt: number;
  finishedAt: number | null;
  /** Where received files were saved (folder). */
  savePath: string | null;
  currentFile: string | null;
}

export type PermissionState = 'granted' | 'denied' | 'notRequired';

export interface Permissions {
  /** macOS Accessibility (needed to control the keyboard and mouse). */
  accessibility: PermissionState;
  /** Whether the global input hook is running. */
  capture: 'running' | 'starting' | 'failed';
  detail: string | null;
}

export type PairingStage = 'connecting' | 'enterCode' | 'showCode' | 'verifying' | 'success' | 'failed';

export interface Pairing {
  peerId: string;
  peerName: string;
  peerOs: Os;
  role: 'initiator' | 'responder';
  stage: PairingStage;
  /** Shown on the responder; the initiator types it in. */
  code: string | null;
  error: string | null;
}

export interface Warning {
  id: string;
  level: 'info' | 'warn' | 'error';
  title: string;
  body: string;
}

export interface Stats {
  eventsPerSec: number;
  bytesSent: number;
  bytesReceived: number;
  uptimeSecs: number;
}

export interface AppState {
  me: DeviceInfo;
  peers: Peer[];
  layout: Layout;
  focus: Focus;
  /** Cursor locked to the current computer. */
  locked: boolean;
  /** Sharing paused (no switching). */
  paused: boolean;
  transfers: Transfer[];
  permissions: Permissions;
  pairing: Pairing | null;
  warnings: Warning[];
  stats: Stats;
}

/** Live cursor position (device-local coordinates), ~30 Hz while it moves. */
export interface PointerEvent {
  device: string;
  x: number;
  y: number;
}

export interface Toast {
  kind: 'info' | 'success' | 'warn' | 'error';
  title: string;
  body: string;
}

/** Sent when files arrive from outside (Send To menu, dock drop): the UI asks where to send them. */
export interface SendRequest {
  paths: string[];
}

// ------------------------------------------------------------------------------------------------
// Settings
// ------------------------------------------------------------------------------------------------

export type ModKey = 'ctrl' | 'shift' | 'alt' | 'meta';

export interface ModifierMap {
  ctrl: ModKey;
  alt: ModKey;
  meta: ModKey;
}

export interface Hotkey {
  ctrl: boolean;
  shift: boolean;
  alt: boolean;
  meta: boolean;
  /** HID usage (see KeyInfo). 0 = not set. */
  key: number;
}

export interface KeyInfo {
  hid: number;
  /** DOM KeyboardEvent.code, e.g. "KeyL". */
  code: string;
  label: string;
}

export type Flavor = 'classic' | 'bbq' | 'sourCream' | 'saltVinegar' | 'sweetChili' | 'truffle';

export interface Settings {
  general: {
    deviceName: string;
    launchAtLogin: boolean;
    startMinimized: boolean;
    closeToTray: boolean;
    notifications: boolean;
    sounds: boolean;
    theme: 'system' | 'dark' | 'light';
    flavor: Flavor;
    reduceMotion: boolean;
    translucentWindow: boolean;
    onboarded: boolean;
  };
  switching: {
    enabled: boolean;
    delayMs: number;
    doubleTap: boolean;
    doubleTapWindowMs: number;
    cornerSize: number;
    corners: { topLeft: boolean; topRight: boolean; bottomLeft: boolean; bottomRight: boolean };
    blockWhileDragging: boolean;
    requiredModifier: 'none' | 'shift' | 'ctrl' | 'alt' | 'meta';
    blockFullscreen: boolean;
    edgeMapping: 'direct' | 'proportional';
    localInputTakesOver: boolean;
  };
  mouse: {
    pointerSpeed: number;
    scrollSpeed: number;
    invertVertical: boolean;
    invertHorizontal: boolean;
    smoothScrolling: boolean;
    relativeMovement: boolean;
  };
  keyboard: {
    macToPc: ModifierMap;
    pcToMac: ModifierMap;
    forwardMediaKeys: boolean;
    syncCapsLock: boolean;
  };
  hotkeys: {
    lockToScreen: Hotkey;
    bringHome: Hotkey;
    nextDevice: Hotkey;
    pauseSharing: Hotkey;
  };
  clipboard: {
    enabled: boolean;
    mode: 'instant' | 'onSwitch';
    text: boolean;
    richText: boolean;
    images: boolean;
    files: boolean;
    maxImageMb: number;
    maxFilesMb: number;
  };
  files: {
    saveDir: string;
    autoAccept: boolean;
    conflict: 'rename' | 'overwrite' | 'skip';
    revealWhenDone: boolean;
    preserveTimestamps: boolean;
    bandwidthLimitMbps: number;
  };
  network: {
    discovery: boolean;
    port: number;
    manualPeers: string[];
    heartbeatTimeoutSecs: number;
  };
  advanced: {
    logLevel: 'error' | 'warn' | 'info' | 'debug' | 'trace';
    showStats: boolean;
  };
}

export interface AppInfo {
  version: string;
  os: Os;
  /** Native window material (Mica / vibrancy) was applied; the UI may use translucent backgrounds. */
  windowEffects: boolean;
  /** Effective save folder (resolves the empty default). */
  saveDir: string;
  logDir: string;
}
