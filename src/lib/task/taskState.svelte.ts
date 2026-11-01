import { onDbChanged } from '$lib/db/client';
import { readDataCache, writeDataCache } from '$lib/db/cache';
import {
	remove,
	findAll,
	update,
	restore,
	bulkDelete,
	bulkUpdateStatus,
	bulkUpdatePriority,
	bulkUpdateProject,
	duplicate,
	reorderInStatus,
	togglePin
} from '$lib/repositories/task.repository';
import { findAll as findProjects } from '$lib/repositories/project.repository';
import {
	findAll as findLabels,
	addLabelToTasks,
	removeLabelFromTasks
} from '$lib/repositories/label.repository';
import { getSettings, setSettings } from '$lib/stores/settings';
import { toast } from 'svelte-sonner';
import { searchTaskIds } from '$lib/search/fts.service';
import { parseSearchQuery, hasOperators, type SearchOperators } from '$lib/search/operators';
import {
	findAll as findViews,
	create as createView,
	remove as removeView,
	rename as renameViewDb,
	reorder as reorderViewsDb
} from '$lib/repositories/view.repository';
import type { SavedView, ViewFilters } from '$lib/types/view';
import { getShortcutRegistry } from '$lib/shortcuts/index.js';
import { sortableItem, dropZone, useDndActive, type DragDropState } from '$lib/dnd';
import type { Project } from '$lib/types/project';
import type { Task, TaskPriority, TaskStatus } from '$lib/types/task';
import type { Label } from '$lib/types/label';
import type { Settings } from '$lib/types/settings';
import { statusOrder } from './constants';
import { isTypingTarget, groupedTasks as computeGroups } from './utils';

// persist collapsed status accordions across page reloads
const COLLAPSED_KEY = 'tack-collapsed-statuses';

function loadCollapsed(): Set<TaskStatus> {
	try {
		const raw = localStorage.getItem(COLLAPSED_KEY);
		if (!raw) return new Set();
		const arr: unknown = JSON.parse(raw);
		if (!Array.isArray(arr)) return new Set();
		return new Set(arr.filter((s): s is TaskStatus => statusOrder.includes(s as TaskStatus)));
	} catch {
		// localStorage unavailable (ssr) or corrupt - start fresh
		return new Set();
	}
}

// a task's calendar span: due date starts it, end date finishes it
function dateRange(t: Task): { start: Date | null; end: Date | null } {
	return {
		start: t.dueDate ? new Date(t.dueDate + 'T00:00:00') : null,
		end: t.endDate ? new Date(t.endDate + 'T00:00:00') : null
	};
}

export { sortableItem, dropZone, useDndActive };

// single shared instance: the home page creates it, the sidebar reads it
let sharedInstance: TaskPageState | undefined;

export class TaskPageState {
	tasks = $state<Task[]>([]);
	loading = $state(true);
	error = $state<string | null>(null);
	dialogOpen = $state(false);
	projectDialogOpen = $state(false);
	editDialogOpen = $state(false);
	editingTask = $state<Task | null>(null);
	projects = $state<Project[]>([]);
	labels = $state<Label[]>([]);
	projectEditOpen = $state(false);
	editingProject = $state<Project | null>(null);

	searchQuery = $state('');
	ftsIds = $state<Set<string> | null>(null);
	searchOps = $state<SearchOperators | null>(null);
	searchText = $state('');
	createDueDate = $state<string | null>(null);
	createInitialTitle = $state<string | null>(null);
	// full prefill payload from the palette's quick-add alt+enter path
	createInitial = $state<Partial<Task> | null>(null);
	createParentId = $state<string | null>(null);
	savedViews = $state<SavedView[]>([]);
	statusFilters = $state<Set<TaskStatus>>(new Set());
	projectFilters = $state<Set<string>>(new Set());
	labelFilters = $state<Set<string>>(new Set());

	selectedIds = $state<Set<string>>(new Set());
	lastSelectedId = $state<string | null>(null);
	collapsedStatuses = $state<Set<TaskStatus>>(loadCollapsed());
	viewMode = $state<'list' | 'board' | 'calendar'>(getSettings().defaultViewMode);
	appSettings = $state<Settings>(getSettings());
	pinnedFilter = $state(false);
	todayFilter = $state(false);
	upcomingFilter = $state(false);
	overdueFilter = $state(false);
	priorityFilter = $state<number | null>(null);
	updatedSinceDays = $state<number | null>(null);

