// Renders the brand SVGs into the app icons Tauri bundles (src-tauri/icons).
//
//   node scripts/build-icons.mjs && npx tauri icon src-tauri/icons/icon-source.png -o src-tauri/icons
//
// The tray icon is rendered separately: a black template image for the macOS menu bar.
import { chromium } from 'playwright-core';
import { readFile, mkdir } from 'node:fs/promises';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const out = `${root}/src-tauri/icons`;
const executablePath = process.env.CHROMIUM_PATH ?? '/opt/pw-browsers/chromium-1194/chrome-linux/chrome';

await mkdir(out, { recursive: true });
const browser = await chromium.launch({ executablePath });
const page = await browser.newPage({ deviceScaleFactor: 1 });

async function render(file, size, dest) {
  const svg = (await readFile(`${root}/branding/${file}`, 'utf8')).replace('<svg', `<svg width="${size}" height="${size}"`);
  await page.setViewportSize({ width: size, height: size });
  await page.setContent(`<!doctype html><html><body style="margin:0;background:transparent">${svg}</body></html>`);
  await page.waitForTimeout(60);
  await page.screenshot({ path: dest, omitBackground: true, clip: { x: 0, y: 0, width: size, height: size } });
}

await render('crispy-icon.svg', 1024, `${out}/icon-source.png`);
await render('tray-template.svg', 44, `${out}/tray-template.png`);
await browser.close();
console.log(`icons rendered to ${out}`);
