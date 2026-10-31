import { colorPresets } from '$lib/theme/presets';
import { getSettings, setSettings } from '$lib/stores/settings';
import { matches } from '../utils';
import type { PaletteItem, PaletteProvider } from '../types';
import type { Theme } from '$lib/types/settings';

const MODES: { id: Theme; label: string }[] = [
	{ id: 'dark', label: 'Dark mode' },
	{ id: 'light', label: 'Light mode' },
	{ id: 'system', label: 'System theme' }
];

function buildItems(): PaletteItem[] {
	const settings = getSettings();
	const modes: PaletteItem[] = MODES.map((mode) => ({
		id: `mode-${mode.id}`,
		label: mode.label,
		icon: 'paint-brush' as const,
		hint: settings.theme === mode.id ? 'Active' : '',
		keywords: 'theme appearance color scheme',
		// the shell previews this on highlight and reverts unless selected
		preview: () => {
			setSettings({ theme: mode.id });
		},
		run: () => {
			setSettings({ theme: mode.id });
		}
	}));
	const presets: PaletteItem[] = colorPresets.map((preset) => ({
		id: `preset-${preset.id}`,
		label: preset.label,
		icon: 'palette' as const,
		hint: settings.themePreset === preset.id ? 'Active' : '',
		keywords: 'theme palette color preset appearance',
		preview: () => {
			setSettings({ themePreset: preset.id });
		},
		run: () => {
			setSettings({ themePreset: preset.id });
		}
	}));
	return [...modes, ...presets];
}

// hidden until typed: three dozen presets would crowd the empty state
export const themesProvider: PaletteProvider = {
	id: 'themes',
	heading: 'Themes',
	search(query) {
		if (!query) return [];
		return buildItems().filter((item) => matches(`${item.label} ${item.keywords ?? ''}`, query));
	},
	resolve(itemId) {
		return buildItems().find((item) => item.id === itemId) ?? null;
	}
};
