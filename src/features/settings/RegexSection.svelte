<script lang="ts">
	import { onMount } from "svelte";
	import { Download, Plus } from "@lucide/svelte";
	import type {
		RegexPlacement,
		RegexScope,
		RegexScriptInput_Deserialize as RegexInput,
		RegexScript_Serialize as RegexScript,
		RegexTarget,
	} from "$ipc/bindings";
	import { Badge, Button, Dialog, EmptyState, Field, Input, Segmented, Select, Switch, Textarea } from "$shared/ui";
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

	async function toggle(s: RegexScript, on: boolean) {
		s.enabled = on;
		await call(commands.upsertRegexScript({ ...s, enabled: on }), src).catch(() => (s.enabled = !on));
	}

	// ---- 편집 ----
	let draft = $state<(RegexInput & { placement: RegexPlacement[]; target: RegexTarget }) | null>(null);
	let open = $state(false);
	let sample = $state("");
	let preview = $state<string | null>(null);
	let previewError = $state<string | null>(null);

	function startNew() {
		draft = {
			id: null,
			name: "",
			findRegex: "",
			replaceString: "",
			trimStrings: [],
			placement: ["ai_output"],
			target: "display",
			scope: "global",
			scopeId: null,
			substituteMacros: "none",
			runOnEdit: true,
			minDepth: null,
			maxDepth: null,
			enabled: true,
			sortOrder: scripts.length,
			sourceFormat: "native",
			extraJson: null,
		};
		sample = "";
		preview = previewError = null;
		open = true;
	}
	function startEdit(s: RegexScript) {
		draft = { ...s, placement: [...s.placement], trimStrings: [...s.trimStrings] };
		sample = "";
		preview = previewError = null;
		open = true;
	}
	function togglePlace(p: RegexPlacement) {
		if (!draft) return;
		draft.placement = draft.placement.includes(p) ? draft.placement.filter((x) => x !== p) : [...draft.placement, p];
	}

	/** 프록시를 벗긴 일반 객체 ($state.snapshot은 이 타입에서 TS가 너무 깊게 풂) */
	const plain = (d: RegexInput): RegexInput => JSON.parse(JSON.stringify(d));

	// 입력이 바뀌면 잠시 뒤 미리보기
	$effect(() => {
		if (!open || !draft || !draft.findRegex || !sample) {
			preview = previewError = null;
			return;
		}
		const input = plain(draft);
		const text = sample;
		const t = setTimeout(async () => {
			try {
				preview = await call(commands.previewRegexScript(input, text, "캐릭터", "나"), { ...src, silent: true });
				previewError = null;
			} catch (e) {
				preview = null;
				previewError = (e as Error).message;
			}
		}, 300);
		return () => clearTimeout(t);
	});

	async function save() {
		if (!draft || !draft.name.trim() || !draft.findRegex.trim() || !draft.placement.length) return;
		const saved = await call(commands.upsertRegexScript({ ...plain(draft), name: draft.name.trim() }), src);
		const i = scripts.findIndex((s) => s.id === saved.id);
		if (i >= 0) scripts[i] = saved;
		else scripts = [...scripts, saved];
		open = false;
		toasts.success("저장함");
	}
	async function remove(s: RegexScript) {
		if (!(await confirmAction({ title: "정규식 삭제", message: `'${s.name}' 정규식을 지움`, confirmLabel: "삭제", danger: true }))) return;
		await call(commands.deleteRegexScript(s.id), src);
		scripts = scripts.filter((x) => x.id !== s.id);
	}
	async function importFile() {
		const [path] = await pickFiles([JSON_FILES], false);
		if (!path) return;
		const r = await call(commands.importSillytavern(path, null, null), src);
		if (r.kind === "regex") toasts.success(`정규식 ${r.scripts.length}개 가져옴`);
		else if (r.kind === "preset") toasts.success(`프리셋 '${r.name}' 가져옴 · 정규식 ${r.regexCount}개`);
		else toasts.info("정규식 파일이 아님");
		for (const w of r.warnings) toasts.info(w);
		await Promise.all([load(), library.load()]);
	}

	const scopeOptions = $derived(
		draft?.scope === "character"
			? library.characters.map((c) => ({ value: c.id, label: c.name }))
			: draft?.scope === "preset"
				? library.presets.map((p) => ({ value: p.id, label: p.name }))
				: [],
	);
	function setScope(scope: RegexScope) {
		if (!draft) return;
		draft.scope = scope;
		draft.scopeId = scope === "global" ? null : (scope === "character" ? library.characters[0]?.id : library.presets[0]?.id) ?? null;
	}
