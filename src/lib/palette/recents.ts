// last palette selections, replayed through each provider's resolve()
const KEY = 'tack-palette-recent';
const MAX = 8;

export type RecentRef = { provider: string; item: string };

export function readRecents(): RecentRef[] {
	try {
		const raw = localStorage.getItem(KEY);
		if (!raw) return [];
		const arr: unknown = JSON.parse(raw);
		if (!Array.isArray(arr)) return [];
		return arr
			.filter(
				(r): r is RecentRef =>
					typeof r === 'object' &&
					r !== null &&
					typeof (r as RecentRef).provider === 'string' &&
					typeof (r as RecentRef).item === 'string'
			)
			.slice(0, MAX);
	} catch {
		return [];
	}
}

export function pushRecent(provider: string, item: string): void {
	try {
		const next = [
			{ provider, item },
			...readRecents().filter((r) => !(r.provider === provider && r.item === item))
		].slice(0, MAX);
		localStorage.setItem(KEY, JSON.stringify(next));
	} catch {
		// ignore quota errors
	}
}