	hasFilters = $derived(
		this.searchQuery.trim().length > 0 ||
			this.statusFilters.size + this.projectFilters.size + this.labelFilters.size > 0 ||
			this.updatedSinceDays !== null
	);
	selectedCount = $derived(this.selectedIds.size);
	hasSelection = $derived(this.selectedCount > 0);
	selectedTasks = $derived(this.tasks.filter((t) => this.selectedIds.has(t.id)));
	labelMap = $derived(new Map(this.labels.map((l) => [l.id, l])));

	filteredTasks = $derived.by(() => {
		let result = this.tasks;

		if (this.pinnedFilter) result = result.filter((t) => t.pinned);

		if (this.todayFilter) {
			const now = new Date();
			now.setHours(0, 0, 0, 0);
			result = result.filter((t) => {
				if (t.status === 'done' || t.status === 'canceled') return false;
				const { start, end } = dateRange(t);
				if (!start && !end) return false;
				// today lies inside the task's span (start <= today <= end)
				const startOk = !start || start.getTime() <= now.getTime();
				const endOk = !end || end.getTime() >= now.getTime();
				return startOk && endOk;
			});
		}

		if (this.upcomingFilter) {
			const now = new Date();
			now.setHours(0, 0, 0, 0);
			const week = new Date(now);
			week.setDate(now.getDate() + 7);
			result = result.filter((t) => {
				if (t.status === 'done' || t.status === 'canceled') return false;
				const { start, end } = dateRange(t);
				if (!start && !end) return false;
				// either end of the span falls within the next 7 days
				const startUpcoming =
					!!start && start.getTime() > now.getTime() && start.getTime() <= week.getTime();
				const endUpcoming =
					!!end && end.getTime() > now.getTime() && end.getTime() <= week.getTime();
				return startUpcoming || endUpcoming;
			});
		}

		if (this.overdueFilter) {
			const now = new Date();
			now.setHours(0, 0, 0, 0);
			result = result.filter((t) => {
				if (t.status === 'done' || t.status === 'canceled') return false;
				const { start, end } = dateRange(t);
				if (!start && !end) return false;
				// a part of the span is already in the past
				const startOverdue = !!start && start.getTime() < now.getTime();
				const endOverdue = !!end && end.getTime() < now.getTime();
				return startOverdue || endOverdue;
			});
		}

		if (this.priorityFilter !== null)
			result = result.filter((t) => t.priority === this.priorityFilter);

		if (this.updatedSinceDays !== null) {
			const cutoff = new Date(Date.now() - this.updatedSinceDays * 864e5).toISOString();
			result = result.filter((t) => t.updatedAt.replace(' ', 'T') >= cutoff);
		}

		const q = this.searchQuery.toLowerCase().trim();
		if (this.searchText) {
			// fts match (title, description, subtasks, labels, project, issue
			// number) with an in-memory title fallback while results are pending
			if (this.ftsIds) result = result.filter((t) => this.ftsIds?.has(t.id));
			else result = result.filter((t) => t.title.toLowerCase().includes(this.searchText));
		} else if (q && !this.searchOps) {
			// first-keystroke fallback before the parser runs
			result = result.filter((t) => t.title.toLowerCase().includes(q));
		}

		if (this.searchOps) {
			const ops = this.searchOps;
			if (ops.statuses.length > 0) result = result.filter((t) => ops.statuses.includes(t.status));
			if (ops.priorities.length > 0)
				result = result.filter((t) => ops.priorities.includes(t.priority));
			if (ops.projectKeys.length > 0) {
				const projectIds = new Set(
					this.projects
						.filter(
							(p) =>
								ops.projectKeys.includes(p.prefix.toLowerCase()) ||
								ops.projectKeys.includes(p.name.toLowerCase())
						)
						.map((p) => p.id)
				);
				const wantsInbox = ops.projectKeys.some((k) => k === 'none' || k === 'inbox');
				result = result.filter((t) => (t.projectId ? projectIds.has(t.projectId) : wantsInbox));
			}
			if (ops.labelNames.length > 0) {
				const labelIds = new Set(
					this.labels.filter((l) => ops.labelNames.includes(l.name.toLowerCase())).map((l) => l.id)
				);
				result = result.filter((t) => (t.labelIds ?? []).some((id) => labelIds.has(id)));
			}
			if (ops.pinned !== null) result = result.filter((t) => !!t.pinned === ops.pinned);
			if (ops.blocked !== null) result = result.filter((t) => !!t.blocked === ops.blocked);
			if (ops.hasParent) result = result.filter((t) => !!t.parentId);
			if (ops.subtask !== null) result = result.filter((t) => !!t.parentId === ops.subtask);
			if (ops.due) {
				const now = new Date();
				now.setHours(0, 0, 0, 0);
				result = result.filter((t) => {
					if (t.status === 'done' || t.status === 'canceled') return false;
					const { start, end } = dateRange(t);
					if (ops.due === 'none') return !start && !end;
					if (!start && !end) return false;
					if (ops.due === 'overdue')
						return (
							(!!start && start.getTime() < now.getTime()) ||
							(!!end && end.getTime() < now.getTime())
						);
					if (ops.due === 'today')
						return (
							(!start || start.getTime() <= now.getTime()) &&
							(!end || end.getTime() >= now.getTime())
						);
					if (ops.due === 'tomorrow') {
						const tomorrow = new Date(now);
						tomorrow.setDate(now.getDate() + 1);
						const dayAfter = new Date(now);
						dayAfter.setDate(now.getDate() + 2);
						return (
							(!!start &&
								start.getTime() >= tomorrow.getTime() &&
								start.getTime() < dayAfter.getTime()) ||
							(!!end && end.getTime() >= tomorrow.getTime() && end.getTime() < dayAfter.getTime())
						);
					}
					// week: either end of the span within the next 7 days
					const week = new Date(now);
					week.setDate(now.getDate() + 7);
					return (
						(!!start && start.getTime() > now.getTime() && start.getTime() <= week.getTime()) ||
						(!!end && end.getTime() > now.getTime() && end.getTime() <= week.getTime())
					);
				});
			}
		}
		if (this.statusFilters.size > 0)
			result = result.filter((t) => this.statusFilters.has(t.status));
		if (this.projectFilters.size > 0)
			result = result.filter((t) => this.projectFilters.has(t.projectId ?? ''));
		if (this.labelFilters.size > 0)
			result = result.filter((t) => (t.labelIds ?? []).some((id) => this.labelFilters.has(id)));

		return result;
	});

