export type Theme = 'light' | 'dark';

const STORAGE_KEY = 'inertia-theme';

export const themeState = $state({
	theme: 'dark' as Theme
});

function applyTheme(theme: Theme) {
	if (typeof document !== 'undefined') {
		document.documentElement.setAttribute('data-theme', theme);
	}
}

export function initTheme() {
	if (typeof localStorage === 'undefined') return;

	const saved = localStorage.getItem(STORAGE_KEY);
	const theme = saved === 'light' || saved === 'dark' ? saved : 'dark';
	themeState.theme = theme;
	applyTheme(theme);
}

export function setTheme(theme: Theme) {
	themeState.theme = theme;
	localStorage.setItem(STORAGE_KEY, theme);
	applyTheme(theme);
}
