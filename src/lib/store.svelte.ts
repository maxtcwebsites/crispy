// The single reactive store behind the whole UI.
//
// `state` is replaced wholesale from every `crispy://state` event (raw state: no deep proxies for
// data we never mutate locally). `settings` is deeply reactive so controls can bind straight to it;
// every change is saved after a short debounce and replaced with the sanitized value the backend
// returns.

import { api, events, isMock, mockControl, onFileDrag, pickFiles, type FileDragEvent } from './api';
import type { AppInfo, AppState, KeyInfo, Os, Peer, PointerEvent, Settings, Toast } from './types';

export type Page = 'devices' | 'arrangement' | 'transfers' | 'settings';
export type SectionId =
  | 'general'
  | 'switching'
  | 'mouse'
  | 'keyboard'
  | 'shortcuts'
  | 'clipboard'
  | 'files'
  | 'network'
  | 'advanced'
  | 'about';

export interface ToastItem extends Toast {
  id: number;
}

export interface ConfirmOptions {
  title: string;
  body?: string;
  confirm?: string;
  cancel?: string;
  danger?: boolean;
}

interface ConfirmRequest extends ConfirmOptions {
  resolve: (ok: boolean) => void;
}

export interface LivePointer extends PointerEvent {
  /** performance.now() when it arrived. */
  t: number;
}

const PAGES: Page[] = ['devices', 'arrangement', 'transfers', 'settings'];

function errorText(e: unknown): string {
  if (e instanceof Error) return e.message;
  if (typeof e === 'string') return e;
  try {
    return JSON.stringify(e);
  } catch {
    return 'Something went wrong';
  }
}

class Store {
  // ---- backend data ----
  state = $state.raw<AppState | null>(null);
  settings = $state<Settings | null>(null);
  info = $state.raw<AppInfo | null>(null);
  keys = $state.raw<KeyInfo[]>([]);
  pointer = $state.raw<LivePointer | null>(null);
  loadError = $state<string | null>(null);

  // ---- UI state ----
  page = $state<Page>('devices');
  section = $state<SectionId>('general');
  toasts = $state<ToastItem[]>([]);
  saveStatus = $state<'idle' | 'saving' | 'saved' | 'error'>('idle');
  dismissedWarnings = $state<string[]>([]);
  systemDark = $state(true);
  confirmReq = $state.raw<ConfirmRequest | null>(null);
  /** Files waiting for the user to choose a destination (send-target picker). */
  sendPaths = $state.raw<string[] | null>(null);
  addByIpOpen = $state(false);
  drag = $state<{ active: boolean; target: string | null; count: number }>({ active: false, target: null, count: 0 });
  windowFocused = $state(true);

  // ---- derived ----
  os: Os = $derived(this.info?.os ?? this.state?.me.os ?? 'macos');
  me = $derived(this.state?.me ?? null);
  peers: Peer[] = $derived(this.state?.peers ?? []);
  paired: Peer[] = $derived(this.peers.filter((p) => p.paired));
  nearby: Peer[] = $derived(this.peers.filter((p) => !p.paired));
  connected: Peer[] = $derived(this.paired.filter((p) => p.status === 'connected' && p.enabled));
  /** The peer our keyboard and mouse are driving right now, if any. */
  focusPeer: Peer | null = $derived.by(() => {
    const s = this.state;
    if (!s || s.focus.kind !== 'remote') return null;
    const id = s.focus.peerId;
    return s.peers.find((p) => p.id === id) ?? null;
  });
  controlledBy: Peer | null = $derived(this.peers.find((p) => p.controllingMe) ?? null);
  activeTransferCount = $derived(
    (this.state?.transfers ?? []).filter((t) => t.state === 'active' || t.state === 'pending' || t.state === 'awaiting')
      .length,
  );
  theme: 'dark' | 'light' = $derived.by(() => {
    const t = this.settings?.general.theme ?? 'system';
    if (t === 'dark' || t === 'light') return t;
    return this.systemDark ? 'dark' : 'light';
  });
  reduceMotion = $derived(!!this.settings?.general.reduceMotion);
  translucent = $derived(!!this.info?.windowEffects && !!this.settings?.general.translucentWindow);
  warnings = $derived((this.state?.warnings ?? []).filter((w) => !this.dismissedWarnings.includes(w.id)));

  // ---- lifecycle ----
  private lastSaved = '';
  private saveTimer: ReturnType<typeof setTimeout> | undefined;
  private savedTimer: ReturnType<typeof setTimeout> | undefined;
  private toastSeq = 0;

  async init() {
    try {
      await Promise.all([
        events.onState((s) => (this.state = s)),
        events.onPointer((p) => (this.pointer = { ...p, t: performance.now() })),
        events.onToast((t) => this.toast(t)),
        events.onSendRequest((r) => this.openSendPicker(r.paths)),
        onFileDrag((e) => this.handleFileDrag(e)),
      ]);
      const [state, settings, info, keys] = await Promise.all([
        api.getState(),
        api.getSettings(),
        api.getAppInfo(),
        api.keyTable(),
      ]);
      this.info = info;
      this.keys = keys;
      this.lastSaved = JSON.stringify(settings);
      this.settings = settings;
      this.state = state;
      const pending = await api.takeSendRequest();
      if (pending.length) this.openSendPicker(pending);
    } catch (e) {
      this.loadError = errorText(e);
      return;
    }

    if (isMock) {
      const m = await mockControl();
      const page = m?.mockHints.page as Page | null;
      if (page && PAGES.includes(page)) this.page = page;
      if (m?.mockHints.section) this.section = m.mockHints.section as SectionId;
    }
  }

