import { getDb } from '$lib/db/client';
import type { Project } from '$lib/types/project';

export type CreateProjectInput = Pick<Project, 'name' | 'prefix'> &
	Partial<Pick<Project, 'description'>>;

export async function create(input: CreateProjectInput): Promise<Project> {
	try {
		const db = await getDb();
		const now = new Date().toISOString();
		const project: Project = {
			id: crypto.randomUUID(),
			name: input.name,
			prefix: input.prefix,
			description: input.description ?? null,
			createdAt: now,
			updatedAt: now
		};

		await db.execute(
			`INSERT INTO projects (id, name, prefix, description, created_at, updated_at)
		 VALUES ($1, $2, $3, $4, $5, $6)`,
			[
				project.id,
				project.name,
				project.prefix,
				project.description,
				project.createdAt,
				project.updatedAt
			]
		);
		return project;
	} catch (error) {
		throw new Error('Failed to create project', { cause: error });
	}
}

export async function findAll(): Promise<Project[]> {
	try {
		const db = await getDb();
		return await db.select<Project[]>(
			`SELECT id, name, prefix, description, created_at AS createdAt, updated_at AS updatedAt
		 FROM projects ORDER BY name COLLATE NOCASE`
		);
	} catch (error) {
		throw new Error('Failed to load projects', { cause: error });
	}
}

export type UpdateProjectInput = Pick<Project, 'name' | 'prefix'> &
	Partial<Pick<Project, 'description'>>;

export async function update(id: string, input: UpdateProjectInput): Promise<Project | null> {
	try {
		const db = await getDb();
		const result = await db.execute(
			`UPDATE projects SET name = $1, prefix = $2, description = $3, updated_at = $4 WHERE id = $5`,
			[input.name, input.prefix, input.description ?? null, new Date().toISOString(), id]
		);
		if (result.rowsAffected === 0) return null;
		const projects = await db.select<Project[]>(
			`SELECT id, name, prefix, description, created_at AS createdAt, updated_at AS updatedAt
		 FROM projects WHERE id = $1`,
			[id]
		);
		return projects[0] ?? null;
	} catch (error) {
		throw new Error('Failed to update project', { cause: error });
	}
}

export async function remove(id: string): Promise<boolean> {
	try {
		const db = await getDb();
		await db.execute('DELETE FROM tasks WHERE project_id = $1', [id]);
		const result = await db.execute('DELETE FROM projects WHERE id = $1', [id]);
		return result.rowsAffected > 0;
	} catch (error) {
		throw new Error('Failed to delete project', { cause: error });
	}
}
