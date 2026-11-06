// shared agent palette, mirrors AGENT_COLORS in the rust cli/api so a name
// renders the same color everywhere
const PALETTE = {
	indigo: '#6366f1',
	amber: '#f59e0b',
	emerald: '#10b981',
	rose: '#f43f5e',
	sky: '#0ea5e9',
	violet: '#8b5cf6',
	orange: '#f97316',
	teal: '#14b8a6',
	pink: '#ec4899',
	lime: '#84cc16',
	cyan: '#06b6d4',
	fuchsia: '#d946ef'
} as const;

const COLOR_NAMES = Object.keys(PALETTE) as (keyof typeof PALETTE)[];

export function agentColor(name: string): string {
	let h = 5381;
	for (const c of name) h = (h * 33 + c.charCodeAt(0)) >>> 0;
	h = (h ^ (h >>> 16)) >>> 0;
	return PALETTE[COLOR_NAMES[h % COLOR_NAMES.length]];
}
