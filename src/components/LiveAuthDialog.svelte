<script lang="ts">
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { completeLiveAuth, loginLive } from '$lib/live/auth.service';

	let open = $state(true);
	let password = $state('');
	let error = $state('');
	let busy = $state(false);

	// ironic replies for closing attempts; the dialog never actually closes
	// while the share is locked
	const eggs = [
		'You clicked close with the confidence of someone who definitely knows the password. Adorable. 😏',
		'That was a brave attempt. The dialog is still here, quietly judging both of us.',
		'You can close the window, but the password will still be waiting for you emotionally.',
		'Nice try. The tasks are not hiding behind this dialog. They are hiding behind your memory.',
		'DevTools can remove the dialog, but sadly it cannot remember passwords for you.',
		'The server has receipts, the dialog has patience, and you have one more chance.',
		'You are doing great. Not unlocking anything yet, but emotionally, great progress.',
		'Closing this again will not make the password appear. I checked. Twice. 🎉',
		'Fun fact: this button has never unlocked anything. It is mostly here for morale.',
		'At this point, we are both pretending this strategy is going somewhere.'
	];
	let eggIndex = $state(-1);
	let eggTimeout: ReturnType<typeof setTimeout> | undefined;

	function resistClose() {
		if (eggTimeout) clearTimeout(eggTimeout);
		eggIndex = (eggIndex + 1) % eggs.length;
		open = true;
		eggTimeout = setTimeout(() => {
			eggIndex = -1;
			eggTimeout = undefined;
		}, 5000);
	}

	async function submit() {
		if (!password || busy) return;
		busy = true;
		error = '';
		const ok = await loginLive(password);
		busy = false;
		if (!ok) {
			error = 'Wrong password';
			return;
		}
		// the session cookie is set; a fresh load re-runs every query and
		// reconnects the event stream with it
		completeLiveAuth();
		location.reload();
	}
</script>

<Dialog.Root bind:open onOpenChange={(v) => (v ? undefined : resistClose())}>
	<Dialog.Content class="w-[calc(100vw-2rem)] max-w-sm gap-4 p-4 sm:p-6">
		<Dialog.Header class="gap-1.5">
			<Dialog.Title>Password required</Dialog.Title>
			<Dialog.Description>
				This shared workspace is protected. Enter the password to continue.
			</Dialog.Description>
		</Dialog.Header>
		<form
			class="flex flex-col gap-3"
			onsubmit={(e) => {
				e.preventDefault();
				void submit();
			}}
		>
			<Input
				type="password"
				placeholder="Password"
				autocomplete="current-password"
				bind:value={password}
				disabled={busy}
			/>
			{#if error}
				<p class="text-xs text-destructive">{error}</p>
			{/if}
			{#if eggIndex >= 0}
				<p class="text-xs text-muted-foreground italic">{eggs[eggIndex]}</p>
			{/if}
			<Dialog.Footer>
				<Button type="submit" disabled={busy || !password}>Unlock</Button>
			</Dialog.Footer>
		</form>
	</Dialog.Content>
</Dialog.Root>
