import { goHomeThenDispatch, matches } from '../utils';
import type { PaletteItem, PaletteProvider } from '../types';
import type { Project } from '$lib/types/project';

function toItem(project: Project): PaletteItem {
	return {
		id: project.id,
		label: project.name,
		icon: 'hashtag',
		hint: project.prefix,
		run: () => goHomeThenDispatch('filter-by-project', project.id)
	};
}

export const projectsProvider: PaletteProvider = {
	id: 'projects',
	heading: 'Projects',
	search(query, ctx) {
		return ctx.projects
			.filter((project) => matches(`${project.name} ${project.prefix}`, query))
			.map(toItem);
	},
	resolve(itemId, ctx) {
		const project = ctx.projects.find((p) => p.id === itemId);
		if (!project) return null;
		return toItem(project);
	}
};
