<script lang="ts">
	// 빠른 답장 세트 (5-E-20-5-3·4, ADR-025). 정규식처럼 전체·캐릭터 전용·프리셋 전용으로 묶는다. 켜짐/꺼짐은 코어 플러그인 slash
	import { onMount } from "svelte";
	import { ChevronRight, Download, Plus, Trash2 } from "@lucide/svelte";
	import { Button, EmptyState, IconButton, Input, Select, Switch, Textarea } from "$shared/ui";
	import { call, commands } from "$shared/ipc/client";
	import { pickFiles, JSON_FILES } from "$shared/ipc/dialog";
	import { confirmAction } from "$shared/stores/confirm.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	import { library } from "$features/character/library.svelte";
	import { newId, quickReplies, toInput, type QuickReplySet, type Scope } from "$features/slash/quickReply.svelte";
	import { aiLimit, MAX_AI_LIMIT } from "$features/slash/commands";
	import { pluginSetting } from "$shared/stores/aiTools.svelte";
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";
	import Row from "./Row.svelte";

	onMount(() => {
		if (!pluginSetting.value) void pluginSetting.load();
		void quickReplies.load(true);
		if (!library.loaded) void library.load();
	});
	let open = $state<Record<string, boolean>>({});

	const SCOPES: { value: Scope; label: string }[] = [
		{ value: "global", label: "전체" },
		{ value: "character", label: "캐릭터 전용" },
		{ value: "preset", label: "프리셋 전용" },
	];
	const ownerName = (s: QuickReplySet) =>
		s.scope === "character" ? (library.character(s.scopeId)?.name ?? "지워진 캐릭터") : (library.presets.find((p) => p.id === s.scopeId)?.name ?? "지워진 프리셋");
	const groups = $derived(
		(["global", "character", "preset"] as const)
			.map((scope) => ({ scope, title: scope === "global" ? "전체에 적용" : scope === "character" ? "캐릭터 전용" : "프리셋 전용", items: quickReplies.sets.filter((s) => s.scope === scope) }))
			.filter((g) => g.items.length),
	);
	function owners(items: QuickReplySet[]) {
		const map = new Map<string, QuickReplySet[]>();
		for (const s of items) map.set(s.scopeId ?? "", [...(map.get(s.scopeId ?? "") ?? []), s]);
		return [...map.entries()].map(([id, list]) => ({ id, name: ownerName(list[0]), items: list }));
	}
	const ownerOptions = (scope: string) =>
		scope === "character" ? library.characters.map((c) => ({ value: c.id, label: c.name })) : library.presets.map((p) => ({ value: p.id, label: p.name }));

	// 고칠 때마다 조금 기다렸다 그 세트만 저장
	const timers = new Map<string, ReturnType<typeof setTimeout>>();
	function changed(s: QuickReplySet) {
		clearTimeout(timers.get(s.id));
		timers.set(
			s.id,
			setTimeout(() => void quickReplies.save(toInput($state.snapshot(s) as QuickReplySet)), 400),
		);
	}
	function setScope(s: QuickReplySet, scope: Scope) {
		if (scope === s.scope) return;
		const first = scope === "global" ? null : (ownerOptions(scope)[0]?.value ?? null);
		if (scope !== "global" && !first) return toasts.error(scope === "character" ? "캐릭터가 없음" : "프리셋이 없음");
		s.scope = scope;
		s.scopeId = first;
		changed(s);
	}

	/** 스크립트 한 번 실행의 AI 호출 상한 (ADR-025 5-E-20-6-0) */
	function setAiLimit(v: string) {
		const cur = pluginSetting.value;
		const n = Math.min(MAX_AI_LIMIT, Math.max(1, Math.round(Number(v)) || 1));
		if (!cur) return;
		void pluginSetting.save({ ...cur, quickReply: { ...(cur.quickReply ?? {}), aiLimit: n } } as typeof cur);
	}
	async function addSet() {
		const s = await quickReplies.save({ id: null, name: "새 세트", scope: "global", scopeId: null, enabled: true, autoSend: false, items: [{ id: newId(), label: "", message: "", hidden: false }], sortOrder: quickReplies.sets.length });
		open[s.id] = true;
	}
	async function removeSet(s: QuickReplySet) {
		if (!(await confirmAction({ title: "세트 삭제", message: `'${s.name}' 세트와 버튼 ${s.items.length}개를 지움`, confirmLabel: "삭제", danger: true }))) return;
		await quickReplies.remove(s.id);
	}
	async function importSt() {
		// 세트(.json)·버튼 하나(.qr.json) 여러 개. 버튼 파일들은 세트 하나로 묶여 온다
		const paths = await pickFiles([JSON_FILES], true);
		if (!paths.length) return;
		const [inputs, warnings] = await call(commands.readQuickReplyFiles(paths), { source: "slash" });
		for (const input of inputs) {
			const s = await quickReplies.save({ ...input, sortOrder: quickReplies.sets.length });
			open[s.id] = true;
		}
		const buttons = inputs.reduce((n, s) => n + s.items.length, 0);
		toasts.success(`세트 ${inputs.length}개를 가져옴 · 버튼 ${buttons}개`);
		if (warnings.length) toasts.info(warnings.join(" · "));
	}
</script>

