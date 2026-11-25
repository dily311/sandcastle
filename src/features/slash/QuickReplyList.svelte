<script lang="ts">
	// 빠른 답장 검색 목록 (5-E-20-5-5): 버튼 줄의 "모두 보기"와 오른쪽 패널 "빠른 답장" 탭이 같이 쓴다.
	// 정규식 리스트와 통일된 디자인: <details> 그룹 접기, 실행(Play) 버튼 분리, 편집·삭제·순서 이동 지원
	import { onMount } from "svelte";
	import { ArrowDown, ArrowUp, ChevronRight, GripVertical, Pencil, Play, Search, Trash2 } from "@lucide/svelte";
	import { Button, Dialog, Field, IconButton, Input, Switch, Textarea } from "$shared/ui";
	import { Sortable, moveItem } from "$shared/dnd/sortable.svelte";
	import { confirmAction } from "$shared/stores/confirm.svelte";
	import type { Conversation } from "$features/character/library.svelte";
	import type { ChatSession } from "$features/character/session.svelte";
	import { buttonsOf, offIds, quickReplies, runQuickReply, setsInScope, toggleForConv, toInput, type QuickButton, type QuickReply, type QuickReplySet } from "./quickReply.svelte";
	import { slashRuns } from "./running.svelte";

	interface Props {
		session: ChatSession;
		conv: Conversation | undefined;
		getDraft: () => string;
		setDraft: (v: string) => void;
		/** 처음에 검색칸에 커서 */
		autofocus?: boolean;
		/** 하나 누른 뒤 (펼친 목록 닫기 등) */
		onran?: () => void;
	}
	let { session, conv, getDraft, setDraft, autofocus = false, onran }: Props = $props();

	onMount(() => void quickReplies.load());
	let query = $state("");
	let input: HTMLInputElement | undefined = $state();
	$effect(() => {
		if (autofocus) input?.focus();
	});
	const sets = $derived(setsInScope(conv).filter((s) => s.enabled));
	const off = $derived(offIds(conv));
	const groups = $derived(
		sets
			.map((set) => ({ set, on: !off.includes(set.id), buttons: buttonsOf([set], query) }))
			.filter((g) => !query.trim() || g.buttons.length),
	);
	const found = $derived(groups.reduce((n, g) => n + (g.on ? g.buttons.length : 0), 0));
	let running = $state(false);
	const canReorder = $derived(!query.trim());

	async function press(b: QuickButton) {
		if (running || session.busy || (!!conv && slashRuns.isRunning(conv.id))) return;
		running = true;
		try {
			await runQuickReply(b.q, b.set, session, { get: getDraft, set: setDraft });
			onran?.();
		} finally {
			running = false;
		}
	}

	// ---- 순서 이동 (드래그 + 위/아래 버튼) ----
	async function reorderSet(set: QuickReplySet, fromId: string, toId: string) {
		const snap = structuredClone($state.snapshot(set)) as QuickReplySet;
		const from = snap.items.findIndex((x) => x.id === fromId);
		const to = snap.items.findIndex((x) => x.id === toId);
		if (from < 0 || to < 0 || from === to) return;
		snap.items = moveItem(snap.items, from, to);
		await quickReplies.save(toInput(snap));
	}
	async function stepItem(set: QuickReplySet, itemId: string, dir: -1 | 1) {
		const snap = structuredClone($state.snapshot(set)) as QuickReplySet;
		const idx = snap.items.findIndex((x) => x.id === itemId);
		const next = idx + dir;
		if (idx < 0 || next < 0 || next >= snap.items.length) return;
		snap.items = moveItem(snap.items, idx, next);
		await quickReplies.save(toInput(snap));
	}
	const sorters = new Map<string, Sortable>();
	function sorterFor(set: QuickReplySet, buttons: QuickButton[]) {
		let s = sorters.get(set.id);
		if (!s) {
			s = new Sortable((from, to) => {
				const curSet = quickReplies.sets.find((x) => x.id === set.id);
				if (!curSet) return;
				const curBtns = buttonsOf([curSet], query);
				const a = curBtns[from]?.q.id;
				const b = curBtns[to]?.q.id;
				if (a && b) void reorderSet(curSet, a, b);
			});
			sorters.set(set.id, s);
		}
		s.enabled = canReorder && buttons.length > 1;
		return s;
	}

	// ---- 편집 다이얼로그 ----
	let editOpen = $state(false);
	let editing = $state<{ setId: string; itemId: string; label: string; message: string; hidden: boolean } | null>(null);
	function startEdit(set: QuickReplySet, q: QuickReply) {
		editing = { setId: set.id, itemId: q.id, label: q.label, message: q.message, hidden: q.hidden };
		editOpen = true;
	}
	async function saveEdit() {
		if (!editing) return;
		const set = quickReplies.sets.find((x) => x.id === editing!.setId);
		if (!set) return;
		const snap = structuredClone($state.snapshot(set)) as QuickReplySet;
		const target = snap.items.find((x) => x.id === editing!.itemId);
		if (!target) return;
		target.label = editing.label;
		target.message = editing.message;
		target.hidden = editing.hidden;
		await quickReplies.save(toInput(snap));
		editOpen = false;
	}

	// ---- 삭제 ----
	async function removeItem(set: QuickReplySet, q: QuickReply) {
		const name = q.label.trim() || q.message.slice(0, 16) || "빠른 답장";
		if (!(await confirmAction({ title: "빠른 답장 삭제", message: `'${name}' 항목을 '${set.name}' 세트에서 삭제함`, confirmLabel: "삭제", danger: true }))) return;
		const snap = structuredClone($state.snapshot(set)) as QuickReplySet;
		snap.items = snap.items.filter((x) => x.id !== q.id);
		await quickReplies.save(toInput(snap));
	}
