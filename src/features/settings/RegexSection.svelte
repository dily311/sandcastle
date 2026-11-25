<script lang="ts">
	import { regexVersion } from "$shared/stores/regexVersion.svelte";
	import { drops, extOf, storeFile } from "$app/drop.svelte";
	import { onMount } from "svelte";
	import { _ } from "svelte-i18n";
	import { ChevronRight, Download, Pencil, Plus, Trash2 } from "@lucide/svelte";
	import type { RegexPlacement, RegexScope, RegexScript_Serialize as RegexScript, RegexTarget } from "$ipc/bindings";
	import { Button, EmptyState, IconButton, Switch } from "$shared/ui";
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

	const PLACE = $derived<Record<RegexPlacement, string>>({
		user_input: $_("settings.regexSec.placeUser"),
		ai_output: $_("settings.regexSec.placeAi"),
		world_info: $_("settings.regexSec.placeWi"),
	});
	const TARGET = $derived<Record<RegexTarget, string>>({
		stored: $_("settings.regexSec.targetStored"),
		prompt: $_("settings.regexSec.targetPrompt"),
		display: $_("settings.regexSec.targetDisplay"),
	});

	function scopeName(s: RegexScript) {
		if (s.scope === "global") return $_("common.all");
		if (s.scope === "character") return library.character(s.scopeId)?.name ?? "—";
		return library.presets.find((p) => p.id === s.scopeId)?.name ?? "—";
	}
	const groups = $derived(
		(["global", "character", "preset"] as RegexScope[])
			.map((scope) => ({
				scope,
				title:
					scope === "global"
						? $_("settings.regexSec.scopeGlobal")
						: scope === "character"
							? $_("settings.regexSec.scopeCharacter")
							: $_("settings.regexSec.scopePreset"),
				items: scripts.filter((s) => s.scope === scope),
			}))
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
		regexVersion.bump();
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
		if (!(await confirmAction({ title: $_("settings.more.deleteRegexTitle"), message: $_("settings.more.deleteRegexMsg", { values: { name: s.name } }), confirmLabel: $_("common.delete"), danger: true }))) return;
		await call(commands.deleteRegexScript(s.id), src);
		regexVersion.bump();
		scripts = scripts.filter((x) => x.id !== s.id);
	}
	async function importFile() {
		const [path] = await pickFiles([JSON_FILES], false);
		if (path) await importPath(path);
	}
	$effect(() =>
		drops.register({
			label: $_("settings.more.dropRegexLabel"),
			hint: $_("settings.more.dropRegexHint"),
			accept: (f) => extOf(f) === "json",
			ondrop: async (files) => {
				for (const f of files) await importPath(await storeFile(f)).catch(() => {});
			},
		}),
	);
	async function importPath(path: string) {
		const r = await call(commands.importSillytavern(path, null, null, null), src);
		regexVersion.bump();
		if (r.kind === "regex") toasts.success($_("settings.more.regexImported", { values: { count: r.scripts.length } }));
		else if (r.kind === "preset") toasts.success($_("settings.more.presetRegexImported", { values: { name: r.name, count: r.regexCount } }));
		else toasts.info($_("settings.more.notRegexFile"));
		for (const w of r.warnings) toasts.info(w);
		await Promise.all([load(), library.load()]);
	}

</script>

{#snippet scriptRow(s: RegexScript)}
	<Row label={s.name} desc="{s.placement.map((p) => PLACE[p]).join(' · ')} → {TARGET[s.target]}">
		<IconButton size="sm" label={$_("settings.more.editNamed", { values: { name: s.name } })} onclick={() => startEdit(s)}><Pencil /></IconButton>
		<IconButton size="sm" variant="danger" label={$_("settings.more.deleteNamed", { values: { name: s.name } })} onclick={() => remove(s)}><Trash2 /></IconButton>
		<Switch checked={s.enabled} label={$_("settings.more.useNamed", { values: { name: s.name } })} hideLabel onchange={(v) => toggle(s, v)} />
	</Row>
{/snippet}

<Section title={$_("settings.regexSec.title")} lead={$_("settings.regexSec.lead")}>
	{#snippet actions()}
		<Button size="sm" onclick={importFile}>{#snippet icon()}<Download />{/snippet}{$_("common.import")}</Button>
		<Button size="sm" variant="primary" onclick={startNew}>{#snippet icon()}<Plus />{/snippet}{$_("settings.regexSec.new")}</Button>
	{/snippet}
	{#if loaded && !scripts.length}
		<Group flush>
			<EmptyState title={$_("settings.regexSec.emptyTitle")} description={$_("settings.regexSec.emptyDesc")}></EmptyState>
		</Group>
	{/if}
	{#each groups as g (g.scope)}
		<Group title={g.title}>
			{#if g.scope === "global"}
				{#each g.items as s (s.id)}{@render scriptRow(s)}{/each}
			{:else}
				{#each owners(g.items) as o (o.id)}
					<details class="owner">
						<summary class="owner-head">
							<ChevronRight class="caret" />
							<b>{o.name}</b>
							<small>{$_("settings.regexSec.ownerCount", { values: { total: o.items.length, on: o.on } })}</small>
						</summary>
						<div class="owner-body">
							{#each o.items as s (s.id)}{@render scriptRow(s)}{/each}
						</div>
					</details>
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
		list-style: none;
		user-select: none;
	}
	.owner-head::-webkit-details-marker {
		display: none;
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
	.owner[open] > .owner-head :global(.caret) {
		transform: rotate(90deg);
	}
	.owner-body {
		padding-left: var(--space-4);
	}
</style>
