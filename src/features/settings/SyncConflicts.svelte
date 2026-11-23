<script lang="ts">
	// 충돌 항목 고르기 (CLAUDE.md 2-E): 양쪽에서 고친 것은 나란히 보고 하나를 고르고,
	// 넣지 못한 받은 행은 보고 버리고, 분기 대화는 열어 보고 확인함
	import { onMount } from "svelte";
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

	const TABLE: Record<string, string> = {
		characters: "캐릭터",
		personas: "페르소나",
		conversations: "대화",
		messages: "메시지",
		worldinfo_books: "로어북",
		worldinfo_entries: "로어북 항목",
		prompt_presets: "프리셋",
		prompt_blocks: "프리셋 블록",
		regex_scripts: "정규식",
		game_projects: "게임",
		phone_contacts: "연락처",
		api_profiles: "API 연결",
	};
	const COLUMN: Record<string, string> = {
		name: "이름",
		title: "제목",
		description: "설명",
		content: "내용",
		first_message: "첫 메시지",
		system_prompt_override: "시스템 프롬프트",
		example_dialogue: "대화 예시",
		tags: "태그",
		meta: "메타",
		pinned: "고정",
		favorite: "즐겨찾기",
		keys: "키워드",
		enabled: "켜짐",
		sort_order: "순서",
	};
	const KIND: Record<string, string> = {
		edit: "양쪽에서 고침",
		unique: "넣지 못함 (같은 이름·키가 있음)",
		orphan: "넣지 못함 (가리키는 항목이 없음)",
		branch: "같은 대화에 양쪽이 이어 씀",
	};
	const show = (v: unknown) => {
		if (emptyish(v)) return "(비어 있음)";
		const s = typeof v === "string" ? v : JSON.stringify(v);
		return s.length > 400 ? `${s.slice(0, 400)}…` : s;
	};

	async function choose(c: SyncConflict, choice: "keep_local" | "use_remote" | "dismiss") {
		busy = c.id;
		try {
			await call(commands.resolveSyncConflict(c.id, choice), src);
			if (choice !== "dismiss") toasts.success(choice === "keep_local" ? "이 기기 것을 둠 — 다음 동기화 때 다른 기기도 이것으로" : "받은 것으로 바꿈 — 다음 동기화 때 다른 기기도 같아짐");
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
		if (!conv) return toasts.error("분기 대화를 찾을 수 없음 (지웠을 수 있음)");
		nav.go({ page: "character", charId: conv.characterId, convId: conv.id });
	}
</script>

{#if items === null}
	<div class="loading"><Spinner /></div>
{:else if visible.length}
	<Group title="충돌 항목">
		<Row label="충돌 {visible.length}건" desc="양쪽에서 고쳤거나 합치지 못한 항목. 확인하고 하나씩 고름" nowrap>
			<Button size="sm" variant="primary" onclick={() => (open = true)}>확인하기</Button>
		</Row>
	</Group>
	<Dialog bind:open title="충돌 항목" description="이 기기 것을 둘지 받은 것으로 바꿀지 고르세요. 고르면 다음 동기화 때 다른 기기도 같아집니다" size="lg">
		<ul class="list">
			{#each visible as c (c.id)}
				<li>
					<div class="head">
						<Badge>{TABLE[c.tbl] ?? c.tbl}</Badge>
						<b>{c.label || "(이름 없음)"}</b>
						<span class="meta" title={absolute(c.createdAt)}>{KIND[c.kind] ?? c.kind} · 기기 {deviceLabel(c.deviceId, peers)} · {relative(c.createdAt)}</span>
					</div>
					{#if c.kind === "edit"}
						{#if realDiff(c).length}
							<table class="diff">
								<thead><tr><th scope="col">칸</th><th scope="col">이 기기</th><th scope="col">받은 것</th></tr></thead>
								<tbody>
									{#each realDiff(c) as d (d.column)}
										<tr><th scope="row">{COLUMN[d.column] ?? d.column}</th><td>{show(d.local)}</td><td>{show(d.remote)}</td></tr>
									{/each}
								</tbody>
							</table>
						{:else}
							<p class="meta">지금은 내용이 같음</p>
						{/if}
						<div class="acts">
							<Button size="sm" disabled={busy !== null} onclick={() => choose(c, "use_remote")}>받은 것으로</Button>
							<Button size="sm" variant="primary" loading={busy === c.id} disabled={busy !== null} onclick={() => choose(c, "keep_local")}>이 기기 것 두기</Button>
						</div>
					{:else if c.kind === "branch"}
						<p class="meta">이 기기 대화는 그대로이고, 받은 메시지는 "… · 다른 기기" 분기 대화에 있음. 필요 없으면 그 대화를 지우면 됨</p>
						<div class="acts">
							<Button size="sm" disabled={busy !== null} onclick={() => openBranch(c)}>분기 대화 열기</Button>
							<Button size="sm" variant="primary" loading={busy === c.id} disabled={busy !== null} onclick={() => choose(c, "dismiss")}>확인함</Button>
						</div>
					{:else}
						<details>
							<summary>받은 내용 보기</summary>
							<pre>{JSON.stringify(c.remote, null, 1)}</pre>
						</details>
						<div class="acts">
							<Button size="sm" variant="ghost" loading={busy === c.id} disabled={busy !== null} onclick={() => choose(c, "dismiss")}>버리기</Button>
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
