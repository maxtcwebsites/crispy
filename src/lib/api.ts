// Thin, typed wrapper around the Tauri backend. Outside Tauri (a plain browser during
// development or for screenshots) every call is served by an in-memory mock instead.

import { invoke as tauriInvoke } from '@tauri-apps/api/core';
import { listen as tauriListen } from '@tauri-apps/api/event';
import type {
  AppInfo,
  AppState,
  KeyInfo,
  PointerEvent,
  Pos,
  SendRequest,
  Settings,
  Toast,
} from './types';

export const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
export const isMock = !isTauri;

type MockModule = typeof import('./mock');
let mockModule: Promise<MockModule> | null = null;
const mock = () => (mockModule ??= import('./mock'));

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (isTauri) return tauriInvoke<T>(cmd, args);
  return (await mock()).mockInvoke(cmd, args) as Promise<T>;
}

export type Unlisten = () => void;

export async function listen<T>(event: string, cb: (payload: T) => void): Promise<Unlisten> {
  if (isTauri) return tauriListen<T>(event, (e) => cb(e.payload));
  return (await mock()).mockListen(event, cb as (p: unknown) => void);
}

export const EVENTS = {
  state: 'crispy://state',
  pointer: 'crispy://pointer',
  toast: 'crispy://toast',
  sendRequest: 'crispy://send-request',
} as const;

export const events = {
  onState: (cb: (s: AppState) => void) => listen<AppState>(EVENTS.state, cb),
  onPointer: (cb: (p: PointerEvent) => void) => listen<PointerEvent>(EVENTS.pointer, cb),
  onToast: (cb: (t: Toast) => void) => listen<Toast>(EVENTS.toast, cb),
  onSendRequest: (cb: (r: SendRequest) => void) => listen<SendRequest>(EVENTS.sendRequest, cb),
};

/** Every backend command. Argument keys are camelCase (Tauri converts them). */
export const api = {
  getState: () => call<AppState>('get_state'),
  getSettings: () => call<Settings>('get_settings'),
  /** Returns the sanitized settings; always use the returned value. */
  setSettings: (settings: Settings) => call<Settings>('set_settings', { settings }),
  getAppInfo: () => call<AppInfo>('get_app_info'),
  keyTable: () => call<KeyInfo[]>('key_table'),

  pairStart: (peerId: string) => call<void>('pair_start', { peerId }),
  pairSubmit: (code: string) => call<void>('pair_submit', { code }),
  pairCancel: () => call<void>('pair_cancel'),

  unpair: (peerId: string) => call<void>('unpair', { peerId }),
  setPeerEnabled: (peerId: string, enabled: boolean) => call<void>('set_peer_enabled', { peerId, enabled }),
  setLayout: (positions: Record<string, Pos>) => call<void>('set_layout', { positions }),

  sendFiles: (peerId: string, paths: string[]) => call<void>('send_files', { peerId, paths }),
  cancelTransfer: (id: string) => call<void>('cancel_transfer', { id }),
  answerTransfer: (id: string, accept: boolean) => call<void>('answer_transfer', { id, accept }),
  clearTransfers: () => call<void>('clear_transfers'),
  revealPath: (path: string) => call<void>('reveal_path', { path }),
  openSaveDir: () => call<void>('open_save_dir'),

  toggleLock: () => call<void>('toggle_lock'),
  togglePause: () => call<void>('toggle_pause'),
  requestPermissions: () => call<void>('request_permissions'),
  openPermissionSettings: () => call<void>('open_permission_settings'),
  connectAddress: (address: string) => call<void>('connect_address', { address }),
  regenerateIdentity: () => call<void>('regenerate_identity'),
  openLogs: () => call<void>('open_logs'),
  quitApp: () => call<void>('quit_app'),
  /** Files handed to Crispy (Explorer "Send to", Dock drop) before the window was listening. */
  takeSendRequest: () => (isTauri ? tauriInvoke<string[]>('take_send_request') : Promise.resolve([] as string[])),
};

// ---- native dialogs ----------------------------------------------------------------------------

export async function pickFiles(title = 'Choose files to send'): Promise<string[] | null> {
  if (isTauri) {
    const { open } = await import('@tauri-apps/plugin-dialog');
    const res = await open({ multiple: true, directory: false, title });
    if (!res) return null;
    return Array.isArray(res) ? res : [res];
  }
  return (await mock()).mockPickFiles();
}

