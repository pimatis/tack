import { getDb } from '$lib/db/client';
import { invoke } from '@tauri-apps/api/core';
import type { TaskAttachment } from '$lib/types/attachment';

export type CreateAttachmentInput = {
	taskId: string;
	fileName: string;
	fileData: string;
	mimeType: string;
	fileSize: number;
};

const METADATA_COLUMNS = `
	id,
	task_id AS taskId,
	file_name AS fileName,
	file_path AS filePath,
	mime_type AS mimeType,
	file_size AS fileSize,
	created_at AS createdAt
`;

export async function create(input: CreateAttachmentInput): Promise<TaskAttachment> {
	try {
		const db = await getDb();
		const id = crypto.randomUUID();
		const now = new Date().toISOString();

		// save file to disk via rust command
		const filePath = await invoke<string>('save_attachment', {
			id,
			fileData: input.fileData
		});

		await db.execute(
			`INSERT INTO task_attachments (id, task_id, file_name, file_path, mime_type, file_size, created_at)
		 VALUES ($1, $2, $3, $4, $5, $6, $7)`,
			[id, input.taskId, input.fileName, filePath, input.mimeType, input.fileSize, now]
		);

		return {
			id,
			taskId: input.taskId,
			fileName: input.fileName,
			filePath,
			mimeType: input.mimeType,
			fileSize: input.fileSize,
			createdAt: now
		};
	} catch (error) {
		throw new Error('Failed to create attachment', { cause: error });
	}
}

export async function findByTaskId(taskId: string): Promise<TaskAttachment[]> {
	try {
		const db = await getDb();
		return await db.select<TaskAttachment[]>(
			`SELECT ${METADATA_COLUMNS} FROM task_attachments WHERE task_id = $1 ORDER BY created_at ASC`,
			[taskId]
		);
	} catch (error) {
		throw new Error('Failed to load attachments', { cause: error });
	}
}

// load file data on demand for display
export async function getAttachmentData(id: string, mimeType: string): Promise<string> {
	return await invoke<string>('read_attachment', { id, mimeType });
}

// copy attachment file to user-selected path
export async function downloadAttachment(id: string, destPath: string): Promise<void> {
	await invoke('download_attachment', { id, destPath: destPath });
}

export async function remove(id: string): Promise<boolean> {
	try {
		const db = await getDb();
		const result = await db.execute('DELETE FROM task_attachments WHERE id = $1', [id]);
		if (result.rowsAffected > 0) {
			void invoke('delete_attachment', { id });
		}
		return result.rowsAffected > 0;
	} catch (error) {
		throw new Error('Failed to delete attachment', { cause: error });
	}
}
