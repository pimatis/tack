<script lang="ts">
	import { today, getLocalTimeZone, parseDate, type DateValue } from '@internationalized/date';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import { Calendar } from '$lib/components/ui/calendar/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Separator } from '$lib/components/ui/separator/index.js';

	type Props = {
		// utc iso timestamp, '' when unset
		value: string;
		title?: string;
		onSelect: (iso: string) => void;
		onClear: () => void;
	};

	let { value, title = 'Reminder', onSelect, onClear }: Props = $props();
	let open = $state(false);

	let selectedDate = $state<DateValue | undefined>(undefined);
	let selectedTime = $state('09:00');

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

	// seed the picker from the current utc timestamp, shown in local time
	$effect(() => {
		if (!open) return;
		const d = value ? new Date(value) : null;
		if (d && !Number.isNaN(d.getTime())) {
			selectedDate = parseDate(`${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`);
			selectedTime = `${pad(d.getHours())}:${pad(d.getMinutes())}`;
			return;
		}
		selectedDate = undefined;
		selectedTime = '09:00';
	});

	function pad(n: number): string {
		return String(n).padStart(2, '0');
	}

	function formatReminder(iso: string): string {
		return new Intl.DateTimeFormat('en-US', {
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		}).format(new Date(iso));
	}

	function handleSelect(date: DateValue | undefined) {
		if (date) selectedDate = date;
	}

	function handleConfirm() {
		if (!selectedDate) return;
		const [hh, mm] = selectedTime.split(':').map(Number);
		// the picked date is local; store the combined value as utc
		const dt = new Date(
			selectedDate.year,
			selectedDate.month - 1,
			selectedDate.day,
			hh || 0,
			mm || 0
		);
		onSelect(dt.toISOString());
		open = false;
	}

	function handleClear() {
		selectedDate = undefined;
		onClear();
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
				<svg class="text-muted-foreground" width="13" height="13" viewBox="0 0 24 24" fill="none"
					><path
						fill="currentColor"
						fill-rule="evenodd"
						d="M6.972 3.777a1 1 0 1 0-1.258-1.554 10.038 10.038 0 0 0-2.602 3.19 1 1 0 1 0 1.776.919 8.038 8.038 0 0 1 2.084-2.555m11.314-1.554a1 1 0 1 0-1.258 1.554 8.038 8.038 0 0 1 2.09 2.568 1 1 0 1 0 1.778-.916 10.04 10.04 0 0 0-2.61-3.206M5 10a7 7 0 0 1 14 0v3.764l1.822 3.644A1.1 1.1 0 0 1 19.838 19H4.162a1.1 1.1 0 0 1-.984-1.592L5 13.764zm4 10h6a2 2 0 0 1-2 2h-2a2 2 0 0 1-2-2"
					/></svg
				>
				{#if value}
					<span>{formatReminder(value)}</span>
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
						<svg width="11" height="11" viewBox="0 0 24 24" fill="none"
							><path
								fill="currentColor"
								d="m12 14.122 5.303 5.303a1.5 1.5 0 0 0 2.122-2.122L14.12 12l5.304-5.303a1.5 1.5 0 1 0-2.122-2.121L12 9.879 6.697 4.576a1.5 1.5 0 1 0-2.122 2.12L9.88 12l-5.304 5.304a1.5 1.5 0 1 0 2.122 2.12z"
							/></svg
						>
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
						<svg width="16" height="16" viewBox="0 0 24 24" fill="none"
							><path
								fill="currentColor"
								d="m12 14.122 5.303 5.303a1.5 1.5 0 0 0 2.122-2.122L14.12 12l5.304-5.303a1.5 1.5 0 1 0-2.122-2.121L12 9.879 6.697 4.576a1.5 1.5 0 1 0-2.122 2.12L9.88 12l-5.304 5.304a1.5 1.5 0 1 0 2.122 2.12z"
							/></svg
						>
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

		<!-- time -->
		<div class="flex items-center justify-between gap-2 px-4 py-3">
			<span class="text-[12px] text-muted-foreground">Time</span>
			<Input
				type="time"
				bind:value={selectedTime}
				class="h-8 w-28 rounded-lg border-border bg-muted/30 px-2.5 text-[12px] text-foreground shadow-none"
				aria-label="Reminder time"
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
