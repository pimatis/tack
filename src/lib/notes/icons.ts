// mingcute is the only icon set this app uses; names come from the iconify
// collection api, and bodies are fetched in batches (one request for many
// icons) because the per-icon .svg route rate-limits and returns empty tiles
const ICON_PREFIX = 'mingcute';

export function iconName(icon: string): string {
	const at = icon.indexOf(':');
	return at === -1 ? icon : icon.slice(at + 1);
}

let namesCache: string[] | null = null;

// full mingcute icon name list, cached for the session
export async function mingcuteNames(): Promise<string[]> {
	if (namesCache) return namesCache;
	const res = await fetch(`https://api.iconify.design/collection?prefix=${ICON_PREFIX}`);
	if (!res.ok) throw new Error(`iconify ${res.status}`);
	const data = (await res.json()) as {
		uncategorized?: string[];
		categories?: Record<string, string[]>;
	};
	const names = data.uncategorized ?? Object.values(data.categories ?? {}).flat();
	namesCache = [...new Set(names)];
	return namesCache;
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
