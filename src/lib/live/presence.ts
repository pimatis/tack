// identity this browser sends to the live server so it shows up in the
// presence list; kept out of live.service to avoid a client <-> service cycle
import { newId } from '$lib/utils';

const PRESENCE_ID_KEY = 'tack-live-client-id';

// stable per-browser id so several sse connections from one tab count as one
function clientId(): string {
	try {
		let id = localStorage.getItem(PRESENCE_ID_KEY);
		if (!id) {
			id = newId();
			localStorage.setItem(PRESENCE_ID_KEY, id);
		}
		return id;
	} catch {
		return 'anonymous';
	}
}

// short, human label for the presence list
function clientName(): string {
	const ua = navigator.userAgent;
	const browser = /Edg\//.test(ua)
		? 'Edge'
		: /Chrome\//.test(ua)
			? 'Chrome'
			: /Firefox\//.test(ua)
				? 'Firefox'
				: /Safari\//.test(ua)
					? 'Safari'
					: 'Browser';
	const os = /Mac/.test(ua)
		? 'macOS'
		: /Windows/.test(ua)
			? 'Windows'
			: /Android/.test(ua)
				? 'Android'
				: /iPhone|iPad/.test(ua)
					? 'iOS'
					: /Linux/.test(ua)
						? 'Linux'
						: '';
	return os ? `${browser} · ${os}` : browser;
}

export function presenceQuery(): string {
	return `client=${encodeURIComponent(clientId())}&name=${encodeURIComponent(clientName())}`;
}
