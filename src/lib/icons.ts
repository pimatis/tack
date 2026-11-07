// material-symbols is the only icon set this app uses; components never name
// an icon directly, they pass a semantic key from the registry below so a set
// or glyph swap is a single-line change here. bodies are fetched in batches
// (one request for many icons) because the per-icon .svg route rate-limits
// and returns empty tiles
const ICON_PREFIX = 'material-symbols';

// semantic key -> material-symbols icon name, the only place icon names live
export const ICONS = {
	add: 'add-box',
	alarm: 'alarm',
	'alert-triangle': 'warning',
	blocked: 'block',
	archive: 'archive',
	'arrow-down': 'arrow-downward',
	'arrow-left': 'arrow-back',
	'arrow-right': 'arrow-forward',
	'arrow-up': 'arrow-upward',
	calendar: 'calendar-month',
	chart: 'bar-chart',
	check: 'check',
	'check-circle': 'check-circle',
	checklist: 'checklist',
	'circle-outline': 'radio-button-unchecked',
	clock: 'schedule',
	close: 'close',
	'close-circle': 'cancel',
	code: 'code',
	compose: 'edit-square',
	copy: 'content-copy',
	database: 'database',
	download: 'download',
	eye: 'visibility',
	flag: 'flag',
	grip: 'drag-handle',
	hashtag: 'tag',
	home: 'home',
	inbox: 'inbox',
	'info-circle': 'info',
	kanban: 'view-kanban',
	keyboard: 'keyboard',
	link: 'link',
	list: 'list',
	magic: 'auto-fix-high',
	menu: 'menu',
	minus: 'remove',
	notification: 'notifications',
	'paint-brush': 'brush',
	palette: 'palette',
	'panel-left': 'left-panel-open',
	paperclip: 'attach-file',
	pencil: 'edit',
	'pie-chart': 'pie-chart',
	pin: 'push-pin',
	play: 'play-arrow',
	repeat: 'repeat',
	search: 'search',
	settings: 'settings',
	share: 'share',
	stop: 'stop-circle',
	stopwatch: 'timer',
	tag: 'sell',
	timeline: 'view-timeline',
	trash: 'delete',
	undo: 'undo',
	widget: 'widgets'
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
	const data = (await res.json()) as {
		icons?: Record<string, { body: string }>;
		aliases?: Record<string, { parent: string }>;
	};
	for (const [name, icon] of Object.entries(data.icons ?? {})) {
		const body = sanitizeIconBody(icon.body);
		bodyCache.set(name, body);
		out[name] = body;
	}
	// aliases name a different icon; resolve them so the requested name caches
	// the shared body instead of ''
	for (const name of missing) {
		const parent = data.aliases?.[name]?.parent;
		if (parent && parent in out) {
			bodyCache.set(name, out[parent]);
			out[name] = out[parent];
		}
	}
	for (const name of missing) {
		if (!(name in out)) bodyCache.set(name, '');
	}
	return out;
}
