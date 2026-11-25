<script lang="ts">
	// 휴지통 (CLAUDE.md 2-A-3): 지운 항목 되살리기 / 완전히 지우기 / 보관 기간
	import { jobs } from "$shared/stores/jobs.svelte";
	import { onMount } from "svelte";
	import { _ } from "svelte-i18n";
	import { RotateCcw, Trash2 } from "@lucide/svelte";
	import type { TrashItem } from "$ipc/bindings";
	import { Badge, Button, Select, Spinner } from "$shared/ui";
	import { call, commands } from "$shared/ipc/client";
	import { appSetting } from "$shared/stores/appSetting.svelte";
	import { confirmAction } from "$shared/stores/confirm.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	import { absolute, relative } from "$shared/time";
	import { restoreFromTrash, TRASH_KIND_LABEL } from "$shared/trash";
	import Group from "./Group.svelte";
	import Row from "./Row.svelte";

	interface Props {
		/** 되살린 뒤 (다른 목록을 다시 읽게) */
		onrestored?: (item: TrashItem) => unknown;
	}
	let { onrestored }: Props = $props();

	const src = { source: "trash" };
	let items = $state<TrashItem[] | null>(null);
	let busy = $state<string | null>(null);

	onMount(load);
	async function load() {
		items = await call(commands.listTrash(), src).catch(() => []);
	}

	const RETENTION = $derived([
		{ value: "7", label: $_("settings.trash.days7") },
		{ value: "30", label: $_("settings.trash.days30") },
		{ value: "90", label: $_("settings.trash.days90") },
		{ value: "", label: $_("settings.trash.keepForever") },
	]);
	const retention = $derived(appSetting.value ? appSetting.value.trashRetentionDays : 30);
	async function setRetention(v: string) {
		const days = v ? Number(v) : null;
		if (days != null && (retention == null || days < retention)) {
			const old = items?.filter((i) => Date.now() - i.deletedAt > days * 86_400_000).length ?? 0;
			if (old) {
				const ok = await confirmAction({
					title: $_("settings.trash.retentionAria"),
					message: $_("settings.trash.retentionConfirmMsg", { values: { days, count: old } }),
					danger: true,
					confirmLabel: $_("settings.trash.retentionConfirmBtn"),
				});
				if (!ok) return;
			}
		}
		await appSetting.update({ trashRetentionDays: days });
		await load();
	}

	async function restore(item: TrashItem) {
		busy = item.id;
		try {
			const r = await restoreFromTrash(item.id);
			if (!r) return;
			toasts.success($_("settings.trash.restoredToast", { values: { label: item.label } }));
			await load();
			await onrestored?.(r);
		} finally {
			busy = null;
		}
	}
	async function purge(item: TrashItem) {
		const ok = await confirmAction({
			title: $_("settings.trash.purgeTitle"),
			message: $_("settings.trash.purgeMsg", {
				values: {
					kind: $_(`settings.trash.kind.${item.kind}`),
					label: item.label,
					extra: item.rows > 1 ? $_("settings.trash.purgeExtra", { values: { count: item.rows - 1 } }) : "",
					files: item.files ? $_("settings.trash.purgeFiles") : "",
				},
			}),
			danger: true,
			confirmLabel: $_("settings.trash.purgeTitle"),
		});
		if (!ok) return;
		await jobs.run(item.label, () => call(commands.purgeTrashItem(item.id), src), { group: $_("settings.trash.purgeTitle") }).catch(() => {});
		await load();
	}
	async function empty() {
		const n = items?.length ?? 0;
		const ok = await confirmAction({
			title: $_("settings.trash.emptyTitle"),
			message: $_("settings.trash.emptyMsg", { values: { count: n } }),
			danger: true,
			confirmLabel: $_("settings.trash.emptyConfirmBtn", { values: { count: n } }),
		});
		if (!ok) return;
		const removed = await jobs.run($_("settings.trash.emptyJob", { values: { count: n } }), () => call(commands.emptyTrash(), src), { group: $_("settings.trash.emptyTitle") });
		if (removed !== undefined) toasts.success($_("settings.trash.emptyDone", { values: { count: removed } }));
		await load();
	}
	const size = (b: number) => (b < 1024 * 1024 ? `${Math.max(1, Math.round(b / 1024))}KB` : `${(b / 1024 / 1024).toFixed(1)}MB`);
</script>

<Group title={$_("settings.trash.title")}>
	{#snippet aside()}
		{#if items?.length}<Button size="sm" variant="danger" onclick={empty}>{#snippet icon()}<Trash2 />{/snippet}{$_("settings.trash.emptyBtn")}</Button>{/if}
	{/snippet}
	<Row label={$_("settings.trash.retention")} desc={$_("settings.trash.retentionDesc")}>
		<Select value={retention == null ? "" : String(retention)} options={RETENTION} aria-label={$_("settings.trash.retentionAria")} onchange={(e) => setRetention(e.currentTarget.value)} />
	</Row>
	{#if items === null}
		<div class="loading"><Spinner /></div>
	{:else if !items.length}
		<p class="empty">{$_("settings.trash.emptyState")}</p>
	{:else}
		<ul class="items">
			{#each items as it (it.id)}
				<li>
					<Badge>{$_(`settings.trash.kind.${it.kind}`)}</Badge>
					<div class="txt">
						<b>{it.label}</b>
						<span class="meta" title={absolute(it.deletedAt)}>
							{$_("settings.trash.deletedMeta", { values: { when: relative(it.deletedAt) } })}{it.rows > 1 ? $_("settings.trash.deletedExtra", { values: { count: it.rows - 1 } }) : ""}{it.files ? $_("settings.trash.deletedFiles", { values: { count: it.files } }) : ""} · {size(it.size)}
						</span>
					</div>
					<div class="acts">
						<Button size="sm" loading={busy === it.id} disabled={busy !== null} onclick={() => restore(it)}>{#snippet icon()}<RotateCcw />{/snippet}{$_("settings.trash.restoreBtn")}</Button>
						<Button size="sm" variant="ghost" disabled={busy !== null} onclick={() => purge(it)}>{$_("settings.trash.purgeBtn")}</Button>
					</div>
				</li>
			{/each}
		</ul>
	{/if}
</Group>

<style>
	.loading {
		display: flex;
		justify-content: center;
		padding: var(--space-4);
	}
	.empty {
		margin: 0;
		padding: var(--space-3) 0;
		color: var(--text-muted);
		font-size: var(--text-sm);
	}
	.items {
		list-style: none;
		margin: 0;
		padding: 0;
		border-top: 1px solid var(--border);
	}
	.items li {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		padding: var(--space-3) 0;
		border-bottom: 1px solid var(--border);
	}
	.txt {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}
	.txt b {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.meta {
		color: var(--text-muted);
		font-size: var(--text-xs);
	}
	.acts {
		display: flex;
		gap: var(--space-1);
		flex-shrink: 0;
	}
	@media (max-width: 600px) {
		.items li {
			flex-wrap: wrap;
		}
		.acts {
			width: 100%;
			justify-content: flex-end;
		}
	}
</style>
