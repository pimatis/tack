import Database from '@tauri-apps/plugin-sql';

const DATABASE_URL = 'sqlite:tack.db';

type QueryResult = { rowsAffected: number; lastInsertId?: number };

// minimal surface shared by the tauri sql plugin and the browser http shim,
// so repositories work unchanged in both the app and the live site
type DbClient = {
	select<T>(query: string, params?: unknown[]): Promise<T>;
	execute(query: string, params?: unknown[]): Promise<QueryResult>;
	close(): Promise<boolean>;
};

// true inside the tauri webview, false in a plain browser (live site)
export function isTauri(): boolean {
	return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

let databasePromise: Promise<DbClient> | undefined;

export async function getDb(): Promise<DbClient> {
	try {
		databasePromise ??= isTauri() ? loadTauriDb() : loadHttpDb();
		return await databasePromise;
	} catch (error) {
		databasePromise = undefined;
		throw new Error('Failed to connect to the database', { cause: error });
	}
}

export function resetDb(): void {
	databasePromise = undefined;
}

function loadTauriDb(): Promise<DbClient> {
	return Database.load(DATABASE_URL);
}

// browser mode: talk to the local server the desktop app embeds
let httpDb: HttpDb | undefined;
function loadHttpDb(): Promise<DbClient> {
	httpDb ??= new HttpDb();
	return Promise.resolve(httpDb);
}

class HttpDb implements DbClient {
	async select<T>(query: string, params?: unknown[]): Promise<T> {
		const data = await this.post<{ rows: unknown }>('/api/select', {
			sql: query,
			params: params ?? []
		});
		return data.rows as T;
	}

	async execute(query: string, params?: unknown[]): Promise<QueryResult> {
		const data = await this.post<{ rowsAffected: number }>('/api/execute', {
			sql: query,
			params: params ?? []
		});
		return { rowsAffected: data.rowsAffected };
	}

	async close(): Promise<boolean> {
		return true;
	}

	private async post<T>(path: string, body: unknown): Promise<T> {
		const res = await fetch(path, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify(body)
		});
		if (!res.ok) throw new Error(await res.text());
		return (await res.json()) as T;
	}
}

// unified db-changed subscription: tauri event in the app, long-poll in the browser
export function onDbChanged(callback: () => void): () => void {
	if (isTauri()) {
		let cancelled = false;
		let unlisten: (() => void) | undefined;
		void import('@tauri-apps/api/event').then(async ({ listen }) => {
			if (cancelled) return;
			unlisten = await listen('db-changed', callback);
		});
		return () => {
			cancelled = true;
			unlisten?.();
		};
	}
	// the local server holds the poll open until a db change lands (or a quiet
	// timeout), so every response completes and we immediately re-poll
	let cancelled = false;
	let timer: ReturnType<typeof setTimeout> | undefined;
	const poll = async () => {
		if (cancelled) return;
		try {
			const res = await fetch('/api/events');
			if (res.ok) {
				const data = (await res.json()) as { changed?: boolean };
				if (data.changed) callback();
			}
		} catch {
			// server not reachable yet; retry shortly
		}
		if (!cancelled) timer = setTimeout(() => void poll(), 250);
	};
	void poll();
	return () => {
		cancelled = true;
		if (timer) clearTimeout(timer);
	};
}
