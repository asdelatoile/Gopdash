import { resetMode, setMode } from 'mode-watcher';
import type { ThemeConfig, ThemeMode, ThemePreset } from './types';

const PRESET_ATTR = 'data-preset';
const PRESET_STORAGE_KEY = 'gopdash-theme-preset';

export const THEME_PRESETS: { id: ThemePreset; label: string }[] = [
	{ id: 'default', label: 'Default' },
	{ id: 'ocean', label: 'Ocean' },
	{ id: 'forest', label: 'Forest' },
	{ id: 'rose', label: 'Rose' }
];

export const themeState = $state({
	preset: 'default' as ThemePreset
});

export function applyTheme(theme: ThemeConfig) {
	if (typeof document === 'undefined') return;

	const savedPreset = localStorage.getItem(PRESET_STORAGE_KEY) as ThemePreset | null;
	const preset =
		savedPreset && THEME_PRESETS.some((p) => p.id === savedPreset) ? savedPreset : theme.preset;

	setPreset(preset);
	setMode(theme.mode as ThemeMode);
}

export function setPreset(preset: ThemePreset) {
	if (typeof document === 'undefined') return;

	document.documentElement.setAttribute(PRESET_ATTR, preset);
	themeState.preset = preset;
	localStorage.setItem(PRESET_STORAGE_KEY, preset);
}

export function setThemeMode(mode: ThemeMode) {
	if (mode === 'system') {
		resetMode();
		return;
	}
	setMode(mode);
}
