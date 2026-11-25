<script lang="ts">
	import { _ } from "svelte-i18n";
	import { confirmStore } from "$shared/stores/confirm.svelte";
	import Button from "./Button.svelte";
	import Dialog from "./Dialog.svelte";
	import Input from "./Input.svelte";

	let value = $state("");
	let picked = $state<string[]>([]);
	const req = $derived(confirmStore.current);
	$effect(() => {
		value = req?.input?.value ?? "";
		picked = [];
	});
	function ok() {
		if (!req) return;
		if (req.choices && req.multiple) {
			confirmStore.close(JSON.stringify(picked));
		} else if (req.input) {
			if (value.trim()) confirmStore.close(value.trim());
		} else confirmStore.close(true);
	}
</script>

{#if req}
	<Dialog open title={req.title} description={req.message} size="sm" onclose={() => confirmStore.close(null)}>
		{#if req.choices && !req.multiple}
			<div class="choices">
				{#each req.choices as c, i (i)}
					<Button variant="ghost" onclick={() => confirmStore.close(c)}>{c}</Button>
				{/each}
			</div>
		{:else if req.choices}
			<div class="choices">
				{#each req.choices as c, i (i)}
					<label class="pick"><input type="checkbox" checked={picked.includes(c)} onchange={(e) => (picked = e.currentTarget.checked ? [...picked, c] : picked.filter((x) => x !== c))} />{c}</label>
				{/each}
			</div>
		{/if}
		{#if req.input}
			<form
				onsubmit={(e) => {
					e.preventDefault();
					ok();
				}}
			>
				<!-- svelte-ignore a11y_autofocus -->
				<Input bind:value placeholder={req.input.placeholder} aria-label={req.title} autofocus />
			</form>
		{/if}
		{#snippet footer()}
			{#if !req.hideCancel}<Button variant="ghost" onclick={() => confirmStore.close(null)}>{req.cancelLabel ?? $_("common.cancel")}</Button>{/if}
			{#if !req.choices || req.multiple}<Button variant={req.danger ? "danger" : "primary"} onclick={ok}>{req.confirmLabel ?? $_("common.confirm")}</Button>{/if}
		{/snippet}
	</Dialog>
{/if}

<style>
	.choices {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		max-height: 50vh;
		overflow-y: auto;
	}
	.choices :global(button) {
		justify-content: flex-start;
	}
	.pick {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-1) var(--space-2);
		font-size: var(--text-sm);
		cursor: pointer;
	}
</style>
