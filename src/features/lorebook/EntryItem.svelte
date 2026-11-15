<script lang="ts">
	// 로어북 항목 하나: 접힌 줄(켜기·제목·키워드) + 펼치면 편집. 고치면 잠시 뒤 저절로 저장
	import { ChevronDown, ChevronRight, Copy, Trash2 } from "@lucide/svelte";
	import { Badge, Field, IconButton, Input, Switch, TagInput, Textarea } from "$shared/ui";
	import { entryTitle, isDraft, isValid, lorebook, type Entry } from "./lorebook.svelte";

	interface Props {
		entry: Entry;
		open?: boolean;
		ontoggle: () => void;
		/** 새 항목이 처음 저장돼 id가 바뀌었을 때 (펼친 상태를 이어가게) */
		onsaved?: (oldId: string, newId: string) => void;
		/** 여러 책을 섞어 보일 때 책 이름 */
		bookName?: string;
	}
	let { entry, open = false, ontoggle, onsaved, bookName }: Props = $props();

	// svelte-ignore state_referenced_locally
	let draft = $state<Entry>(structuredClone($state.snapshot(entry)));
	// svelte-ignore state_referenced_locally
	let saved = $state(JSON.stringify(entry));
	let saving = $state(false);
	let failed = $state(false);

	// 바깥(다른 화면)에서 같은 항목이 바뀌면 따라감
	$effect(() => {
		const s = JSON.stringify(entry);
		if (s !== saved && !saving) {
			draft = structuredClone($state.snapshot(entry));
			saved = s;
		}
	});

	const dirty = $derived(JSON.stringify(draft) !== saved);
	const valid = $derived(isValid(draft));

	$effect(() => {
		if (!dirty || !valid) return;
		const t = setTimeout(save, 600);
		return () => clearTimeout(t);
	});
	async function save() {
		if (saving || !isValid(draft)) return;
		saving = true;
		failed = false;
		const snapshot = $state.snapshot(draft) as Entry;
		try {
			const r = await lorebook.saveEntry(snapshot);
			saved = JSON.stringify(r);
			if (r.id !== snapshot.id) onsaved?.(snapshot.id, r.id);
			if (isDraft(snapshot) || JSON.stringify({ ...snapshot, id: r.id }) === saved) draft = structuredClone(r);
		} catch {
			failed = true;
		} finally {
			saving = false;
		}
	}
	const status = $derived(saving ? "저장 중…" : failed ? "저장 못 함" : !valid ? "키워드를 넣거나 '항상 넣기'를 켜야 저장됨" : dirty ? "고치는 중" : isDraft(draft) ? "" : "저장됨");
</script>

<li class="entry" class:open class:off={!draft.enabled} class:draft={isDraft(draft)}>
	<div class="head">
		<Switch bind:checked={draft.enabled} label="{entryTitle(draft)} 사용" hideLabel />
		<button class="toggle" aria-expanded={open} onclick={ontoggle}>
			{#if open}<ChevronDown />{:else}<ChevronRight />{/if}
			<span class="t">
				<b>{entryTitle(draft)}</b>
				{#if !open}
					<small>{draft.constant ? "항상 넣음" : draft.keys.length ? draft.keys.join(" · ") : "키워드 없음"}{bookName ? ` — ${bookName}` : ""}</small>
				{/if}
			</span>
		</button>
		{#if draft.constant}<Badge tone="accent">상시</Badge>{/if}
		<span class="prio" title="우선순위">{draft.priority}</span>
	</div>
	{#if open}
		<div class="body">
			<Field label="제목" hint="목록에서 알아보는 이름. AI에게는 보내지 않음">
				<Input bind:value={draft.comment} placeholder={draft.keys.join(", ") || "예: 엘다리아 왕국"} />
			</Field>
			<Field label="키워드" hint="대화에 이 낱말이 나오면 내용을 넣음. Enter로 하나씩">
				<TagInput bind:value={draft.keys} placeholder="키워드 입력" label="키워드 추가" />
			</Field>
			<Field label="보조 키워드" hint="있으면 키워드와 함께 이 중 하나도 나와야 넣음">
				<TagInput bind:value={draft.secondaryKeys} placeholder="(선택)" label="보조 키워드 추가" />
			</Field>
			<Field label="내용" hint={"AI에게 보낼 설정. {{char}}, {{user}} 사용 가능"}>
				<Textarea bind:value={draft.content} rows={4} autosize maxRows={18} />
			</Field>
			<div class="opts">
				<Switch bind:checked={draft.constant} label="항상 넣기" description="키워드와 상관없이 늘 보냄" />
				<label class="prio-in">
					<span>우선순위</span>
					<Input type="number" bind:value={draft.priority} aria-label="우선순위" />
				</label>
			</div>
			<div class="foot">
				<span class="status" class:warn={!valid || failed}>{status}</span>
				<IconButton label="복제" size="sm" disabled={isDraft(draft)} onclick={() => lorebook.duplicate(draft)}><Copy /></IconButton>
				<IconButton label="삭제" size="sm" onclick={() => lorebook.removeEntry(draft)}><Trash2 /></IconButton>
			</div>
		</div>
	{/if}
</li>

<style>
	.entry {
		list-style: none;
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		background: var(--surface);
	}
	.entry:not(:first-child) {
		margin-top: var(--space-2);
	}
	.entry.open {
		border-color: var(--border-strong);
	}
	.entry.draft {
		border-style: dashed;
	}
	.head {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-2) var(--space-3);
	}
	.toggle {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		flex: 1;
		min-width: 0;
		padding: 2px 0;
		border: none;
		background: transparent;
		text-align: left;
		color: inherit;
		font: inherit;
		cursor: pointer;
	}
	.toggle :global(svg) {
		flex: none;
		width: 15px;
		height: 15px;
		color: var(--text-muted);
	}
	.t {
		display: flex;
		flex-direction: column;
		min-width: 0;
		line-height: 1.35;
	}
	.t b {
		font-size: var(--text-sm);
		font-weight: 600;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.t small {
		font-size: var(--text-xs);
		color: var(--text-muted);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.off .t b {
		color: var(--text-faint);
	}
	.prio {
		flex: none;
		min-width: 28px;
		text-align: right;
		font-size: var(--text-xs);
		color: var(--text-faint);
		font-variant-numeric: tabular-nums;
	}
	.body {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		padding: var(--space-2) var(--space-3) var(--space-3);
		border-top: 1px solid var(--border);
	}
	.opts {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-3);
	}
	.prio-in {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		font-size: var(--text-sm);
	}
	.prio-in :global(input) {
		width: 80px;
	}
	.foot {
		display: flex;
		align-items: center;
		gap: 2px;
	}
	.status {
		margin-right: auto;
		font-size: var(--text-xs);
		color: var(--text-faint);
	}
	.status.warn {
		color: var(--warning, #c98a1b);
	}
</style>
