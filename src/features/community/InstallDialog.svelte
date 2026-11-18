<script lang="ts">
	// 설치 미리보기 (CLAUDE.md 6-D-8): 아무것도 쓰기 전에 무엇을 받는지 보여 준다. 설치해도 꺼진 채로 둔다 (켜기는 따로 동의)
	import { TriangleAlert } from "@lucide/svelte";
	import type { PluginPreview } from "$ipc/bindings";
	import { Badge, Button, Dialog } from "$shared/ui";
	import PermissionList from "./PermissionList.svelte";

	let {
		open = $bindable(false),
		target,
		busy = false,
		oninstall,
	}: { open?: boolean; target: { path: string; preview: PluginPreview } | null; busy?: boolean; oninstall: () => void } = $props();

	const p = $derived(target?.preview ?? null);
	const m = $derived(p?.manifest ?? null);
	const update = $derived(!!p?.existing);
	const size = (b: number) => (b >= 1024 * 1024 ? `${(b / 1024 / 1024).toFixed(1)}MB` : `${Math.max(1, Math.round(b / 1024))}KB`);
</script>

<Dialog bind:open title={update ? "플러그인 업데이트" : "플러그인 설치"} description={update ? "같은 id의 플러그인이 이미 있어 파일을 바꿉니다" : "설치해도 꺼진 채로 둡니다. 켤 때 권한에 따로 동의합니다"} size="md">
	{#if p && m}
		<div class="warn">
			<TriangleAlert />
			<span>플러그인은 남이 만든 코드입니다. <b>믿을 수 있는 곳에서 받은 것만</b> 설치하세요. 격리된 곳에서 허락한 권한만으로 돌지만, 허락한 일은 할 수 있습니다.</span>
		</div>

		<section class="info">
			<h3>{m.name} <small>{m.id}</small></h3>
			<p class="meta">
				버전 {m.version}{m.author ? ` · ${m.author}` : ""} · 파일 {p.files}개 · {size(p.size)} · 코드 지문 <code>{p.hash.slice(0, 12)}</code>
			</p>
			{#if m.description}<p>{m.description}</p>{/if}
			{#if update && p.existing}
				<p class="upd">
					지금 설치된 버전 {p.existing.version || "?"}{p.existing.enabled ? " (켜져 있음)" : ""} → {m.version}
					{#if p.downgrade}<Badge tone="warning">더 낮은 버전</Badge>{/if}
				</p>
				{#if p.addedPermissions.length}
					<p class="upd danger"><b>권한이 늘었습니다.</b> 설치하면 이 플러그인은 꺼지고, 다시 켤 때 늘어난 권한까지 동의해야 합니다.</p>
				{/if}
			{/if}
		</section>

		<h4>요청하는 권한</h4>
		<PermissionList perms={m.permissions ?? []} added={update ? p.addedPermissions : []} />
	{/if}

	{#snippet footer()}
		<Button variant="ghost" onclick={() => (open = false)}>취소</Button>
		<Button variant="primary" loading={busy} onclick={oninstall}>{update ? "업데이트" : "설치 (꺼진 채로)"}</Button>
	{/snippet}
</Dialog>

<style>
	.warn {
		display: flex;
		gap: var(--space-2);
		align-items: flex-start;
		padding: var(--space-3);
		border-radius: var(--radius-sm);
		background: var(--warning-soft);
		font-size: var(--text-sm);
		margin-bottom: var(--space-3);
	}
	.warn :global(svg) {
		flex: none;
		color: var(--warning);
		width: 18px;
		height: 18px;
	}
	h3 {
		margin: 0;
		font-size: var(--text-lg);
	}
	h3 small {
		font-weight: 400;
		color: var(--text-faint);
		font-family: var(--font-mono);
		font-size: var(--text-xs);
	}
	h4 {
		margin: var(--space-3) 0 var(--space-2);
		font-size: var(--text-md);
	}
	.info p {
		margin: var(--space-1) 0;
		font-size: var(--text-sm);
	}
	.meta {
		color: var(--text-muted);
	}
	code {
		font-family: var(--font-mono);
	}
	.upd {
		color: var(--text-muted);
	}
	.upd.danger {
		color: var(--danger);
	}
</style>
