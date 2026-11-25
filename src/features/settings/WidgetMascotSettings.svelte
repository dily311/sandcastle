<script lang="ts">
	import { onMount } from "svelte";
	import { _ } from "svelte-i18n";
	import { Plus, Trash2, Upload, X } from "@lucide/svelte";
	import { Button, Input, Select, Switch, Textarea } from "$shared/ui";
	import { IMAGE_FILES, pickFiles } from "$shared/ipc/dialog";
	import { fileSrc } from "$shared/ipc/file";
	import { call, commands } from "$shared/ipc/client";
	import { onEvent } from "$shared/ipc/events";
	import { toasts } from "$shared/stores/toast.svelte";
	import { library } from "$features/character/library.svelte";
	import ModelPicker from "./ModelPicker.svelte";
	import { chatApiOptions, orNull } from "./pluginSettingForm.svelte";
	import Group from "./Group.svelte";
	import Row from "./Row.svelte";
	import type {
		MascotBehaviorWeights_Serialize,
		MascotCharacterConfig_Serialize,
		WidgetMascotSetting_Serialize,
	} from "$ipc/bindings";
	import {
		createMascotConfig,
		defaultWidgetMascotSetting,
		MASCOT_DIALOGUE_FIELDS,
		MASCOT_IMAGE_SLOTS,
		type MascotDialogueKey,
		type MascotImageSlot,
	} from "$lib/features/impl/widget/mascot/simejiType";

	const src = { source: "widget.mascot" };

	let setting = $state<WidgetMascotSetting_Serialize>(defaultWidgetMascotSetting());
	let selectedId = $state<string | null>(null);
	let ready = $state(false);
	let uploadingSlot = $state<MascotImageSlot | null>(null);

	const selected = $derived(setting.characters.find((c) => c.id === selectedId) ?? null);
	const characterOptions = $derived([
		{ value: "", label: $_("settings.mascot.unlinked") },
		...library.characters.map((c) => ({ value: c.id, label: c.name })),
	]);
	const personaOptions = $derived([
		{ value: "", label: $_("settings.mascot.appDefaultPersona") },
		...library.personas.map((p) => ({ value: p.id, label: p.name })),
	]);
	const presetOptions = $derived([
		{ value: "", label: $_("settings.mascot.appDefaultPreset") },
		...library.presets.map((p) => ({ value: p.id, label: p.name })),
	]);

	const WEIGHT_ITEMS = $derived<{ key: keyof MascotBehaviorWeights_Serialize; label: string }[]>([
		{ key: "idle", label: $_("settings.mascot.wIdle") },
		{ key: "walk", label: $_("settings.mascot.wWalk") },
		{ key: "sitting", label: $_("settings.mascot.wSitting") },
		{ key: "sleep", label: $_("settings.mascot.wSleep") },
		{ key: "working", label: $_("settings.mascot.wWorking") },
	]);

	const totalWeight = $derived.by(() => {
		if (!selected) return 1;
		const w = selected.behaviorWeights;
		return Math.max(1, w.idle + w.walk + w.sitting + w.sleep + w.working);
	});

	onMount(() => {
		let unlisten: (() => void) | undefined;
		void (async () => {
			if (!library.loaded) await library.load().catch(() => {});
			const loaded = await call(commands.getWidgetMascotSettings(), { ...src, silent: true }).catch(() =>
				defaultWidgetMascotSetting(),
			);
			setting = loaded;
			selectedId = loaded.characters[0]?.id ?? null;
			ready = true;
			unlisten = await onEvent<WidgetMascotSetting_Serialize>("widget_mascot::changed", (next) => {
				setting = next;
				if (selectedId && !next.characters.some((c) => c.id === selectedId)) {
					selectedId = next.characters[0]?.id ?? null;
				}
			});
		})();
		return () => unlisten?.();
	});

	async function persist(next: WidgetMascotSetting_Serialize) {
		setting = next;
		await call(commands.saveWidgetMascotSettings(next), src).catch(() => {});
	}

	function patchGlobal(patch: Partial<WidgetMascotSetting_Serialize>) {
		void persist({ ...setting, ...patch });
	}

	function patchCharacter(id: string, patch: Partial<MascotCharacterConfig_Serialize>) {
		const next: WidgetMascotSetting_Serialize = {
			...setting,
			characters: setting.characters.map((c) => (c.id === id ? { ...c, ...patch } : c)),
		};
		void persist(next);
	}

	function addMascot() {
		const idx = setting.characters.length + 1;
		const created = createMascotConfig({ name: $_("settings.mascot.defaultName", { values: { idx } }) });
		const next: WidgetMascotSetting_Serialize = {
			...setting,
			characters: [...setting.characters, created],
		};
		selectedId = created.id;
		void persist(next);
	}

	async function removeMascot(id: string) {
		const target = setting.characters.find((c) => c.id === id);
		if (!target) return;
		const remaining = setting.characters.filter((c) => c.id !== id);
		if (selectedId === id) {
			selectedId = remaining[0]?.id ?? null;
		}
		await persist({ ...setting, characters: remaining });
		// 업로드된 이미지 파일 정리
		const allPaths = Object.values(target.images).flat();
		for (const p of allPaths) {
			void commands.deleteWidgetMascotImage(p).catch(() => {});
		}
		toasts.info($_("settings.mascot.deletedToast", { values: { name: target.name } }));
	}

	function onLinkCharacter(id: string, charIdRaw: string) {
		const characterId = orNull(charIdRaw);
		const cur = setting.characters.find((c) => c.id === id);
		if (!cur) return;
		const linked = characterId ? library.characters.find((c) => c.id === characterId) : null;
		const shouldRename = linked && (!cur.name.trim() || /^시메지\s*\d*$|^새 시메지$/.test(cur.name.trim()));
		patchCharacter(id, {
			characterId,
			...(shouldRename ? { name: linked.name } : {}),
		});
	}

	async function uploadImages(mascotId: string, slot: MascotImageSlot) {
		const files = await pickFiles([IMAGE_FILES], true);
		if (!files.length) return;
		uploadingSlot = slot;
		try {
			const uploaded: string[] = [];
			for (const filePath of files) {
				const savedPath = await call(commands.uploadWidgetMascotImage(mascotId, slot, filePath), src);
				if (savedPath) uploaded.push(savedPath);
			}
			const cur = setting.characters.find((c) => c.id === mascotId);
			if (!cur || !uploaded.length) return;
			patchCharacter(mascotId, {
				images: {
					...cur.images,
					[slot]: [...cur.images[slot], ...uploaded],
				},
			});
		} finally {
			uploadingSlot = null;
		}
	}

	async function removeImage(mascotId: string, slot: MascotImageSlot, index: number) {
		const cur = setting.characters.find((c) => c.id === mascotId);
		if (!cur) return;
		const targetPath = cur.images[slot][index];
		const nextList = cur.images[slot].filter((_, i) => i !== index);
		patchCharacter(mascotId, {
			images: { ...cur.images, [slot]: nextList },
		});
		if (targetPath) {
			void commands.deleteWidgetMascotImage(targetPath).catch(() => {});
		}
	}

	function updateWeight(mascotId: string, key: keyof MascotBehaviorWeights_Serialize, value: number) {
		const cur = setting.characters.find((c) => c.id === mascotId);
		if (!cur) return;
		patchCharacter(mascotId, {
			behaviorWeights: { ...cur.behaviorWeights, [key]: Math.max(0, Math.min(100, Math.round(value))) },
		});
	}

	function updateDialogue(mascotId: string, key: MascotDialogueKey, rawText: string) {
		const cur = setting.characters.find((c) => c.id === mascotId);
		if (!cur) return;
		const lines = rawText
			.split(/\r?\n/)
			.map((l) => l.trim())
			.filter(Boolean);
		patchCharacter(mascotId, {
			dialogues: { ...cur.dialogues, [key]: lines },
		});
	}

	function previewOf(c: MascotCharacterConfig_Serialize): string | null {
		const first = c.images.idle[0] ?? Object.values(c.images).flat()[0];
		if (first) return fileSrc(first);
		const linked = c.characterId ? library.characters.find((ch) => ch.id === c.characterId) : null;
		return linked?.avatar ? fileSrc(linked.avatar) : null;
	}
