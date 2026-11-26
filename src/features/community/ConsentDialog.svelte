<script lang="ts">
	// 켜기 동의 (CLAUDE.md 6-D-8): 플러그인이 요청한 권한 전부를 보고 한 번에 동의해야 켜진다 (일부만 허락하는 것은 없음)
	import { _ } from "svelte-i18n";
	import type { UserPlugin_Serialize as UserPlugin } from "$ipc/bindings";
	import { Button, Dialog } from "$shared/ui";
	import PermissionList from "./PermissionList.svelte";
	import { highestRisk } from "./permissions";

	let {
		open = $bindable(false),
		plugin,
		busy = false,
		onconfirm,
	}: { open?: boolean; plugin: UserPlugin | null; busy?: boolean; onconfirm: () => void } = $props();

	const perms = $derived(plugin?.manifest?.permissions ?? []);
	/** 이미 허락한 것보다 늘어난 권한 (업데이트로 늘었을 때 강조) */
	const added = $derived(perms.filter((p) => !(plugin?.granted ?? []).includes(p)).filter(() => (plugin?.granted.length ?? 0) > 0));
	const risk = $derived(highestRisk(perms));
</script>

<Dialog bind:open title={$_("communityUi.consentTitle", { values: { name: plugin?.manifest?.name ?? "", id: plugin?.id ?? "" } })} description={$_("communityUi.consentDesc")} size="md">
	{#if plugin?.codeChanged}
		<p class="note warn">{$_("communityUi.consentCodeChanged")}</p>
	{/if}
	<PermissionList {perms} {added} />
	{#if risk === "high"}
		<p class="note">{$_("communityUi.consentHighRiskNote")}</p>
	{/if}
	{#snippet footer()}
		<Button variant="ghost" onclick={() => (open = false)}>{$_("common.cancel")}</Button>
		<Button variant="primary" loading={busy} onclick={onconfirm}>{$_("communityUi.consentConfirm")}</Button>
	{/snippet}
</Dialog>

<style>
	.note {
		margin: var(--space-3) 0 0;
		font-size: var(--text-sm);
		color: var(--text-muted);
	}
	.note.warn {
		margin: 0 0 var(--space-3);
		color: var(--danger);
	}
</style>
