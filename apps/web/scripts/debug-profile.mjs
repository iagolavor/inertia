/**
 * Dev helper: open profile pages with puppeteer-core and report failing API calls.
 * Usage: node scripts/debug-profile.mjs [baseUrl]
 */
import puppeteer from 'puppeteer-core';
import { existsSync } from 'node:fs';

const BASE = process.argv[2] || 'http://[::1]:5173';
const CHROME =
	process.env.CHROME_PATH ||
	'/tmp/inertia-chrome/chrome/linux-150.0.7871.115/chrome-linux64/chrome';

if (!existsSync(CHROME)) {
	console.error('Chrome not found at', CHROME);
	process.exit(1);
}

const failures = [];
const consoleErrors = [];

const browser = await puppeteer.launch({
	executablePath: CHROME,
	headless: true,
	args: ['--no-sandbox', '--disable-setuid-sandbox', '--disable-dev-shm-usage']
});

async function probe(path) {
	const page = await browser.newPage();
	const localFails = [];
	page.on('console', (msg) => {
		if (msg.type() === 'error') {
			consoleErrors.push({ path, text: msg.text() });
		}
	});
	page.on('pageerror', (err) => {
		consoleErrors.push({ path, text: String(err) });
	});
	const apiHits = [];
	page.on('response', async (res) => {
		const url = res.url();
		if (!url.includes('/api/')) return;
		const status = res.status();
		let body = '';
		try {
			body = (await res.text()).slice(0, 200);
		} catch {
			/* ignore */
		}
		apiHits.push({ url: url.replace(/^https?:\/\/[^/]+/, ''), status, body });
		if (status >= 400) {
			const entry = { path, url, status, body };
			localFails.push(entry);
			failures.push(entry);
		}
	});

	console.log(`\n=== GET ${BASE}${path} ===`);
	// SSE keeps networkidle from ever settling; use DOM ready + settle delay.
	await page.goto(`${BASE}${path}`, { waitUntil: 'domcontentloaded', timeout: 30_000 });
	// Wait for identity boot (layout refreshIdentity) before asserting page content.
	await page.waitForFunction(
		() => {
			const t = document.body?.innerText || '';
			return !t.includes('Loading...') || t.includes('Posts') || t.includes('Files') || t.includes('Friend');
		},
		{ timeout: 15_000 }
	).catch(() => {});
	await new Promise((r) => setTimeout(r, 2000));

	// Click Files tab if present
	const filesClicked = await page.evaluate(() => {
		const buttons = [...document.querySelectorAll('button.grid-tab, .grid-tab')];
		const files = buttons.find((b) => /files/i.test(b.textContent || ''));
		if (files) {
			files.click();
			return true;
		}
		return false;
	});
	if (filesClicked) {
		console.log('Clicked Files tab');
		await new Promise((r) => setTimeout(r, 1500));
	}

	const text = await page.evaluate(() => document.body?.innerText?.slice(0, 800) || '');
	console.log('Visible text snippet:\n', text.replace(/\n+/g, '\n').slice(0, 500));
	console.log(
		'API hits:',
		apiHits
			.filter((h) => !h.url.includes('/p2p/events'))
			.map((h) => `${h.status} ${h.url}`)
			.join('\n  ') || 'none'
	);
	console.log('API failures on this page:', localFails.length ? localFails : 'none');
	await page.close();
}

try {
	await probe('/profile');
	const contacts = await fetch('http://127.0.0.1:4783/api/contacts').then((r) => r.json());
	const friend =
		contacts.find((c) => c.connection_state === 'online') ||
		contacts.find((c) => c.connection_state === 'reachable') ||
		contacts[0];
	if (friend) {
		console.log(`Probing friend ${friend.display_name} (${friend.id.slice(0, 8)}…)`);
		await probe(`/friends/${friend.id}/profile`);
	} else {
		console.log('No contacts to probe friend profile');
	}
} finally {
	await browser.close();
}

console.log('\n=== SUMMARY ===');
const realFails = failures.filter((f) => !f.url.includes('favicon'));
console.log('API failures:', JSON.stringify(realFails, null, 2));
console.log(
	'Console errors:',
	JSON.stringify(
		consoleErrors.filter((e) => !/favicon|404 \(Not Found\)/.test(e.text)),
		null,
		2
	)
);
process.exit(
	realFails.some((f) => f.status >= 500 && !f.body?.includes('friend_offline')) ? 1 : 0
);
