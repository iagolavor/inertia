/**
 * Capture Files tab screenshot for PR description.
 * Usage: node scripts/screenshot-files-tab.mjs [outPath] [baseUrl]
 */
import puppeteer from 'puppeteer-core';
import { existsSync, mkdirSync } from 'node:fs';
import { dirname } from 'node:path';

const OUT = process.argv[2] || 'docs/assets/files-tab-preview.png';
const BASE = process.argv[3] || 'http://[::1]:5173';
const CHROME =
	process.env.CHROME_PATH ||
	'/tmp/inertia-chrome/chrome/linux-150.0.7871.115/chrome-linux64/chrome';

if (!existsSync(CHROME)) {
	console.error('Chrome not found at', CHROME);
	process.exit(1);
}

mkdirSync(dirname(OUT), { recursive: true });

const browser = await puppeteer.launch({
	executablePath: CHROME,
	headless: true,
	defaultViewport: { width: 980, height: 900, deviceScaleFactor: 2 },
	args: ['--no-sandbox', '--disable-setuid-sandbox', '--disable-dev-shm-usage']
});

const page = await browser.newPage();
await page.goto(`${BASE}/profile`, { waitUntil: 'domcontentloaded', timeout: 30_000 });
await page
	.waitForFunction(
		() => {
			const t = document.body?.innerText || '';
			return t.includes('Files') && (t.includes('Posts') || t.includes('Iago'));
		},
		{ timeout: 15_000 }
	)
	.catch(() => {});
await new Promise((r) => setTimeout(r, 1500));

await page.evaluate(() => {
	const buttons = [...document.querySelectorAll('button.grid-tab, .grid-tab')];
	const files = buttons.find((b) => /files/i.test(b.textContent || ''));
	files?.click();
});
await new Promise((r) => setTimeout(r, 1200));

// Prefer clipping the finder pane; fall back to profile main content.
const clip = await page.evaluate(() => {
	const finder = document.querySelector('.finder');
	const el = finder || document.querySelector('.profile-header')?.parentElement;
	if (!el) return null;
	const r = el.getBoundingClientRect();
	return {
		x: Math.max(0, r.x - 12),
		y: Math.max(0, r.y - 12),
		width: Math.min(window.innerWidth - 8, r.width + 24),
		height: Math.min(window.innerHeight - 8, r.height + 24)
	};
});

if (clip && clip.width > 40 && clip.height > 40) {
	await page.screenshot({ path: OUT, clip, type: 'png' });
} else {
	await page.screenshot({ path: OUT, fullPage: false, type: 'png' });
}

await browser.close();
console.log('Wrote', OUT);
