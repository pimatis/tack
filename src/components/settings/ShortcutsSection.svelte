<script lang="ts">
	import * as Card from '$lib/components/ui/card/index.js';
	import { SHORTCUTS, shortcutLabel } from '$lib/shortcuts/index.js';

	const shortcuts = SHORTCUTS;
</script>

{#each shortcuts as s (s.id)}
	<Card.Root size="sm">
		<Card.Content class="flex items-center justify-between">
			<span class="text-[13px] text-muted-foreground">{s.label}</span>
			<kbd
				class="rounded-md bg-foreground px-2 py-0.5 text-[11px] font-medium text-background"
				>{shortcutLabel(s)}</kbd
			>
		</Card.Content>
	</Card.Root>
{/each}