</script>

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
			{#each g.items as s (s.id)}
				<Row label={s.name} desc="{s.placement.map((p) => PLACE[p]).join(' · ')} → {TARGET[s.target]}{s.scope !== 'global' ? ` · ${scopeName(s)}` : ''}">
					<Button size="sm" variant="ghost" onclick={() => remove(s)}>삭제</Button>
					<Button size="sm" onclick={() => startEdit(s)}>편집</Button>
					<Switch checked={s.enabled} label="{s.name} 사용" hideLabel onchange={(v) => toggle(s, v)} />
				</Row>
			{/each}
		</Group>
	{/each}
</Section>

<Dialog bind:open title={draft?.id ? "정규식 편집" : "새 정규식"} size="lg">
	{#if draft}
		<div class="form">
			<Field label="이름" required><Input bind:value={draft.name} placeholder="예: 생각 태그 숨기기" /></Field>
			<Field label="찾을 패턴" hint="/패턴/플래그 형식. g가 없으면 첫 번째만 바꿈" required>
				<Input bind:value={draft.findRegex} placeholder="/<think>[\s\S]*?<\/think>/g" spellcheck={false} class="mono" />
			</Field>
			<Field label="바꿀 내용" hint={"$1 같은 그룹, {{match}} 사용 가능. 비우면 지움"}>
				<Textarea bind:value={draft.replaceString} mono rows={2} autosize maxRows={8} spellcheck={false} />
			</Field>
			<div class="two">
				<Field label="적용할 글">
					<div class="chips">
						{#each Object.entries(PLACE) as [id, label] (id)}
							<button class="chip" class:on={draft.placement.includes(id as RegexPlacement)} aria-pressed={draft.placement.includes(id as RegexPlacement)} onclick={() => togglePlace(id as RegexPlacement)}>{label}</button>
						{/each}
					</div>
				</Field>
				<Field label="바꾸는 곳">
					<Segmented label="바꾸는 곳" bind:value={draft.target} options={(Object.entries(TARGET) as [RegexTarget, string][]).map(([value, label]) => ({ value, label }))} />
				</Field>
			</div>
			<div class="two">
				<Field label="범위">
					<Segmented
						label="범위"
						value={draft.scope ?? "global"}
						options={[
							{ value: "global", label: "전체" },
							{ value: "character", label: "캐릭터" },
							{ value: "preset", label: "프리셋" },
						]}
						onchange={setScope}
					/>
				</Field>
				{#if draft.scope !== "global"}
					<Field label={draft.scope === "character" ? "캐릭터" : "프리셋"}>
						<Select value={draft.scopeId ?? ""} options={scopeOptions} placeholder="고르기" onchange={(e) => draft && (draft.scopeId = e.currentTarget.value)} />
					</Field>
				{/if}
			</div>
			<div class="two">
				<Field label="매크로 치환" hint={"패턴 안의 {{char}}, {{user}}"}>
					<Select
						bind:value={draft.substituteMacros}
						options={[
							{ value: "none", label: "안 함" },
							{ value: "raw", label: "그대로" },
							{ value: "escaped", label: "특수문자 이스케이프" },
						]}
					/>
				</Field>
				<Field label="적용 깊이" hint="최근 메시지에서 몇 번째까지 (비우면 전부)">
					<div class="depth">
						<Input type="number" min="0" placeholder="최소" bind:value={draft.minDepth} />
						<span>~</span>
						<Input type="number" min="0" placeholder="최대" bind:value={draft.maxDepth} />
					</div>
				</Field>
			</div>
			<Switch bind:checked={draft.runOnEdit} label="메시지를 고칠 때도 적용" />

			<div class="preview">
				<Field label="미리보기" hint={"예시 글을 넣으면 바뀐 결과를 보여줌. {{char}}=캐릭터, {{user}}=나"}>
					<Textarea bind:value={sample} rows={3} autosize maxRows={8} placeholder="예시 글" />
				</Field>
				{#if previewError}
					<p class="err">{previewError}</p>
				{:else if preview !== null}
					<div class="out">
						<Badge tone={preview === sample ? "neutral" : "success"}>{preview === sample ? "바뀐 곳 없음" : "결과"}</Badge>
						<pre>{preview}</pre>
					</div>
				{/if}
			</div>
		</div>
	{/if}
	{#snippet footer()}
		<Button variant="ghost" onclick={() => (open = false)}>취소</Button>
		<Button variant="primary" disabled={!draft?.name.trim() || !draft?.findRegex.trim() || !draft?.placement.length} onclick={save}>저장</Button>
	{/snippet}
</Dialog>

<style>
	.form {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
	}
	.two {
		display: grid;
		grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
		gap: var(--space-3);
	}
	.form :global(.mono input) {
		font-family: var(--font-mono);
	}
	.chips {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-1);
	}
	.chip {
		height: 28px;
		padding: 0 var(--space-3);
		border: 1px solid var(--border-strong);
		border-radius: 999px;
		background: transparent;
		color: var(--text-muted);
		font: inherit;
		font-size: var(--text-sm);
		cursor: pointer;
	}
	.chip.on {
		border-color: var(--accent);
		background: var(--accent-soft);
		color: var(--text);
	}
	.depth > :global(:not(span)) {
		flex: 1;
		min-width: 0;
	}
	.depth {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		color: var(--text-faint);
	}
	.preview {
		padding-top: var(--space-3);
		border-top: 1px solid var(--border);
	}
	.out {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: var(--space-2);
		margin-top: var(--space-2);
	}
	pre {
		width: 100%;
		margin: 0;
		padding: var(--space-3);
		border-radius: var(--radius-sm);
		background: var(--bg-sunken);
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}
	.err {
		margin-top: var(--space-2);
		color: var(--danger);
		font-size: var(--text-sm);
	}
</style>
