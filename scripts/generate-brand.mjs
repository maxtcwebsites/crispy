// Generates the Crispy brand artwork in branding/ from a small 3D model of a ruffled chip.
//
//   node scripts/generate-brand.mjs
//
// The chip is a saddle surface z = A(u² − v²) seen from a raised camera. The silhouette, the
// shading gradients, the specular highlight and the ruffle lines are all projected from that
// one surface, so every detail follows the same curvature. Output is plain, hand-editable SVG.
import { writeFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const OUT = process.env.OUT ?? resolve(dirname(fileURLToPath(import.meta.url)), '../branding');
const f = (n) => +n.toFixed(1);
const deg = Math.PI / 180;

const P = {
  A: 0.22, // saddle strength
  yaw: 0, // rotation of the chip in its own plane
  pitch: 0.85, // camera elevation (rad)
  roll: -0.2, // screen tilt (rad)
  S: 186, // px per model unit
  cx: 256,
  cy: 258,
  su: 1.1, // footprint stretch along u / v
  sv: 0.98,
  light: [0.5, 0.55, 0.67],
  ridges: [-0.52, -0.2, 0.12, 0.44], // v positions of the ruffles
  ridgeOffset: 0.022,
  ridgeLight: 0.5,
  ridgeDark: 0.28,
  brightness: [0.2, 0.98],
};

// ---- model ---------------------------------------------------------------------------------
const R = (t) =>
  1 + 0.01 * Math.sin(3 * t + 0.7) + 0.024 * Math.sin(5 * t + 2.1) + 0.014 * Math.sin(7 * t + 0.4) +
  0.008 * Math.sin(11 * t + 1.3) + 0.004 * Math.sin(17 * t + 0.2);
const z = (u, v) => P.A * (u * u - v * v);
const boundary = (t) => [Math.cos(t) * R(t) * P.su, Math.sin(t) * R(t) * P.sv];
const inside = (u, v) => Math.hypot(u / P.su, v / P.sv) < R(Math.atan2(v / P.sv, u / P.su)) * 0.995;

function project(u, v, h = z(u, v)) {
  const x = u * Math.cos(P.yaw) - v * Math.sin(P.yaw);
  const y = u * Math.sin(P.yaw) + v * Math.cos(P.yaw);
  const up = y * Math.sin(P.pitch) + h * Math.cos(P.pitch);
  const xr = x * Math.cos(P.roll) + up * Math.sin(P.roll);
  const yr = x * Math.sin(P.roll) - up * Math.cos(P.roll);
  return [P.cx + xr * P.S, P.cy + yr * P.S];
}

const norm = (a) => { const l = Math.hypot(...a); return a.map((c) => c / l); };
const dot = (a, b) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
function normal(u, v) {
  const e = 1e-3;
  const n = norm([-(z(u + e, v) - z(u - e, v)) / (2 * e), -(z(u, v + e) - z(u, v - e)) / (2 * e), 1]);
  return [n[0] * Math.cos(P.yaw) - n[1] * Math.sin(P.yaw), n[0] * Math.sin(P.yaw) + n[1] * Math.cos(P.yaw), n[2]];
}
const L = norm(P.light);
const V = [0, -Math.cos(P.pitch), Math.sin(P.pitch)];
const H = norm(L.map((c, i) => c + V[i]));

function catmull(points, closed = true) {
  const n = points.length;
  const Q = (i) => points[closed ? (i + n) % n : Math.max(0, Math.min(n - 1, i))];
  let d = `M${f(Q(0)[0])} ${f(Q(0)[1])}`;
  for (let i = 0; i < (closed ? n : n - 1); i++) {
    const p0 = Q(i - 1), p1 = Q(i), p2 = Q(i + 1), p3 = Q(i + 2);
    d += `C${f(p1[0] + (p2[0] - p0[0]) / 6)} ${f(p1[1] + (p2[1] - p0[1]) / 6)} ${f(p2[0] - (p3[0] - p1[0]) / 6)} ${f(p2[1] - (p3[1] - p1[1]) / 6)} ${f(p2[0])} ${f(p2[1])}`;
  }
  return closed ? d + 'Z' : d;
}

const outline = catmull(Array.from({ length: 48 }, (_, i) => project(...boundary((i / 48) * 2 * Math.PI))));

// ---- shading -------------------------------------------------------------------------------
const ramp = [
  [0.0, [166, 86, 20]],
  [0.25, [208, 124, 30]],
  [0.5, [240, 168, 52]],
  [0.7, [250, 196, 84]],
  [0.86, [255, 220, 126]],
  [1.0, [255, 240, 194]],
];
function color(b) {
  b = Math.max(0, Math.min(1, b));
  for (let i = 1; i < ramp.length; i++) {
    if (b <= ramp[i][0]) {
      const [t0, c0] = ramp[i - 1], [t1, c1] = ramp[i];
      const k = (b - t0) / (t1 - t0);
      return '#' + c0.map((x, j) => Math.round(x + (c1[j] - x) * k).toString(16).padStart(2, '0')).join('').toUpperCase();
    }
  }
  return '#FFF0C2';
}
const lam = (u, v) => Math.max(0, dot(normal(u, v), L));
let LMIN = 9, LMAX = -9;
for (let u = -1.1; u <= 1.1; u += 0.05) for (let v = -1; v <= 1; v += 0.05) {
  if (!inside(u, v)) continue;
  LMIN = Math.min(LMIN, lam(u, v)); LMAX = Math.max(LMAX, lam(u, v));
}
const shade = (u, v) => P.brightness[0] + (P.brightness[1] - P.brightness[0]) * (lam(u, v) - LMIN) / (LMAX - LMIN);

const uA = project(-1.15, 0), uB = project(1.15, 0);
const ustops = Array.from({ length: 13 }, (_, i) => {
  const u = -1.15 + (2.3 * i) / 12;
  return `<stop offset="${(i / 12).toFixed(3)}" stop-color="${color(shade(u, 0))}"/>`;
});
const vA = project(0, -1.05), vB = project(0, 1.05);
const vstops = Array.from({ length: 9 }, (_, i) => {
  const v = -1.05 + (2.1 * i) / 8;
  const d = shade(0, v) - shade(0, 0);
  return `<stop offset="${(i / 8).toFixed(3)}" stop-color="${d > 0 ? '#FFF6D8' : '#8A430C'}" stop-opacity="${Math.min(0.9, Math.abs(d) * 1.1).toFixed(3)}"/>`;
});

let spec = [0, 0, -1];
for (let u = -1; u <= 1; u += 0.02) for (let v = -0.9; v <= 0.9; v += 0.02) {
  const s = dot(normal(u, v), H);
  if (s > spec[2] && Math.hypot(u / P.su, v / P.sv) < 0.72) spec = [u, v, s];
}

// ---- ruffles -------------------------------------------------------------------------------
function ruffle(v0, inset = 0.02) {
  let lo = null, hi = null;
  for (let u = -1.3; u <= 1.3; u += 0.005) if (inside(u, v0)) { if (lo === null) lo = u; hi = u; }
  return catmull(Array.from({ length: 17 }, (_, i) => project(lo + inset + ((hi - lo - 2 * inset) * i) / 16, v0)), false);
}

const bubbles = [[-0.55, 0.25, 1], [0.42, -0.35, 0.8], [0.2, 0.52, 1.1], [-0.12, -0.58, 0.7], [0.72, 0.1, 0.7], [-0.78, -0.18, 0.6]];
const salt = [[-0.48, -0.2, 8, 18], [0.12, -0.42, 6.5, -12], [0.5, 0.18, 7, 34], [-0.2, 0.36, 5.5, 8], [0.0, 0.02, 4.5, 52], [0.66, -0.44, 4.5, 20]];

function defs(p) {
  return `
    <path id="${p}o" d="${outline}"/>
    <clipPath id="${p}clip"><use href="#${p}o"/></clipPath>
    <linearGradient id="${p}u" x1="${f(uA[0])}" y1="${f(uA[1])}" x2="${f(uB[0])}" y2="${f(uB[1])}" gradientUnits="userSpaceOnUse">
      ${ustops.join('\n      ')}
    </linearGradient>
    <linearGradient id="${p}v" x1="${f(vA[0])}" y1="${f(vA[1])}" x2="${f(vB[0])}" y2="${f(vB[1])}" gradientUnits="userSpaceOnUse">
      ${vstops.join('\n      ')}
    </linearGradient>
    <linearGradient id="${p}under" x1="0" y1="0" x2="1" y2="0">
      <stop offset="0" stop-color="#B9681D"/>
      <stop offset="1" stop-color="#8E4712"/>
    </linearGradient>
    <linearGradient id="${p}rim" x1="${f(project(-1, 0.9)[0])}" y1="${f(project(-1, 0.9)[1])}" x2="${f(project(0.9, -0.6)[0])}" y2="${f(project(0.9, -0.6)[1])}" gradientUnits="userSpaceOnUse">
      <stop offset="0" stop-color="#FFF8E1" stop-opacity="0.95"/>
      <stop offset="0.45" stop-color="#FFF1C4" stop-opacity="0.5"/>
      <stop offset="0.75" stop-color="#FFF1C4" stop-opacity="0"/>
    </linearGradient>
    <radialGradient id="${p}spec" cx="0.5" cy="0.5" r="0.5">
      <stop offset="0" stop-color="#FFFFFF" stop-opacity="0.95"/>
      <stop offset="0.3" stop-color="#FFF8DE" stop-opacity="0.65"/>
      <stop offset="1" stop-color="#FFF3C8" stop-opacity="0"/>
    </radialGradient>
    <radialGradient id="${p}fade" cx="${P.cx}" cy="${P.cy}" r="${f(P.S * 1.05)}" gradientUnits="userSpaceOnUse">
      <stop offset="0.5" stop-color="#fff"/>
      <stop offset="1" stop-color="#fff" stop-opacity="0"/>
    </radialGradient>
    <mask id="${p}rm" maskUnits="userSpaceOnUse" x="0" y="0" width="512" height="512"><rect width="512" height="512" fill="url(#${p}fade)"/></mask>
    <filter id="${p}b1" x="-10%" y="-10%" width="120%" height="120%"><feGaussianBlur stdDeviation="1"/></filter>
    <filter id="${p}b3" x="-20%" y="-20%" width="140%" height="140%"><feGaussianBlur stdDeviation="3"/></filter>
    <filter id="${p}b8" x="-30%" y="-30%" width="160%" height="160%"><feGaussianBlur stdDeviation="8"/></filter>
    <filter id="${p}b16" x="-40%" y="-40%" width="180%" height="180%"><feGaussianBlur stdDeviation="16"/></filter>
    <filter id="${p}salt" x="-60%" y="-60%" width="220%" height="220%"><feDropShadow dx="0.5" dy="1.2" stdDeviation="0.7" flood-color="#6E3A0A" flood-opacity="0.5"/></filter>`;
}

function body(p, { shadow = true } = {}) {
  const light = P.ridges.map((v) => ruffle(v + P.ridgeOffset));
  const dark = P.ridges.map((v) => ruffle(v - P.ridgeOffset));
  const sheen = P.ridges.map((v) => ruffle(v + P.ridgeOffset * 1.4, 0.35));
  const [sx, sy] = project(spec[0], spec[1]);
  const rollDeg = P.roll / deg;
  const salts = salt.map(([u, v, s, r]) => {
    const [x, y] = project(u, v);
    return `<rect x="${f(x - s / 2)}" y="${f(y - s / 2)}" width="${s}" height="${s}" rx="${f(s * 0.3)}" transform="rotate(${r} ${f(x)} ${f(y)})" fill="#FFFDF5"/>`;
  });
  const marks = bubbles.map(([u, v, s]) => {
    const [x, y] = project(u, v);
    const t = `rotate(${f(rollDeg - 8)} ${f(x)} ${f(y)})`;
    return `<ellipse cx="${f(x)}" cy="${f(y)}" rx="${f(6 * s)}" ry="${f(3.4 * s)}" transform="${t}" fill="#B4661E" opacity="0.26"/><ellipse cx="${f(x - 0.8)}" cy="${f(y - 1.6)}" rx="${f(4.2 * s)}" ry="${f(1.5 * s)}" transform="${t}" fill="#FFF3CC" opacity="0.35"/>`;
  });
  return `
  ${shadow ? `<ellipse cx="${P.cx + 4}" cy="${P.cy + 150}" rx="${f(P.S * 0.8)}" ry="${f(P.S * 0.11)}" fill="#4A2405" opacity="0.2" filter="url(#${p}b16)"/>` : ''}
  <!-- thickness under the near edge -->
  <use href="#${p}o" transform="translate(1 6)" fill="url(#${p}under)"/>
  <!-- surface -->
  <use href="#${p}o" fill="url(#${p}u)"/>
  <g clip-path="url(#${p}clip)">
    <use href="#${p}o" fill="url(#${p}v)"/>
    <!-- ruffles -->
    <g mask="url(#${p}rm)" fill="none" stroke-linecap="round">
      ${dark.map((d) => `<path d="${d}" stroke="#A9561A" stroke-opacity="${P.ridgeDark}" stroke-width="9" filter="url(#${p}b3)"/>`).join('\n      ')}
      ${light.map((d) => `<path d="${d}" stroke="#FFF5D2" stroke-opacity="${P.ridgeLight}" stroke-width="4" filter="url(#${p}b3)"/>`).join('\n      ')}
      ${sheen.slice(0, 3).map((d, i) => `<path d="${d}" stroke="#FFFFFF" stroke-opacity="${[0.5, 0.35, 0.22][i]}" stroke-width="1.6" filter="url(#${p}b1)"/>`).join('\n      ')}
    </g>
    <!-- toasted edge -->
    <use href="#${p}o" fill="none" stroke="#B0601A" stroke-opacity="0.5" stroke-width="14" filter="url(#${p}b8)"/>
    <use href="#${p}o" fill="none" stroke="#A1531A" stroke-opacity="0.55" stroke-width="2.5"/>
    <!-- blisters -->
    ${marks.join('\n    ')}
    <!-- specular -->
    <ellipse cx="${f(sx)}" cy="${f(sy)}" rx="70" ry="30" transform="rotate(${f(rollDeg - 26)} ${f(sx)} ${f(sy)})" fill="url(#${p}spec)" opacity="0.85"/>
    <!-- light catching the rim -->
    <use href="#${p}o" fill="none" stroke="url(#${p}rim)" stroke-width="5"/>
  </g>
  <!-- salt -->
  <g filter="url(#${p}salt)">
    ${salts.join('\n    ')}
  </g>`;
}

const mark = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" fill="none">
  <title>Crispy</title>
  <defs>${defs('c-')}
  </defs>${body('c-')}
</svg>
`;
writeFileSync(`${OUT}/crispy-mark.svg`, mark);
// The UI's favicon is the same mark.
if (!process.env.OUT) writeFileSync(resolve(OUT, '../public/favicon.svg'), mark);

// ---- app icon (1024, macOS grid: 824 squircle, 185 radius) -------------------------------------
function squircle(x, y, w, h, Rr, s = 0.6) {
  const p = Math.min((1 + s) * Rr, w / 2);
  const rad = (d) => (d * Math.PI) / 180;
  const arcMeasure = 90 * (1 - s);
  const arcLen = Math.sin(rad(arcMeasure / 2)) * Rr * Math.SQRT2;
  const alpha = (90 - arcMeasure) / 2;
  const c = Rr * Math.tan(rad(alpha / 2)) * Math.cos(rad(45 * s));
  const d = c * Math.tan(rad(45 * s));
  const b = (p - arcLen - c - d) / 3;
  const a = 2 * b;
  const F = (n) => +n.toFixed(2);
  return [
    `M${F(x + w - p)} ${F(y)}`,
    `c${F(a)} 0 ${F(a + b)} 0 ${F(a + b + c)} ${F(d)}`,
    `a${Rr} ${Rr} 0 0 1 ${F(arcLen)} ${F(arcLen)}`,
    `c${F(d)} ${F(c)} ${F(d)} ${F(b + c)} ${F(d)} ${F(a + b + c)}`,
    `L${F(x + w)} ${F(y + h - p)}`,
    `c0 ${F(a)} 0 ${F(a + b)} ${F(-d)} ${F(a + b + c)}`,
    `a${Rr} ${Rr} 0 0 1 ${F(-arcLen)} ${F(arcLen)}`,
    `c${F(-c)} ${F(d)} ${F(-(b + c))} ${F(d)} ${F(-(a + b + c))} ${F(d)}`,
    `L${F(x + p)} ${F(y + h)}`,
    `c${F(-a)} 0 ${F(-(a + b))} 0 ${F(-(a + b + c))} ${F(-d)}`,
    `a${Rr} ${Rr} 0 0 1 ${F(-arcLen)} ${F(-arcLen)}`,
    `c${F(-d)} ${F(-c)} ${F(-d)} ${F(-(b + c))} ${F(-d)} ${F(-(a + b + c))}`,
    `L${F(x)} ${F(y + p)}`,
    `c0 ${F(-a)} 0 ${F(-(a + b))} ${F(d)} ${F(-(a + b + c))}`,
    `a${Rr} ${Rr} 0 0 1 ${F(arcLen)} ${F(-arcLen)}`,
    `c${F(c)} ${F(-d)} ${F(b + c)} ${F(-d)} ${F(a + b + c)} ${F(-d)}`,
    'Z',
  ].join('');
}

const icon = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024" fill="none">
  <title>Crispy</title>
  <defs>
    <path id="sq" d="${squircle(100, 100, 824, 824, 185)}"/>
    <clipPath id="sqclip"><use href="#sq"/></clipPath>
    <linearGradient id="bg" x1="512" y1="100" x2="512" y2="924" gradientUnits="userSpaceOnUse">
      <stop offset="0" stop-color="#2F2823"/>
      <stop offset="0.48" stop-color="#1A1613"/>
      <stop offset="1" stop-color="#0C0A09"/>
    </linearGradient>
    <radialGradient id="glow" cx="512" cy="500" r="430" gradientUnits="userSpaceOnUse">
      <stop offset="0" stop-color="#F2B33D" stop-opacity="0.36"/>
      <stop offset="0.42" stop-color="#E08A2A" stop-opacity="0.13"/>
      <stop offset="1" stop-color="#E08A2A" stop-opacity="0"/>
    </radialGradient>
    <linearGradient id="edge" x1="512" y1="100" x2="512" y2="924" gradientUnits="userSpaceOnUse">
      <stop offset="0" stop-color="#FFFFFF" stop-opacity="0.24"/>
      <stop offset="0.1" stop-color="#FFFFFF" stop-opacity="0.07"/>
      <stop offset="0.55" stop-color="#FFFFFF" stop-opacity="0"/>
      <stop offset="1" stop-color="#FFFFFF" stop-opacity="0.04"/>
    </linearGradient>
    <linearGradient id="sheen" x1="512" y1="100" x2="512" y2="520" gradientUnits="userSpaceOnUse">
      <stop offset="0" stop-color="#FFFFFF" stop-opacity="0.07"/>
      <stop offset="1" stop-color="#FFFFFF" stop-opacity="0"/>
    </linearGradient>
    <filter id="ishadow" x="-30%" y="-30%" width="160%" height="160%"><feGaussianBlur stdDeviation="24"/></filter>
    ${defs('i-')}
  </defs>
  <use href="#sq" fill="url(#bg)"/>
  <g clip-path="url(#sqclip)">
    <rect x="100" y="100" width="824" height="824" fill="url(#glow)"/>
    <ellipse cx="512" cy="300" rx="520" ry="260" fill="url(#sheen)"/>
    <ellipse cx="518" cy="728" rx="240" ry="40" fill="#000" opacity="0.6" filter="url(#ishadow)"/>
    <g transform="translate(512 508) scale(1.32) translate(-256 -258)">${body('i-', { shadow: false })}
    </g>
  </g>
  <use href="#sq" fill="none" stroke="url(#edge)" stroke-width="3"/>
</svg>
`;
writeFileSync(`${OUT}/crispy-icon.svg`, icon);

// ---- tray / menu bar template (64, flat black, ruffles cut out) --------------------------------
const trayRidges = [-0.24, 0.2].map((v) => ruffle(v, 0.3));
const tray = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64">
  <title>Crispy</title>
  <defs>
    <mask id="cut" maskUnits="userSpaceOnUse" x="0" y="0" width="512" height="512">
      <rect width="512" height="512" fill="#fff"/>
      <g fill="none" stroke="#000" stroke-width="24" stroke-linecap="round">
        ${trayRidges.map((d) => `<path d="${d}"/>`).join('\n        ')}
      </g>
    </mask>
  </defs>
  <g transform="translate(32 32) scale(${(64 / 512 * 1.17).toFixed(4)}) translate(-256 -258)">
    <path d="${outline}" fill="#000" mask="url(#cut)"/>
  </g>
</svg>
`;
writeFileSync(`${OUT}/tray-template.svg`, tray);
console.log(`wrote ${OUT}/crispy-mark.svg, crispy-icon.svg, tray-template.svg`);
