<script lang="ts">
	import { confirmStore } from "$shared/stores/confirm.svelte";
	import Button from "./Button.svelte";
	import Dialog from "./Dialog.svelte";
	import Input from "./Input.svelte";

	let value = $state("");
	const req = $derived(confirmStore.current);
	$effect(() => {
		value = req?.input?.value ?? "";
	});
	function ok() {
		if (!req) return;
		if (req.input) {
			if (value.trim()) confirmStore.close(value.trim());
		} else confirmStore.close(true);
	}
</script>

{#if req}
	<Dialog open title={req.title} description={req.message} size="sm" onclose={() => confirmStore.close(null)}>
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
			<Button variant="ghost" onclick={() => confirmStore.close(null)}>취소</Button>
			<Button variant={req.danger ? "danger" : "primary"} onclick={ok}>{req.confirmLabel ?? "확인"}</Button>
		{/snippet}
	</Dialog>
{/if}
