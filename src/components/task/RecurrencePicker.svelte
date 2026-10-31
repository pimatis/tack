<script lang="ts">
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Separator } from '$lib/components/ui/separator/index.js';
	import Icon from '$lib/components/Icon.svelte';
	import {
		parseRecurrence,
		recurrenceLabel,
		serializeRecurrence,
		WEEKDAYS,
		type RecurrenceFrequency
	} from '$lib/task/recurrence';

	type Props = {
		value: string;
		title?: string;
		// due date (yyyy-mm-dd) the schedule anchors to, so a weekly or monthly
		// rule starts on the day the user already picked
		anchorDate?: string | null;
		onSelect: (rule: string) => void;
		onClear: () => void;
	};

	let { value, title = 'Repeat', anchorDate = null, onSelect, onClear }: Props = $props();
	let open = $state(false);

	let triggerRef = $state<HTMLButtonElement | null>(null);
	let contentWidth = $state<number | null>(null);

	let frequency = $state<RecurrenceFrequency | 'none'>('none');
	let weekday = $state(1);
	let monthDay = $state(1);

	$effect(() => {
		if (!open) return;
		const measured = triggerRef?.offsetWidth ?? 0;
		const max = window.innerWidth - 32;
		contentWidth = Math.min(Math.max(300, measured), max);
	});

	// seed the picker from the task's current rule, or from its due date
	$effect(() => {
		if (!open) return;
		const parsed = parseRecurrence(value);
		if (parsed) {
			frequency = parsed.frequency;
			weekday = parsed.weekday ?? 1;
			monthDay = parsed.monthDay ?? 1;
			return;
		}
		const anchor = anchorDate ? new Date(anchorDate + 'T00:00:00') : new Date();
		frequency = 'none';
		weekday = anchor.getDay() === 0 ? 7 : anchor.getDay();
		monthDay = anchor.getDate();
	});

	function handleConfirm() {
		if (frequency === 'none') return;
		onSelect(
			serializeRecurrence({
				frequency,
				weekday: frequency === 'weekly' ? weekday : undefined,
				monthDay: frequency === 'monthly' ? monthDay : undefined
			})
		);
		open = false;
	}

	function handleClear() {
		frequency = 'none';
		onClear();
		open = false;
	}

	const frequencies: { value: RecurrenceFrequency | 'none'; label: string }[] = [
		{ value: 'none', label: 'Never' },
		{ value: 'daily', label: 'Daily' },
		{ value: 'weekly', label: 'Weekly' },
		{ value: 'monthly', label: 'Monthly' }
	];
</script>

<Dialog.Root bind:open>
	<Dialog.Trigger>
		{#snippet child({ props })}
			<Button
				bind:ref={triggerRef}
				{...props}
				variant="outline"
				size="sm"
				class="flex h-8 w-full items-center justify-start gap-1.5 rounded-lg border-border bg-muted/30 px-2.5 text-[12px] font-normal shadow-none transition-colors hover:bg-muted/50"
			>
				<Icon icon="repeat" size={13} class="shrink-0 text-muted-foreground" />
				{#if value}
					<span class="truncate">{recurrenceLabel(value)}</span>
					<span
						role="button"
						tabindex="0"
						class="ml-0.5 text-muted-foreground/50 transition-colors hover:text-foreground"
						onclick={(e) => {
							e.stopPropagation();
							handleClear();
						}}
						onkeydown={(e) => {
							if (e.key === 'Enter' || e.key === ' ') {
								e.preventDefault();
								e.stopPropagation();
								handleClear();
							}
						}}
						aria-label="Clear {title.toLowerCase()}"
					>
						<Icon icon="close" size={11} />
					</span>
				{:else}
					<span>{title}</span>
				{/if}
			</Button>
		{/snippet}
	</Dialog.Trigger>
	<Dialog.Content
		class="w-[calc(100vw-2rem)] gap-0 p-0"
		style={contentWidth ? `width: ${contentWidth}px` : undefined}
		showCloseButton={false}
	>
		<Dialog.Title class="sr-only">Pick {title.toLowerCase()}</Dialog.Title>

		<div class="flex items-center justify-between px-4 pt-4 pb-3">
			<span class="text-[13px] font-medium text-foreground">{title}</span>
			<Dialog.Close>
				{#snippet child({ props })}
					<Button
						{...props}
						variant="ghost"
						size="icon-sm"
						class="text-muted-foreground hover:text-foreground"
					>
						<Icon icon="close" size={16} />
					</Button>
				{/snippet}
			</Dialog.Close>
		</div>

		<!-- frequency -->
		<div class="flex flex-wrap items-center gap-1.5 px-4 pb-3">
			{#each frequencies as option (option.value)}
				<Button
					variant="outline"
					size="xs"
					class="rounded-md border-border px-2.5 py-1 text-[11px] font-medium transition-colors {frequency ===
					option.value
						? 'bg-primary/10 text-foreground'
						: 'bg-muted/30 text-muted-foreground hover:bg-muted/50 hover:text-foreground'}"
					onclick={() => (frequency = option.value)}
				>
					{option.label}
				</Button>
			{/each}
		</div>

		{#if frequency === 'weekly' || frequency === 'monthly'}
			<Separator />
			<div class="px-4 py-3">
				{#if frequency === 'weekly'}
					<span class="pb-1.5 text-[11px] text-muted-foreground/60">Day of the week</span>
					<div class="flex flex-wrap gap-1">
						{#each WEEKDAYS as day (day.value)}
							<Button
								variant="outline"
								size="xs"
								class="size-8 rounded-md border-border p-0 text-[11px] font-medium transition-colors {weekday ===
								day.value
									? 'bg-primary/10 text-foreground'
									: 'bg-muted/30 text-muted-foreground hover:bg-muted/50 hover:text-foreground'}"
								aria-label={day.label}
								onclick={() => (weekday = day.value)}
							>
								{day.label.slice(0, 2)}
							</Button>
						{/each}
					</div>
				{:else}
					<span class="pb-1.5 text-[11px] text-muted-foreground/60">Day of the month</span>
					<div class="grid grid-cols-7 gap-1">
						{#each Array.from({ length: 31 }, (_, i) => i + 1) as day (day)}
							<Button
								variant="outline"
								size="xs"
								class="h-8 rounded-md border-border p-0 text-[11px] font-medium transition-colors {monthDay ===
								day
									? 'bg-primary/10 text-foreground'
									: 'bg-muted/30 text-muted-foreground hover:bg-muted/50 hover:text-foreground'}"
								aria-label={`Day ${day}`}
								onclick={() => (monthDay = day)}
							>
								{day}
							</Button>
						{/each}
					</div>
				{/if}
			</div>
		{/if}

		<Separator />

		<div class="flex flex-wrap items-center justify-between gap-2 px-4 py-3">
			<Button
				type="button"
				variant="ghost"
				size="sm"
				onclick={handleClear}
				disabled={!value && frequency === 'none'}
			>
				Clear
			</Button>
			<Button type="button" size="sm" onclick={handleConfirm} disabled={frequency === 'none'}>
				{frequency === 'none' ? 'Pick a rule' : 'Set repeat'}
			</Button>
		</div>
	</Dialog.Content>
</Dialog.Root>