{#snippet setCard(s: QuickReplySet)}
	<div class="set">
		<div class="set-head">
			<button class="toggle" aria-expanded={!!open[s.id]} onclick={() => (open[s.id] = !open[s.id])}>
				<ChevronRight class="caret" />
				<b>{s.name || "이름 없는 세트"}</b>
				<small>버튼 {s.items.length}개</small>
			</button>
			<Switch checked={s.enabled} label="{s.name} 사용" hideLabel onchange={(v) => ((s.enabled = v), changed(s))} />
		</div>
		{#if open[s.id]}
			<div class="set-body">
				<Row label="이름">
					<Input bind:value={s.name} oninput={() => changed(s)} aria-label="세트 이름" />
					<IconButton label="세트 삭제" size="sm" variant="ghost" onclick={() => removeSet(s)}><Trash2 /></IconButton>
				</Row>
				<Row label="적용">
					<Select value={s.scope} options={SCOPES} onchange={(e) => setScope(s, e.currentTarget.value as Scope)} aria-label="적용 범위" />
					{#if s.scope !== "global"}
						<Select value={s.scopeId ?? ""} options={ownerOptions(s.scope)} onchange={(e) => ((s.scopeId = e.currentTarget.value), changed(s))} aria-label="대상" />
					{/if}
				</Row>
				<Row label="글은 바로 보내기" desc="켜면 누를 때 AI 답장 요금이 듦 · 끄면 입력칸에 넣기만">
					<Switch checked={s.autoSend} label="글은 바로 보내기" hideLabel onchange={(v) => ((s.autoSend = v), changed(s))} />
				</Row>
				<div class="items">
					{#each s.items as q, j (q.id)}
						<div class="item">
							<div class="head">
								<Input bind:value={q.label} oninput={() => changed(s)} placeholder="버튼 이름" aria-label="버튼 이름" />
								<Switch checked={!q.hidden} label="버튼 줄에 보이기" hideLabel onchange={(v) => ((q.hidden = !v), changed(s))} />
								<IconButton label="버튼 삭제" size="sm" variant="ghost" onclick={() => (s.items.splice(j, 1), changed(s))}><Trash2 /></IconButton>
							</div>
							<Textarea bind:value={q.message} oninput={() => changed(s)} rows={2} autosize maxRows={12} placeholder="보낼 글 또는 /input … | /gen … | /sendas" aria-label="내용" />
						</div>
					{/each}
					<div><Button size="sm" variant="ghost" onclick={() => (s.items.push({ id: newId(), label: "", message: "", hidden: false }), changed(s))}>{#snippet icon()}<Plus />{/snippet}버튼 추가</Button></div>
				</div>
			</div>
		{/if}
	</div>
{/snippet}

<Section title="빠른 답장" lead="입력칸 위 버튼. /로 시작하는 글은 스크립트로 실행">
	{#snippet actions()}
		<Button size="sm" onclick={importSt}>{#snippet icon()}<Download />{/snippet}SillyTavern에서 가져오기</Button>
		<Button size="sm" variant="primary" onclick={addSet}>{#snippet icon()}<Plus />{/snippet}새 세트</Button>
	{/snippet}
	<Group>
		<Row label="AI 호출 상한" desc="스크립트 한 번 실행에서 /gen 등으로 AI를 부를 수 있는 횟수 (1~{MAX_AI_LIMIT}) · 늘리면 그만큼 요금이 들 수 있음">
			<Input type="number" min={1} max={MAX_AI_LIMIT} value={String(aiLimit())} onchange={(e) => setAiLimit(e.currentTarget.value)} aria-label="AI 호출 상한" />
		</Row>
	</Group>
	{#if quickReplies.loaded && !quickReplies.sets.length}
		<Group flush>
			<EmptyState title="세트 없음" description="새로 만들거나 SillyTavern 빠른 답장 세트(.json)를 가져옴"></EmptyState>
		</Group>
	{/if}
	{#each groups as g (g.scope)}
		<Group title={g.title}>
			{#if g.scope === "global"}
				{#each g.items as s (s.id)}{@render setCard(s)}{/each}
			{:else}
				{#each owners(g.items) as o (o.id)}
					<div class="owner">
						<span class="owner-name">{o.name}</span>
						{#each o.items as s (s.id)}{@render setCard(s)}{/each}
					</div>
				{/each}
			{/if}
		</Group>
	{/each}
</Section>

<style>
	.set + .set,
	.owner + .owner {
		border-top: 1px solid var(--border);
	}
	.owner-name {
		display: block;
		padding: var(--space-2) var(--space-3) 0;
		font-size: var(--text-xs);
		color: var(--text-faint);
	}
	.set-head {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding-right: var(--space-3);
	}
	.toggle {
		flex: 1;
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-3) var(--space-2);
		border: none;
		background: transparent;
		color: var(--text);
		font: inherit;
		text-align: left;
		cursor: pointer;
	}
	.toggle:hover {
		background: var(--surface-hover);
	}
	.toggle small {
		margin-left: auto;
		color: var(--text-faint);
		font-size: var(--text-xs);
	}
	.toggle :global(.caret) {
		width: 14px;
		height: 14px;
		color: var(--text-muted);
		transition: transform var(--duration-fast) var(--ease);
	}
	.toggle[aria-expanded="true"] :global(.caret) {
		transform: rotate(90deg);
	}
	.set-body {
		padding-left: var(--space-4);
	}
	.items {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		padding: var(--space-3) var(--space-4) var(--space-4);
	}
	.item {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		padding-bottom: var(--space-3);
		border-bottom: 1px solid var(--border);
	}
	.head {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}
	.head :global(input) {
		flex: 1;
	}
</style>
