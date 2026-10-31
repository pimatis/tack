// color presets the user can pick from in Settings -> Themes
// `default` is the hand-tuned theme in layout.css and has no override block.
// every other preset keeps the same light/dark surface ladder but tints the
// whole palette (background, cards, popovers, sidebar) with a single hue.
// one preset per colour family: varied, not a pile of near-identical variants
export type ColorPresetId =
	| 'default'
	| 'rose'
	| 'ember'
	| 'amber'
	| 'gold'
	| 'olive'
	| 'lime'
	| 'forest'
	| 'mint'
	| 'teal'
	| 'sky'
	| 'ocean'
	| 'indigo'
	| 'violet'
	| 'magenta'
	| 'crimson'
	| 'sand'
	| 'coffee'
	| 'slate'
	| 'sage'
	| 'lavender'
	| 'blush'
	| 'nord'
	| 'dracula'
	| 'catppuccin'
	| 'solarized'
	| 'gruvbox'
	| 'onedark'
	| 'onedarkpro'
	| 'oled'
	| 'contrast'
	| 'k9crypt';

export type ColorPreset = {
	id: ColorPresetId;
	label: string;
	// preview colors mixed into the picker swatch (light -> dark gradient)
	light: { background: string; primary: string };
	dark: { background: string; primary: string };
};