export async function pickFolder(defaultPath?: string): Promise<string | null> {
  if (isTauri) {
    const { open } = await import('@tauri-apps/plugin-dialog');
    const res = await open({ directory: true, multiple: false, defaultPath, title: 'Choose where received files go' });
    return typeof res === 'string' ? res : null;
  }
  return (await mock()).mockPickFolder();
}

// ---- window ----------------------------------------------------------------------------------

export const appWindow = {
  async minimize() {
    if (!isTauri) return;
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    await getCurrentWindow().minimize();
  },
  async toggleMaximize() {
    if (!isTauri) return;
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    await getCurrentWindow().toggleMaximize();
  },
  async close() {
    if (!isTauri) return;
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    await getCurrentWindow().close();
  },
  async isMaximized(): Promise<boolean> {
    if (!isTauri) return false;
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    return getCurrentWindow().isMaximized();
  },
  async onResized(cb: () => void): Promise<Unlisten> {
    if (!isTauri) {
      window.addEventListener('resize', cb);
      return () => window.removeEventListener('resize', cb);
    }
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    return getCurrentWindow().onResized(cb);
  },
};

// ---- file drag & drop ------------------------------------------------------------------------

export type FileDragEvent =
  | { type: 'enter'; paths: string[]; x: number; y: number }
  | { type: 'over'; x: number; y: number }
  | { type: 'drop'; paths: string[]; x: number; y: number }
  | { type: 'leave' };

/**
 * Files dragged onto the window. In Tauri this uses the webview's native drag-drop event (HTML5
 * drag events do not carry file paths there). Positions are converted to CSS pixels.
 * In the browser mock, HTML5 drag events are used with made-up paths.
 */
export async function onFileDrag(cb: (e: FileDragEvent) => void): Promise<Unlisten> {
  if (isTauri) {
    const { getCurrentWebview } = await import('@tauri-apps/api/webview');
    return getCurrentWebview().onDragDropEvent((event) => {
      const p = event.payload;
      const dpr = window.devicePixelRatio || 1;
      if (p.type === 'leave') return cb({ type: 'leave' });
      const x = p.position.x / dpr;
      const y = p.position.y / dpr;
      if (p.type === 'over') return cb({ type: 'over', x, y });
      cb({ type: p.type, paths: p.paths, x, y });
    });
  }

  let depth = 0;
  const hasFiles = (e: DragEvent) => !!e.dataTransfer && Array.from(e.dataTransfer.types).includes('Files');
  const fake = (e: DragEvent) => {
    const files = Array.from(e.dataTransfer?.files ?? []);
    const names = files.length ? files.map((f) => f.name) : ['Dropped file.pdf'];
    return names.map((n) => `/Users/maya/Desktop/${n}`);
  };
  const enter = (e: DragEvent) => {
    if (!hasFiles(e)) return;
    e.preventDefault();
    if (depth++ === 0) cb({ type: 'enter', paths: fake(e), x: e.clientX, y: e.clientY });
  };
  const over = (e: DragEvent) => {
    if (!hasFiles(e)) return;
    e.preventDefault();
    cb({ type: 'over', x: e.clientX, y: e.clientY });
  };
  const leave = (e: DragEvent) => {
    if (!hasFiles(e)) return;
    if (--depth <= 0) {
      depth = 0;
      cb({ type: 'leave' });
    }
  };
  const drop = (e: DragEvent) => {
    if (!hasFiles(e)) return;
    e.preventDefault();
    depth = 0;
    cb({ type: 'drop', paths: fake(e), x: e.clientX, y: e.clientY });
  };
  window.addEventListener('dragenter', enter);
  window.addEventListener('dragover', over);
  window.addEventListener('dragleave', leave);
  window.addEventListener('drop', drop);
  return () => {
    window.removeEventListener('dragenter', enter);
    window.removeEventListener('dragover', over);
    window.removeEventListener('dragleave', leave);
    window.removeEventListener('drop', drop);
  };
}

/** Mock-only helpers used by the screenshot harness and dev tools. */
export async function mockControl(): Promise<MockModule | null> {
  return isTauri ? null : mock();
}
