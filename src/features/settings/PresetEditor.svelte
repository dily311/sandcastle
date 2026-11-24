<script lang="ts">
	import { ChevronDown, ChevronRight, GripVertical, Plus, Trash2 } from "@lucide/svelte";
	import { flip } from "svelte/animate";
	import type { PromptBlocksInput_Deserialize as BlockInput, PromptPresetDetail_Serialize as Detail } from "$ipc/bindings";
	import { Badge, Button, Field, Input, Select, Slider, Switch, Textarea } from "$shared/ui";
	import { appSetting } from "$shared/stores/appSetting.svelte";
	import { library } from "$features/character/library.svelte";
	import { reasoningOf } from "./providers";
	import Group from "./Group.svelte";
	import Row from "./Row.svelte";
	import PresetUtilPrompts from "./PresetUtilPrompts.svelte";

	interface Props {
		detail: Detail;
		saving: boolean;
		/** 저장한 결과를 돌려주면 새 블록의 id를 받아 둠 (자동 저장에서 같은 블록이 또 생기지 않게) */
		onsave: (preset: Detail["preset"], blocks: BlockInput[]) => void | Promise<Detail | void>;
		oncancel?: () => void;
		/** 빠른 편집 (대화 패널): 이름·설명은 숨김 */
		quick?: boolean;
		/** 저장·취소 버튼 없이 고치면 잠시 뒤 저절로 저장 */
		autosave?: boolean;
		/** 이 프리셋으로 대화할 모델 (추론 강도를 고를 수 있는지 판단). 없으면 앱 기본 API의 모델 */
		model?: string | null;
	}
	let { detail, saving, onsave, oncancel, quick = false, autosave = false, model = null }: Props = $props();

	// 추론 강도: 모델 목록(default_text_models.json)에 그 값을 받는다고 적힌 모델일 때만 보인다
	const effModel = $derived(model || library.apis.find((a) => a.id === appSetting.value?.selectedApiProfile)?.model || null);
	const reasoning = $derived(reasoningOf(effModel));

	// 마커 블록: 내용 대신 앱이 채움
	const MARKERS: Record<string, string> = {
		chardescription: "캐릭터 설명",
		personadescription: "페르소나 설명",
		dialogueexamples: "대화 예시",
		mesexamples: "대화 예시",
		worldinfo: "월드인포",
		worldinfobefore: "월드인포",
		worldinfoafter: "월드인포",
		chathistory: "대화 기록",
		charpersonality: "비워 둠 (설명에 합쳐짐)",
		scenario: "비워 둠 (설명에 합쳐짐)",
	};
	function markerOf(b: { identifier: string; extraJson?: unknown }) {
		const key = b.identifier.toLowerCase().replace(/_/g, "");
		if (MARKERS[key]) return MARKERS[key];
		const extra = b.extraJson as { marker?: boolean } | null | undefined;
		return extra?.marker ? "앱이 채움" : null;
	}

	// ---- 편집 사본 (처음 받은 detail 기준) ----
	// svelte-ignore state_referenced_locally
	const p = detail.preset;
	let name = $state(p.name);
	let description = $state(p.description ?? "");
	const settings0 = { ...((p.configJson as object) ?? {}), ...((p.settingsJson as object) ?? {}) } as Record<string, unknown>;
	const pick = (...keys: string[]) => keys.map((k) => settings0[k]).find((v) => v !== undefined && v !== null);
	const numStr = (v: unknown) => (typeof v === "number" ? String(v) : "");
	let temperature = $state(numStr(pick("temperature", "temp_openai")));
	let maxTokens = $state(numStr(pick("max_tokens", "maxTokens", "openai_max_tokens")));
	let contextSize = $state(numStr(pick("context_size", "contextSize", "openai_max_context")));
	// 월드인포 비율: 저장은 0~1, 화면은 %
	const wi0 = pick("world_info_ratio", "worldInfoRatio", "world_info_budget");
	let worldInfoPercent = $state(typeof wi0 === "number" ? String(Math.round(wi0 > 1 ? Math.min(wi0, 100) : wi0 * 100)) : "");
	let topP = $state(numStr(pick("top_p", "topP")));
	let topK = $state(numStr(pick("top_k", "topK")));
	let stream = $state(pick("stream", "stream_openai", "streaming") !== false);
	// "auto"는 정하지 않음 (SillyTavern 프리셋)
	const re0 = pick("reasoning_effort", "reasoningEffort");
	let effort = $state(typeof re0 === "string" && re0.toLowerCase() !== "auto" ? re0.toLowerCase() : "");
	// 유틸 프롬프트 (5-F-7-3, ADR-030): 프리셋 > 전역 > 기본. 글만 받는다
	const ap0 = settings0.appPrompts;
	let appPrompts = $state<Record<string, string>>(
		ap0 && typeof ap0 === "object" ? Object.fromEntries(Object.entries(ap0).filter((e): e is [string, string] => typeof e[1] === "string" && !!e[1].trim())) : {},
	);

	type Block = BlockInput & { key: string };
	let seq = 0;
	let blocks = $state<Block[]>(
		// svelte-ignore state_referenced_locally
		detail.blocks.map((b) => ({
			key: `b${seq++}`,
			id: b.id,
			presetId: b.presetId,
			name: b.name,
			identifier: b.identifier,
			blockType: b.blockType,
			role: b.role,
			content: b.content,
			enabled: b.enabled,
			position: b.position,
			depth: b.depth,
			sortOrder: b.sortOrder,
			triggersJson: b.triggersJson,
			extraJson: b.extraJson,
		})),
	);
	let open = $state<string | null>(null);

	function moveTo(from: number, to: number) {
		if (from === to || to < 0 || to >= blocks.length) return;
		const next = [...blocks];
		const [item] = next.splice(from, 1);
		next.splice(to, 0, item);
		blocks = next;
	}

	// ---- 손잡이를 잡고 끌어서 순서 바꾸기 (포인터 이벤트: 창의 파일 끌어 놓기와 상관없이 동작) ----
	let list: HTMLOListElement | undefined = $state();
	let dragging = $state<string | null>(null);
	function grab(e: PointerEvent, key: string) {
		if (e.button !== 0) return;
		e.preventDefault();
		try {
			(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
		} catch {
			/* 캡처 못 해도 끌기는 됨 */
		}
		dragging = key;
	}
	function drag(e: PointerEvent) {
		if (!dragging || !list) return;
		const from = blocks.findIndex((b) => b.key === dragging);
		const items = [...list.children] as HTMLElement[];
		// 포인터가 어느 블록의 가운데를 넘었는지
		let to = items.findIndex((el) => {
			const r = el.getBoundingClientRect();
			return e.clientY < r.top + r.height / 2;
		});
		if (to < 0) to = items.length - 1;
		else if (to > from) to -= 1;
		moveTo(from, to);
	}
	function drop() {
		dragging = null;
	}
	/** 키보드: 손잡이에서 ↑/↓ */
	function gripKey(e: KeyboardEvent, i: number) {
		if (e.key === "ArrowUp" || e.key === "ArrowDown") {
			e.preventDefault();
			moveTo(i, i + (e.key === "ArrowUp" ? -1 : 1));
			void Promise.resolve().then(() => (list?.querySelectorAll<HTMLElement>(".grip")[i + (e.key === "ArrowUp" ? -1 : 1)])?.focus());
		}
	}
	function add() {
		const b: Block = {
			key: `b${seq++}`,
			id: null,
			presetId: p.id,
			name: "새 블록",
			identifier: `custom-${Date.now().toString(36)}`,
			blockType: "text",
			role: "system",
			content: "",
			enabled: true,
			position: "relative",
			depth: 0,
			sortOrder: blocks.length,
			triggersJson: [],
			extraJson: null,
		};
		blocks = [...blocks, b];
		open = b.key;
	}
	function remove(i: number) {
		blocks = blocks.filter((_, k) => k !== i);
	}

	// ---- 자동 저장: 값이 바뀌고 잠시 멈추면 ----
	const snapshot = () => JSON.stringify([name, description, temperature, maxTokens, contextSize, worldInfoPercent, topP, topK, stream, effort, appPrompts, blocks.map(({ key: _k, id: _i, ...b }) => b)]);
	let lastSaved = snapshot();
	let autoState = $state<"" | "dirty" | "saving" | "saved" | "failed">("");
	$effect(() => {
		if (!autosave) return;
		const now = snapshot();
		if (now === lastSaved) return;
		autoState = "dirty";
		const t = setTimeout(async () => {
			lastSaved = now;
			autoState = "saving";
			try {
				await save();
				autoState = "saved";
			} catch {
				autoState = "failed";
				lastSaved = "";
			}
		}, 700);
		return () => clearTimeout(t);
	});

	async function save() {
		const num = (s: string) => (String(s).trim() === "" || Number.isNaN(Number(s)) ? undefined : Number(s));
		// settings_json이 config_json보다 우선 → 화면 값은 settings_json에 앱 키로 씀
		const settings: Record<string, unknown> = { ...((p.settingsJson as object) ?? {}) };
		const put = (k: string, v: unknown) => {
			if (v === undefined) delete settings[k];
			else settings[k] = v;
		};
		put("temperature", num(temperature));
		put("max_tokens", num(maxTokens));
		put("context_size", num(contextSize));
		const wi = num(worldInfoPercent);
		put("world_info_ratio", wi === undefined ? undefined : Math.min(Math.max(wi, 0), 100) / 100);
		// 가져온 프리셋의 퍼센트 키는 화면 값으로 대신한다
		delete settings.worldInfoRatio;
		delete settings.world_info_budget;
		put("top_p", num(topP));
		put("top_k", num(topK));
		put("stream", stream);
		// 이 모델이 못 받는 값이어도 지우지 않는다 (모델을 바꾸면 다시 쓰임). 백엔드가 받는 모델에만 보냄
		put("reasoning_effort", effort || undefined);
		put("appPrompts", Object.keys(appPrompts).length ? { ...appPrompts } : undefined);
		const saved = await onsave(
			{ ...p, name: name.trim(), description: description.trim() || null, settingsJson: settings as Detail["preset"]["settingsJson"] },
			blocks.map(({ key: _k, ...b }) => ({ ...b, depth: Number(b.depth) || 0 })),
		);
		// 새 블록은 저장된 id를 받아 둠 (identifier로 짝지음)
		if (saved) for (const b of blocks) if (!b.id) b.id = saved.blocks.find((x) => x.identifier === b.identifier)?.id ?? null;
	}

	const roleLabel: Record<string, string> = { system: "시스템", user: "사용자", assistant: "AI" };
	const TRIGGERS = [
		{ id: "normal", label: "보내기" },
		{ id: "continue", label: "이어쓰기" },
		{ id: "regenerate", label: "다시 생성" },
		{ id: "swipe", label: "스와이프" },
	];
	function triggers(b: Block): string[] {
		return Array.isArray(b.triggersJson) ? (b.triggersJson as string[]) : [];
	}
	function toggleTrigger(b: Block, id: string) {
		const cur = triggers(b);
		b.triggersJson = cur.includes(id) ? cur.filter((t) => t !== id) : [...cur, id];
	}
</script>

{#if autosave && autoState === "failed"}
	<p class="auto" class:warn={autoState === "failed"}>{autoState === "failed" ? "저장 못 함 · 다시 고치면 또 시도" : autoState === "dirty" ? "고치는 중" : "고치면 저절로 저장됨"}</p>
{/if}

{#if !autosave}
	<div class="bar">
		<Button variant="ghost" onclick={oncancel}>취소</Button>
		<Button variant="primary" loading={saving} disabled={!name.trim()} onclick={save}>저장</Button>
	</div>
{/if}

{#if !quick}
	<Group>
		<Row label="이름" stacked><Input bind:value={name} aria-label="프리셋 이름" /></Row>
		<Row label="설명" stacked><Textarea bind:value={description} rows={2} autosize maxRows={6} aria-label="프리셋 설명" /></Row>
	</Group>
{/if}

<Group title="생성 옵션">
	<div class="sliders">
		<Slider label="컨텍스트 크기 (토큰)" hint="넘치면 오래된 대화부터 뺌. 0은 제한 없음" bind:value={contextSize} min={0} max={200000} step={1} fallback={4095} unsetLabel="기본 (4,095)" format={(n) => (n ? n.toLocaleString() : "제한 없음")} />
		<Slider label="응답 길이 (토큰)" hint="AI가 한 번에 쓰는 최대 길이" bind:value={maxTokens} min={64} max={128000} step={1} fallback={1024} unsetLabel="1,024" format={(n) => n.toLocaleString()} />

	</div>
	<div class="sliders">
		<Slider label="온도" hint="높을수록 다양하고 낮을수록 일정함" bind:value={temperature} min={0} max={2} step={0.05} fallback={1} unsetLabel="공급자 기본" format={(n) => n.toFixed(2)} />
		<Slider label="월드인포 비율 (%)" hint="고정 프롬프트를 뺀 나머지 중 월드인포에 줄 몫. 남는 만큼 대화 기록이 들어감" bind:value={worldInfoPercent} min={0} max={100} step={5} fallback={50} unsetLabel="기본 (50%)" format={(n) => `${n}%`} />
		<Slider label="Top P" hint="확률이 높은 낱말 몇 %에서만 고름" bind:value={topP} min={0} max={1} step={0.01} fallback={1} unsetLabel="공급자 기본" format={(n) => n.toFixed(2)} />
		<Slider label="Top K" hint="후보 낱말을 몇 개까지 볼지. 0은 끔" bind:value={topK} min={0} max={200} step={1} fallback={0} unsetLabel="공급자 기본" format={(n) => (n ? String(n) : "끔")} />
	</div>

	<Row label="실시간 응답 (스트리밍)" desc="끄면 응답이 다 만들어진 뒤 한 번에 보임">
		<Switch bind:checked={stream} label="스트리밍" hideLabel />
	</Row>
</Group>

<Group title="유틸 프롬프트">
	<p class="hint">이 프리셋으로 대화할 때만. 비우면 설정 &gt; 앱 프롬프트의 값</p>
	<PresetUtilPrompts bind:value={appPrompts} />
</Group>

<Group title="프롬프트 블록">
	{#snippet aside()}<Button size="sm" onclick={add}>{#snippet icon()}<Plus />{/snippet}블록 추가</Button>{/snippet}
	<p class="hint">위에서부터 순서대로 보냄. 앱 고정 규칙은 항상 맨 앞</p>
	<ol class="blocks" bind:this={list} class:sorting={!!dragging}>
		{#each blocks as b, i (b.key)}
			{@const marker = markerOf(b)}
			<li class:off={!b.enabled} class:dragging={dragging === b.key} animate:flip={{ duration: 150 }}>
				<div class="head">
					<button
						class="grip"
						aria-label="{b.name} 끌어서 순서 바꾸기 (↑/↓ 키도 됨)"
						title="끌어서 순서 바꾸기"
						onpointerdown={(e) => grab(e, b.key)}
						onpointermove={drag}
						onpointerup={drop}
						onpointercancel={drop}
						onkeydown={(e) => gripKey(e, i)}><GripVertical /></button>
					<button class="toggle" aria-expanded={open === b.key} onclick={() => (open = open === b.key ? null : b.key)}>
						{#if open === b.key}<ChevronDown />{:else}<ChevronRight />{/if}
						<span class="nm">{b.name}</span>
					</button>
					<span class="badges">
						<Badge>{roleLabel[b.role] ?? b.role}</Badge>
						{#if marker}<Badge tone="info">{marker}</Badge>{/if}
						{#if b.position === "in_chat"}<Badge tone="accent">대화 안 · 깊이 {b.depth}</Badge>{/if}
					</span>
					<span class="sw"><Switch bind:checked={b.enabled} label="{b.name} 사용" hideLabel /></span>
				</div>
				{#if open === b.key}
					<div class="body">
						<div class="grid">
							<Field label="이름"><Input bind:value={b.name} /></Field>
							<Field label="역할">
								<Select bind:value={b.role} options={[{ value: "system", label: "시스템" }, { value: "user", label: "사용자" }, { value: "assistant", label: "AI" }]} />
							</Field>
							<Field label="위치">
								<Select bind:value={b.position} options={[{ value: "relative", label: "순서대로" }, { value: "in_chat", label: "대화 기록 안" }]} />
							</Field>
							{#if b.position === "in_chat"}
								<Field label="깊이" hint="최근 메시지에서 몇 개 앞"><Input type="number" min="0" bind:value={b.depth} /></Field>
							{/if}
						</div>
						{#if marker}
							<p class="hint">이 블록은 내용 대신 {marker} 자리를 표시함</p>
						{:else}
							<Field label="내용" hint={"{{char}}, {{user}} 같은 매크로 사용 가능"}>
								<Textarea bind:value={b.content} mono rows={4} autosize maxRows={20} />
							</Field>
						{/if}
						<div class="trig">
							<span>쓸 때</span>
							{#each TRIGGERS as t (t.id)}
								<button class="chip" class:on={triggers(b).includes(t.id)} aria-pressed={triggers(b).includes(t.id)} onclick={() => toggleTrigger(b, t.id)}>{t.label}</button>
							{/each}
							<span class="faint">{triggers(b).length ? "" : "고르지 않으면 항상"}</span>
							<span class="sp"></span>
							<Button size="sm" variant="ghost" onclick={() => remove(i)}>{#snippet icon()}<Trash2 />{/snippet}블록 삭제</Button>
						</div>
					</div>
				{/if}
			</li>
		{/each}
	</ol>
</Group>

<style>
	.auto {
		margin: 0;
		font-size: var(--text-xs);
		color: var(--text-faint);
	}
	.auto.warn {
		color: var(--warning, #c98a1b);
	}
	.bar {
		position: sticky;
		top: 0;
		z-index: calc(var(--z-float) + 1);
		display: flex;
		justify-content: flex-end;
		gap: var(--space-2);
		margin: calc(var(--space-2) * -1) 0 var(--space-3);
		padding: var(--space-2) 0;
		background: var(--surface-2);
	}
	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
		gap: var(--space-3);
		padding: var(--space-4) 0 0;
	}
	.hint {
		padding-top: var(--space-3);
		font-size: var(--text-sm);
		color: var(--text-muted);
	}
	.blocks {
		list-style: none;
		margin: 0;
		padding: var(--space-2) 0 var(--space-4);
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}
	.blocks li {
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		background: var(--surface);
	}
	.sliders {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
		gap: var(--space-4) var(--space-6);
		padding: var(--space-4) 0 var(--space-2);
	}
	.grip {
		display: grid;
		place-items: center;
		width: 22px;
		height: 28px;
		padding: 0;
		border: none;
		border-radius: var(--radius-xs);
		background: none;
		color: var(--text-faint);
		cursor: grab;
		touch-action: none;
	}
	.grip:hover,
	.grip:focus-visible {
		background: var(--surface-hover);
		color: var(--text);
	}
	.grip :global(svg) {
		width: 16px;
		height: 16px;
	}
	.sorting,
	.sorting .grip {
		cursor: grabbing;
		user-select: none;
	}
	.blocks li.dragging {
		position: relative;
		z-index: var(--z-float);
		border-color: var(--accent);
		box-shadow: var(--shadow-md);
	}
	.blocks li.off .nm {
		color: var(--text-faint);
		text-decoration: line-through;
	}
	.head {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-1) var(--space-2);
	}
	/* 좁아도 스위치가 밖으로 밀려나지 않게: 이름은 줄이고 배지는 줄바꿈 */
	.toggle {
		display: flex;
		align-items: center;
		gap: var(--space-1);
		flex: 1 1 0;
		min-width: 60px;
		padding: 4px;
		border: none;
		background: transparent;
		color: var(--text);
		font: inherit;
		font-weight: 600;
		cursor: pointer;
	}
	.toggle :global(svg) {
		width: 16px;
		height: 16px;
		flex-shrink: 0;
		color: var(--text-faint);
	}
	.nm {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.badges {
		display: flex;
		flex-wrap: wrap;
		justify-content: flex-end;
		gap: 4px;
		min-width: 0;
		flex: 0 1 auto;
	}
	.sw {
		flex: none;
		display: flex;
	}
	.body {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		padding: 0 var(--space-3) var(--space-3);
		border-top: 1px solid var(--border);
	}
	.trig {
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: var(--space-1);
		font-size: var(--text-sm);
		color: var(--text-muted);
	}
	.chip {
		height: 26px;
		padding: 0 var(--space-2);
		border: 1px solid var(--border-strong);
		border-radius: 999px;
		background: transparent;
		color: var(--text-muted);
		font: inherit;
		font-size: var(--text-xs);
		cursor: pointer;
	}
	.chip.on {
		border-color: var(--accent);
		background: var(--accent-soft);
		color: var(--text);
	}
	.faint {
		color: var(--text-faint);
		font-size: var(--text-xs);
	}
</style>