</script>

{#if ready}
	<Group title={$_("settings.mascot.pairChatGroup")}>
		<p class="group-desc">{$_("settings.mascot.pairChatGroupDesc")}</p>
		<Row label={$_("settings.mascot.pairChatEnable")} desc={$_("settings.mascot.pairChatEnableDesc")}>
			<Switch
				checked={setting.pairChatEnabled}
				label={$_("settings.mascot.pairChatEnable")}
				hideLabel
				onchange={(v) => patchGlobal({ pairChatEnabled: v })}
			/>
		</Row>
		{#if setting.pairChatEnabled}
			<Row label={$_("settings.mascot.pairChatAi")} desc={$_("settings.mascot.pairChatAiDesc")}>
				<Switch
					checked={setting.pairChatUseAi}
					label={$_("settings.mascot.pairChatAi")}
					hideLabel
					onchange={(v) => patchGlobal({ pairChatUseAi: v })}
				/>
			</Row>
			<Row label={$_("settings.mascot.pairChatInterval")} desc={$_("settings.mascot.pairChatIntervalDesc", { values: { sec: setting.pairChatIntervalSec } })}>
				<div class="range-inline">
					<input
						type="range"
						min="20"
						max="600"
						step="10"
						value={setting.pairChatIntervalSec}
						aria-label={$_("settings.mascot.pairChatIntervalAria")}
						onchange={(e) => patchGlobal({ pairChatIntervalSec: Number(e.currentTarget.value) })}
					/>
					<span class="num">{$_("settings.mascot.secUnit", { values: { sec: setting.pairChatIntervalSec } })}</span>
				</div>
			</Row>
		{/if}
	</Group>

	<Group title={$_("settings.mascot.listGroup")}>
		<p class="group-desc">{$_("settings.mascot.listGroupDesc")}</p>
		<div class="mascot-bar">
			{#each setting.characters as c (c.id)}
				{@const thumb = previewOf(c)}
				<div class="mascot-pill" class:active={selectedId === c.id}>
					<button type="button" class="pill-main" onclick={() => (selectedId = c.id)}>
						{#if thumb}
							<img src={thumb} alt="" class="pill-thumb" />
						{:else}
							<span class="pill-thumb placeholder">{c.name.slice(0, 1) || "?"}</span>
						{/if}
						<span class="pill-name">{c.name}</span>
					</button>
					<Switch
						checked={c.enabled}
						label={$_("settings.mascot.showAria", { values: { name: c.name } })}
						hideLabel
						onchange={(v) => patchCharacter(c.id, { enabled: v })}
					/>
				</div>
			{/each}
			<Button size="sm" variant="primary" onclick={addMascot}>
				<Plus size={14} /> {$_("settings.mascot.addBtn")}
			</Button>
		</div>
	</Group>

	{#if selected}
		<Group title={$_("settings.mascot.basicGroup")}>
			<Row label={$_("settings.mascot.displayName")}>
				<Input
					value={selected.name}
					maxlength={40}
					aria-label={$_("settings.mascot.nameAria")}
					onchange={(e) => patchCharacter(selected.id, { name: e.currentTarget.value.trim() || $_("settings.mascot.fallbackName") })}
				/>
			</Row>
			<Row label={$_("settings.mascot.showOnScreen")}>
				<Switch
					checked={selected.enabled}
					label={$_("settings.mascot.showOnScreen")}
					hideLabel
					onchange={(v) => patchCharacter(selected.id, { enabled: v })}
				/>
			</Row>
			<Row label={$_("settings.mascot.linkedChar")} desc={$_("settings.mascot.linkedCharDesc")}>
				<Select
					value={selected.characterId ?? ""}
					options={characterOptions}
					aria-label={$_("settings.mascot.linkedChar")}
					onchange={(e) => onLinkCharacter(selected.id, e.currentTarget.value)}
				/>
			</Row>
			<Row label={$_("settings.mascot.linkedPersona")} desc={$_("settings.mascot.linkedPersonaDesc")}>
				<Select
					value={selected.personaId ?? ""}
					options={personaOptions}
					aria-label={$_("settings.mascot.linkedPersona")}
					onchange={(e) => patchCharacter(selected.id, { personaId: orNull(e.currentTarget.value) })}
				/>
			</Row>
			<Row label={$_("settings.mascot.linkedApi")} desc={$_("settings.mascot.linkedApiDesc")}>
				<Select
					value={selected.apiProfileId ?? ""}
					options={chatApiOptions($_("settings.mascot.appDefaultApi"))}
					aria-label={$_("settings.mascot.linkedApi")}
					onchange={(e) => patchCharacter(selected.id, { apiProfileId: orNull(e.currentTarget.value), model: null })}
				/>
			</Row>
			{#if selected.apiProfileId}
				<Row label={$_("settings.mascot.linkedModel")} desc={$_("settings.mascot.linkedModelDesc")}>
					<ModelPicker
						profile={library.apis.find((a) => a.id === selected.apiProfileId)}
						value={selected.model || null}
						onchange={(m) => patchCharacter(selected.id, { model: m ?? null })}
						label={$_("settings.mascot.modelAria")}
					/>
				</Row>
			{/if}
			<Row label={$_("settings.mascot.linkedPreset")} desc={$_("settings.mascot.linkedPresetDesc")}>
				<Select
					value={selected.presetId ?? ""}
					options={presetOptions}
					aria-label={$_("settings.mascot.linkedPreset")}
					onchange={(e) => patchCharacter(selected.id, { presetId: orNull(e.currentTarget.value) })}
				/>
			</Row>
			<Row label={$_("settings.mascot.useAiDialogue")} desc={$_("settings.mascot.useAiDialogueDesc")}>
				<Switch
					checked={selected.useAiDialogue}
					label={$_("settings.mascot.useAiDialogue")}
					hideLabel
					onchange={(v) => patchCharacter(selected.id, { useAiDialogue: v })}
				/>
			</Row>
			<Row label={$_("settings.mascot.scale")} desc="{Math.round(selected.scale * 100)}%">
				<div class="range-inline">
					<input
						type="range"
						min="0.5"
						max="2.0"
						step="0.1"
						value={selected.scale}
						aria-label={$_("settings.mascot.scaleAria")}
						onchange={(e) => patchCharacter(selected.id, { scale: Number(e.currentTarget.value) })}
					/>
					<span class="num">{selected.scale.toFixed(1)}x</span>
				</div>
			</Row>
			<Row label={$_("settings.mascot.speed")} desc={$_("settings.mascot.speedDesc")}>
				<div class="range-inline">
					<input
						type="range"
						min="30"
						max="200"
						step="10"
						value={selected.speed}
						aria-label={$_("settings.mascot.speedAria")}
						onchange={(e) => patchCharacter(selected.id, { speed: Number(e.currentTarget.value) })}
					/>
					<span class="num">{Math.round(selected.speed)}</span>
				</div>
			</Row>
		</Group>

		<Group title={$_("settings.mascot.imagesGroup")}>
			<p class="group-desc">{$_("settings.mascot.imagesGroupDesc")}</p>
			<div class="slot-grid">
				{#each MASCOT_IMAGE_SLOTS as slot (slot.key)}
					{@const imgs = selected.images[slot.key] ?? []}
					{@const slabel = $_(`widgetExtra.imageSlots.${slot.key}.label`, { default: slot.label })}
					{@const shint = $_(`widgetExtra.imageSlots.${slot.key}.hint`, { default: slot.hint })}
					<div class="slot-card">
						<div class="slot-head">
							<div>
								<div class="slot-title">{slabel}</div>
								<div class="slot-hint">{shint}</div>
							</div>
							<Button
								size="sm"
								variant="ghost"
								disabled={uploadingSlot === slot.key}
								onclick={() => void uploadImages(selected.id, slot.key)}
							>
								<Upload size={13} /> {$_("settings.mascot.uploadBtn")}
							</Button>
						</div>
						{#if imgs.length > 0}
							<div class="thumb-list">
								{#each imgs as imgPath, idx (imgPath + idx)}
									<div class="thumb-item">
										<img src={fileSrc(imgPath) ?? ""} alt="{slabel} {idx + 1}" />
										<button
											type="button"
											class="thumb-del"
											aria-label={$_("settings.mascot.deleteImgAria")}
											onclick={() => void removeImage(selected.id, slot.key, idx)}
										>
											<X size={11} />
										</button>
									</div>
								{/each}
							</div>
						{:else}
							<div class="slot-empty">{$_("settings.mascot.noImages")}</div>
						{/if}
					</div>
				{/each}
			</div>
		</Group>

		<Group title={$_("settings.mascot.behaviorGroup")}>
			<p class="group-desc">{$_("settings.mascot.behaviorGroupDesc")}</p>
			<Row label={$_("settings.mascot.actionInterval")} desc={$_("settings.mascot.actionIntervalDesc", { values: { sec: selected.actionIntervalSec } })}>
				<div class="range-inline">
					<input
						type="range"
						min="2"
						max="30"
						step="1"
						value={selected.actionIntervalSec}
						aria-label={$_("settings.mascot.actionInterval")}
						onchange={(e) => patchCharacter(selected.id, { actionIntervalSec: Number(e.currentTarget.value) })}
					/>
					<span class="num">{$_("settings.mascot.secUnit", { values: { sec: selected.actionIntervalSec } })}</span>
				</div>
			</Row>
			{#each WEIGHT_ITEMS as item (item.key)}
				{@const val = selected.behaviorWeights[item.key]}
				{@const pct = Math.round((val / totalWeight) * 100)}
				<Row label={item.label} desc={$_("settings.mascot.weightPct", { values: { pct } })}>
					<div class="range-inline">
						<input
							type="range"
							min="0"
							max="100"
							step="5"
							value={val}
							aria-label={$_("settings.mascot.weightAria", { values: { label: item.label } })}
							onchange={(e) => updateWeight(selected.id, item.key, Number(e.currentTarget.value))}
						/>
						<span class="num">{val}</span>
					</div>
				</Row>
			{/each}
		</Group>

		<Group title={$_("settings.mascot.dialogueGroup")}>
			<p class="group-desc">{$_("settings.mascot.dialogueGroupDesc")}</p>
			<div class="dialogue-list">
				{#each MASCOT_DIALOGUE_FIELDS as field (field.key)}
					{@const flabel = $_(`widgetExtra.dialogueFields.${field.key}.label`, { default: field.label })}
					{@const fhint = $_(`widgetExtra.dialogueFields.${field.key}.hint`, { default: field.hint })}
					<label class="dialogue-field">
						<div class="dialogue-head">
							<span class="dialogue-label">{flabel}</span>
							<span class="dialogue-hint">{fhint}</span>
						</div>
						<Textarea
							value={(selected.dialogues[field.key] ?? []).join("\n")}
							rows={2}
							autosize
							maxRows={6}
							placeholder={$_("settings.mascot.dialoguePlaceholder")}
							aria-label={flabel}
							onchange={(e) => updateDialogue(selected.id, field.key, e.currentTarget.value)}
						/>
					</label>
				{/each}
			</div>
		</Group>

		<div class="danger-row">
			<Button size="sm" variant="danger" onclick={() => void removeMascot(selected.id)}>
				<Trash2 size={14} /> {$_("settings.mascot.deleteNamedBtn", { values: { name: selected.name } })}
			</Button>
		</div>
	{:else}
		<div class="empty-mascot">
			<p>{$_("settings.mascot.emptyDesc")}</p>
			<Button size="sm" variant="primary" onclick={addMascot}>
				<Plus size={14} /> {$_("settings.mascot.createFirstBtn")}
			</Button>
		</div>
	{/if}
{/if}

<style>
	.group-desc {
		margin: 0;
		padding-top: var(--space-3);
		font-size: var(--text-xs);
		color: var(--text-faint);
	}
	.mascot-bar {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-3) 0;
	}
	.mascot-pill {
		display: inline-flex;
		align-items: center;
		gap: var(--space-2);
		padding: 5px 10px 5px 6px;
		border: 1px solid var(--border-strong);
		border-radius: 999px;
		background: var(--surface);
		transition: border-color 0.15s ease, background 0.15s ease;
	}
	.mascot-pill.active {
		border-color: var(--accent);
		background: color-mix(in srgb, var(--accent) 10%, var(--surface));
	}
	.pill-main {
		display: inline-flex;
		align-items: center;
		gap: 8px;
		border: none;
		background: transparent;
		color: var(--text);
		cursor: pointer;
		padding: 0;
	}
	.pill-thumb {
		width: 24px;
		height: 24px;
		border-radius: 50%;
		object-fit: cover;
		background: var(--bg-sunken);
	}
	.pill-thumb.placeholder {
		display: grid;
		place-items: center;
		font-size: 11px;
		font-weight: 700;
		color: var(--text-muted);
	}
	.pill-name {
		font-size: var(--text-sm);
		font-weight: 600;
	}
	.range-inline {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		min-width: 170px;
	}
	.range-inline input[type="range"] {
		flex: 1;
		accent-color: var(--accent);
		cursor: pointer;
	}
	.range-inline .num {
		width: 44px;
		text-align: right;
		font-size: var(--text-xs);
		font-variant-numeric: tabular-nums;
		color: var(--text-muted);
	}
	.slot-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
		gap: var(--space-3);
		padding: var(--space-3) 0;
	}
	.slot-card {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		padding: var(--space-3);
		border: 1px solid var(--border);
		border-radius: var(--radius-sm);
		background: var(--bg-sunken);
	}
	.slot-head {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: var(--space-2);
	}
	.slot-title {
		font-size: var(--text-sm);
		font-weight: 600;
		color: var(--text);
	}
	.slot-hint {
		font-size: var(--text-xs);
		color: var(--text-faint);
	}
	.thumb-list {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
	}
	.thumb-item {
		position: relative;
		width: 52px;
		height: 52px;
		border-radius: var(--radius-xs);
		border: 1px solid var(--border-strong);
		background: var(--surface);
		overflow: hidden;
	}
	.thumb-item img {
		width: 100%;
		height: 100%;
		object-fit: contain;
	}
	.thumb-del {
		position: absolute;
		top: 2px;
		right: 2px;
		width: 18px;
		height: 18px;
		border-radius: 50%;
		border: none;
		background: rgba(0, 0, 0, 0.65);
		color: #fff;
		display: grid;
		place-items: center;
		cursor: pointer;
		padding: 0;
	}
	.slot-empty {
		font-size: var(--text-xs);
		color: var(--text-faint);
		padding: var(--space-2) 0;
	}
	.dialogue-list {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		padding: var(--space-3) 0;
	}
	.dialogue-field {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}
	.dialogue-head {
		display: flex;
		flex-wrap: wrap;
		align-items: baseline;
		gap: 8px;
	}
	.dialogue-label {
		font-size: var(--text-sm);
		font-weight: 600;
		color: var(--text);
	}
	.dialogue-hint {
		font-size: var(--text-xs);
		color: var(--text-faint);
	}
	.danger-row {
		display: flex;
		justify-content: flex-end;
		padding-top: var(--space-2);
	}
	.empty-mascot {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--space-3);
		padding: var(--space-6);
		border: 1px dashed var(--border-strong);
		border-radius: var(--radius-md);
		color: var(--text-muted);
		font-size: var(--text-sm);
		text-align: center;
	}
	.empty-mascot p {
		margin: 0;
	}
</style>
