<script lang="ts">
	import * as Select from '$lib/components/ui/select/index.js';
	import { Switch } from '$lib/components/ui/switch/index.js';
	import { Separator } from '$lib/components/ui/separator/index.js';
	import type { Settings, Theme } from '$lib/types/settings';

	let {
		settings,
		update
	}: {
		settings: Settings;
		update: <K extends keyof Settings>(key: K, value: Settings[K]) => void;
	} = $props();

	const themeOptions = [
		{ value: 'dark', label: 'Dark' },
		{ value: 'light', label: 'Light' },
		{ value: 'system', label: 'System' }
	];
</script>

<!-- theme -->
<div class="flex items-center justify-between">
	<div>
		<p class="text-[13px] font-medium">Theme</p>
		<p class="text-xs text-muted-foreground">Choose how tack looks to you</p>
	</div>
	<Select.Root
		type="single"
		value={settings.theme}
		onValueChange={(v) => update('theme', v as Theme)}
	>
		<Select.Trigger class="w-32">
			{settings.theme.charAt(0).toUpperCase() + settings.theme.slice(1)}
		</Select.Trigger>
		<Select.Content>
			{#each themeOptions as opt (opt.value)}
				<Select.Item value={opt.value} label={opt.label}>{opt.label}</Select.Item>
			{/each}
		</Select.Content>
	</Select.Root>
</div>

<Separator />

<!-- sidebar collapse -->
<div class="flex items-center justify-between">
	<div>
		<p class="text-[13px] font-medium">Collapse sidebar</p>
		<p class="text-xs text-muted-foreground">Hide sidebar labels and project list</p>
	</div>
	<Switch
		checked={settings.sidebarCollapsed}
		onCheckedChange={(v) => update('sidebarCollapsed', v)}
	/>
</div>

<Separator />

<!-- default view mode -->
<div class="flex items-center justify-between">
	<div>
		<p class="text-[13px] font-medium">Default view</p>
		<p class="text-xs text-muted-foreground">Which view to open by default</p>
	</div>
	<Select.Root
		type="single"
		value={settings.defaultViewMode}
		onValueChange={(v) => update('defaultViewMode', v as 'list' | 'board' | 'calendar')}
	>
		<Select.Trigger class="w-32">
			{settings.defaultViewMode === 'list'
				? 'List'
				: settings.defaultViewMode === 'board'
					? 'Board'
					: 'Calendar'}
		</Select.Trigger>
		<Select.Content>
			<Select.Item value="list" label="List">List</Select.Item>
			<Select.Item value="board" label="Board">Board</Select.Item>
			<Select.Item value="calendar" label="Calendar">Calendar</Select.Item>
		</Select.Content>
	</Select.Root>
</div>
