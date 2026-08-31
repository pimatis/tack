import { listen } from '@tauri-apps/api/event';
import {
	remove,
	findAll,
	update,
	bulkDelete,
	bulkUpdateStatus,
	bulkUpdatePriority,
	bulkUpdateProject,
	duplicate,
	reorderInStatus,
	togglePin
} from '$lib/repositories/task.repository';
import { findAll as findProjects } from '$lib/repositories/project.repository';
import { findAll as findLabels } from '$lib/repositories/label.repository';
import { getSettings, setSettings } from '$lib/stores/settings';
import { searchTaskIds } from '$lib/search/fts.service';
import { getShortcutRegistry } from '$lib/shortcuts/index.js';
import { sortableItem, dropZone, useDndActive, type DragDropState } from '$lib/dnd';
import type { Project } from '$lib/types/project';
import type { Task, TaskPriority, TaskStatus } from '$lib/types/task';
import type { Label } from '$lib/types/label';
import type { Settings } from '$lib/types/settings';
import { statusOrder } from './constants';
import { isTypingTarget, groupedTasks as computeGroups } from './utils';

// a task's calendar span: due date starts it, end date finishes it
function dateRange(t: Task): { start: Date | null; end: Date | null } {
	return {
		start: t.dueDate ? new Date(t.dueDate + 'T00:00:00') : null,
		end: t.endDate ? new Date(t.endDate + 'T00:00:00') : null
	};
}

export { sortableItem, dropZone, useDndActive };

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
	createDueDate = $state<string | null>(null);
	statusFilters = $state<Set<TaskStatus>>(new Set());
	projectFilters = $state<Set<string>>(new Set());
	labelFilters = $state<Set<string>>(new Set());

	selectedIds = $state<Set<string>>(new Set());
	lastSelectedId = $state<string | null>(null);
	viewMode = $state<'list' | 'board' | 'calendar'>(getSettings().defaultViewMode);
	appSettings = $state<Settings>(getSettings());
	pinnedFilter = $state(false);
	todayFilter = $state(false);
	upcomingFilter = $state(false);
	overdueFilter = $state(false);
	priorityFilter = $state<number | null>(null);

	hasFilters = $derived(
		this.searchQuery.trim().length > 0 ||
			this.statusFilters.size + this.projectFilters.size + this.labelFilters.size > 0
	);
	selectedCount = $derived(this.selectedIds.size);
	hasSelection = $derived(this.selectedCount > 0);
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

		const q = this.searchQuery.toLowerCase().trim();
		if (q) {
			// fts match (title, description, subtasks) with in-memory fallback while pending
			if (this.ftsIds) result = result.filter((t) => this.ftsIds?.has(t.id));
			else result = result.filter((t) => t.title.toLowerCase().includes(q));
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

	async searchFts(query: string) {
		const q = query.trim();
		if (!q) {
			this.ftsIds = null;
			return;
		}
		try {
			this.ftsIds = await searchTaskIds(q);
		} catch {
			this.ftsIds = null;
		}
	}

	clearFilters() {
		this.searchQuery = '';
		this.statusFilters = new Set();
		this.projectFilters = new Set();
		this.labelFilters = new Set();
		this.pinnedFilter = false;
		this.todayFilter = false;
		this.upcomingFilter = false;
		this.overdueFilter = false;
		this.priorityFilter = null;
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

	// refresh and CRUD
	async refresh() {
		this.loading = true;
		this.error = null;
		try {
			[this.tasks, this.projects, this.labels] = await Promise.all([
				findAll(),
				findProjects(),
				findLabels()
			]);
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
		this.editingTask = task;
		this.editDialogOpen = true;
	}

	handleUpdated(task: Task) {
		this.tasks = this.tasks.map((item) => (item.id === task.id ? task : item));
		window.dispatchEvent(new Event('tasks-changed'));
	}

	async handleDelete(id: string) {
		try {
			await remove(id);
			await this.refresh();
		} catch {
			this.error = 'Failed to delete task';
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
	}

	// mount lifecycle
	init() {
		const openDialog = () => (this.dialogOpen = true);
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
		const filterByProject = (event: Event) => {
			const projectId = (event as CustomEvent<string>).detail;
			this.pinnedFilter = false;
			this.todayFilter = false;
			this.upcomingFilter = false;
			this.overdueFilter = false;
			this.priorityFilter = null;
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
			this.searchQuery = '';
			this.statusFilters = new Set();
			this.projectFilters = new Set();
			this.labelFilters = new Set();
		};
		const clearFiltersHandler = () => this.clearFilters();
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
		window.addEventListener('filter-by-project', filterByProject);
		window.addEventListener('filter-pinned', filterPinned);
		window.addEventListener('filter-by-status', filterByStatus);
		window.addEventListener('filter-by-priority', filterByPriority);
		window.addEventListener('filter-by-label', filterByLabel);
		window.addEventListener('filter-today', filterToday);
		window.addEventListener('filter-upcoming', filterUpcoming);
		window.addEventListener('filter-overdue', filterOverdue);
		window.addEventListener('clear-filters', clearFiltersHandler);

		let refreshTimer: ReturnType<typeof setTimeout> | null = null;
		const unlistenDbChanged = listen('db-changed', () => {
			if (refreshTimer) clearTimeout(refreshTimer);
			refreshTimer = setTimeout(() => void this.refresh(), 200);
		});

		void this.refresh();

		return () => {
			window.removeEventListener('open-task-dialog', openDialog);
			window.removeEventListener('open-project-dialog', openProjectDialog);
			window.removeEventListener('open-project-edit-dialog', openProjectEditDialog);
			window.removeEventListener('edit-task-from-command', editTaskFromCommand);
			window.removeEventListener('delete-task-from-panel', deleteTaskFromPanel);
			window.removeEventListener('filter-by-project', filterByProject);
			window.removeEventListener('filter-pinned', filterPinned);
			window.removeEventListener('filter-by-status', filterByStatus);
			window.removeEventListener('filter-by-priority', filterByPriority);
			window.removeEventListener('filter-by-label', filterByLabel);
			window.removeEventListener('filter-today', filterToday);
			window.removeEventListener('filter-upcoming', filterUpcoming);
			window.removeEventListener('filter-overdue', filterOverdue);
			window.removeEventListener('clear-filters', clearFiltersHandler);
			unregisterSelectAll();
			unregisterToggleView();
			unregisterClearSelection();
			unlistenDbChanged.then((fn) => fn());
		};
	}
}