	groupedTasks = $derived(computeGroups(this.filteredTasks, statusOrder));

	// filters
	toggleStatusFilter(status: TaskStatus) {
		const next = new Set(this.statusFilters);
		if (next.has(status)) next.delete(status);
		else next.add(status);
		this.statusFilters = next;
	}

	toggleProjectFilter(projectId: string) {
		const next = new Set(this.projectFilters);
		if (next.has(projectId)) next.delete(projectId);
		else next.add(projectId);
		this.projectFilters = next;
	}

	toggleLabelFilter(labelId: string) {
		const next = new Set(this.labelFilters);
		if (next.has(labelId)) next.delete(labelId);
		else next.add(labelId);
		this.labelFilters = next;
	}

	toggleStatusCollapsed(status: TaskStatus) {
		const next = new Set(this.collapsedStatuses);
		if (next.has(status)) next.delete(status);
		else next.add(status);
		this.collapsedStatuses = next;
		try {
			localStorage.setItem(COLLAPSED_KEY, JSON.stringify([...next]));
		} catch {
			// ignore quota errors
		}
	}

	async searchFts(query: string) {
		const q = query.trim();
		if (!q) {
			this.ftsIds = null;
			this.searchOps = null;
			this.searchText = '';
			return;
		}
		const parsed = parseSearchQuery(q);
		this.searchOps = hasOperators(parsed.ops) ? parsed.ops : null;
		this.searchText = parsed.text.toLowerCase();
		if (!parsed.text) {
			this.ftsIds = null;
			return;
		}
		try {
			this.ftsIds = await searchTaskIds(parsed.text);
		} catch {
			this.ftsIds = null;
		}
	}

	clearFilters() {
		this.searchQuery = '';
		this.searchOps = null;
		this.searchText = '';
		this.statusFilters = new Set();
		this.projectFilters = new Set();
		this.labelFilters = new Set();
		this.pinnedFilter = false;
		this.todayFilter = false;
		this.upcomingFilter = false;
		this.overdueFilter = false;
		this.priorityFilter = null;
		this.updatedSinceDays = null;
	}

