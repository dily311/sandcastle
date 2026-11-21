<script lang="ts">
	// 정규식 편집 대화상자 (설정 > 정규식, 대화 오른쪽 패널이 같이 씀). 부모가 bind:this로 startNew/startEdit를 부른다
	import { regexVersion } from "$shared/stores/regexVersion.svelte";
	import type {
		RegexPlacement,
		RegexScope,
		RegexScriptInput_Deserialize as RegexInput,
		RegexScript_Serialize as RegexScript,
		RegexTarget,
	} from "$ipc/bindings";
	import { Badge, Button, Dialog, Field, Input, Segmented, Select, Switch, Textarea } from "$shared/ui";
	import { call, commands } from "$shared/ipc/client";
	import { toasts } from "$shared/stores/toast.svelte";
	import { library } from "$features/character/library.svelte";

	interface Props {
		/** 새로 만들 때 순서 값 */
		sortOrder?: number;
		onsaved?: (s: RegexScript) => void;
	}
	let { sortOrder = 0, onsaved }: Props = $props();
	const src = { source: "regex" };
	const PLACE: Record<RegexPlacement, string> = { user_input: "내 메시지", ai_output: "AI 응답", world_info: "월드인포" };
	const TARGET: Record<RegexTarget, string> = { stored: "저장 내용", prompt: "AI에게만", display: "화면에만" };
	let open = $state(false);

	// ---- 편집 ----
	let draft = $state<(RegexInput & { placement: RegexPlacement[]; target: RegexTarget }) | null>(null);
	let sample = $state("");
	let preview = $state<string | null>(null);
	let previewError = $state<string | null>(null);

	function startNew(defaults: Partial<RegexInput> = {}) {
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
			sortOrder,
			sourceFormat: "native",
			extraJson: null,
			...defaults,
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
		regexVersion.bump();
		open = false;
		toasts.success("저장함");
		onsaved?.(saved);
	}

	export { startNew, startEdit };

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
						<Select value={draft.scopeId ?? ""} options={scopeOptions} placeholder="미선택" onchange={(e) => draft && (draft.scopeId = e.currentTarget.value)} />
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
