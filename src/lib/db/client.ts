import Database from '@tauri-apps/plugin-sql';

const DATABASE_URL = 'sqlite:tack.db';

let databasePromise: Promise<Database> | undefined;

export async function getDb(): Promise<Database> {
	try {
		databasePromise ??= Database.load(DATABASE_URL);
		return await databasePromise;
	} catch (error) {
		databasePromise = undefined;
		throw new Error('Failed to connect to the database', { cause: error });
	}
}

export function resetDb(): void {
	databasePromise = undefined;
}
