<script lang="ts">
	// 충돌 항목 고르기 (CLAUDE.md 2-E): 양쪽에서 고친 것은 나란히 보고 하나를 고르고,
	// 넣지 못한 받은 행은 보고 버리고, 분기 대화는 열어 보고 확인함
	import { onMount } from "svelte";
	import { _ } from "svelte-i18n";
	import type { SyncConflict } from "$ipc/bindings";
	import { Badge, Button, Dialog, Spinner } from "$shared/ui";
	import { call, commands } from "$shared/ipc/client";
	import { toasts } from "$shared/stores/toast.svelte";
	import { absolute, relative } from "$shared/time";
	import { nav } from "$app/nav.svelte";
	import { library } from "$features/character/library.svelte";
	import Group from "./Group.svelte";
	import Row from "./Row.svelte";
	import { deviceLabel } from "./deviceLabel";
	import type { SyncPeer } from "$ipc/bindings";

	interface Props {
		/** 고른 뒤 (남은 수를 다시 읽게) */
		onchange?: () => unknown;
		/** 이름을 아는 기기들 (기기 id 대신 이름으로, 5-E-16-1) */
		peers?: readonly SyncPeer[];
	}
	let { onchange, peers = [] }: Props = $props();

	const src = { source: "sync" };
	let items = $state<SyncConflict[] | null>(null);
	let busy = $state<string | null>(null);

	/** 한쪽이 비어 있는 것과 같은 값: 비어 있음끼리는 다른 것이 아니다 (null · "" · [] · {}) */
	const emptyish = (v: unknown) => v === null || v === undefined || v === "" || v === "[]" || v === "{}" || (Array.isArray(v) && v.length === 0);
	/** 진짜 다른 칸만 (양쪽이 다 비어 있는 칸은 충돌이 아니라 표기 차이) */
	const realDiff = (c: SyncConflict) => c.diff.filter((d) => !(emptyish(d.local) && emptyish(d.remote)));
	/** 사람이 고를 것이 없는 충돌: 칸이 모두 비어 있음끼리라 같은 내용 */
	const phantom = (c: SyncConflict) => c.kind === "edit" && realDiff(c).length === 0;
	const visible = $derived((items ?? []).filter((c) => !phantom(c)));

	onMount(load);
	async function load() {
		items = await call(commands.listSyncConflicts(), src).catch(() => []);
		// 같은 내용인 충돌 기록은 고를 것이 없으니 조용히 치운다
		for (const c of items.filter(phantom)) await call(commands.resolveSyncConflict(c.id, "dismiss"), { ...src, silent: true }).catch(() => {});
	}
	let open = $state(false);
	$effect(() => {
		if (items !== null && visible.length === 0) open = false;
	});

	const TABLE = $derived<Record<string, string>>({
		characters: $_("settings.dataSec.counts.characters"),
		personas: $_("settings.dataSec.counts.personas"),
		conversations: $_("settings.dataSec.counts.conversations"),
		messages: $_("settings.dataSec.counts.messages"),
		worldinfo_books: $_("settings.dataSec.counts.worldInfoBooks"),
		worldinfo_entries: $_("settings.dataSec.counts.worldInfoBooks"),
		prompt_presets: $_("settings.dataSec.counts.promptPresets"),
		prompt_blocks: $_("settings.dataSec.counts.promptPresets"),
		regex_scripts: $_("settings.dataSec.counts.regexScripts"),
		game_projects: $_("settings.dataSec.counts.games"),
		phone_contacts: $_("settings.dataSec.phoneBadge"),
		api_profiles: $_("settings.more3.apiKeysLabel"),
	});
	const KIND = $derived<Record<string, string>>({
		edit: $_("settings.more3.conflictKindEdit"),
		unique: $_("settings.more3.conflictKindUnique"),
		orphan: $_("settings.more3.conflictKindOrphan"),
		branch: $_("settings.more3.conflictKindBranch"),
	});
	const show = (v: unknown) => {
		if (emptyish(v)) return $_("settings.more3.conflictEmptyVal");
		const s = typeof v === "string" ? v : JSON.stringify(v);
		return s.length > 400 ? `${s.slice(0, 400)}…` : s;
	};

	async function choose(c: SyncConflict, choice: "keep_local" | "use_remote" | "dismiss") {
		busy = c.id;
		try {
			await call(commands.resolveSyncConflict(c.id, choice), src);
			if (choice !== "dismiss") toasts.success(choice === "keep_local" ? $_("settings.more3.conflictKeepLocalToast") : $_("settings.more3.conflictUseRemoteToast"));
			if (choice === "use_remote") void library.load();
			await load();
			await onchange?.();
		} finally {
			busy = null;
		}
	}
	function openBranch(c: SyncConflict) {
		const id = (c.remote as { branchId?: string } | null)?.branchId;
		const conv = library.conversations.find((x) => x.id === id);
		if (!conv) return toasts.error($_("settings.more3.conflictBranchMissing"));
		nav.go({ page: "character", charId: conv.characterId, convId: conv.id });
	}
