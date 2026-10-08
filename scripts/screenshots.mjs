// Screenshots of the UI running against the in-browser mock backend.
//
//   node scripts/screenshots.mjs [outDir] [--only=name1,name2] [--prefix=pattern]
//
// Starts a Vite dev server, opens each scenario in headless Chromium and saves a PNG.
// Uses playwright-core with a preinstalled Chromium (override with CHROMIUM_PATH).
import { createServer } from 'vite';
import { chromium } from 'playwright-core';
import { mkdir } from 'node:fs/promises';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const args = process.argv.slice(2);
const outDir = resolve(args.find((a) => !a.startsWith('--')) ?? `${root}/screenshots`);
const only = args.find((a) => a.startsWith('--only='))?.slice(7).split(',');
const executablePath = process.env.CHROMIUM_PATH ?? '/opt/pw-browsers/chromium-1194/chrome-linux/chrome';

const W = { width: 1100, height: 740 };
const scrollBy = (y) => async (p) => {
  await p.evaluate((dy) => document.querySelector('.scroll')?.scrollBy(0, dy), y);
  await p.waitForTimeout(250);
};
const S = { width: 880, height: 600 };

/** name, query string, viewport, optional interaction */
const shots = [
  ['devices-dark', 'theme=dark', W],
  ['devices-light', 'theme=light', W],
  ['devices-windows-dark', 'theme=dark&os=windows', W],
  ['devices-windows-light', 'theme=light&os=windows&flavor=saltVinegar', W],
  ['devices-min-880', 'theme=dark', S],
  ['devices-empty', 'theme=dark&empty=1', W],
  ['devices-permission', 'theme=dark&perm=denied&warning=1', W],
  ['devices-stats-bbq', 'theme=dark&flavor=bbq&stats=1', W],
  ['devices-translucent', 'theme=dark&effects=1&flavor=sourCream', W],
  ['devices-translucent-light', 'theme=light&effects=1', W],
  ['arrangement-dark', 'theme=dark&page=arrangement', W],
  ['arrangement-light', 'theme=light&page=arrangement', W],
  ['arrangement-min-880', 'theme=dark&page=arrangement&os=windows', S],
  ['transfers-dark', 'theme=dark&page=transfers', W],
  ['transfers-light', 'theme=light&page=transfers&flavor=truffle', W],
  ['transfers-min-880', 'theme=dark&page=transfers', S],
  ['settings-general-dark', 'theme=dark&page=settings&section=general', W],
  ['settings-general-light', 'theme=light&page=settings&section=general', W],
  ['settings-switching-dark', 'theme=dark&page=settings&section=switching', W],
  ['settings-mouse-light', 'theme=light&page=settings&section=mouse', W],
  ['settings-keyboard-dark', 'theme=dark&page=settings&section=keyboard', W],
  ['settings-shortcuts-light', 'theme=light&page=settings&section=shortcuts', W],
  ['settings-clipboard-dark', 'theme=dark&page=settings&section=clipboard', W],
  ['settings-files-dark', 'theme=dark&page=settings&section=files&os=windows', W],
  ['settings-network-dark', 'theme=dark&page=settings&section=network', W],
  ['settings-advanced-light', 'theme=light&page=settings&section=advanced', W],
  ['settings-about-dark', 'theme=dark&page=settings&section=about', W],
  ['settings-switching-scrolled', 'theme=dark&page=settings&section=switching', W, scrollBy(760)],
  ['settings-network-scrolled-light', 'theme=light&page=settings&section=network', W, scrollBy(700)],
  ['settings-about-scrolled', 'theme=dark&page=settings&section=about', W, scrollBy(600)],
  ['settings-files-scrolled', 'theme=light&page=settings&section=files&flavor=bbq', W, scrollBy(300)],
  ['settings-min-880', 'theme=dark&page=settings&section=keyboard', S],
  ['pairing-enter-dark', 'theme=dark&pairing=enterCode', W, async (p) => {
    await p.keyboard.type('4829');
    await p.waitForTimeout(200);
  }],
  ['pairing-show-light', 'theme=light&pairing=showCode', W],
  ['pairing-success-dark', 'theme=dark&pairing=success', W, async (p) => p.waitForTimeout(1200)],
  ['pairing-failed-dark', 'theme=dark&pairing=failed', W],
  ['send-picker-dark', 'theme=dark&send=1', W],
  ['add-by-ip-light', 'theme=light', W, async (p) => {
    await p.getByRole('button', { name: 'Add by IP' }).click();
    await p.keyboard.type('192.168.1.88');
    await p.waitForTimeout(400);
  }],
  ['card-menu-dark', 'theme=dark', W, async (p) => {
    await p.getByRole('button', { name: 'Studio PC options' }).click();
    await p.waitForTimeout(400);
  }],
  ['onboarding-1-dark', 'theme=dark&onboarding=1', W],
  ['onboarding-2-dark', 'theme=dark&onboarding=1&step=2', W],
  ['onboarding-3-dark', 'theme=dark&onboarding=1&step=3', W],
  ['onboarding-4-dark', 'theme=dark&onboarding=1&step=4', W],
  ['onboarding-1-light', 'theme=light&onboarding=1&flavor=sourCream', W],
  ['onboarding-3-windows', 'theme=dark&onboarding=1&step=3&os=windows', W],
  ['onboarding-min-880', 'theme=dark&onboarding=1&step=4', S],
];

await mkdir(outDir, { recursive: true });
const server = await createServer({
  root,
  logLevel: 'error',
  server: { port: 4321, strictPort: false },
});
await server.listen();
const base = server.resolvedUrls.local[0];

const browser = await chromium.launch({ executablePath });
const errors = [];
for (const [name, query, viewport, act] of shots) {
  if (only && !only.some((o) => name.startsWith(o))) continue;
  const scheme = query.includes('theme=light') ? 'light' : 'dark';
  const page = await browser.newPage({ viewport, deviceScaleFactor: 2, colorScheme: scheme });
  page.on('console', (m) => {
    if (m.type() === 'error' || m.type() === 'warning') errors.push(`${name}: [${m.type()}] ${m.text()}`);
  });
  page.on('pageerror', (e) => errors.push(`${name}: [pageerror] ${e.message}`));
  await page.goto(`${base}?still=1&${query}`, { waitUntil: 'networkidle' });
  await page.evaluate(() => document.fonts.ready);
  await page.waitForTimeout(900);
  if (act) await act(page);
  await page.screenshot({ path: `${outDir}/${name}.png` });
  await page.close();
  console.log('shot', name);
}
await browser.close();
await server.close();
if (errors.length) {
  console.log('\nConsole problems:');
  for (const e of errors) console.log('  ' + e);
}
console.log(`\nsaved to ${outDir}`);
