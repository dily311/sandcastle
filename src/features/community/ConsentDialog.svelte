<script lang="ts">
	// 켜기 동의 (CLAUDE.md 6-D-8): 플러그인이 요청한 권한 전부를 보고 한 번에 동의해야 켜진다 (일부만 허락하는 것은 없음)
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

<Dialog bind:open title={`'${plugin?.manifest?.name ?? ""}' (${plugin?.id ?? ""}) 켜기`} description="켜면 이 플러그인의 코드가 실행됩니다. 아래 권한을 모두 허락하는 것입니다 — 일부만 허락할 수는 없습니다" size="md">
	{#if plugin?.codeChanged}
		<p class="note warn">이 플러그인의 파일이 처음 켤 때와 달라졌습니다. 직접 고친 것이 아니라면 켜지 마세요.</p>
	{/if}
	<PermissionList {perms} {added} />
	{#if risk === "high"}
		<p class="note">'주의' 권한이 있습니다. AI 호출은 요금이 들 수 있고, 대화 읽기는 대화 내용을 플러그인이 본다는 뜻입니다.</p>
	{/if}
	{#snippet footer()}
		<Button variant="ghost" onclick={() => (open = false)}>취소</Button>
		<Button variant="primary" loading={busy} onclick={onconfirm}>허락하고 켜기</Button>
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
