export type RelationType = 'blocks' | 'relates' | 'duplicates';

// a resolved relation row joined with the other task's display fields;
// `direction` is 'out' when this task points at the other, 'in' otherwise
export type TaskRelation = {
	taskId: string;
	relatedTaskId: string;
	type: RelationType;
	direction: 'out' | 'in';
	otherId: string;
	otherNumber: number;
	otherTitle: string;
	otherStatus: string;
	otherPrefix: string | null;
};

export const relationLabels: Record<RelationType, { out: string; in: string }> = {
	blocks: { out: 'Blocks', in: 'Blocked by' },
	relates: { out: 'Relates to', in: 'Relates to' },
	duplicates: { out: 'Duplicates', in: 'Duplicated by' }
};
