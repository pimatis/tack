<script lang="ts">
	import * as Select from '$lib/components/ui/select/index.js';
	import { Separator } from '$lib/components/ui/separator/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import PriorityIcon from '../PriorityIcon.svelte';
	import type { Settings } from '$lib/types/settings';

	let {
		settings,
		update
	}: {
		settings: Settings;
		update: <K extends keyof Settings>(key: K, value: Settings[K]) => void;
	} = $props();

	const statusOptions = [
		{ value: 'todo', label: 'Todo' },
		{ value: 'in_progress', label: 'In progress' }
	];

	const priorityOptions = [
		{ value: '0', label: 'No priority' },
		{ value: '1', label: 'Urgent' },
		{ value: '2', label: 'High' },
		{ value: '3', label: 'Medium' },
		{ value: '4', label: 'Low' }
	];
</script>

<!-- default status -->
<div class="flex flex-wrap items-center justify-between gap-2">
	<div>
		<p class="text-[13px] font-medium">Default status</p>
		<p class="text-xs text-muted-foreground">Status assigned to new tasks</p>
	</div>
	<Select.Root
		type="single"
		value={settings.defaultStatus}
		onValueChange={(v) => update('defaultStatus', v as 'todo' | 'in_progress')}
	>
		<Select.Trigger class="w-32">
			{settings.defaultStatus === 'todo' ? 'Todo' : 'In progress'}
		</Select.Trigger>
		<Select.Content>
			{#each statusOptions as opt (opt.value)}
				<Select.Item value={opt.value} label={opt.label}>{opt.label}</Select.Item>
			{/each}
		</Select.Content>
	</Select.Root>
</div>

<Separator />

<!-- default priority -->
<div class="flex flex-wrap items-center justify-between gap-2">
	<div>
		<p class="text-[13px] font-medium">Default priority</p>
		<p class="text-xs text-muted-foreground">Priority assigned to new tasks</p>
	</div>
	<Select.Root
		type="single"
		value={String(settings.defaultPriority)}
		onValueChange={(v) => update('defaultPriority', Number(v) as 0 | 1 | 2 | 3 | 4)}
	>
		<Select.Trigger class="w-36">
			{#if Number(settings.defaultPriority) > 0}
				<PriorityIcon priority={Number(settings.defaultPriority)} size={13} />
			{/if}
			{priorityOptions.find((o) => o.value === String(settings.defaultPriority))?.label ??
				'No priority'}
		</Select.Trigger>
		<Select.Content>
			{#each priorityOptions as opt (opt.value)}
				<Select.Item value={opt.value} label={opt.label}>
					<PriorityIcon priority={Number(opt.value)} size={14} />
					<span>{opt.label}</span>
				</Select.Item>
			{/each}
		</Select.Content>
	</Select.Root>
</div>

<Separator />

<!-- due soon threshold -->
<div class="flex flex-wrap items-center justify-between gap-2">
	<div>
		<p class="text-[13px] font-medium">Due soon threshold</p>
		<p class="text-xs text-muted-foreground">Days ahead to flag tasks as due soon</p>
	</div>
	<div class="flex items-center gap-2">
		<Input
			type="number"
			min="1"
			max="30"
			value={settings.dueSoonThreshold}
			oninput={(e) =>
				update('dueSoonThreshold', Math.max(1, Number((e.target as HTMLInputElement).value) || 1))}
			class="w-16 text-center"
		/>
		<span class="text-xs text-muted-foreground">days</span>
	</div>
</div>

<Separator />

<!-- prefix padding -->
<div class="flex flex-wrap items-center justify-between gap-2">
	<div>
		<p class="text-[13px] font-medium">Task id padding</p>
		<p class="text-xs text-muted-foreground">Zero-pad task numbers (0 = no padding, 3 = TSK-001)</p>
	</div>
	<div class="flex items-center gap-2">
		<Input
			type="number"
			min="0"
			max="6"
			value={settings.prefixPadding}
			oninput={(e) =>
				update(
					'prefixPadding',
					Math.max(0, Math.min(6, Number((e.target as HTMLInputElement).value) || 0))
				)}
			class="w-16 text-center"
		/>
		<span class="text-xs text-muted-foreground">digits</span>
	</div>
</div>