	// selection
	toggleSelect(taskId: string, shiftKey: boolean) {
		if (shiftKey && this.lastSelectedId) {
			const ids = this.filteredTasks.map((t) => t.id);
			const start = ids.indexOf(this.lastSelectedId);
			const end = ids.indexOf(taskId);
			if (start !== -1 && end !== -1) {
				const from = Math.min(start, end);
				const to = Math.max(start, end);
				const range = ids.slice(from, to + 1);
				const next = new Set(this.selectedIds);
				for (const id of range) next.add(id);
				this.selectedIds = next;
				return;
			}
		}
		const next = new Set(this.selectedIds);
		if (next.has(taskId)) next.delete(taskId);
		else next.add(taskId);
		this.selectedIds = next;
		this.lastSelectedId = taskId;
	}

	selectAll() {
		this.selectedIds = new Set(this.filteredTasks.map((t) => t.id));
	}

	clearSelection() {
		this.selectedIds = new Set();
		this.lastSelectedId = null;
	}

	isAllSelected() {
		return this.filteredTasks.length > 0 && this.selectedIds.size === this.filteredTasks.length;
	}

	toggleSelectAll() {
		if (this.isAllSelected()) this.clearSelection();
		else this.selectAll();
	}

	// bulk operations
	async bulkDeleteTasks() {
		const ids = [...this.selectedIds];
		this.clearSelection();
		try {
			await bulkDelete(ids);
			await this.refresh();
			toast.success(
				ids.length === 1 ? 'Task moved to trash' : `${ids.length} tasks moved to trash`,
				{
					action: { label: 'Undo', onClick: () => void this.undoDelete(ids) }
				}
			);
		} catch {
			this.error = 'Failed to delete tasks';
		}
	}

	async bulkDuplicateTasks() {
		const ids = [...this.selectedIds];
		this.clearSelection();
		try {
			for (const id of ids) await duplicate(id);
			await this.refresh();
		} catch {
			this.error = 'Failed to duplicate tasks';
		}
	}

	async bulkChangeStatus(status: TaskStatus) {
		const ids = [...this.selectedIds];
		this.clearSelection();
		try {
			await bulkUpdateStatus(ids, status);
			await this.refresh();
		} catch {
			this.error = 'Failed to update status';
		}
	}

	async bulkChangePriority(priority: TaskPriority) {
		const ids = [...this.selectedIds];
		this.clearSelection();
		try {
			await bulkUpdatePriority(ids, priority);
			await this.refresh();
		} catch {
			this.error = 'Failed to update priority';
		}
	}

	async bulkMoveProject(projectId: string | null) {
		const ids = [...this.selectedIds];
		this.clearSelection();
		try {
			await bulkUpdateProject(ids, projectId);
			await this.refresh();
		} catch {
			this.error = 'Failed to move tasks';
		}
	}

	// label click in the bulk bar: every selected task not carrying the label
	// gets it, and when all of them already have it the click removes it
	async bulkApplyLabel(labelId: string, add: boolean) {
		const ids = [...this.selectedIds];
		this.clearSelection();
		try {
			if (add) await addLabelToTasks(ids, labelId);
			else await removeLabelFromTasks(ids, labelId);
			await this.refresh();
		} catch {
			this.error = 'Failed to update labels';
		}
	}

	// refresh and CRUD
	async refresh() {
		// keep showing cached data while refreshing; only show the loading
		// state when there is nothing on screen yet
		const hasData = this.tasks.length > 0;
		if (!hasData) this.loading = true;
		this.error = null;
		try {
			const [tasks, projects, labels, views] = await Promise.all([
				findAll(),
				findProjects(),
				findLabels(),
				findViews().catch(() => [] as SavedView[])
			]);
			this.tasks = tasks;
			this.projects = projects;
			this.labels = labels;
			this.savedViews = views;
			writeDataCache({ tasks, projects, labels });
			window.dispatchEvent(new Event('tasks-changed'));
			window.dispatchEvent(new Event('projects-changed'));
		} catch {
			this.error = 'Failed to load tasks';
		} finally {
			this.loading = false;
		}
	}

	handleCreated(task: Task) {
		this.tasks = [task, ...this.tasks];
		window.dispatchEvent(new Event('tasks-changed'));
	}

	handleProjectCreated(project: Project) {
		this.projects = [...this.projects, project].sort((a, b) => a.name.localeCompare(b.name));
		window.dispatchEvent(new Event('projects-changed'));
	}

	handleLabelCreated(label: Label) {
		this.labels = [...this.labels, label].sort((a, b) => a.name.localeCompare(b.name));
	}

	handleLabelUpdated(label: Label) {
		this.labels = this.labels.map((l) => (l.id === label.id ? label : l));
	}

