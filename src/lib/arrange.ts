// Geometry for the Arrangement canvas: placing devices, snapping, overlap resolution and the
// edges where the cursor can cross from one computer to another.
//
// World units are device-native screen units (points on macOS, pixels on Windows). A device's
// layout position is the top-left of the bounding box of all its screens.

import type { Os, Pos, Screen } from './types';
import { screenBounds, type Bounds } from './util';

export interface ArrangeDevice {
  id: string;
  name: string;
  os: Os;
  screens: Screen[];
  bounds: Bounds;
  isMe: boolean;
  online: boolean;
}

export interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface WorldScreen extends Rect {
  device: string;
  screen: Screen;
}

export function makeDevice(d: Omit<ArrangeDevice, 'bounds'>): ArrangeDevice {
  return { ...d, bounds: screenBounds(d.screens) };
}

/** Positions for every device; devices without one go to the right of the arrangement. */
export function withMissing(devices: ArrangeDevice[], positions: Record<string, Pos>): Record<string, Pos> {
  const out: Record<string, Pos> = {};
  for (const d of devices) if (positions[d.id]) out[d.id] = { ...positions[d.id] };
  const missing = devices.filter((d) => !out[d.id]).sort((a, b) => a.id.localeCompare(b.id));
  for (const d of missing) {
    const placed = devices.filter((o) => out[o.id]).map((o) => ({ x: out[o.id].x, y: out[o.id].y, w: o.bounds.w, h: o.bounds.h }));
    if (!placed.length) {
      out[d.id] = { x: 0, y: 0 };
      continue;
    }
    const u = union(placed);
    out[d.id] = { x: u.x + u.w, y: Math.round(u.y + (u.h - d.bounds.h) / 2) };
  }
  return out;
}

/** A tidy default: everything side by side, vertically centred, this computer first. */
export function defaultArrangement(devices: ArrangeDevice[]): Record<string, Pos> {
  const ordered = [...devices].sort((a, b) => Number(b.isMe) - Number(a.isMe) || a.name.localeCompare(b.name));
  const tallest = Math.max(...ordered.map((d) => d.bounds.h), 0);
  const out: Record<string, Pos> = {};
  let x = 0;
  for (const d of ordered) {
    out[d.id] = { x, y: Math.round((tallest - d.bounds.h) / 2) };
    x += d.bounds.w;
  }
  return out;
}

export function worldScreens(devices: ArrangeDevice[], positions: Record<string, Pos>): WorldScreen[] {
  const out: WorldScreen[] = [];
  for (const d of devices) {
    const p = positions[d.id];
    if (!p) continue;
    for (const s of d.screens) {
      out.push({ device: d.id, screen: s, x: p.x + s.x - d.bounds.x, y: p.y + s.y - d.bounds.y, w: s.width, h: s.height });
    }
  }
  return out;
}

export function union(rects: Rect[]): Rect {
  const x = Math.min(...rects.map((r) => r.x));
  const y = Math.min(...rects.map((r) => r.y));
  const r = Math.max(...rects.map((r) => r.x + r.w));
  const b = Math.max(...rects.map((r) => r.y + r.h));
  return { x, y, w: r - x, h: b - y };
}

const EPS = 0.5;

function overlap(a: Rect, b: Rect): { x: number; y: number } | null {
  const ox = Math.min(a.x + a.w, b.x + b.w) - Math.max(a.x, b.x);
  const oy = Math.min(a.y + a.h, b.y + b.h) - Math.max(a.y, b.y);
  return ox > EPS && oy > EPS ? { x: ox, y: oy } : null;
}

export interface Guide {
  axis: 'x' | 'y';
  at: number;
}

/**
 * Snap a dragged device to the edges and centres of the others, then push it out of any overlap.
 * Returns the adjusted position and the guide lines that snapped (for display).
 */
