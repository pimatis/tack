<script lang="ts">
	import { today, getLocalTimeZone, parseDate, type DateValue } from '@internationalized/date';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import { Calendar } from '$lib/components/ui/calendar/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Separator } from '$lib/components/ui/separator/index.js';

	type Props = {
		value: string;
		title?: string;
		onSelect: (date: string) => void;
		onClear?: () => void;
	};

	let { value, title = 'Due date', onSelect, onClear }: Props = $props();
	let open = $state(false);

	let selectedDate = $state<DateValue | undefined>(undefined);

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
		return new Intl.DateTimeFormat(undefined, { month: 'short', day: 'numeric' }).format(d);
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
				{...props}
				variant="outline"
				size="sm"
				class="flex h-8 w-full items-center justify-start gap-1.5 rounded-lg border-border bg-muted/30 px-2.5 text-[12px] font-normal shadow-none transition-colors hover:bg-muted/50"
			>
				<svg class="text-muted-foreground" width="13" height="13" viewBox="0 0 24 24" fill="none"
					><path
						fill="currentColor"
						d="M16 3a1 1 0 0 1 1 1v1h2a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7a2 2 0 0 1 2-2h2V4a1 1 0 0 1 2 0v1h6V4a1 1 0 0 1 1-3M8.01 16H8a1 1 0 0 0-.117 1.993L8.01 18a1 1 0 1 0 0-2m4 0H12a1 1 0 0 0-.117 1.993l.127.007a1 1 0 1 0 0-2m4 0H16a1 1 0 0 0-.117 1.993l.127.007a1 1 0 1 0 0-2m-8-4H8a1 1 0 0 0-.117 1.993L8.01 14a1 1 0 1 0 0-2m4 0H12a1 1 0 0 0-.117 1.993l.127.007a1 1 0 1 0 0-2m4 0H16a1 1 0 0 0-.117 1.993l.127.007a1 1 0 1 0 0-2M19 7H5v2h14z"
					/></svg
				>
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
	<Dialog.Content class="max-w-auto w-fit gap-0 p-0" showCloseButton={false}>
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
		<div class="flex items-center gap-1.5 px-4 pb-3">
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
				class="rounded-lg"
			/>
		</div>

		<Separator />

		<!-- footer -->
		<div class="flex items-center justify-between gap-2 px-4 py-3">
			<Button type="button" variant="ghost" size="sm" onclick={handleClear} disabled={!value}>
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
