<script lang="ts">
	import { onMount } from "svelte";
	import { ChevronLeft, Download, Plus } from "@lucide/svelte";
	import type { PromptBlocksInput_Deserialize as BlockInput, PromptPresetDetail_Serialize as Detail } from "$ipc/bindings";
	import { Badge, Button, EmptyState, IconButton } from "$shared/ui";
	import { call, commands } from "$shared/ipc/client";
	import { pickFiles, JSON_FILES } from "$shared/ipc/dialog";
	import { appSetting } from "$shared/stores/appSetting.svelte";
	import { confirmAction, promptText } from "$shared/stores/confirm.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	import { library, type Preset } from "$features/character/library.svelte";
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";
	import Row from "./Row.svelte";
	import PresetEditor from "./PresetEditor.svelte";

	const src = { source: "preset" };
	let editing = $state<Detail | null>(null);
	let saving = $state(false);

	onMount(async () => {
		await Promise.all([appSetting.load(), library.loaded ? null : library.load()]);
	});

	async function reload() {
		library.presets = await call(commands.listPromptPresets(), src);
	}
	async function edit(p: Preset) {
		editing = await call(commands.getPromptPreset(p.id), src);
	}
	async function create() {
		const name = await promptText({ title: "새 프리셋", input: { value: "", placeholder: "프리셋 이름" }, confirmLabel: "만들기" });
		if (!name?.trim()) return;
		const d = await call(
			commands.savePromptPreset(
				{ id: null, name: name.trim(), description: null, presetType: "chat_completion", sourceFormat: "native", configJson: {}, settingsJson: { temperature: 1, max_tokens: 1024, stream: true } },
				[
					block("main", "기본 지시", "system", "{{char}}가 되어 {{user}}와 대화를 이어감.", 0),
					block("charDescription", "캐릭터 설명", "system", "", 1),
					block("personaDescription", "페르소나 설명", "system", "", 2),
					block("chatHistory", "대화 기록", "system", "", 3),
				],
			),
			src,
		);
		await reload();
		editing = d;
	}
	function block(identifier: string, name: string, role: string, content: string, sortOrder: number): BlockInput {
		return { id: null, presetId: "", name, identifier, blockType: "text", role, content, enabled: true, position: "relative", depth: 0, sortOrder, triggersJson: [], extraJson: null };
	}
	async function duplicate(p: Preset) {
		const d = await call(commands.getPromptPreset(p.id), src);
		await call(
			commands.savePromptPreset(
				{ ...d.preset, id: null, name: `${d.preset.name} 사본` },
				// 새 블록으로 만들어야 원본 블록이 옮겨가지 않음
				d.blocks.map((b) => ({ ...b, id: null })),
			),
			src,
		);
		await reload();
		toasts.success("복제함");
	}
	async function remove(p: Preset) {
		if (!(await confirmAction({ title: "프리셋 삭제", message: `'${p.name}' 프리셋과 블록을 지움. 이 프리셋을 쓰던 대화는 앱 기본값으로 바뀜`, confirmLabel: "삭제", danger: true }))) return;
		await call(commands.deletePromptPreset(p.id), src);
		await reload();
	}
	async function importFile() {
		const [path] = await pickFiles([JSON_FILES], false);
		if (!path) return;
		const r = await call(commands.importSillytavern(path, null, null), src);
		if (r.kind !== "preset") {
			toasts.info("프리셋 파일이 아님. 캐릭터·대화는 캐릭터 화면에서 가져옴");
			await library.load();
			return;
		}
		await reload();
		toasts.success(`'${r.name}' 가져옴 · 블록 ${r.blockCount}개${r.regexCount ? ` · 정규식 ${r.regexCount}개` : ""}`);
		for (const w of r.warnings) toasts.info(w);
	}
	async function save(preset: Detail["preset"], blocks: BlockInput[]) {
		saving = true;
		try {
			const d = await call(commands.savePromptPreset({ ...preset, id: preset.id }, blocks), src);
			editing = null;
			await reload();
			toasts.success(`'${d.preset.name}' 저장함`);
		} finally {
			saving = false;
		}
	}
	async function cancel() {
		if (await confirmAction({ title: "편집 그만두기", message: "저장하지 않은 바뀐 내용은 버림", confirmLabel: "그만두기" })) editing = null;
	}
	const isDefault = (p: Preset) => appSetting.value?.selectedPreset === p.id;
</script>

{#if editing}
	{#key editing.preset.id}
		<div class="back">
			<IconButton label="프리셋 목록으로" size="sm" onclick={cancel}><ChevronLeft /></IconButton>
			<span>프리셋</span>
		</div>
		<Section title={editing.preset.name} lead={editing.preset.sourceFormat === "sillytavern" ? "SillyTavern에서 가져온 프리셋" : undefined}>
			<PresetEditor detail={editing} {saving} onsave={save} oncancel={cancel} />
		</Section>
	{/key}
{:else}
	<Section title="프리셋" lead="AI에게 보내는 프롬프트 구성과 생성 옵션">
		{#snippet actions()}
			<Button size="sm" onclick={importFile}>{#snippet icon()}<Download />{/snippet}가져오기</Button>
			<Button size="sm" variant="primary" onclick={create}>{#snippet icon()}<Plus />{/snippet}새 프리셋</Button>
		{/snippet}
		{#if library.presets.length}
			<Group>
				{#each library.presets as p (p.id)}
					<Row label={p.name} desc={p.description || (p.sourceFormat === "sillytavern" ? "SillyTavern에서 가져옴" : "설명 없음")}>
						{#if isDefault(p)}
							<Badge tone="accent">기본</Badge>
						{:else}
							<Button size="sm" variant="ghost" onclick={() => appSetting.update({ selectedPreset: p.id })}>기본으로</Button>
						{/if}
						<Button size="sm" variant="ghost" onclick={() => remove(p)}>삭제</Button>
						<Button size="sm" variant="ghost" onclick={() => duplicate(p)}>복제</Button>
						<Button size="sm" variant="soft" onclick={() => edit(p)}>편집</Button>
					</Row>
				{/each}
			</Group>
		{:else}
			<Group flush>
				<EmptyState title="프리셋 없음" description="새로 만들거나 SillyTavern 프리셋(.json)을 가져옴">
					{#snippet action()}<Button variant="primary" onclick={create}>새 프리셋</Button>{/snippet}
				</EmptyState>
			</Group>
		{/if}
	</Section>
{/if}

<style>
	.back {
		display: flex;
		align-items: center;
		gap: var(--space-1);
		margin: calc(var(--space-4) * -1) 0 var(--space-2) calc(var(--space-2) * -1);
		font-size: var(--text-sm);
		color: var(--text-muted);
	}
</style>