  // ---- settings autosave ----

  /** Called by an effect whenever the settings object changes. */
  settingsChanged(snapshot: string) {
    if (snapshot === this.lastSaved) return;
    clearTimeout(this.saveTimer);
    this.saveStatus = 'saving';
    this.saveTimer = setTimeout(() => this.flushSettings(), 300);
  }

  async flushSettings() {
    if (!this.settings) return;
    const sent = JSON.stringify(this.settings);
    if (sent === this.lastSaved) {
      this.saveStatus = 'idle';
      return;
    }
    try {
      const result = await api.setSettings($state.snapshot(this.settings) as Settings);
      this.lastSaved = JSON.stringify(result);
      // Only adopt the sanitized value if nothing changed while the request was in flight.
      if (JSON.stringify(this.settings) === sent) this.settings = result;
      this.saveStatus = 'saved';
      clearTimeout(this.savedTimer);
      this.savedTimer = setTimeout(() => {
        if (this.saveStatus === 'saved') this.saveStatus = 'idle';
      }, 1800);
    } catch (e) {
      this.saveStatus = 'error';
      this.toast({ kind: 'error', title: 'Couldn’t save settings', body: errorText(e) });
    }
  }

  // ---- toasts ----

  toast(t: Toast, ms = 5200) {
    const id = ++this.toastSeq;
    this.toasts.push({ ...t, id });
    if (this.toasts.length > 4) this.toasts.splice(0, this.toasts.length - 4);
    setTimeout(() => this.dismissToast(id), t.kind === 'error' ? ms + 3000 : ms);
  }

  dismissToast(id: number) {
    const i = this.toasts.findIndex((t) => t.id === id);
    if (i >= 0) this.toasts.splice(i, 1);
  }

  // ---- confirm dialog ----

  ask(o: ConfirmOptions): Promise<boolean> {
    this.confirmReq?.resolve(false);
    return new Promise((resolve) => (this.confirmReq = { ...o, resolve }));
  }

  answer(ok: boolean) {
    this.confirmReq?.resolve(ok);
    this.confirmReq = null;
  }

  // ---- actions with error handling ----

  /** Run a backend call, turning failures into an error toast. Returns false on failure. */
  async run(fn: () => Promise<unknown>, failTitle = 'Something went wrong'): Promise<boolean> {
    try {
      await fn();
      return true;
    } catch (e) {
      this.toast({ kind: 'error', title: failTitle, body: errorText(e) });
      return false;
    }
  }

  go(page: Page, section?: SectionId) {
    this.page = page;
    if (section) this.section = section;
  }

  peer(id: string): Peer | undefined {
    return this.peers.find((p) => p.id === id);
  }

  // ---- sending files ----

  async sendTo(peerId: string, paths: string[]) {
    const p = this.peer(peerId);
    if (!paths.length) return;
    if (p && (p.status !== 'connected' || !p.enabled)) {
      this.toast({ kind: 'warn', title: `${p.name} isn’t connected`, body: 'Files can only go to computers that are online.' });
      return;
    }
    const ok = await this.run(() => api.sendFiles(peerId, paths), 'Couldn’t send files');
    if (ok) {
      const n = paths.length;
      this.toast({ kind: 'info', title: `Sending ${n} ${n === 1 ? 'file' : 'files'}`, body: `to ${p?.name ?? 'the other computer'}` }, 3200);
    }
  }

  /** Send to the only connected computer, or ask which one when there are several. */
  sendAnywhere(paths: string[]) {
    if (!paths.length) return;
    const c = this.connected;
    if (c.length === 0) {
      this.toast({ kind: 'warn', title: 'No computers connected', body: 'Connect another computer to send files.' });
    } else if (c.length === 1) {
      void this.sendTo(c[0].id, paths);
    } else {
      this.openSendPicker(paths);
    }
  }

  openSendPicker(paths: string[]) {
    this.sendPaths = paths;
  }

  async chooseAndSend(peerId?: string) {
    const paths = await pickFiles();
    if (!paths?.length) return;
    if (peerId) await this.sendTo(peerId, paths);
    else this.sendAnywhere(paths);
  }

  // ---- window file drag & drop ----

  private dropTargetAt(x: number, y: number): string | null {
    const el = document.elementFromPoint(x, y);
    const t = el?.closest<HTMLElement>('[data-drop]');
    return t?.dataset.drop ?? null;
  }

  private handleFileDrag(e: FileDragEvent) {
    if (e.type === 'enter') {
      this.drag = { active: true, target: this.dropTargetAt(e.x, e.y), count: e.paths.length };
    } else if (e.type === 'over') {
      if (!this.drag.active) this.drag.active = true;
      const t = this.dropTargetAt(e.x, e.y);
      if (t !== this.drag.target) this.drag.target = t;
    } else if (e.type === 'leave') {
      this.drag = { active: false, target: null, count: 0 };
    } else if (e.type === 'drop') {
      const target = this.dropTargetAt(e.x, e.y);
      this.drag = { active: false, target: null, count: 0 };
      if (this.state?.pairing || !e.paths.length) return;
      if (target?.startsWith('peer:')) void this.sendTo(target.slice(5), e.paths);
      else this.sendAnywhere(e.paths);
    }
  }
}

export const store = new Store();
