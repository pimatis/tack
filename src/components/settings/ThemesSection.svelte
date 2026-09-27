<script lang="ts">
	import { colorPresets } from '$lib/theme/presets';
	import type { Settings } from '$lib/types/settings';

	let {
		settings,
		update
	}: {
		settings: Settings;
		update: <K extends keyof Settings>(key: K, value: Settings[K]) => void;
	} = $props();
</script>

<div class="flex flex-col gap-0.5">
	<p class="text-[13px] font-medium">Color theme</p>
	<p class="text-xs text-muted-foreground">
		Pick a palette. Every theme adapts to both dark and light.
	</p>
</div>

<div class="mt-3 grid grid-cols-2 gap-2 sm:grid-cols-3 md:grid-cols-4">
	{#each colorPresets as preset (preset.id)}
		{@const active = settings.themePreset === preset.id}
		<button
			type="button"
			onclick={() => update('themePreset', preset.id)}
			aria-pressed={active}
			title={preset.label}
			class="flex flex-col items-center gap-2 rounded-xl border p-3 transition-colors {active
				? 'border-primary/50 bg-primary/5'
				: 'border-border hover:bg-muted/40'}"
		>
			<!-- light -> dark gradient so one swatch shows the whole palette -->
			<span
				class="size-10 rounded-full ring-1 ring-foreground/10"
				style="background-image: linear-gradient(135deg, {preset.light.background} 0%, {preset.light
					.primary} 48%, {preset.dark.primary} 52%, {preset.dark.background} 100%);"
			></span>
			<span class="text-[12px] font-medium {active ? 'text-foreground' : 'text-muted-foreground'}"
				>{preset.label}</span
			>
		</button>
	{/each}
</div>
