// color presets the user can pick from in Settings -> Themes
// `default` is the hand-tuned theme in layout.css and has no override block.
// every other preset keeps the same light/dark surface ladder but tints the
// whole palette (background, cards, popovers, sidebar) with a single hue.
// one preset per colour family: varied, not a pile of near-identical variants
export type ColorPresetId =
	| 'default'
	| 'rose'
	| 'ember'
	| 'gold'
	| 'lime'
	| 'forest'
	| 'teal'
	| 'ocean'
	| 'violet'
	| 'magenta'
	| 'sand'
	| 'slate'
	| 'k9crypt';

export type ColorPreset = {
	id: ColorPresetId;
	label: string;
	// preview colors mixed into the picker swatch (light -> dark gradient)
	light: { background: string; primary: string };
	dark: { background: string; primary: string };
};

// ordered by hue (warm -> cool) so the picker reads like a color wheel;
// the two muted neutrals come next and the branded K9Crypt theme sits last
export const colorPresets: ColorPreset[] = [
	{
		id: 'default',
		label: 'Default',
		light: { background: 'oklch(0.975 0.0015 264)', primary: 'oklch(0.145 0.004 264)' },
		dark: { background: 'oklch(0.145 0.003 264)', primary: 'oklch(0.985 0.001 264)' }
	},
	{
		id: 'rose',
		label: 'Rose',
		light: { background: 'oklch(0.965 0.026 15)', primary: 'oklch(0.55 0.17 15)' },
		dark: { background: 'oklch(0.145 0.022 15)', primary: 'oklch(0.73 0.14 15)' }
	},
	{
		id: 'ember',
		label: 'Ember',
		light: { background: 'oklch(0.965 0.026 40)', primary: 'oklch(0.5 0.14 40)' },
		dark: { background: 'oklch(0.145 0.022 40)', primary: 'oklch(0.76 0.14 40)' }
	},
	{
		id: 'gold',
		label: 'Gold',
		light: { background: 'oklch(0.965 0.026 80)', primary: 'oklch(0.5 0.12 80)' },
		dark: { background: 'oklch(0.145 0.022 80)', primary: 'oklch(0.8 0.13 80)' }
	},
	{
		id: 'lime',
		label: 'Lime',
		light: { background: 'oklch(0.965 0.026 125)', primary: 'oklch(0.5 0.13 125)' },
		dark: { background: 'oklch(0.145 0.022 125)', primary: 'oklch(0.74 0.13 125)' }
	},
	{
		id: 'forest',
		label: 'Forest',
		light: { background: 'oklch(0.965 0.026 150)', primary: 'oklch(0.5 0.13 150)' },
		dark: { background: 'oklch(0.145 0.022 150)', primary: 'oklch(0.72 0.12 150)' }
	},
	{
		id: 'teal',
		label: 'Teal',
		light: { background: 'oklch(0.965 0.026 190)', primary: 'oklch(0.53 0.12 190)' },
		dark: { background: 'oklch(0.145 0.022 190)', primary: 'oklch(0.73 0.11 190)' }
	},
	{
		id: 'ocean',
		label: 'Ocean',
		light: { background: 'oklch(0.965 0.026 245)', primary: 'oklch(0.55 0.16 245)' },
		dark: { background: 'oklch(0.145 0.022 245)', primary: 'oklch(0.73 0.13 245)' }
	},
	{
		id: 'violet',
		label: 'Violet',
		light: { background: 'oklch(0.965 0.026 300)', primary: 'oklch(0.55 0.18 300)' },
		dark: { background: 'oklch(0.145 0.022 300)', primary: 'oklch(0.74 0.14 300)' }
	},
	{
		id: 'magenta',
		label: 'Magenta',
		light: { background: 'oklch(0.965 0.026 335)', primary: 'oklch(0.55 0.18 335)' },
		dark: { background: 'oklch(0.145 0.022 335)', primary: 'oklch(0.73 0.15 335)' }
	},
	{
		id: 'sand',
		label: 'Sand',
		light: { background: 'oklch(0.965 0.014 65)', primary: 'oklch(0.4 0.05 65)' },
		dark: { background: 'oklch(0.145 0.012 65)', primary: 'oklch(0.78 0.06 65)' }
	},
	{
		id: 'slate',
		label: 'Slate',
		light: { background: 'oklch(0.965 0.01 250)', primary: 'oklch(0.4 0.04 250)' },
		dark: { background: 'oklch(0.145 0.009 250)', primary: 'oklch(0.78 0.04 250)' }
	},
	{
		id: 'k9crypt',
		label: 'K9Crypt',
		light: { background: 'oklch(0.965 0.011 118)', primary: 'oklch(0.5 0.13 118)' },
		dark: { background: 'oklch(0.161 0.008 118.75)', primary: 'oklch(1 0 0)' }
	}
];

export const DEFAULT_COLOR_PRESET: ColorPresetId = 'default';

export function isColorPresetId(value: unknown): value is ColorPresetId {
	return typeof value === 'string' && colorPresets.some((preset) => preset.id === value);
}
