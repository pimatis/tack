<script lang="ts">
	import { onMount } from 'svelte';
	import TaskCreateDialog from '../components/TaskCreateDialog.svelte';
	import TaskDetailPanel from '../components/TaskDetailPanel.svelte';
	import ProjectCreateDialog from '../components/ProjectCreateDialog.svelte';
	import ProjectEditDialog from '../components/ProjectEditDialog.svelte';
	import TaskHeader from '../components/task/TaskHeader.svelte';
	import BulkActionBar from '../components/task/BulkActionBar.svelte';
	import FilterBar from '../components/task/FilterBar.svelte';
	import TaskStates from '../components/task/TaskStates.svelte';
	import TaskList from '../components/task/TaskList.svelte';
	import BoardView from '../components/task/BoardView.svelte';
	import CalendarView from '../components/task/CalendarView.svelte';
	import { Separator } from '$lib/components/ui/separator/index.js';
	import { TaskPageState } from '$lib/task/taskState.svelte';
	import { statusOrder } from '$lib/task/constants';

	const state = new TaskPageState();

	onMount(() => state.init());
</script>

<svelte:head>
	<title>Tasks | Tack</title>
</svelte:head>

<TaskCreateDialog
	bind:open={state.dialogOpen}
	projects={state.projects}
	labels={state.labels}
	initialDueDate={state.createDueDate}
	onCreated={(t) => state.handleCreated(t)}
	onLabelCreated={(l) => state.handleLabelCreated(l)}
	onLabelUpdated={(l) => state.handleLabelUpdated(l)}
	onLabelRemoved={(id) => state.handleLabelRemoved(id)}
/>
<ProjectCreateDialog
	bind:open={state.projectDialogOpen}
	onCreated={(p) => state.handleProjectCreated(p)}
/>
<ProjectEditDialog
	bind:open={state.projectEditOpen}
	project={state.editingProject}
	onUpdated={(p) => state.handleProjectUpdated(p)}
/>
<TaskDetailPanel
	bind:open={state.editDialogOpen}
	task={state.editingTask}
	prefix={state.projects.find((project) => project.id === state.editingTask?.projectId)?.prefix ??
		''}
	labels={state.labels}
	onUpdated={(t) => state.handleUpdated(t)}
	onLabelCreated={(l) => state.handleLabelCreated(l)}
	onLabelUpdated={(l) => state.handleLabelUpdated(l)}
	onLabelRemoved={(id) => state.handleLabelRemoved(id)}
/>

<section class="flex h-full flex-col px-8 py-8">
	<TaskHeader
		pinnedFilter={state.pinnedFilter}
		hasFilters={state.hasFilters}
		filteredCount={state.filteredTasks.length}
		totalCount={state.tasks.length}
		bind:viewMode={state.viewMode}
	/>

	{#if !state.loading && !state.error && state.tasks.length > 0}
		{#if state.hasSelection}
			<BulkActionBar
				selectedCount={state.selectedCount}
				isAllSelected={state.isAllSelected()}
				{statusOrder}
				projects={state.projects}
				onBulkChangeStatus={(s) => state.bulkChangeStatus(s)}
				onBulkChangePriority={(p) => state.bulkChangePriority(p)}
				onBulkMoveProject={(id) => state.bulkMoveProject(id)}
				onBulkDuplicate={() => state.bulkDuplicateTasks()}
				onBulkDelete={() => state.bulkDeleteTasks()}
				onClearSelection={() => state.clearSelection()}
				onToggleSelectAll={() => state.toggleSelectAll()}
			/>
			<Separator class="mb-4" />
		{:else}
			<FilterBar
				bind:searchQuery={state.searchQuery}
				statusFilters={state.statusFilters}
				projectFilters={state.projectFilters}
				labelFilters={state.labelFilters}
				projects={state.projects}
				labels={state.labels}
				hasFilters={state.hasFilters}
				onToggleStatusFilter={(s) => state.toggleStatusFilter(s)}
				onToggleProjectFilter={(id) => state.toggleProjectFilter(id)}
				onToggleLabelFilter={(id) => state.toggleLabelFilter(id)}
				onClearFilters={() => state.clearFilters()}
			/>
			<Separator class="mb-4" />
		{/if}
	{/if}

	{#if state.loading || state.error || state.tasks.length === 0 || state.filteredTasks.length === 0}
		<TaskStates
			loading={state.loading}
			error={state.error}
			tasksEmpty={state.tasks.length === 0}
			filteredEmpty={state.tasks.length > 0 && state.filteredTasks.length === 0}
			onRefresh={() => state.refresh()}
			onClearFilters={() => state.clearFilters()}
		/>
	{:else if state.viewMode === 'list'}
		<TaskList
			groups={state.groupedTasks}
			statusOrderList={statusOrder}
			projects={state.projects}
			appSettings={state.appSettings}
			labelMap={state.labelMap}
			selectedIds={state.selectedIds}
			onToggleSelect={(id, shift) => state.toggleSelect(id, shift)}
			onChangePriority={(t, p) => state.changePriority(t, p)}
			onChangeStatus={(t, s) => state.changeStatus(t, s)}
			onEdit={(t) => state.handleEdit(t)}
			onTogglePin={(t) => state.handleTogglePin(t)}
			onDuplicate={(id) => state.duplicateTask(id)}
			onDelete={(id) => state.handleDelete(id)}
			onListDrop={(s, t) => state.handleListDrop(s, t)}
		/>
	{:else if state.viewMode === 'board'}
		<BoardView
			groups={state.groupedTasks}
			statusOrderList={statusOrder}
			projects={state.projects}
			appSettings={state.appSettings}
			labelMap={state.labelMap}
			selectedIds={state.selectedIds}
			bind:dialogOpen={state.dialogOpen}
			onEdit={(t) => state.handleEdit(t)}
			onChangeStatus={(t, s) => state.changeStatus(t, s)}
			onChangePriority={(t, p) => state.changePriority(t, p)}
			onTogglePin={(t) => state.handleTogglePin(t)}
			onDuplicate={(id) => state.duplicateTask(id)}
			onDelete={(id) => state.handleDelete(id)}
			onBoardDrop={(s, t) => state.handleBoardDrop(s, t)}
		/>
	{:else}
		<CalendarView
			tasks={state.filteredTasks}
			projects={state.projects}
			appSettings={state.appSettings}
			labelMap={state.labelMap}
			onEdit={(t) => state.handleEdit(t)}
			onChangeStatus={(t, s) => state.changeStatus(t, s)}
			onChangePriority={(t, p) => state.changePriority(t, p)}
			onTogglePin={(t) => state.handleTogglePin(t)}
			onDuplicate={(id) => state.duplicateTask(id)}
			onDelete={(id) => state.handleDelete(id)}
			onAddTask={(d) => {
				state.createDueDate = d;
				state.dialogOpen = true;
			}}
		/>
	{/if}
</section>
