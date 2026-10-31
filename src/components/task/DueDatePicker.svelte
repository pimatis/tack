<script lang="ts">
	import { today, getLocalTimeZone, parseDate, type DateValue } from '@internationalized/date';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import { Calendar } from '$lib/components/ui/calendar/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Separator } from '$lib/components/ui/separator/index.js';
	import Icon from '$lib/components/Icon.svelte';

	type Props = {
		value: string;
		title?: string;
		onSelect: (date: string) => void;
		onClear?: () => void;
	};

	let { value, title = 'Due date', onSelect, onClear }: Props = $props();
	let open = $state(false);

	let selectedDate = $state<DateValue | undefined>(undefined);

	// dialog width follows the field card that opens it
	let triggerRef = $state<HTMLButtonElement | null>(null);
	let contentWidth = $state<number | null>(null);

	$effect(() => {
		if (!open) return;
		const measured = triggerRef?.offsetWidth ?? 0;
		// the calendar needs room: at least 320px, never wider than the viewport
		const max = window.innerWidth - 32;
		contentWidth = Math.min(Math.max(320, measured), max);
	});

	$effect(() => {
		if (open && value) {
			try {
				selectedDate = parseDate(value);
			} catch {
				selectedDate = undefined;
			}
		}
	});

	function formatDueDate(dateStr: string): string {
		const d = new Date(dateStr + 'T00:00:00');
		return new Intl.DateTimeFormat('en-US', { month: 'short', day: 'numeric' }).format(d);
	}

	function handleSelect(date: DateValue | undefined) {
		if (!date) return;
		selectedDate = date;
	}

	function handleConfirm() {
		if (selectedDate) {
			onSelect(selectedDate.toString());
		}
		open = false;
	}

	function handleClear() {
		selectedDate = undefined;
		onClear?.();
		open = false;
	}

	function quickSelect(days: number) {
		const now = today(getLocalTimeZone());
		selectedDate = now.add({ days });
	}
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
				<Icon icon="calendar" size={13} class="text-muted-foreground" />
				{#if value}
					<span>{formatDueDate(value)}</span>
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

		<!-- header -->
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

		<!-- quick options -->
		<div class="flex flex-wrap items-center gap-1.5 px-4 pb-3">
			<Button
				variant="outline"
				size="xs"
				class="rounded-md border-border bg-muted/30 px-2.5 py-1 text-[11px] font-medium text-muted-foreground transition-colors hover:bg-muted/50 hover:text-foreground"
				onclick={() => quickSelect(0)}
			>
				Today
			</Button>
			<Button
				variant="outline"
				size="xs"
				class="rounded-md border-border bg-muted/30 px-2.5 py-1 text-[11px] font-medium text-muted-foreground transition-colors hover:bg-muted/50 hover:text-foreground"
				onclick={() => quickSelect(1)}
			>
				Tomorrow
			</Button>
			<Button
				variant="outline"
				size="xs"
				class="rounded-md border-border bg-muted/30 px-2.5 py-1 text-[11px] font-medium text-muted-foreground transition-colors hover:bg-muted/50 hover:text-foreground"
				onclick={() => quickSelect(3)}
			>
				In 3 days
			</Button>
			<Button
				variant="outline"
				size="xs"
				class="rounded-md border-border bg-muted/30 px-2.5 py-1 text-[11px] font-medium text-muted-foreground transition-colors hover:bg-muted/50 hover:text-foreground"
				onclick={() => quickSelect(7)}
			>
				In a week
			</Button>
		</div>

		<Separator />

		<!-- calendar -->
		<div class="p-2">
			<Calendar
				type="single"
				value={selectedDate}
				onValueChange={handleSelect}
				captionLayout="dropdown"
				style="--cell-size: 2.5rem"
				class="rounded-lg"
			/>
		</div>

		<Separator />

		<!-- footer -->
		<div class="flex flex-wrap items-center justify-between gap-2 px-4 py-3">
			<Button
				type="button"
				variant="ghost"
				size="sm"
				onclick={handleClear}
				disabled={!value && !selectedDate}
			>
				Clear
			</Button>
			<Button type="button" size="sm" onclick={handleConfirm} disabled={!selectedDate}>
				{#if selectedDate}
					Set {title.toLowerCase()}
				{:else}
					Select a date
				{/if}
			</Button>
		</div>
	</Dialog.Content>
</Dialog.Root>
