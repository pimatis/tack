import type { TaskStatus } from '$lib/types/task';

// structured filters extracted from a query like "bug label:ui status:todo";
// the remaining words go through full-text search
export type SearchOperators = {
	statuses: TaskStatus[];
	priorities: number[];
	projectKeys: string[];
	labelNames: string[];
	due: 'today' | 'tomorrow' | 'week' | 'overdue' | 'none' | null;
	pinned: boolean | null;
	blocked: boolean | null;
	hasParent: boolean | null;
	subtask: boolean | null;
};

const STATUS_ALIASES: Record<string, TaskStatus> = {
	backlog: 'backlog',
	todo: 'todo',
	open: 'todo',
	in_progress: 'in_progress',
	inprogress: 'in_progress',
	doing: 'in_progress',
	done: 'done',
	canceled: 'canceled',
	cancelled: 'canceled'
};

const PRIORITY_NAMES: Record<string, number> = {
	none: 0,
	no: 0,
	urgent: 1,
	high: 2,
	medium: 3,
	low: 4
};

const DUE_VALUES = new Set(['today', 'tomorrow', 'week', 'overdue', 'none']);

function parseBool(value: string): boolean | null {
	if (['1', 'true', 'yes'].includes(value)) return true;
	if (['0', 'false', 'no'].includes(value)) return false;
	return null;
}

// pull "key:value" tokens out of the query; unknown keys and bare words stay
// in the text part so nothing silently filters to zero results
export function parseSearchQuery(query: string): { text: string; ops: SearchOperators } {
	const ops: SearchOperators = {
		statuses: [],
		priorities: [],
		projectKeys: [],
		labelNames: [],
		due: null,
		pinned: null,
		blocked: null,
		hasParent: null,
		subtask: null
	};
	const text: string[] = [];

	for (const token of query.trim().split(/\s+/)) {
		const colon = token.indexOf(':');
		const key = colon > 0 ? token.slice(0, colon).toLowerCase() : '';
		const value = colon > 0 ? token.slice(colon + 1).toLowerCase() : '';

		if (!value) {
			text.push(token);
			continue;
		}

		if (key === 'status' && STATUS_ALIASES[value] !== undefined) {
			ops.statuses.push(STATUS_ALIASES[value]);
			continue;
		}
		if ((key === 'priority' || key === 'p') && /^\d$/.test(value)) {
			ops.priorities.push(Number(value));
			continue;
		}
		if (key === 'priority' && PRIORITY_NAMES[value] !== undefined) {
			ops.priorities.push(PRIORITY_NAMES[value]);
			continue;
		}
		if (key === 'project') {
			ops.projectKeys.push(value);
			continue;
		}
		if (key === 'label') {
			ops.labelNames.push(value);
			continue;
		}
		if (key === 'due' && DUE_VALUES.has(value)) {
			ops.due = value as SearchOperators['due'];
			continue;
		}
		if (key === 'is' && (value === 'blocked' || value === 'unblocked')) {
			ops.blocked = value === 'blocked';
			continue;
		}
		if (key === 'has' && value === 'parent') {
			ops.hasParent = true;
			continue;
		}
		if (key === 'type' && (value === 'subtask' || value === 'toplevel')) {
			ops.subtask = value === 'subtask';
			continue;
		}
		if (key === 'pinned') {
			const parsed = parseBool(value);
			if (parsed !== null) {
				ops.pinned = parsed;
				continue;
			}
		}
		text.push(token);
	}

	return { text: text.join(' '), ops };
}

export function hasOperators(ops: SearchOperators): boolean {
	return (
		ops.statuses.length > 0 ||
		ops.priorities.length > 0 ||
		ops.projectKeys.length > 0 ||
		ops.labelNames.length > 0 ||
		ops.due !== null ||
		ops.pinned !== null ||
		ops.blocked !== null ||
		ops.hasParent !== null ||
		ops.subtask !== null
	);
}
