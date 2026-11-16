<script lang="ts">
	import { drops, extOf, storeFile } from "$app/drop.svelte";
	import { onMount } from "svelte";
	import { ChevronRight, Download, Plus } from "@lucide/svelte";
	import type { RegexPlacement, RegexScope, RegexScript_Serialize as RegexScript, RegexTarget } from "$ipc/bindings";
	import { Button, EmptyState, Switch } from "$shared/ui";
	import RegexEditDialog from "./RegexEditDialog.svelte";
	import { call, commands } from "$shared/ipc/client";
	import { pickFiles, JSON_FILES } from "$shared/ipc/dialog";
	import { confirmAction } from "$shared/stores/confirm.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	import { library } from "$features/character/library.svelte";
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";
	import Row from "./Row.svelte";

	const src = { source: "regex" };
	let scripts = $state<RegexScript[]>([]);
	let loaded = $state(false);

	onMount(async () => {
		await Promise.all([load(), library.loaded ? null : library.load()]);
	});
	async function load() {
		scripts = await call(commands.listRegexScripts(), src).catch(() => []);
		loaded = true;
	}

	const PLACE: Record<RegexPlacement, string> = { user_input: "내 메시지", ai_output: "AI 응답", world_info: "월드인포" };
	const TARGET: Record<RegexTarget, string> = { stored: "저장 내용", prompt: "AI에게만", display: "화면에만" };

	function scopeName(s: RegexScript) {
		if (s.scope === "global") return "전체";
		if (s.scope === "character") return library.character(s.scopeId)?.name ?? "지워진 캐릭터";
		return library.presets.find((p) => p.id === s.scopeId)?.name ?? "지워진 프리셋";
	}
	const groups = $derived(
		(["global", "character", "preset"] as RegexScope[])
			.map((scope) => ({ scope, title: scope === "global" ? "전체에 적용" : scope === "character" ? "캐릭터 전용" : "프리셋 전용", items: scripts.filter((s) => s.scope === scope) }))
			.filter((g) => g.items.length),
	);
	/** 캐릭터·프리셋 전용 정규식은 주인별로 묶어 이름만 보이고, 누르면 펼친다 (많아져도 한눈에) */
	function owners(items: RegexScript[]) {
		const map = new Map<string, RegexScript[]>();
		for (const s of items) map.set(s.scopeId ?? "", [...(map.get(s.scopeId ?? "") ?? []), s]);
		return [...map.entries()]
			.map(([id, list]) => ({ id, name: scopeName(list[0]), items: list, on: list.filter((s) => s.enabled).length }))
			.sort((a, b) => a.name.localeCompare(b.name, "ko"));
	}
	/** 펼친 묶음 (화면에 머무는 동안 기억) */
	let expanded = $state<Record<string, boolean>>({});

	async function toggle(s: RegexScript, on: boolean) {
		s.enabled = on;
		await call(commands.upsertRegexScript({ ...s, enabled: on }), src).catch(() => (s.enabled = !on));
	}

	// ---- 편집 (대화상자: RegexEditDialog) ----
	let editor: RegexEditDialog | undefined = $state();
	const startNew = () => editor?.startNew();
	const startEdit = (s: RegexScript) => editor?.startEdit(s);
	function onsaved(saved: RegexScript) {
		const i = scripts.findIndex((s) => s.id === saved.id);
		if (i >= 0) scripts[i] = saved;
		else scripts = [...scripts, saved];
	}
	async function remove(s: RegexScript) {
		if (!(await confirmAction({ title: "정규식 삭제", message: `'${s.name}' 정규식을 지움`, confirmLabel: "삭제", danger: true }))) return;
		await call(commands.deleteRegexScript(s.id), src);
		scripts = scripts.filter((x) => x.id !== s.id);
	}
	async function importFile() {
		const [path] = await pickFiles([JSON_FILES], false);
		if (path) await importPath(path);
	}
	$effect(() =>
		drops.register({
			label: "정규식 가져오기",
			hint: "SillyTavern 정규식 JSON (프리셋에 든 정규식도)",
			accept: (f) => extOf(f) === "json",
			ondrop: async (files) => {
				for (const f of files) await importPath(await storeFile(f)).catch(() => {});
			},
		}),
	);
	async function importPath(path: string) {
		const r = await call(commands.importSillytavern(path, null, null, null), src);
		if (r.kind === "regex") toasts.success(`정규식 ${r.scripts.length}개 가져옴`);
		else if (r.kind === "preset") toasts.success(`프리셋 '${r.name}' 가져옴 · 정규식 ${r.regexCount}개`);
		else toasts.info("정규식 파일이 아님");
		for (const w of r.warnings) toasts.info(w);
		await Promise.all([load(), library.load()]);
	}

</script>

{#snippet scriptRow(s: RegexScript)}
	<Row label={s.name} desc="{s.placement.map((p) => PLACE[p]).join(' · ')} → {TARGET[s.target]}">
		<Button size="sm" variant="ghost" onclick={() => remove(s)}>삭제</Button>
		<Button size="sm" onclick={() => startEdit(s)}>편집</Button>
		<Switch checked={s.enabled} label="{s.name} 사용" hideLabel onchange={(v) => toggle(s, v)} />
	</Row>
{/snippet}

<Section title="정규식" lead="메시지 글자를 찾아 바꿈. 위에 있는 것부터 차례로 적용">
	{#snippet actions()}
		<Button size="sm" onclick={importFile}>{#snippet icon()}<Download />{/snippet}가져오기</Button>
		<Button size="sm" variant="primary" onclick={startNew}>{#snippet icon()}<Plus />{/snippet}새 정규식</Button>
	{/snippet}
	{#if loaded && !scripts.length}
		<Group flush>
			<EmptyState title="정규식 없음" description="새로 만들거나 SillyTavern 정규식(.json)을 가져옴"></EmptyState>
		</Group>
	{/if}
	{#each groups as g (g.scope)}
		<Group title={g.title}>
			{#if g.scope === "global"}
				{#each g.items as s (s.id)}{@render scriptRow(s)}{/each}
			{:else}
				{#each owners(g.items) as o (o.id)}
					{@const key = `${g.scope}:${o.id}`}
					<div class="owner">
						<button class="owner-head" aria-expanded={!!expanded[key]} onclick={() => (expanded[key] = !expanded[key])}>
							<ChevronRight class="caret" />
							<b>{o.name}</b>
							<small>정규식 {o.items.length}개 · 켜짐 {o.on}개</small>
						</button>
						{#if expanded[key]}
							<div class="owner-body">
								{#each o.items as s (s.id)}{@render scriptRow(s)}{/each}
							</div>
						{/if}
					</div>
				{/each}
			{/if}
		</Group>
	{/each}
</Section>

<RegexEditDialog bind:this={editor} sortOrder={scripts.length} {onsaved} />

<style>
	.owner + .owner {
		border-top: 1px solid var(--border);
	}
	.owner-head {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		width: 100%;
		padding: var(--space-3) var(--space-2);
		border: none;
		background: transparent;
		color: var(--text);
		font: inherit;
		text-align: left;
		cursor: pointer;
	}
	.owner-head:hover {
		background: var(--surface-hover);
	}
	.owner-head small {
		margin-left: auto;
		color: var(--text-faint);
		font-size: var(--text-xs);
	}
	.owner-head :global(.caret) {
		width: 14px;
		height: 14px;
		color: var(--text-muted);
		transition: transform var(--duration-fast) var(--ease);
	}
	.owner-head[aria-expanded="true"] :global(.caret) {
		transform: rotate(90deg);
	}
	.owner-body {
		padding-left: var(--space-4);
	}
</style>
