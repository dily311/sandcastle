<script lang="ts">
	import { onMount } from "svelte";
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
		{ value: "", label: "연결 안 함 (직접 설정)" },
		...library.characters.map((c) => ({ value: c.id, label: c.name })),
	]);
	const personaOptions = $derived([
		{ value: "", label: "앱 기본 페르소나" },
		...library.personas.map((p) => ({ value: p.id, label: p.name })),
	]);
	const presetOptions = $derived([
		{ value: "", label: "앱 기본 프리셋" },
		...library.presets.map((p) => ({ value: p.id, label: p.name })),
	]);

	const WEIGHT_ITEMS: { key: keyof MascotBehaviorWeights_Serialize; label: string }[] = [
		{ key: "idle", label: "가만히 있기" },
		{ key: "walk", label: "걸어다니기" },
		{ key: "sitting", label: "앉아있기" },
		{ key: "sleep", label: "잠자기" },
		{ key: "working", label: "활동 / 혼잣말" },
	];

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
		const created = createMascotConfig({ name: `시메지 ${idx}` });
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
		toasts.info(`'${target.name}' 시메지를 삭제함`);
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
	<Group title="시메지 간 대화">
		<p class="group-desc">여러 시메지를 동시에 띄워두면 서로 다가가서 대화를 나눕니다</p>
		<Row label="시메지끼리 서로 대화하기" desc="2명 이상의 시메지가 활성화되어 있을 때 동작">
			<Switch
				checked={setting.pairChatEnabled}
				label="시메지끼리 서로 대화하기"
				hideLabel
				onchange={(v) => patchGlobal({ pairChatEnabled: v })}
			/>
		</Row>
		{#if setting.pairChatEnabled}
			<Row label="AI로 대화 생성" desc="연결된 API·캐릭터 정보로 대화를 만듦 (실패 시 기본 대사 사용)">
				<Switch
					checked={setting.pairChatUseAi}
					label="AI로 대화 생성"
					hideLabel
					onchange={(v) => patchGlobal({ pairChatUseAi: v })}
				/>
			</Row>
			<Row label="대화 주기" desc="{setting.pairChatIntervalSec}초마다 시메지들이 만나 대화">
				<div class="range-inline">
					<input
						type="range"
						min="20"
						max="600"
						step="10"
						value={setting.pairChatIntervalSec}
						aria-label="시메지 간 대화 주기"
						onchange={(e) => patchGlobal({ pairChatIntervalSec: Number(e.currentTarget.value) })}
					/>
					<span class="num">{setting.pairChatIntervalSec}초</span>
				</div>
			</Row>
		{/if}
	</Group>

	<Group title="시메지 목록">
		<p class="group-desc">여러 시메지를 추가하고 켤 수 있습니다</p>
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
						label="{c.name} 표시"
						hideLabel
						onchange={(v) => patchCharacter(c.id, { enabled: v })}
					/>
				</div>
			{/each}
			<Button size="sm" variant="primary" onclick={addMascot}>
				<Plus size={14} /> 시메지 추가
			</Button>
		</div>
	</Group>

	{#if selected}
		<Group title="연결 및 기본 설정">
			<Row label="표시 이름">
				<Input
					value={selected.name}
					maxlength={40}
					aria-label="시메지 이름"
					onchange={(e) => patchCharacter(selected.id, { name: e.currentTarget.value.trim() || "시메지" })}
				/>
			</Row>
			<Row label="화면에 띄우기">
				<Switch
					checked={selected.enabled}
					label="화면에 띄우기"
					hideLabel
					onchange={(v) => patchCharacter(selected.id, { enabled: v })}
				/>
			</Row>
			<Row label="연결 캐릭터" desc="시메지의 성격·말투·기본 아바타에 사용">
				<Select
					value={selected.characterId ?? ""}
					options={characterOptions}
					aria-label="연결 캐릭터"
					onchange={(e) => onLinkCharacter(selected.id, e.currentTarget.value)}
				/>
			</Row>
			<Row label="연결 페르소나" desc={"대사의 {{user}} 이름과 대화 맥락에 사용"}>
				<Select
					value={selected.personaId ?? ""}
					options={personaOptions}
					aria-label="연결 페르소나"
					onchange={(e) => patchCharacter(selected.id, { personaId: orNull(e.currentTarget.value) })}
				/>
			</Row>
			<Row label="연결 API" desc="대화하기·선물·시메지 간 대화 생성에 사용">
				<Select
					value={selected.apiProfileId ?? ""}
					options={chatApiOptions("앱 기본 API")}
					aria-label="연결 API"
					onchange={(e) => patchCharacter(selected.id, { apiProfileId: orNull(e.currentTarget.value), model: null })}
				/>
			</Row>
			{#if selected.apiProfileId}
				<Row label="연결 모델" desc="비우면 해당 API의 기본 모델 사용">
					<ModelPicker
						profile={library.apis.find((a) => a.id === selected.apiProfileId)}
						value={selected.model || null}
						onchange={(m) => patchCharacter(selected.id, { model: m ?? null })}
						label="시메지 모델"
					/>
				</Row>
			{/if}
			<Row label="연결 프리셋" desc="프롬프트 구성 및 생성 옵션에 사용">
				<Select
					value={selected.presetId ?? ""}
					options={presetOptions}
					aria-label="연결 프리셋"
					onchange={(e) => patchCharacter(selected.id, { presetId: orNull(e.currentTarget.value) })}
				/>
			</Row>
			<Row label="AI 대사 생성 사용" desc="대화하기·선물 반응 시 연결된 API로 응답 생성">
				<Switch
					checked={selected.useAiDialogue}
					label="AI 대사 생성 사용"
					hideLabel
					onchange={(v) => patchCharacter(selected.id, { useAiDialogue: v })}
				/>
			</Row>
			<Row label="크기 배율" desc="{Math.round(selected.scale * 100)}%">
				<div class="range-inline">
					<input
						type="range"
						min="0.5"
						max="2.0"
						step="0.1"
						value={selected.scale}
						aria-label="시메지 크기 배율"
						onchange={(e) => patchCharacter(selected.id, { scale: Number(e.currentTarget.value) })}
					/>
					<span class="num">{selected.scale.toFixed(1)}x</span>
				</div>
			</Row>
			<Row label="이동 속도" desc="화면에서 걸어다니는 속도">
				<div class="range-inline">
					<input
						type="range"
						min="30"
						max="200"
						step="10"
						value={selected.speed}
						aria-label="시메지 이동 속도"
						onchange={(e) => patchCharacter(selected.id, { speed: Number(e.currentTarget.value) })}
					/>
					<span class="num">{Math.round(selected.speed)}</span>
				</div>
			</Row>
		</Group>

		<Group title="캐릭터 이미지 업로드">
			<p class="group-desc">상태별로 이미지를 여러 장 올리면 프레임 애니메이션으로 재생됩니다. 비어 있으면 기본(대기) 이미지를 사용합니다.</p>
			<div class="slot-grid">
				{#each MASCOT_IMAGE_SLOTS as slot (slot.key)}
					{@const imgs = selected.images[slot.key] ?? []}
					<div class="slot-card">
						<div class="slot-head">
							<div>
								<div class="slot-title">{slot.label}</div>
								<div class="slot-hint">{slot.hint}</div>
							</div>
							<Button
								size="sm"
								variant="ghost"
								disabled={uploadingSlot === slot.key}
								onclick={() => void uploadImages(selected.id, slot.key)}
							>
								<Upload size={13} /> 올리기
							</Button>
						</div>
						{#if imgs.length > 0}
							<div class="thumb-list">
								{#each imgs as imgPath, idx (imgPath + idx)}
									<div class="thumb-item">
										<img src={fileSrc(imgPath) ?? ""} alt="{slot.label} {idx + 1}" />
										<button
											type="button"
											class="thumb-del"
											aria-label="이미지 삭제"
											onclick={() => void removeImage(selected.id, slot.key, idx)}
										>
											<X size={11} />
										</button>
									</div>
								{/each}
							</div>
						{:else}
							<div class="slot-empty">이미지 없음</div>
						{/if}
					</div>
				{/each}
			</div>
		</Group>

		<Group title="행동 빈도">
			<p class="group-desc">시메지가 얼마나 자주, 어떤 행동을 할지 비중을 조절합니다</p>
			<Row label="행동 전환 주기" desc="평균 {selected.actionIntervalSec}초마다 다음 행동을 결정">
				<div class="range-inline">
					<input
						type="range"
						min="2"
						max="30"
						step="1"
						value={selected.actionIntervalSec}
						aria-label="행동 전환 주기"
						onchange={(e) => patchCharacter(selected.id, { actionIntervalSec: Number(e.currentTarget.value) })}
					/>
					<span class="num">{selected.actionIntervalSec}초</span>
				</div>
			</Row>
			{#each WEIGHT_ITEMS as item (item.key)}
				{@const val = selected.behaviorWeights[item.key]}
				{@const pct = Math.round((val / totalWeight) * 100)}
				<Row label={item.label} desc="비중 {pct}%">
					<div class="range-inline">
						<input
							type="range"
							min="0"
							max="100"
							step="5"
							value={val}
							aria-label="{item.label} 빈도"
							onchange={(e) => updateWeight(selected.id, item.key, Number(e.currentTarget.value))}
						/>
						<span class="num">{val}</span>
					</div>
				</Row>
			{/each}
		</Group>

		<Group title="상황별 대사">
			<p class="group-desc">{"한 줄에 대사 하나씩 입력하면 무작위로 출력됩니다. {{user}}, {{char}}, {{notify}} 치환을 지원합니다."}</p>
			<div class="dialogue-list">
				{#each MASCOT_DIALOGUE_FIELDS as field (field.key)}
					<label class="dialogue-field">
						<div class="dialogue-head">
							<span class="dialogue-label">{field.label}</span>
							<span class="dialogue-hint">{field.hint}</span>
						</div>
						<Textarea
							value={(selected.dialogues[field.key] ?? []).join("\n")}
							rows={2}
							autosize
							maxRows={6}
							placeholder="대사를 한 줄씩 입력..."
							aria-label={field.label}
							onchange={(e) => updateDialogue(selected.id, field.key, e.currentTarget.value)}
						/>
					</label>
				{/each}
			</div>
		</Group>

		<div class="danger-row">
			<Button size="sm" variant="danger" onclick={() => void removeMascot(selected.id)}>
				<Trash2 size={14} /> '{selected.name}' 시메지 삭제
			</Button>
		</div>
	{:else}
		<div class="empty-mascot">
			<p>등록된 시메지가 없습니다. 시메지를 추가해 바탕화면에 띄워보세요!</p>
			<Button size="sm" variant="primary" onclick={addMascot}>
				<Plus size={14} /> 첫 시메지 만들기
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