	handleLabelRemoved(id: string) {
		this.labels = this.labels.filter((l) => l.id !== id);
		this.tasks = this.tasks.map((t) => ({
			...t,
			labelIds: (t.labelIds ?? []).filter((lId) => lId !== id)
		}));
	}

	handleProjectUpdated(project: Project) {
		this.projects = this.projects.map((item) => (item.id === project.id ? project : item));
		window.dispatchEvent(new Event('projects-changed'));
	}

	handleEdit(task: Task) {
		// the detail panel is an overlay; remember the list scroll so closing
		// the panel returns to the same spot instead of the top
		this.listScrollTop =
			document.querySelector('main .overflow-auto')?.scrollTop ?? this.listScrollTop;
		this.editingTask = task;
		this.editDialogOpen = true;
	}

	// apply a saved view's filter snapshot
	applyView(view: SavedView) {
		const f = view.filters;
		this.statusFilters = new Set(f.statuses ?? []);
		this.projectFilters = new Set(f.projectIds ?? []);
		this.labelFilters = new Set(f.labelIds ?? []);
		this.priorityFilter = f.priority ?? null;
		this.pinnedFilter = f.pinned ?? false;
		this.todayFilter = f.due === 'today';
		this.upcomingFilter = f.due === 'upcoming';
		this.overdueFilter = f.due === 'overdue';
		this.updatedSinceDays = f.updatedSinceDays ?? null;
		this.searchQuery = '';
	}

	// the filter snapshot a saved view would restore right now
	currentViewFilters = $derived<ViewFilters>({
		statuses: [...this.statusFilters],
		projectIds: [...this.projectFilters],
		labelIds: [...this.labelFilters],
		priority: this.priorityFilter,
		pinned: this.pinnedFilter,
		due: this.todayFilter
			? 'today'
			: this.upcomingFilter
				? 'upcoming'
				: this.overdueFilter
					? 'overdue'
					: null,
		updatedSinceDays: this.updatedSinceDays
	});

	async saveCurrentView(name: string) {
		const trimmed = name.trim();
		if (!trimmed) return;
		try {
			const view = await createView(trimmed, this.currentViewFilters);
			this.savedViews = [...this.savedViews, view];
			toast.success(`View "${trimmed}" saved`);
		} catch {
			this.error = 'Failed to save view';
		}
	}

	async renameView(id: string, name: string) {
		const trimmed = name.trim();
		if (!trimmed) return;
		try {
			await renameViewDb(id, trimmed);
			this.savedViews = this.savedViews.map((v) => (v.id === id ? { ...v, name: trimmed } : v));
		} catch {
			this.error = 'Failed to rename view';
		}
	}

	reorderViews(ordered: SavedView[]) {
		// optimistic: the sidebar re-renders immediately, the write catches up
		this.savedViews = ordered;
		void reorderViewsDb(ordered.map((v) => v.id)).catch(() => {
			this.error = 'Failed to reorder views';
		});
	}

	async deleteView(id: string) {
		try {
			await removeView(id);
			this.savedViews = this.savedViews.filter((v) => v.id !== id);
		} catch {
			this.error = 'Failed to delete view';
		}
	}

	private listScrollTop = 0;

	handleUpdated(task: Task) {
		this.tasks = this.tasks.map((item) => (item.id === task.id ? task : item));
		window.dispatchEvent(new Event('tasks-changed'));
	}

	async handleDelete(id: string) {
		try {
			await remove(id);
			await this.refresh();
			toast.success('Task moved to trash', {
				action: { label: 'Undo', onClick: () => void this.undoDelete([id]) }
			});
		} catch {
			this.error = 'Failed to delete task';
		}
	}

	// undo path for the delete toast: put every task back in one refresh
	async undoDelete(ids: string[]) {
		try {
			for (const id of ids) await restore(id);
			await this.refresh();
		} catch {
			this.error = 'Failed to restore task';
		}
	}

	async changeStatus(task: Task, newStatus: TaskStatus) {
		try {
			const updated = await update(task.id, { status: newStatus });
			if (updated) this.handleUpdated(updated);
		} catch {
			this.error = 'Failed to update status';
		}
	}

	async changePriority(task: Task, newPriority: TaskPriority) {
		try {
			const updated = await update(task.id, { priority: newPriority });
			if (updated) this.handleUpdated(updated);
		} catch {
			this.error = 'Failed to update priority';
		}
	}