</script>

<div class="qr-list">
	<label class="search">
		<Search />
		<input bind:this={input} bind:value={query} type="search" placeholder="빠른 답장 검색" aria-label="빠른 답장 검색" />
		{#if query.trim()}<small>{found}개</small>{/if}
	</label>
	{#if !sets.length}
		<p class="none">이 대화에 쓸 수 있는 세트가 없음</p>
	{:else if !groups.length}
		<p class="none">찾는 빠른 답장이 없음</p>
	{/if}
	{#each groups as g (g.set.id)}
		{@const sort = sorterFor(g.set, g.buttons)}
		<details class="qr-group" class:off={!g.on} open>
			<summary class="qr-head">
				<ChevronRight class="caret" />
				<b>{g.set.name || "이름 없는 세트"}</b>
				<small>{g.set.scope === "global" ? "전체" : g.set.scope === "character" ? "캐릭터" : "프리셋"} · {g.buttons.length}</small>
				<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
				<span class="head-sw" onclick={(e) => e.stopPropagation()}>
					<Switch checked={g.on} label="이 대화에서 {g.set.name} 쓰기" hideLabel onchange={() => conv && void toggleForConv(conv, g.set.id)} />
				</span>
			</summary>
			{#if g.on}
				<ul class="qr-items">
					{#each g.buttons as b, i (b.q.id)}
						{@const busy = running || session.busy || (!!conv && slashRuns.isRunning(conv.id))}
						<li {...sort.item(i)} class:dragging={sort.from === i} class:over={sort.over === i && sort.from !== i}>
							{#if canReorder}
								<span class="grip" aria-hidden="true" title="끌어서 순서 바꾸기" {...sort.handle(i)}><GripVertical /></span>
							{/if}
							<div class="info" title={b.q.message}>
								<span class="label">{b.q.label.trim() || b.q.message.slice(0, 16)}</span>
								<span class="msg">{b.q.message}</span>
							</div>
							<div class="acts">
								{#if canReorder && g.buttons.length > 1}
									<IconButton size="sm" label="위로 이동" disabled={i === 0} onclick={() => stepItem(g.set, b.q.id, -1)}><ArrowUp /></IconButton>
									<IconButton size="sm" label="아래로 이동" disabled={i === g.buttons.length - 1} onclick={() => stepItem(g.set, b.q.id, 1)}><ArrowDown /></IconButton>
								{/if}
								<IconButton size="sm" label="{b.q.label || '빠른 답장'} 편집" onclick={() => startEdit(g.set, b.q)}><Pencil /></IconButton>
								<IconButton size="sm" variant="danger" label="{b.q.label || '빠른 답장'} 삭제" onclick={() => removeItem(g.set, b.q)}><Trash2 /></IconButton>
								<IconButton size="sm" variant="soft" label="{b.q.label || '빠른 답장'} 실행" disabled={busy} onclick={() => press(b)}><Play /></IconButton>
							</div>
						</li>
					{/each}
				</ul>
			{/if}
		</details>
	{/each}
</div>

<Dialog bind:open={editOpen} title="빠른 답장 편집" size="sm">
	{#if editing}
		<div class="edit-form">
			<Field label="이름"><Input bind:value={editing.label} placeholder="버튼 이름" /></Field>
			<Field label="내용" hint="보낼 글 또는 /로 시작하는 슬래시 커맨드">
				<Textarea bind:value={editing.message} rows={4} autosize maxRows={14} mono placeholder="/gen … 또는 보낼 글" />
			</Field>
			<Switch checked={!editing.hidden} label="빠른 답장 줄에 보이기" onchange={(v) => editing && (editing.hidden = !v)} />
		</div>
	{/if}
	{#snippet footer()}
		<Button variant="ghost" onclick={() => (editOpen = false)}>취소</Button>
		<Button variant="primary" onclick={saveEdit}>저장</Button>
	{/snippet}
</Dialog>

<style>
	.qr-list {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
	}
	.search {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: 6px 10px;
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		background: var(--surface);
	}
	.search:focus-within {
		border-color: var(--accent);
	}
	.search :global(svg) {
		width: 14px;
		height: 14px;
		color: var(--text-faint);
	}
	.search input {
		flex: 1;
		min-width: 0;
		border: none;
		background: none;
		color: var(--text);
		font: inherit;
		font-size: var(--text-sm);
		outline: none;
	}
	.search small {
		color: var(--text-faint);
		font-size: var(--text-xs);
	}
	.none {
		margin: 0;
		font-size: var(--text-sm);
		color: var(--text-faint);
	}
	.qr-group {
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		background: var(--surface);
		overflow: hidden;
	}
	.qr-head {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 6px 8px;
		font-size: var(--text-xs);
		cursor: pointer;
		list-style: none;
		user-select: none;
		background: var(--surface-2);
	}
	.qr-head::-webkit-details-marker {
		display: none;
	}
	.qr-head:hover {
		background: var(--surface-hover);
	}
	.qr-head :global(.caret) {
		flex: none;
		width: 14px;
		height: 14px;
		color: var(--text-muted);
		transition: transform var(--duration-fast) var(--ease);
	}
	.qr-group[open] > .qr-head :global(.caret) {
		transform: rotate(90deg);
	}
	.qr-head b {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: var(--text-sm);
	}
	.qr-head small {
		flex: none;
		color: var(--text-faint);
		font-size: var(--text-xs);
	}
	.head-sw {
		flex: none;
		display: inline-flex;
	}
	.qr-group.off .qr-head b {
		color: var(--text-faint);
	}
	.qr-items {
		list-style: none;
		margin: 0;
		padding: 4px;
		border-top: 1px solid var(--border);
		display: flex;
		flex-direction: column;
		gap: 2px;
	}
	.qr-items li {
		display: flex;
		align-items: center;
		gap: 4px;
		padding: 4px 4px 4px 2px;
		border-radius: var(--radius-sm);
		transition: background var(--duration-fast) var(--ease);
	}
	.qr-items li:hover {
		background: var(--surface-hover);
	}
	.qr-items li.dragging {
		opacity: 0.4;
	}
	.qr-items li.over {
		box-shadow: inset 0 2px 0 var(--accent);
	}
	.grip {
		flex: none;
		display: grid;
		place-items: center;
		color: var(--text-faint);
		cursor: grab;
		padding: 2px;
	}
	.grip :global(svg) {
		width: 14px;
		height: 14px;
	}
	.info {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 1px;
	}
	.label {
		font-size: var(--text-sm);
		font-weight: 600;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.msg {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: var(--text-xs);
		color: var(--text-faint);
	}
	.acts {
		flex: none;
		display: inline-flex;
		align-items: center;
		gap: 1px;
	}
	.acts :global(.icon-btn) {
		width: 24px;
		height: 24px;
	}
	.acts :global(.icon-btn svg) {
		width: 13px;
		height: 13px;
	}
	.edit-form {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
	}
</style>
