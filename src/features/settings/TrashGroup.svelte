<script lang="ts">
	// 휴지통 (CLAUDE.md 2-A-3): 지운 항목 되살리기 / 완전히 지우기 / 보관 기간
	import { jobs } from "$shared/stores/jobs.svelte";
	import { onMount } from "svelte";
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

	const RETENTION = [
		{ value: "7", label: "7일" },
		{ value: "30", label: "30일" },
		{ value: "90", label: "90일" },
		{ value: "", label: "계속 보관" },
	];
	const retention = $derived(appSetting.value ? appSetting.value.trashRetentionDays : 30);
	async function setRetention(v: string) {
		const days = v ? Number(v) : null;
		if (days != null && (retention == null || days < retention)) {
			const old = items?.filter((i) => Date.now() - i.deletedAt > days * 86_400_000).length ?? 0;
			if (old) {
				const ok = await confirmAction({
					title: "휴지통 보관 기간",
					message: `${days}일보다 오래된 항목 ${old}개를 지금 완전히 지움. 되돌릴 수 없음`,
					danger: true,
					confirmLabel: "바꾸고 정리",
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
			toasts.success(`"${item.label}" 되살림`);
			await load();
			await onrestored?.(r);
		} finally {
			busy = null;
		}
	}
	async function purge(item: TrashItem) {
		const ok = await confirmAction({
			title: "완전히 지우기",
			message: `${TRASH_KIND_LABEL[item.kind]} "${item.label}"${item.rows > 1 ? `과 같이 지운 ${item.rows - 1}개 항목` : ""}${item.files ? "·파일" : ""}을 완전히 지움. 되돌릴 수 없음`,
			danger: true,
			confirmLabel: "완전히 지우기",
		});
		if (!ok) return;
		await jobs.run(item.label, () => call(commands.purgeTrashItem(item.id), src), { group: "완전히 지우기" }).catch(() => {});
		await load();
	}
	async function empty() {
		const n = items?.length ?? 0;
		const ok = await confirmAction({ title: "휴지통 비우기", message: `휴지통의 ${n}개 항목을 완전히 지움. 되돌릴 수 없음`, danger: true, confirmLabel: `${n}개 완전히 지우기` });
		if (!ok) return;
		const removed = await jobs.run(`${n}개`, () => call(commands.emptyTrash(), src), { group: "휴지통 비우기" });
		if (removed !== undefined) toasts.success(`${removed}개를 완전히 지움`);
		await load();
	}
	const size = (b: number) => (b < 1024 * 1024 ? `${Math.max(1, Math.round(b / 1024))}KB` : `${(b / 1024 / 1024).toFixed(1)}MB`);
</script>

<Group title="휴지통">
	{#snippet aside()}
		{#if items?.length}<Button size="sm" variant="danger" onclick={empty}>{#snippet icon()}<Trash2 />{/snippet}비우기</Button>{/if}
	{/snippet}
	<Row label="보관 기간" desc="지난 항목은 휴지통을 열거나 새로 지울 때 완전히 지움">
		<Select value={retention == null ? "" : String(retention)} options={RETENTION} aria-label="휴지통 보관 기간" onchange={(e) => setRetention(e.currentTarget.value)} />
	</Row>
	{#if items === null}
		<div class="loading"><Spinner /></div>
	{:else if !items.length}
		<p class="empty">휴지통이 비었음. 캐릭터·대화·페르소나·월드인포·프리셋·게임·플레이 기록을 지우면 여기로 옴</p>
	{:else}
		<ul class="items">
			{#each items as it (it.id)}
				<li>
					<Badge>{TRASH_KIND_LABEL[it.kind]}</Badge>
					<div class="txt">
						<b>{it.label}</b>
						<span class="meta" title={absolute(it.deletedAt)}>
							{relative(it.deletedAt)} 지움{it.rows > 1 ? ` · 함께 지운 항목 ${it.rows - 1}개` : ""}{it.files ? ` · 파일 ${it.files}` : ""} · {size(it.size)}
						</span>
					</div>
					<div class="acts">
						<Button size="sm" loading={busy === it.id} disabled={busy !== null} onclick={() => restore(it)}>{#snippet icon()}<RotateCcw />{/snippet}되살리기</Button>
						<Button size="sm" variant="ghost" disabled={busy !== null} onclick={() => purge(it)}>완전히 지우기</Button>
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