	async handleTogglePin(task: Task) {
		try {
			const updated = await togglePin(task.id);
			if (updated) this.handleUpdated(updated);
		} catch {
			this.error = 'Failed to toggle pin';
		}
	}

	async duplicateTask(id: string) {
		try {
			await duplicate(id);
		} catch {
			this.error = 'Failed to duplicate task';
		}
	}

	// dnd
	async handleBoardDrop(state: DragDropState<Task>, targetTask: Task | null) {
		const draggedTask = state.draggedItem;
		if (!draggedTask || !draggedTask.id || !state.dropPosition) return;

		const sourceStatus = (state.sourceContainer || '').replace('board-', '') as TaskStatus;
		const targetStatus = (state.targetContainer || '').replace('board-', '') as TaskStatus;
		if (!sourceStatus || !targetStatus) return;

		const sourceGroup = this.groupedTasks[sourceStatus];
		const targetGroup = this.groupedTasks[targetStatus];
		const draggedIndex = sourceGroup.findIndex((t) => t.id === draggedTask.id);

		if (sourceStatus === targetStatus && targetTask) {
			const targetIndex = targetGroup.findIndex((t) => t.id === targetTask.id);
			if (targetIndex !== -1) {
				if (state.dropPosition === 'before' && draggedIndex === targetIndex - 1) return;
				if (state.dropPosition === 'after' && draggedIndex === targetIndex + 1) return;
				if (draggedIndex === targetIndex) return;
			}
		}

		const groupTasks = targetGroup.filter((t) => t.id !== draggedTask.id);
		if (targetTask) {
			groupTasks.splice(
				state.dropPosition === 'after'
					? groupTasks.indexOf(targetTask) + 1
					: groupTasks.indexOf(targetTask),
				0,
				draggedTask
			);
		} else {
			groupTasks.push(draggedTask);
		}

		try {
			if (sourceStatus !== targetStatus) await update(draggedTask.id, { status: targetStatus });
			await reorderInStatus(
				targetStatus,
				groupTasks.map((t) => t.id)
			);
			if (sourceStatus !== targetStatus) {
				const remainingSource = sourceGroup.filter((t) => t.id !== draggedTask.id).map((t) => t.id);
				await reorderInStatus(sourceStatus, remainingSource);
			}
			await this.refresh();
		} catch {
			this.error = 'Failed to move task';
		}
	}

	async handleListDrop(state: DragDropState<Task>, targetTask: Task) {
		const draggedTask = state.draggedItem;
		if (!draggedTask || !draggedTask.id || !state.dropPosition) return;

		const sourceStatus = (state.sourceContainer || '').replace('list-', '') as TaskStatus;
		const targetStatus = (state.targetContainer || '').replace('list-', '') as TaskStatus;
		if (!sourceStatus || !targetStatus || sourceStatus !== targetStatus) return;

		const group = this.groupedTasks[targetStatus];
		const draggedIndex = group.findIndex((t) => t.id === draggedTask.id);
		const targetIndex = group.findIndex((t) => t.id === targetTask.id);

		if (draggedIndex === targetIndex) return;
		if (state.dropPosition === 'before' && draggedIndex === targetIndex - 1) return;
		if (state.dropPosition === 'after' && draggedIndex === targetIndex + 1) return;

		const groupTasks = group.filter((t) => t.id !== draggedTask.id);
		groupTasks.splice(
			state.dropPosition === 'after'
				? groupTasks.indexOf(targetTask) + 1
				: groupTasks.indexOf(targetTask),
			0,
			draggedTask
		);

		try {
			await reorderInStatus(
				targetStatus,
				groupTasks.map((t) => t.id)
			);
			await this.refresh();
		} catch {
			this.error = 'Failed to reorder task';
		}
	}

	constructor() {
		this.seedFromCache();
		$effect(() => {
			setSettings({ defaultViewMode: this.viewMode });
		});
		$effect(() => {
			useDndActive();
		});
		$effect(() => {
			const query = this.searchQuery;
			const timer = setTimeout(() => void this.searchFts(query), 150);
			return () => clearTimeout(timer);
		});
		$effect(() => {
			// create-dialog inputs are one-shot: clear them on close so a dialog
			// opened without the event (e.g. board "+") never sees stale values
			if (!this.dialogOpen) {
				this.createInitialTitle = null;
				this.createDueDate = null;
				this.createParentId = null;
				this.createInitial = null;
			}
		});
		$effect(() => {
			// the detail panel closes -> put the list scroll back where it was
			if (this.editDialogOpen) return;
			const el = document.querySelector('main .overflow-auto');
			if (el) requestAnimationFrame(() => (el.scrollTop = this.listScrollTop));
		});
	}

