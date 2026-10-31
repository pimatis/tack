// solar is the only icon set this app uses; components never name a solar
// icon directly, they pass a semantic key from the registry below so a set
// or glyph swap is a single-line change here. bodies are fetched in batches
// (one request for many icons) because the per-icon .svg route rate-limits
// and returns empty tiles
const ICON_PREFIX = 'solar';

// semantic key -> solar icon name, the only place icon names live
export const ICONS = {
	add: 'add-square-bold',
	alarm: 'alarm-bold',
	'alert-triangle': 'danger-triangle-bold',
	archive: 'archive-bold',
	'arrow-down': 'alt-arrow-down-bold',
	'arrow-left': 'alt-arrow-left-bold',
	'arrow-right': 'alt-arrow-right-bold',
	'arrow-up': 'alt-arrow-up-bold',
	calendar: 'calendar-bold',
	check: 'check-bold',
	'check-circle': 'check-circle-bold',
	checklist: 'checklist-bold',
	'circle-outline': 'record-outline',
	clock: 'clock-circle-bold',
	close: 'close-bold',
	'close-circle': 'close-circle-bold',
	code: 'code-bold',
	compose: 'pen-new-square-bold',
	copy: 'copy-bold',
	database: 'database-bold',
	download: 'download-bold',
	eye: 'eye-bold',
	flag: 'flag-bold',
	grip: 'reorder-2-bold',
	hashtag: 'hashtag-bold',
	home: 'home-bold',
	inbox: 'inbox-bold',
	'info-circle': 'info-circle-bold',
	kanban: 'kanban-bold',
	keyboard: 'keyboard-bold',
	list: 'list-bold',
	menu: 'hamburger-menu-bold',
	minus: 'minus-bold',
	notification: 'bell-bold',
	'paint-brush': 'paint-brush-bold',
	palette: 'palette-bold',
	'panel-left': 'panel-left-bold',
	paperclip: 'paperclip-bold',
	pencil: 'pen-bold',
	'pie-chart': 'pie-chart-2-bold',
	pin: 'pin-bold',
	play: 'play-bold',
	repeat: 'repeat-bold',
	search: 'magnifier-bold',
	settings: 'settings-bold',
	share: 'share-bold',
	stop: 'stop-circle-bold',
	tag: 'tag-bold',
	trash: 'trash-bin-2-bold',
	undo: 'undo-left-bold',
	widget: 'widget-bold'
} as const;

export type IconName = keyof typeof ICONS;

export function iconName(icon: string): string {
	const at = icon.indexOf(':');
	return at === -1 ? icon : icon.slice(at + 1);
}

// icon bodies are raw svg fragments; they are inlined with {@html}, so strip
// anything scriptable before it reaches the dom
function sanitizeIconBody(body: string): string {
	return body
		.replace(/<script[\s\S]*?<\/script>/gi, '')
		.replace(/\son\w+\s*=\s*("[^"]*"|'[^']*'|[^\s>]+)/gi, '')
		.replace(/javascript:/gi, '');
}

// cache value '' marks a name the api could not resolve, so it is never
// requested again and callers can fall back instead of showing a blank icon
const bodyCache = new Map<string, string>();

export function peekIconBody(name: string): string | null {
	return bodyCache.get(name) ?? null;
}

// fetch bodies for many icons in one request; names the api cannot resolve are
// left out of the result so callers never render a blank icon
export async function loadIconBodies(names: string[]): Promise<Record<string, string>> {
	const out: Record<string, string> = {};
	const missing: string[] = [];
	for (const name of names) {
		const cached = bodyCache.get(name);
		if (cached !== undefined) out[name] = cached;
		else missing.push(name);
	}
	if (missing.length === 0) return out;
	const res = await fetch(
		`https://api.iconify.design/${ICON_PREFIX}.json?icons=${missing.join(',')}`
	);
	if (!res.ok) throw new Error(`iconify ${res.status}`);
	const data = (await res.json()) as { icons?: Record<string, { body: string }> };
	for (const [name, icon] of Object.entries(data.icons ?? {})) {
		const body = sanitizeIconBody(icon.body);
		bodyCache.set(name, body);
		out[name] = body;
	}
	for (const name of missing) {
		if (!(name in out)) bodyCache.set(name, '');
	}
	return out;
}
