<script lang="ts">
	import { Input } from '$lib/components/ui/input/index.js';

	type Props = {
		// utc iso timestamp, '' when unset
		value: string;
		onSelect: (iso: string) => void;
		onClear: () => void;
	};

	let { value, onSelect, onClear }: Props = $props();

	function pad(n: number): string {
		return String(n).padStart(2, '0');
	}

	// utc iso -> local "YYYY-MM-DDTHH:mm" for the native picker
	const localValue = $derived.by(() => {
		if (!value) return '';
		const d = new Date(value);
		if (Number.isNaN(d.getTime())) return '';
		return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
	});

	function handleInput(event: Event) {
		const raw = (event.currentTarget as HTMLInputElement).value;
		if (!raw) {
			onClear();
			return;
		}
		// the native value is local time; store it as utc so comparisons work
		const d = new Date(raw);
		if (Number.isNaN(d.getTime())) return;
		onSelect(d.toISOString());
	}
</script>

<div class="flex items-center gap-1">
	<Input
		type="datetime-local"
		value={localValue}
		oninput={handleInput}
		class="h-8 flex-1 rounded-lg border-border bg-muted/30 px-2.5 text-[12px] text-foreground shadow-none"
		aria-label="Reminder"
	/>
	{#if value}
		<button
			type="button"
			class="flex size-5 shrink-0 items-center justify-center rounded text-muted-foreground/50 transition-colors hover:text-foreground"
			aria-label="Clear reminder"
			onclick={onClear}
		>
			<svg width="11" height="11" viewBox="0 0 24 24" fill="none"
				><path
					fill="currentColor"
					d="m12 14.122 5.303 5.303a1.5 1.5 0 0 0 2.122-2.122L14.12 12l5.304-5.303a1.5 1.5 0 1 0-2.122-2.121L12 9.879 6.697 4.576a1.5 1.5 0 1 0-2.122 2.12L9.88 12l-5.304 5.304a1.5 1.5 0 1 0 2.122 2.12z"
				/></svg
		>
	</button>
	{/if}
</div>