	// returns the shared instance, creating it on first call
	static get(): TaskPageState {
		sharedInstance ??= new TaskPageState();
		return sharedInstance;
	}

	// seed the stores from the last cached snapshot, so first paint has data
	// immediately while the db refresh revalidates it
	private seedFromCache(): void {
		const cached = readDataCache();
		if (!cached) return;
		this.tasks = cached.tasks;
		this.projects = cached.projects;
		this.labels = cached.labels;
		this.loading = false;
	}

	// mount lifecycle
	init() {
		const openDialog = (event: Event) => {
			// the palette's "Create task '<query>'" item prefills the title
			const detail = (event as CustomEvent<Partial<Task>>).detail;
			this.createInitial = detail ?? null;
			this.createInitialTitle = detail?.title ?? null;
			this.createParentId = detail?.parentId ?? null;
			this.dialogOpen = true;
		};
		const openProjectDialog = () => (this.projectDialogOpen = true);
		const openProjectEditDialog = (event: Event) => {
			this.editingProject = (event as CustomEvent<Project>).detail;
			this.projectEditOpen = true;
		};
		const editTaskFromCommand = (event: Event) => {
			const task = (event as CustomEvent<Task>).detail;
			this.handleEdit(task);
		};
		const deleteTaskFromPanel = (event: Event) => {
			const taskId = (event as CustomEvent<string>).detail;
			void this.handleDelete(taskId);
		};
		const taskCreated = (event: Event) => {
			this.handleCreated((event as CustomEvent<Task>).detail);
		};
		const filterByProject = (event: Event) => {
			const projectId = (event as CustomEvent<string>).detail;
			this.pinnedFilter = false;
			this.todayFilter = false;
			this.upcomingFilter = false;
			this.overdueFilter = false;
			this.priorityFilter = null;
			this.updatedSinceDays = null;
			this.searchQuery = '';
			this.statusFilters = new Set();
			this.labelFilters = new Set();
			this.projectFilters = new Set([projectId]);
		};
		const filterPinned = () => {
			this.pinnedFilter = true;
			this.todayFilter = false;
			this.upcomingFilter = false;
			this.overdueFilter = false;
			this.priorityFilter = null;
			this.updatedSinceDays = null;
			this.searchQuery = '';
			this.statusFilters = new Set();
			this.projectFilters = new Set();
			this.labelFilters = new Set();
		};
		const filterByStatus = (event: Event) => {
			const status = (event as CustomEvent<TaskStatus>).detail;
			this.pinnedFilter = false;
			this.todayFilter = false;
			this.upcomingFilter = false;
			this.overdueFilter = false;
			this.priorityFilter = null;
			this.updatedSinceDays = null;
			this.searchQuery = '';
			this.projectFilters = new Set();
			this.labelFilters = new Set();
			this.statusFilters = new Set([status]);
		};
		const filterByPriority = (event: Event) => {
			const priority = (event as CustomEvent<number>).detail;
			this.pinnedFilter = false;
			this.todayFilter = false;
			this.upcomingFilter = false;
			this.overdueFilter = false;
			this.searchQuery = '';
			this.statusFilters = new Set();
			this.projectFilters = new Set();
			this.labelFilters = new Set();
			this.priorityFilter = priority;
		};
		const filterByLabel = (event: Event) => {
			const labelId = (event as CustomEvent<string>).detail;
			this.pinnedFilter = false;
			this.todayFilter = false;
			this.upcomingFilter = false;
			this.overdueFilter = false;
			this.priorityFilter = null;
			this.updatedSinceDays = null;
			this.searchQuery = '';
			this.statusFilters = new Set();
			this.projectFilters = new Set();
			this.labelFilters = new Set([labelId]);
		};
		const filterToday = () => {
			this.pinnedFilter = false;
			this.todayFilter = true;
			this.upcomingFilter = false;
			this.overdueFilter = false;
			this.priorityFilter = null;
			this.updatedSinceDays = null;
			this.searchQuery = '';
			this.statusFilters = new Set();
			this.projectFilters = new Set();
			this.labelFilters = new Set();
		};
		const filterUpcoming = () => {
			this.pinnedFilter = false;
			this.todayFilter = false;
			this.upcomingFilter = true;
			this.overdueFilter = false;
			this.priorityFilter = null;
			this.updatedSinceDays = null;
			this.searchQuery = '';
			this.statusFilters = new Set();
			this.projectFilters = new Set();
			this.labelFilters = new Set();
		};
		const filterOverdue = () => {
			this.pinnedFilter = false;
			this.todayFilter = false;
			this.upcomingFilter = false;
			this.overdueFilter = true;
			this.priorityFilter = null;
			this.updatedSinceDays = null;
			this.searchQuery = '';
			this.statusFilters = new Set();
			this.projectFilters = new Set();
			this.labelFilters = new Set();
		};
		const clearFiltersHandler = () => this.clearFilters();
		const applyViewHandler = (event: Event) => {
			this.applyView((event as CustomEvent<SavedView>).detail);
		};
		const setView = (event: Event) => {
			const mode = (event as CustomEvent<string>).detail;
			if (mode === 'list' || mode === 'board' || mode === 'calendar') this.viewMode = mode;
		};
		const registry = getShortcutRegistry();

		const unregisterSelectAll = registry.register({
			id: 'select-all',
			enabled: () => this.filteredTasks.length > 0 && !isTypingTarget(),
			run: () => this.toggleSelectAll()
		});

		const unregisterToggleView = registry.register({
			id: 'toggle-view',
			run: () => {
				const order = ['list', 'board', 'calendar'] as const;
				const next = order[(order.indexOf(this.viewMode) + 1) % order.length];
				this.viewMode = next;
			}
		});

		const unregisterClearSelection = registry.register({
			id: 'close',
			enabled: () => this.hasSelection,
			run: () => this.clearSelection()
		});

		const savedView = getSettings().defaultViewMode;
		if (savedView === 'list' || savedView === 'board' || savedView === 'calendar')
			this.viewMode = savedView;

		window.addEventListener('open-task-dialog', openDialog);
		window.addEventListener('open-project-dialog', openProjectDialog);
		window.addEventListener('open-project-edit-dialog', openProjectEditDialog);
		window.addEventListener('edit-task-from-command', editTaskFromCommand);
		window.addEventListener('delete-task-from-panel', deleteTaskFromPanel);
		window.addEventListener('task-created', taskCreated);
		window.addEventListener('filter-by-project', filterByProject);
		window.addEventListener('filter-pinned', filterPinned);
		window.addEventListener('filter-by-status', filterByStatus);
		window.addEventListener('filter-by-priority', filterByPriority);
		window.addEventListener('filter-by-label', filterByLabel);
		window.addEventListener('filter-today', filterToday);
		window.addEventListener('filter-upcoming', filterUpcoming);
		window.addEventListener('filter-overdue', filterOverdue);
		window.addEventListener('clear-filters', clearFiltersHandler);
		window.addEventListener('apply-view', applyViewHandler);
		window.addEventListener('set-view', setView);

		let refreshTimer: ReturnType<typeof setTimeout> | null = null;
		const unlistenDbChanged = onDbChanged(() => {
			if (refreshTimer) clearTimeout(refreshTimer);
			refreshTimer = setTimeout(() => void this.refresh(), 800);
		});

		void this.refresh();

		return () => {
			window.removeEventListener('open-task-dialog', openDialog);
			window.removeEventListener('open-project-dialog', openProjectDialog);
			window.removeEventListener('open-project-edit-dialog', openProjectEditDialog);
			window.removeEventListener('edit-task-from-command', editTaskFromCommand);
			window.removeEventListener('delete-task-from-panel', deleteTaskFromPanel);
			window.removeEventListener('task-created', taskCreated);
			window.removeEventListener('filter-by-project', filterByProject);
			window.removeEventListener('filter-pinned', filterPinned);
			window.removeEventListener('filter-by-status', filterByStatus);
			window.removeEventListener('filter-by-priority', filterByPriority);
			window.removeEventListener('filter-by-label', filterByLabel);
			window.removeEventListener('filter-today', filterToday);
			window.removeEventListener('filter-upcoming', filterUpcoming);
			window.removeEventListener('filter-overdue', filterOverdue);
			window.removeEventListener('clear-filters', clearFiltersHandler);
			window.removeEventListener('apply-view', applyViewHandler);
			window.removeEventListener('set-view', setView);
			unregisterSelectAll();
			unregisterToggleView();
			unregisterClearSelection();
			unlistenDbChanged();
		};
	}
}
