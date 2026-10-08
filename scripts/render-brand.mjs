// Renders the brand SVGs in branding/ to PNG previews with headless Chromium.
//
//   node scripts/render-brand.mjs [outDir] [--export]
//
// --export also writes production PNGs to branding/png/ (app icon 1024/512, mark, tray template).
//
// Uses playwright-core with a preinstalled Chromium (set CHROMIUM_PATH to override).
import { chromium } from 'playwright-core';
import { readFile, mkdir } from 'node:fs/promises';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const outArg = process.argv.slice(2).find((a) => !a.startsWith('--'));
const outDir = resolve(outArg ?? `${root}/branding/previews`);
const executablePath = process.env.CHROMIUM_PATH ?? '/opt/pw-browsers/chromium-1194/chrome-linux/chrome';

const jobs = [
  { file: 'crispy-mark.svg', sizes: [512, 128, 32, 16], bgs: ['transparent', '#0F0E0D', '#F7F5F0'] },
  { file: 'crispy-icon.svg', sizes: [1024, 256, 64, 32], bgs: ['transparent', '#F7F5F0', '#1d1c1a'] },
  { file: 'tray-template.svg', sizes: [64, 32, 22, 16], bgs: ['#F7F5F0', '#2b2a28'] },
];

await mkdir(outDir, { recursive: true });
const browser = await chromium.launch({ executablePath });
const page = await browser.newPage({ deviceScaleFactor: 1 });

for (const job of jobs) {
  let svg = await readFile(`${root}/branding/${job.file}`, 'utf8');
  const base = job.file.replace(/\.svg$/, '');
  for (const size of job.sizes) {
    for (const bg of job.bgs) {
      const tray = job.file.startsWith('tray') && bg !== '#F7F5F0';
      const html = `<!doctype html><html><body style="margin:0;background:${bg};">
        <div style="width:${size}px;height:${size}px;${tray ? 'filter:invert(1)' : ''}">${svg.replace('<svg', `<svg width="${size}" height="${size}"`)}</div></body></html>`;
      await page.setViewportSize({ width: size, height: size });
      await page.setContent(html);
      await page.waitForTimeout(50);
      const tag = bg === 'transparent' ? 'clear' : bg.replace('#', '');
      await page.screenshot({ path: `${outDir}/${base}-${size}-${tag}.png`, omitBackground: bg === 'transparent' });
    }
  }
}

// Production PNGs (transparent) next to the SVGs, for `tauri icon` and the tray.
if (process.argv.includes('--export')) {
  await mkdir(`${root}/branding/png`, { recursive: true });
  const exports = [
    ['crispy-icon.svg', 1024, 'crispy-icon-1024.png'],
    ['crispy-icon.svg', 512, 'crispy-icon-512.png'],
    ['crispy-mark.svg', 512, 'crispy-mark-512.png'],
    ['crispy-mark.svg', 128, 'crispy-mark-128.png'],
    ['tray-template.svg', 22, 'tray-template.png'],
    ['tray-template.svg', 44, 'tray-template@2x.png'],
    ['tray-template.svg', 32, 'tray-32.png'],
    ['tray-template.svg', 64, 'tray-64.png'],
  ];
  for (const [file, size, name] of exports) {
    const svg = await readFile(`${root}/branding/${file}`, 'utf8');
    await page.setViewportSize({ width: size, height: size });
    await page.setContent(`<!doctype html><body style="margin:0;background:transparent">${svg.replace('<svg', `<svg width="${size}" height="${size}"`)}</body>`);
    await page.waitForTimeout(50);
    await page.screenshot({ path: `${root}/branding/png/${name}`, omitBackground: true });
  }
}

// A contact sheet with all sizes side by side, dark and light.
const mark = await readFile(`${root}/branding/crispy-mark.svg`, 'utf8');
const icon = await readFile(`${root}/branding/crispy-icon.svg`, 'utf8');
const tray = await readFile(`${root}/branding/tray-template.svg`, 'utf8');
const sized = (s, n) => s.replace('<svg', `<svg width="${n}" height="${n}"`);
const row = (bg, fg) => `<div style="display:flex;gap:28px;align-items:center;padding:28px;background:${bg};color:${fg}">
  ${[256, 128, 64, 32, 16].map((n) => sized(mark, n)).join('')}
  ${[256, 64, 32].map((n) => sized(icon, n)).join('')}
  <div style="display:flex;gap:14px;${bg === '#0F0E0D' ? 'filter:invert(1)' : ''}">${[32, 22, 16].map((n) => sized(tray, n)).join('')}</div>
</div>`;
await page.setViewportSize({ width: 1400, height: 640 });
await page.setContent(`<!doctype html><body style="margin:0">${row('#0F0E0D', '#fff')}${row('#F7F5F0', '#000')}</body>`);
await page.waitForTimeout(80);
await page.screenshot({ path: `${outDir}/contact-sheet.png`, fullPage: true });

await browser.close();
console.log(`rendered to ${outDir}`);