export function snapAndResolve(
  dragged: ArrangeDevice,
  want: Pos,
  others: WorldScreen[],
  threshold: number,
): { pos: Pos; guides: Guide[]; valid: boolean } {
  const rectsAt = (p: Pos) =>
    dragged.screens.map((s) => ({ x: p.x + s.x - dragged.bounds.x, y: p.y + s.y - dragged.bounds.y, w: s.width, h: s.height }));

  let best: { dx: number; gx: number } | null = null;
  let bestY: { dy: number; gy: number } | null = null;
  for (const r of rectsAt(want)) {
    for (const o of others) {
      const xs: [number, number][] = [
        [o.x + o.w - r.x, o.x + o.w], // our left to their right
        [o.x - (r.x + r.w), o.x], // our right to their left
        [o.x - r.x, o.x], // left edges aligned
        [o.x + o.w - (r.x + r.w), o.x + o.w], // right edges aligned
        [o.x + o.w / 2 - (r.x + r.w / 2), o.x + o.w / 2], // centres
      ];
      for (const [dx, gx] of xs) if (Math.abs(dx) <= threshold && (!best || Math.abs(dx) < Math.abs(best.dx))) best = { dx, gx };
      const ys: [number, number][] = [
        [o.y + o.h - r.y, o.y + o.h],
        [o.y - (r.y + r.h), o.y],
        [o.y - r.y, o.y],
        [o.y + o.h - (r.y + r.h), o.y + o.h],
        [o.y + o.h / 2 - (r.y + r.h / 2), o.y + o.h / 2],
      ];
      for (const [dy, gy] of ys) if (Math.abs(dy) <= threshold && (!bestY || Math.abs(dy) < Math.abs(bestY.dy))) bestY = { dy, gy };
    }
  }

  let pos = { x: want.x + (best?.dx ?? 0), y: want.y + (bestY?.dy ?? 0) };
  const guides: Guide[] = [];
  if (best) guides.push({ axis: 'x', at: best.gx });
  if (bestY) guides.push({ axis: 'y', at: bestY.gy });

  // Push out of overlaps along the shallowest axis, a few times for multi-screen collisions.
  for (let iter = 0; iter < 8; iter++) {
    let worst: { ov: { x: number; y: number }; r: Rect; o: Rect } | null = null;
    for (const r of rectsAt(pos)) {
      for (const o of others) {
        const ov = overlap(r, o);
        if (ov && (!worst || ov.x * ov.y > worst.ov.x * worst.ov.y)) worst = { ov, r, o };
      }
    }
    if (!worst) return { pos: round(pos), guides, valid: true };
    const { ov, r, o } = worst;
    if (ov.x < ov.y) {
      pos = { ...pos, x: pos.x + (r.x + r.w / 2 < o.x + o.w / 2 ? -ov.x : ov.x) };
    } else {
      pos = { ...pos, y: pos.y + (r.y + r.h / 2 < o.y + o.h / 2 ? -ov.y : ov.y) };
    }
  }
  return { pos: round(pos), guides: [], valid: false };
}

function round(p: Pos): Pos {
  return { x: Math.round(p.x), y: Math.round(p.y) };
}

export interface Edge {
  /** The two devices that share this edge. */
  a: string;
  b: string;
  x1: number;
  y1: number;
  x2: number;
  y2: number;
}

/** Shared borders between screens of different devices — where the cursor can cross. */
export function crossingEdges(screens: WorldScreen[]): Edge[] {
  const edges: Edge[] = [];
  const near = (a: number, b: number) => Math.abs(a - b) <= 1;
  for (let i = 0; i < screens.length; i++) {
    for (let j = i + 1; j < screens.length; j++) {
      const a = screens[i];
      const b = screens[j];
      if (a.device === b.device) continue;
      const y1 = Math.max(a.y, b.y);
      const y2 = Math.min(a.y + a.h, b.y + b.h);
      if (y2 - y1 > 1) {
        if (near(a.x + a.w, b.x)) edges.push({ a: a.device, b: b.device, x1: b.x, y1, x2: b.x, y2 });
        else if (near(b.x + b.w, a.x)) edges.push({ a: a.device, b: b.device, x1: a.x, y1, x2: a.x, y2 });
      }
      const x1 = Math.max(a.x, b.x);
      const x2 = Math.min(a.x + a.w, b.x + b.w);
      if (x2 - x1 > 1) {
        if (near(a.y + a.h, b.y)) edges.push({ a: a.device, b: b.device, x1, y1: b.y, x2, y2: b.y });
        else if (near(b.y + b.h, a.y)) edges.push({ a: a.device, b: b.device, x1, y1: a.y, x2, y2: a.y });
      }
    }
  }
  return edges;
}

/** Devices that touch no other device (the cursor can't reach them). */
export function isolated(devices: ArrangeDevice[], screens: WorldScreen[]): ArrangeDevice[] {
  if (devices.length < 2) return [];
  const edges = crossingEdges(screens);
  const touching = new Set<string>();
  for (const e of edges) {
    for (const s of screens) {
      const onV = e.x1 === e.x2 && (Math.abs(s.x - e.x1) <= 1 || Math.abs(s.x + s.w - e.x1) <= 1) && s.y < e.y2 && s.y + s.h > e.y1;
      const onH = e.y1 === e.y2 && (Math.abs(s.y - e.y1) <= 1 || Math.abs(s.y + s.h - e.y1) <= 1) && s.x < e.x2 && s.x + s.w > e.x1;
      if (onV || onH) touching.add(s.device);
    }
  }
  return devices.filter((d) => !touching.has(d.id));
}
