<script lang="ts">
	import { presetMacros } from "$features/character/presetMacros.svelte";
	import { jobs } from "$shared/stores/jobs.svelte";
	import { trashedToast } from "$shared/trash";
	import { drops, extOf, storeFile } from "$app/drop.svelte";
	import { onMount } from "svelte";
	import { _ } from "svelte-i18n";
	import { ChevronLeft, Copy, Download, Pencil, Plus, Trash, Upload } from "@lucide/svelte";
	import type { PromptBlocksInput_Deserialize as BlockInput, PromptPresetDetail_Serialize as Detail } from "$ipc/bindings";
	import { Badge, Button, EmptyState, IconButton } from "$shared/ui";
	import { call, commands } from "$shared/ipc/client";
	import { pickFiles, saveTo, JSON_FILES } from "$shared/ipc/dialog";
	import { appSetting } from "$shared/stores/appSetting.svelte";
	import { confirmAction, promptText } from "$shared/stores/confirm.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	import { library, type Preset } from "$features/character/library.svelte";
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";
	import Row from "./Row.svelte";
	import PresetEditor from "./PresetEditor.svelte";
	import { importPresetPaths } from "./presetImport";

	const src = { source: "preset" };
	let editing = $state<Detail | null>(null);
	let saving = $state(false);

	onMount(async () => {
		await Promise.all([appSetting.load(), library.loaded ? null : library.load()]);
	});

	/** SillyTavern Chat Completion 프리셋(.json)으로 (5-E-20-4-3) */
	async function exportSt(p: Preset) {
		const done = await saveTo(`${p.name}.json`, [JSON_FILES], (path) => call(commands.exportPresetSillytavern(p.id, path), src).then(() => true));
		if (done) toasts.success($_("settings.more.stPresetExported"));
	}
	async function reload() {
		library.presets = await call(commands.listPromptPresets(), src);
	}
	async function edit(p: Preset) {
		editing = await call(commands.getPromptPreset(p.id), src);
	}
	async function create() {
		const name = await promptText({ title: $_("settings.more.newPresetTitle"), input: { value: "", placeholder: $_("settings.more.presetNamePlaceholder") }, confirmLabel: $_("settings.more.createBtn") });
		if (!name?.trim()) return;
		const d = await call(
			commands.savePromptPreset(
				{ id: null, name: name.trim(), description: null, presetType: "chat_completion", sourceFormat: "native", configJson: {}, settingsJson: { temperature: 1, max_tokens: 1024, stream: true } },
				[
					block("main", $_("settings.more.blockMain"), "system", $_("settings.more.blockMainContent"), 0),
					block("charDescription", $_("settings.more.blockCharDesc"), "system", "", 1),
					block("personaDescription", $_("settings.more.blockPersonaDesc"), "system", "", 2),
					block("chatHistory", $_("settings.more.blockChatHistory"), "system", "", 3),
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
				{ ...d.preset, id: null, name: $_("settings.more.copySuffix", { values: { name: d.preset.name } }) },
				// 새 블록으로 만들어야 원본 블록이 옮겨가지 않음
				d.blocks.map((b) => ({ ...b, id: null })),
			),
			src,
		);
		await reload();
		toasts.success($_("settings.more.duplicatedToast"));
	}
	async function remove(p: Preset) {
		if (!(await confirmAction({ title: $_("settings.more.deletePresetTitle"), message: $_("settings.more.deletePresetMsg", { values: { name: p.name } }), confirmLabel: $_("common.delete"), danger: true }))) return;
		const item = await jobs.run(p.name, () => call(commands.deletePromptPreset(p.id), src), { group: $_("settings.more.deletePresetGroup") });
		await reload();
		if (item) trashedToast(item, reload);
	}
	async function importFile() {
		// 프리셋과 그 프리셋용 정규식 JSON을 함께 고를 수 있다
		const paths = await pickFiles([JSON_FILES], true);
		if (paths.length) await importPaths(paths);
	}
	$effect(() =>
		drops.register({
			label: $_("settings.more.dropPresetLabel"),
			hint: $_("settings.more.dropPresetHint"),
			accept: (f) => extOf(f) === "json",
			ondrop: async (files) => {
				const paths: string[] = [];
				for (const f of files) paths.push(await storeFile(f));
				await importPaths(paths).catch(() => {});
			},
		}),
	);
	const importPaths = importPresetPaths;
	async function save(preset: Detail["preset"], blocks: BlockInput[]) {
		saving = true;
		try {
			const d = await call(commands.savePromptPreset({ ...preset, id: preset.id }, blocks).finally(() => presetMacros.forget(preset.id)), src);
			editing = null;
			await reload();
			toasts.success($_("settings.more.savedNamed", { values: { name: d.preset.name } }));
		} finally {
			saving = false;
		}
	}
	async function cancel() {
		if (await confirmAction({ title: $_("settings.more.stopEditTitle"), message: $_("settings.more.stopEditMsg"), confirmLabel: $_("settings.more.stopEditConfirm") })) editing = null;
	}
	const isDefault = (p: Preset) => appSetting.value?.selectedPreset === p.id;
</script>

{#if editing}
	{#key editing.preset.id}
		<div class="back">
			<IconButton label={$_("settings.more.backToPresetsAria")} size="sm" onclick={cancel}><ChevronLeft /></IconButton>
			<span>{$_("settings.more.presetLabel")}</span>
		</div>
		<Section title={editing.preset.name} lead={editing.preset.sourceFormat === "sillytavern" ? $_("settings.presetSec.importedSt") : undefined}>
			<PresetEditor detail={editing} {saving} onsave={save} oncancel={cancel} />
		</Section>
	{/key}
{:else}
	<Section title={$_("settings.presetSec.title")} lead={$_("settings.presetSec.lead")}>
		{#snippet actions()}
			<Button size="sm" onclick={importFile}>{#snippet icon()}<Download />{/snippet}{$_("common.import")}</Button>
			<Button size="sm" variant="primary" onclick={create}>{#snippet icon()}<Plus />{/snippet}{$_("settings.presetSec.new")}</Button>
		{/snippet}
		{#if library.presets.length}
			<Group>
				{#each library.presets as p (p.id)}
					<Row label={p.name} desc={p.description || (p.sourceFormat === "sillytavern" ? $_("settings.presetSec.importedSt") : $_("settings.presetSec.noDesc"))}>
						{#if isDefault(p)}
							<Badge tone="accent">{$_("common.default")}</Badge>
						{:else}
							<Button size="sm" variant="ghost" onclick={() => appSetting.update({ selectedPreset: p.id })}>{$_("settings.presetSec.setDefault")}</Button>
						{/if}
						<IconButton label={$_("common.edit")} size="sm" variant="ghost" onclick={() => edit(p)}><Pencil /></IconButton>
						<IconButton label={$_("common.duplicate")} size="sm" variant="ghost" onclick={() => duplicate(p)}><Copy /></IconButton>
						<IconButton label={$_("common.export")} size="sm" variant="ghost" onclick={() => exportSt(p)}><Upload /></IconButton>
						<IconButton label={$_("common.delete")} size="sm" variant="ghost" onclick={() => remove(p)}><Trash /></IconButton>
					</Row>
				{/each}
			</Group>
		{:else}
			<Group flush>
				<EmptyState title={$_("settings.presetSec.emptyTitle")} description={$_("settings.presetSec.emptyDesc")}></EmptyState>
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
