import { invoke } from '@tauri-apps/api/core';
import { getSettings } from '$lib/stores/settings';
import { getDb, resetDb } from '$lib/db/client';

export type BackupInfo = {
	name: string;
	createdAt: string;
	sizeBytes: number;
};

export function listBackups(): Promise<BackupInfo[]> {
	return invoke<BackupInfo[]>('list_backups');
}

export function createBackup(keep?: number): Promise<string> {
	return invoke<string>('create_backup', { keep: keep ?? 7 });
}

export async function restoreBackup(name: string): Promise<void> {
	await invoke('restore_backup', { name });
	// drop cached pool so the reload reopens the restored files
	try {
		await (await getDb()).close();
	} catch {
		// already closed
	}
	resetDb();
}

export function deleteBackup(name: string): Promise<void> {
	return invoke<void>('delete_backup', { name });
}

// ponytail: scheduler only runs while the app is open, missed intervals fire on next launch
export async function runScheduledBackup(): Promise<string | null> {
	const { backupEnabled, backupIntervalHours, backupKeepCount } = getSettings();
	if (!backupEnabled) return null;
	const backups = await listBackups().catch(() => [] as BackupInfo[]);
	const latest = backups[0];
	if (
		latest &&
		Date.now() - new Date(latest.createdAt).getTime() < backupIntervalHours * 3_600_000
	) {
		return null;
	}
	return createBackup(backupKeepCount);
}

export function startBackupScheduler(): () => void {
	void runScheduledBackup().catch(() => {});
	const interval = setInterval(
		() => {
			void runScheduledBackup().catch(() => {});
		},
		15 * 60 * 1000
	);
	return () => clearInterval(interval);
}
