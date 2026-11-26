<script lang="ts">
	// 빠른 답장 세트 (5-E-20-5-3·4, ADR-025). 정규식처럼 전체·캐릭터 전용·프리셋 전용으로 묶는다. 켜짐/꺼짐은 코어 플러그인 slash
	import { onMount } from "svelte";
	import { _ } from "svelte-i18n";
	import { ChevronRight, Download, Plus, Trash2 } from "@lucide/svelte";
	import { Button, EmptyState, IconButton, Input, Select, Switch, Textarea } from "$shared/ui";
	import { localizeImportWarning } from "$lib/utils/backendI18n";
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

	const SCOPES = $derived<{ value: Scope; label: string }[]>([
		{ value: "global", label: $_("settings.quickReplySec.scopeGlobal") },
		{ value: "character", label: $_("settings.quickReplySec.scopeChar") },
		{ value: "preset", label: $_("settings.quickReplySec.scopePreset") },
	]);
	const ownerName = (s: QuickReplySet) =>
		s.scope === "character" ? (library.character(s.scopeId)?.name ?? $_("settings.quickReplySec.deletedChar")) : (library.presets.find((p) => p.id === s.scopeId)?.name ?? $_("settings.quickReplySec.deletedPreset"));
	const groups = $derived(
		(["global", "character", "preset"] as const)
			.map((scope) => ({
				scope,
				title: scope === "global" ? $_("settings.quickReplySec.groupGlobal") : scope === "character" ? $_("settings.quickReplySec.scopeChar") : $_("settings.quickReplySec.scopePreset"),
				items: quickReplies.sets.filter((s) => s.scope === scope),
			}))
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
		if (scope !== "global" && !first) return toasts.error(scope === "character" ? $_("settings.quickReplySec.noChar") : $_("settings.quickReplySec.noPreset"));
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
		const s = await quickReplies.save({ id: null, name: $_("settings.quickReplySec.newSetName"), scope: "global", scopeId: null, enabled: true, autoSend: false, items: [{ id: newId(), label: "", message: "", hidden: false }], sortOrder: quickReplies.sets.length });
		open[s.id] = true;
	}
	async function removeSet(s: QuickReplySet) {
		if (!(await confirmAction({ title: $_("settings.quickReplySec.deleteTitle"), message: $_("settings.quickReplySec.deleteMsg", { values: { name: s.name, count: s.items.length } }), confirmLabel: $_("common.delete"), danger: true }))) return;
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
		toasts.success($_("settings.quickReplySec.importedToast", { values: { sets: inputs.length, buttons } }));
		if (warnings.length) toasts.info(warnings.map(localizeImportWarning).join(" · "));
	}
</script>

{#snippet setCard(s: QuickReplySet)}
	<div class="set">
		<div class="set-head">
			<button class="toggle" aria-expanded={!!open[s.id]} onclick={() => (open[s.id] = !open[s.id])}>
				<ChevronRight class="caret" />
				<b>{s.name || $_("settings.quickReplySec.unnamedSet")}</b>
				<small>{$_("settings.quickReplySec.buttonCount", { values: { count: s.items.length } })}</small>
			</button>
			<Switch checked={s.enabled} label={$_("settings.quickReplySec.useSetAria", { values: { name: s.name } })} hideLabel onchange={(v) => ((s.enabled = v), changed(s))} />
		</div>
		{#if open[s.id]}
			<div class="set-body">
				<Row label={$_("settings.quickReplySec.name")}>
					<Input bind:value={s.name} oninput={() => changed(s)} aria-label={$_("settings.quickReplySec.setNameAria")} />
					<IconButton label={$_("settings.quickReplySec.deleteTitle")} size="sm" variant="ghost" onclick={() => removeSet(s)}><Trash2 /></IconButton>
				</Row>
				<Row label={$_("settings.quickReplySec.applyScope")}>
					<Select value={s.scope} options={SCOPES} onchange={(e) => setScope(s, e.currentTarget.value as Scope)} aria-label={$_("settings.quickReplySec.scopeAria")} />
					{#if s.scope !== "global"}
						<Select value={s.scopeId ?? ""} options={ownerOptions(s.scope)} onchange={(e) => ((s.scopeId = e.currentTarget.value), changed(s))} aria-label={$_("settings.quickReplySec.targetAria")} />
					{/if}
				</Row>
				<Row label={$_("settings.quickReplySec.autoSend")} desc={$_("settings.quickReplySec.autoSendDesc")}>
					<Switch checked={s.autoSend} label={$_("settings.quickReplySec.autoSend")} hideLabel onchange={(v) => ((s.autoSend = v), changed(s))} />
				</Row>
				<div class="items">
					{#each s.items as q, j (q.id)}
						<div class="item">
							<div class="head">
								<Input bind:value={q.label} oninput={() => changed(s)} placeholder={$_("settings.quickReplySec.buttonNamePlaceholder")} aria-label={$_("settings.quickReplySec.buttonNamePlaceholder")} />
								<Switch checked={!q.hidden} label={$_("settings.quickReplySec.showInBarAria")} hideLabel onchange={(v) => ((q.hidden = !v), changed(s))} />
								<IconButton label={$_("settings.quickReplySec.deleteButtonAria")} size="sm" variant="ghost" onclick={() => (s.items.splice(j, 1), changed(s))}><Trash2 /></IconButton>
							</div>
							<Textarea bind:value={q.message} oninput={() => changed(s)} rows={2} autosize maxRows={12} placeholder={$_("settings.quickReplySec.contentPlaceholder")} aria-label={$_("settings.quickReplySec.contentAria")} />
						</div>
					{/each}
					<div><Button size="sm" variant="ghost" onclick={() => (s.items.push({ id: newId(), label: "", message: "", hidden: false }), changed(s))}>{#snippet icon()}<Plus />{/snippet}{$_("settings.quickReplySec.addButton")}</Button></div>
				</div>
			</div>
		{/if}
	</div>
{/snippet}

<Section title={$_("settings.quickReplySec.title")} lead={$_("settings.quickReplySec.lead")}>
	{#snippet actions()}
		<Button size="sm" onclick={importSt}>{#snippet icon()}<Download />{/snippet}{$_("common.import")}</Button>
		<Button size="sm" variant="primary" onclick={addSet}>{#snippet icon()}<Plus />{/snippet}{$_("common.add")}</Button>
	{/snippet}
	<Group>
		<Row label={$_("settings.quickReplySec.aiLimit")} desc={$_("settings.quickReplySec.aiLimitDesc", { values: { max: MAX_AI_LIMIT } })}>
			<Input type="number" min={1} max={MAX_AI_LIMIT} value={String(aiLimit())} onchange={(e) => setAiLimit(e.currentTarget.value)} aria-label={$_("settings.quickReplySec.aiLimit")} />
		</Row>
	</Group>
	{#if quickReplies.loaded && !quickReplies.sets.length}
		<Group flush>
			<EmptyState title={$_("settings.quickReplySec.emptyTitle")} description={$_("settings.quickReplySec.emptyDesc")}></EmptyState>
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
