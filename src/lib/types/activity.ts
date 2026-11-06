export type ActivityAction =
	| 'created'
	| 'status_changed'
	| 'priority_changed'
	| 'title_changed'
	| 'description_changed'
	| 'due_date_changed'
	| 'end_date_changed'
	| 'reminder_changed'
	| 'recurrence_changed'
	| 'recurrence_spawned'
	| 'label_added'
	| 'label_removed'
	| 'attachment_added'
	| 'attachment_removed'
	| 'subtask_added'
	| 'subtask_completed'
	| 'subtask_uncompleted'
	| 'subtask_removed'
	| 'estimate_changed'
	| 'parent_changed'
	| 'timer_started'
	| 'timer_stopped'
	| 'relation_added'
	| 'relation_removed'
	| 'assigned'
	| 'unassigned'
	| 'delegated'
	| 'note'
	| 'trashed'
	| 'restored';

export type ActivityLog = {
	id: string;
	taskId: string;
	action: ActivityAction;
	field?: string | null;
	oldValue?: string | null;
	newValue?: string | null;
	// 'gui' for the app, 'cli' for the terminal, 'agent' or 'agent:<harness>' for agents
	source?: string;
	createdAt: string;
};
