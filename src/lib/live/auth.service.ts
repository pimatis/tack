// browser (live site) side of the shared password: when the server answers
// 401, db queries pause on this gate until the login dialog completes
let gate: Promise<void> | undefined;
let release: (() => void) | undefined;

let authRequired = false;

// true once the server asked for the shared password; the layout uses this
// to hide the app (including already-fetched data) behind the lock screen
export function isLiveAuthRequired(): boolean {
	return authRequired;
}

export function requireLiveAuth(): Promise<void> {
	if (!gate) {
		gate = new Promise((resolve) => {
			release = resolve;
		});
		authRequired = true;
		window.dispatchEvent(new CustomEvent('live-auth-required'));
	}
	return gate;
}

export function completeLiveAuth(): void {
	const done = release;
	gate = undefined;
	release = undefined;
	done?.();
}

export async function loginLive(password: string): Promise<{ ok: boolean; error?: string }> {
	const res = await fetch('/api/auth', {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ password })
	});
	if (res.ok) return { ok: true };
	if (res.status === 429) return { ok: false, error: 'Too many attempts - try again later' };
	return { ok: false, error: 'Wrong password' };
}
