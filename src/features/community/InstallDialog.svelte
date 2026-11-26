<script lang="ts">
	// 설치 미리보기 (CLAUDE.md 6-D-8): 아무것도 쓰기 전에 무엇을 받는지 보여 준다. 설치해도 꺼진 채로 둔다 (켜기는 따로 동의)
	import { _ } from "svelte-i18n";
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

<Dialog bind:open title={update ? $_("communityUi.installTitleUpdate") : $_("communityUi.installTitleNew")} description={update ? $_("communityUi.installDescUpdate") : $_("communityUi.installDescNew")} size="md">
	{#if p && m}
		<div class="warn">
			<TriangleAlert />
			<span>{$_("communityUi.installWarnLead")} <b>{$_("communityUi.installWarnBold")}</b> {$_("communityUi.installWarnTail")}</span>
		</div>

		<section class="info">
			<h3>{m.name} <small>{m.id}</small></h3>
			<p class="meta">
				{$_("communityUi.installMeta", { values: { version: m.version, author: m.author ? ` · ${m.author}` : "", files: p.files, size: size(p.size) } })} <code>{p.hash.slice(0, 12)}</code>
			</p>
			{#if m.description}<p>{m.description}</p>{/if}
			{#if update && p.existing}
				<p class="upd">
					{$_("communityUi.installCurrentVer", { values: { oldVer: p.existing.version || "?", enabled: p.existing.enabled ? $_("communityUi.installEnabledSuffix") : "", newVer: m.version } })}
					{#if p.downgrade}<Badge tone="warning">{$_("communityUi.installDowngrade")}</Badge>{/if}
				</p>
				{#if p.addedPermissions.length}
					<p class="upd danger"><b>{$_("communityUi.installPermsAddedBold")}</b> {$_("communityUi.installPermsAddedTail")}</p>
				{/if}
			{/if}
		</section>

		<h4>{$_("communityUi.installRequestedPerms")}</h4>
		<PermissionList perms={m.permissions ?? []} added={update ? p.addedPermissions : []} />
	{/if}

	{#snippet footer()}
		<Button variant="ghost" onclick={() => (open = false)}>{$_("common.cancel")}</Button>
		<Button variant="primary" loading={busy} onclick={oninstall}>{update ? $_("communityUi.installBtnUpdate") : $_("communityUi.installBtnNew")}</Button>
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