// ordered by hue (warm -> cool) so the picker reads like a color wheel;
// muted variants come next, then ported palettes and the accessibility
// preset, and the branded K9Crypt theme sits last
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
		id: 'amber',
		label: 'Amber',
		light: { background: 'oklch(0.965 0.026 60)', primary: 'oklch(0.52 0.13 60)' },
		dark: { background: 'oklch(0.145 0.022 60)', primary: 'oklch(0.78 0.13 60)' }
	},
	{
		id: 'gold',
		label: 'Gold',
		light: { background: 'oklch(0.965 0.026 80)', primary: 'oklch(0.5 0.12 80)' },
		dark: { background: 'oklch(0.145 0.022 80)', primary: 'oklch(0.8 0.13 80)' }
	},
	{
		id: 'olive',
		label: 'Olive',
		light: { background: 'oklch(0.965 0.026 105)', primary: 'oklch(0.5 0.11 105)' },
		dark: { background: 'oklch(0.145 0.022 105)', primary: 'oklch(0.75 0.11 105)' }
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
		id: 'mint',
		label: 'Mint',
		light: { background: 'oklch(0.965 0.026 168)', primary: 'oklch(0.55 0.13 168)' },
		dark: { background: 'oklch(0.145 0.022 168)', primary: 'oklch(0.75 0.12 168)' }
	},
	{
		id: 'teal',
		label: 'Teal',
		light: { background: 'oklch(0.965 0.026 190)', primary: 'oklch(0.53 0.12 190)' },
		dark: { background: 'oklch(0.145 0.022 190)', primary: 'oklch(0.73 0.11 190)' }
	},
	{
		id: 'sky',
		label: 'Sky',
		light: { background: 'oklch(0.965 0.026 215)', primary: 'oklch(0.58 0.15 215)' },
		dark: { background: 'oklch(0.145 0.022 215)', primary: 'oklch(0.76 0.13 215)' }
	},
	{
		id: 'ocean',
		label: 'Ocean',
		light: { background: 'oklch(0.965 0.026 245)', primary: 'oklch(0.55 0.16 245)' },
		dark: { background: 'oklch(0.145 0.022 245)', primary: 'oklch(0.73 0.13 245)' }
	},
	{
		id: 'indigo',
		label: 'Indigo',
		light: { background: 'oklch(0.965 0.026 272)', primary: 'oklch(0.52 0.18 272)' },
		dark: { background: 'oklch(0.145 0.022 272)', primary: 'oklch(0.72 0.15 272)' }
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
		id: 'crimson',
		label: 'Crimson',
		light: { background: 'oklch(0.965 0.026 355)', primary: 'oklch(0.55 0.19 355)' },
		dark: { background: 'oklch(0.145 0.022 355)', primary: 'oklch(0.7 0.17 355)' }
	},
	{
		id: 'sand',
		label: 'Sand',
		light: { background: 'oklch(0.965 0.014 65)', primary: 'oklch(0.4 0.05 65)' },
		dark: { background: 'oklch(0.145 0.012 65)', primary: 'oklch(0.78 0.06 65)' }
	},
	{
		id: 'coffee',
		label: 'Coffee',
		light: { background: 'oklch(0.965 0.013 40)', primary: 'oklch(0.42 0.06 40)' },
		dark: { background: 'oklch(0.145 0.011 40)', primary: 'oklch(0.78 0.05 40)' }
	},
	{
		id: 'slate',
		label: 'Slate',
		light: { background: 'oklch(0.965 0.01 250)', primary: 'oklch(0.4 0.04 250)' },
		dark: { background: 'oklch(0.145 0.009 250)', primary: 'oklch(0.78 0.04 250)' }
	},
	{
		id: 'sage',
		label: 'Sage',
		light: { background: 'oklch(0.965 0.013 150)', primary: 'oklch(0.42 0.05 150)' },
		dark: { background: 'oklch(0.145 0.011 150)', primary: 'oklch(0.76 0.05 150)' }
	},
	{
		id: 'lavender',
		label: 'Lavender',
		light: { background: 'oklch(0.965 0.013 290)', primary: 'oklch(0.45 0.06 290)' },
		dark: { background: 'oklch(0.145 0.011 290)', primary: 'oklch(0.77 0.06 290)' }
	},
	{
		id: 'blush',
		label: 'Blush',
		light: { background: 'oklch(0.965 0.013 350)', primary: 'oklch(0.48 0.07 350)' },
		dark: { background: 'oklch(0.145 0.011 350)', primary: 'oklch(0.78 0.07 350)' }
	},
	{
		id: 'nord',
		label: 'Nord',
		light: { background: 'oklch(0.965 0.016 220)', primary: 'oklch(0.55 0.1 220)' },
		dark: { background: '#2E3440', primary: '#88C0D0' }
	},
	{
		id: 'dracula',
		label: 'Dracula',
		light: { background: 'oklch(0.965 0.012 265)', primary: 'oklch(0.55 0.12 265)' },
		dark: { background: '#282A36', primary: '#BD93F9' }
	},
	{
		id: 'catppuccin',
		label: 'Catppuccin',
		light: { background: 'oklch(0.965 0.013 270)', primary: 'oklch(0.55 0.12 270)' },
		dark: { background: '#1E1E2E', primary: '#CBA6F7' }
	},
	{
		id: 'solarized',
		label: 'Solarized',
		light: { background: 'oklch(0.965 0.016 50)', primary: 'oklch(0.5 0.09 50)' },
		dark: { background: '#002B36', primary: '#268BD2' }
	},
	{
		id: 'gruvbox',
		label: 'Gruvbox',
		light: { background: 'oklch(0.965 0.017 55)', primary: 'oklch(0.5 0.1 55)' },
		dark: { background: '#282828', primary: '#FABD2F' }
	},
	{
		id: 'onedark',
		label: 'One Dark',
		light: { background: 'oklch(0.965 0.014 215)', primary: 'oklch(0.55 0.1 215)' },
		dark: { background: '#282C34', primary: '#61AFEF' }
	},
	{
		id: 'onedarkpro',
		label: 'One Dark Pro',
		light: { background: 'oklch(0.965 0.013 218)', primary: 'oklch(0.55 0.1 218)' },
		dark: { background: '#282C34', primary: '#4D78CC' }
	},
	{
		id: 'oled',
		label: 'OLED',
		light: { background: 'oklch(0.965 0.003 264)', primary: 'oklch(0.4 0.03 264)' },
		dark: { background: '#000000', primary: '#F2F2F2' }
	},
	{
		id: 'contrast',
		label: 'High Contrast',
		light: { background: '#FFFFFF', primary: '#000000' },
		dark: { background: '#000000', primary: '#FFFFFF' }
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