</script>

{#if items === null}
	<div class="loading"><Spinner /></div>
{:else if visible.length}
	<Group title={$_("settings.more3.conflictTitle")}>
		<Row label={$_("settings.more3.conflictRowLabel", { values: { count: visible.length } })} desc={$_("settings.more3.conflictRowDesc")} nowrap>
			<Button size="sm" variant="primary" onclick={() => (open = true)}>{$_("settings.more3.conflictReviewBtn")}</Button>
		</Row>
	</Group>
	<Dialog bind:open title={$_("settings.more3.conflictTitle")} description={$_("settings.more3.conflictDialogDesc")} size="lg">
		<ul class="list">
			{#each visible as c (c.id)}
				<li>
					<div class="head">
						<Badge>{TABLE[c.tbl] ?? c.tbl}</Badge>
						<b>{c.label || $_("settings.more3.conflictUnnamed")}</b>
						<span class="meta" title={absolute(c.createdAt)}>{KIND[c.kind] ?? c.kind} · {$_("settings.syncSec.deviceBadge", { values: { name: deviceLabel(c.deviceId, peers) } })} · {relative(c.createdAt)}</span>
					</div>
					{#if c.kind === "edit"}
						{#if realDiff(c).length}
							<table class="diff">
								<thead><tr><th scope="col">{$_("settings.more3.conflictColField")}</th><th scope="col">{$_("settings.more3.conflictColLocal")}</th><th scope="col">{$_("settings.more3.conflictColRemote")}</th></tr></thead>
								<tbody>
									{#each realDiff(c) as d (d.column)}
										<tr><th scope="row">{d.column}</th><td>{show(d.local)}</td><td>{show(d.remote)}</td></tr>
									{/each}
								</tbody>
							</table>
						{:else}
							<p class="meta">{$_("settings.more3.conflictSameNow")}</p>
						{/if}
						<div class="acts">
							<Button size="sm" disabled={busy !== null} onclick={() => choose(c, "use_remote")}>{$_("settings.more3.conflictUseRemoteBtn")}</Button>
							<Button size="sm" variant="primary" loading={busy === c.id} disabled={busy !== null} onclick={() => choose(c, "keep_local")}>{$_("settings.more3.conflictKeepLocalBtn")}</Button>
						</div>
					{:else if c.kind === "branch"}
						<p class="meta">{$_("settings.more3.conflictBranchHint")}</p>
						<div class="acts">
							<Button size="sm" disabled={busy !== null} onclick={() => openBranch(c)}>{$_("settings.more3.conflictOpenBranchBtn")}</Button>
							<Button size="sm" variant="primary" loading={busy === c.id} disabled={busy !== null} onclick={() => choose(c, "dismiss")}>{$_("settings.more3.conflictAckBtn")}</Button>
						</div>
					{:else}
						<details>
							<summary>{$_("settings.more3.conflictViewRemote")}</summary>
							<pre>{JSON.stringify(c.remote, null, 1)}</pre>
						</details>
						<div class="acts">
							<Button size="sm" variant="ghost" loading={busy === c.id} disabled={busy !== null} onclick={() => choose(c, "dismiss")}>{$_("settings.more3.discardBtn")}</Button>
						</div>
					{/if}
				</li>
			{/each}
		</ul>
	</Dialog>
{/if}

<style>
	.loading {
		display: flex;
		justify-content: center;
		padding: var(--space-4);
	}
	.list {
		list-style: none;
		margin: 0;
		padding: 0;
	}
	.list li {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		padding: var(--space-3) 0;
		border-top: 1px solid var(--border);
	}
	.head {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		flex-wrap: wrap;
	}
	.meta {
		color: var(--text-muted);
		font-size: var(--text-xs);
		margin: 0;
	}
	.diff {
		width: 100%;
		border-collapse: collapse;
		font-size: var(--text-sm);
		table-layout: fixed;
	}
	.diff th,
	.diff td {
		text-align: left;
		vertical-align: top;
		padding: var(--space-1) var(--space-2);
		border: 1px solid var(--border);
		overflow-wrap: anywhere;
		white-space: pre-wrap;
	}
	.diff thead th {
		color: var(--text-muted);
		font-weight: 500;
	}
	.diff tbody th {
		width: 6.5rem;
		color: var(--text-muted);
		font-weight: 500;
	}
	pre {
		margin: var(--space-1) 0 0;
		max-height: 12rem;
		overflow: auto;
		font-size: var(--text-xs);
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}
	details summary {
		cursor: pointer;
		font-size: var(--text-sm);
	}
	.acts {
		display: flex;
		justify-content: flex-end;
		gap: var(--space-2);
		flex-wrap: wrap;
	}
</style>
