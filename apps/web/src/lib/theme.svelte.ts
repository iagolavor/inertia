export type Theme = 'light' | 'dark';
export type Palette = 'sandstone' | 'midnight';

const STORAGE_KEY = 'inertia-appearance';
const LEGACY_KEY = 'inertia-theme';

export const paletteLabels = {
	sandstone: {
		name: 'Sandstone',
		light: 'Driftwood',
		dark: 'Night'
	},
	midnight: {
		name: 'Midnight',
		light: 'Mist',
		dark: 'Classic'
	}
} as const;

export const themeState = $state({
	palette: 'midnight' as Palette,
	mode: 'dark' as Theme
});

function applyAppearance(palette: Palette, mode: Theme) {
	if (typeof document === 'undefined') return;
	document.documentElement.setAttribute('data-palette', palette);
	document.documentElement.setAttribute('data-theme', mode);
}

function persistAppearance(palette: Palette, mode: Theme) {
	localStorage.setItem(STORAGE_KEY, JSON.stringify({ palette, mode }));
}

function readSavedAppearance(): { palette: Palette; mode: Theme } | null {
	const raw = localStorage.getItem(STORAGE_KEY);
	if (!raw) return null;

	try {
		const parsed = JSON.parse(raw) as { palette?: string; mode?: string };
		const palette = parsed.palette === 'sandstone' || parsed.palette === 'midnight' ? parsed.palette : null;
		const mode = parsed.mode === 'light' || parsed.mode === 'dark' ? parsed.mode : null;
		if (palette && mode) return { palette, mode };
	} catch {
		/* ignore malformed storage */
	}

	return null;
}

export function initTheme() {
	if (typeof localStorage === 'undefined') return;

	const saved = readSavedAppearance();
	if (saved) {
		themeState.palette = saved.palette;
		themeState.mode = saved.mode;
		applyAppearance(saved.palette, saved.mode);
		return;
	}

	const legacy = localStorage.getItem(LEGACY_KEY);
	const mode: Theme = legacy === 'light' || legacy === 'dark' ? legacy : 'dark';
	themeState.palette = 'midnight';
	themeState.mode = mode;
	applyAppearance('midnight', mode);
	persistAppearance('midnight', mode);
}

export function setPalette(palette: Palette) {
	themeState.palette = palette;
	persistAppearance(palette, themeState.mode);
	applyAppearance(palette, themeState.mode);
}

/** Light or dark mode within the active palette. */
export function setTheme(mode: Theme) {
	themeState.mode = mode;
	persistAppearance(themeState.palette, mode);
	applyAppearance(themeState.palette, mode);
}

export function activeVariantLabel(palette: Palette, mode: Theme): string {
	return paletteLabels[palette][mode];
}
