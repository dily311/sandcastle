<script lang="ts">
	// 모델 고르기 (ADR-020): 대화·기능이 쓸 모델과(지원 시) 추론 강도를 한 팝오버에서 고른다.
	// 목록 = 앱에 내장된 이 공급자의 모델 → 내가 추가한 모델 → (요청하면) 공급자에서 불러온 모델.
	// 목록에 없으면 "직접 입력…" — 입력한 이름은 이 프로필의 "내가 추가한 모델"로 남는다.
	import { tick } from "svelte";
	import { Check, ChevronDown, ChevronRight, Ellipsis, Search, TriangleAlert, X } from "@lucide/svelte";
	import type { ApiProfiles } from "$ipc/bindings";
	import { ContextMenu, IconButton, type MenuItem } from "$shared/ui";
	import { call, commands } from "$shared/ipc/client";
	import { promptText } from "$shared/stores/confirm.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	import { library } from "$features/character/library.svelte";
	import { apiLists, modelLabel, reasoningOf } from "./providers";

	interface Props {
		/** 모델을 고를 API 프로필 */
		profile: ApiProfiles | undefined;
		/** 고른 모델. null이면 미선택(또는 기존 프로필의 기본 모델) */
		value: string | null | undefined;
		onchange: (model: string | null) => void;
		label?: string;
		/** 추론 강도 ("auto" | "low" | "medium" | "high" 등). oneffort를 주면 팝오버 하단에 선택 UI 표시 */
		effort?: string;
		oneffort?: (effort: string) => void;
	}
	let { profile, value, onchange, label = "모델", effort = "auto", oneffort }: Props = $props();
	const src = { source: "model" };

	let fetched = $state<string[]>([]);
	// 프로필이 바뀌면 불러온 목록과 열린 팝오버를 정리
	$effect(() => {
		void profile?.id;
		fetched = [];
		closeAll();
	});

	// 공급자 이름은 대소문자를 가리지 않고 맞춘다 (예전에 저장된 연결은 "OpenAI", 카탈로그는 "openai")
	const entry = $derived(apiLists.find((p) => p.provider.toLowerCase() === (profile?.provider ?? "").toLowerCase()));
	const shortNameOf = (id: string) => modelLabel(entry, id);

	/** 목록에 보일 모델과 묶음 이름: 내장 목록은 카탈로그의 묶음, 그 뒤로 내가 추가한 모델 · 불러온 모델 */
	const groups = $derived.by(() => {
		if (!profile) return [];
		const seen = new Set<string>();
		const out: { label: string; items: { id: string; name: string }[] }[] = [];
		const map = new Map<string, { id: string; name: string }[]>();
		const add = (id: string | null | undefined, group: string) => {
			if (!id || seen.has(id)) return;
			seen.add(id);
			let list = map.get(group);
			if (!list) {
				list = [];
				map.set(group, list);
				out.push({ label: group, items: list });
			}
			list.push({ id, name: shortNameOf(id) });
		};
		if (profile.model) add(profile.model, "기본 모델");
		for (const g of entry?.modelList ?? []) for (const m of g.model) add(m.modelId, g.label);
		for (const m of profile.customModels ?? []) add(m, "내가 추가한 모델");
		for (const m of fetched) add(m, "공급자에서 불러온 모델");
		add(value, "내가 추가한 모델");
		return out;
	});

	const effectiveModel = $derived(value || profile?.model || null);
	const reasoning = $derived(oneffort ? reasoningOf(effectiveModel) : null);

	const EFFORT_LABELS: Record<string, string> = {
		auto: "자동",
		none: "끔",
		off: "끔",
		minimal: "최소",
		low: "낮음",
		medium: "중간",
		high: "높음",
		xhigh: "엑스트라",
		max: "최대",
	};
	const effortLabel = (lvl: string) => EFFORT_LABELS[lvl.toLowerCase()] ?? lvl;

	// ---- 팝오버 / 추론 서브메뉴 상태 ----
	let open = $state(false);
	let subOpen = $state(false);
	let query = $state("");
	let triggerEl = $state<HTMLButtonElement | undefined>();
	let popoverEl = $state<HTMLDivElement | undefined>();
	let searchEl = $state<HTMLInputElement | undefined>();
	let subEl = $state<HTMLDivElement | undefined>();
	let popPos = $state({ left: 0, top: 0, width: 240, maxHeight: 340 });
	let subPos = $state({ left: 0, top: 0 });
	let subCloseTimer: ReturnType<typeof setTimeout> | null = null;

	const filteredGroups = $derived.by(() => {
		const q = query.trim().toLowerCase();
		if (!q) return groups;
		return groups
			.map((g) => ({
				label: g.label,
				items: g.items.filter((m) => m.name.toLowerCase().includes(q) || m.id.toLowerCase().includes(q)),
			}))
			.filter((g) => g.items.length > 0);
	});

	function cancelSubTimer() {
		if (subCloseTimer) {
			clearTimeout(subCloseTimer);
			subCloseTimer = null;
		}
	}
	function scheduleSubClose() {
		cancelSubTimer();
		subCloseTimer = setTimeout(() => {
			subOpen = false;
		}, 140);
	}
	function closeAll() {
		cancelSubTimer();
		open = false;
		subOpen = false;
		query = "";
	}

	async function positionMain() {
		if (!triggerEl) return;
		const r = triggerEl.getBoundingClientRect();
		const width = Math.min(320, Math.max(r.width, 240));
		const pad = 8;
		const left = Math.max(pad, Math.min(r.left, window.innerWidth - width - pad));
		const spaceBelow = window.innerHeight - r.bottom - pad;
		const spaceAbove = r.top - pad;
		const openUp = spaceBelow < 220 && spaceAbove > spaceBelow;
		const maxHeight = Math.max(180, Math.min(380, (openUp ? spaceAbove : spaceBelow) - 8));
		popPos = {
			left,
			top: openUp ? Math.max(pad, r.top - maxHeight - 6) : r.bottom + 6,
			width,
			maxHeight,
		};
		await tick();
		if (openUp && popoverEl) {
			const actualH = popoverEl.getBoundingClientRect().height;
			popPos = { ...popPos, top: Math.max(pad, r.top - actualH - 6) };
		}
	}

	async function openSub() {
		cancelSubTimer();
		if (!reasoning || !oneffort) return;
		subOpen = true;
		await tick();
		if (!popoverEl) return;
		const mr = popoverEl.getBoundingClientRect();
		const subW = subEl?.getBoundingClientRect().width || 272;
		const subH = subEl?.getBoundingClientRect().height || 260;
		const gap = 8;
		const pad = 8;
		// 오른쪽 공간이 충분하면 오른쪽에, ChatPanel처럼 화면 우측 끝이면 왼쪽으로 자동 전환
		const fitsRight = mr.right + gap + subW <= window.innerWidth - pad;
		const left = fitsRight ? mr.right + gap : Math.max(pad, mr.left - gap - subW);
		const top = Math.max(pad, Math.min(mr.bottom - subH, window.innerHeight - subH - pad));
		subPos = { left, top };
	}

	async function toggleOpen() {
		if (!profile) return;
		if (open) {
			closeAll();
			return;
		}
		open = true;
		subOpen = false;
		query = "";
		await positionMain();
		searchEl?.focus();
	}

	function selectModel(next: string | null) {
		onchange(next);
		if (oneffort && effort !== "auto") {
			const r = reasoningOf(next || profile?.model || null);
			if (!r || !r.levels.includes(effort)) oneffort("auto");
		}
		closeAll();
	}

	function selectEffort(lvl: string) {
		oneffort?.(lvl);
		closeAll();
	}

	function onKey(e: KeyboardEvent) {
		if (!open) return;
		if (e.key === "Escape") {
			e.stopPropagation();
			closeAll();
			triggerEl?.focus();
		}
	}
	function onWindowDown(e: PointerEvent) {
		if (!open) return;
		const t = e.target as Node | null;
		if (!t) return;
		if (triggerEl?.contains(t) || popoverEl?.contains(t) || subEl?.contains(t)) return;
		closeAll();
	}

	// ---- 더 보기 메뉴 (직접 입력 · 목록 불러오기) ----
	let menu = $state<{ x: number; y: number; align?: "start" | "end" } | null>(null);
	const items = $derived<MenuItem[]>([
		{ label: "직접 입력…", onselect: typeModel },
		{ label: "공급자에서 목록 불러오기", disabled: !profile, onselect: fetchModels },
	]);
	function openMenu(e: MouseEvent) {
		closeAll();
		const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
		menu = { x: r.right, y: r.bottom + 4, align: "end" };
	}

	async function typeModel() {
		if (!profile) return;
		const t = await promptText({ title: "모델 직접 입력", message: "이 API의 \"내가 추가한 모델\"로 남음", input: { value: "", placeholder: "모델 이름" }, confirmLabel: "고르기" });
		const name = t?.trim();
		if (!name) return;
		const list = await call(commands.addApiProfileModel(profile.id, name), src).catch(() => null);
		if (list) {
			const p = library.apis.find((a) => a.id === profile.id);
			if (p) p.customModels = list;
		}
		selectModel(profile.model && name === profile.model ? null : name);
	}
	async function fetchModels() {
		if (!profile) return;
		try {
			fetched = await call(commands.getModels(profile.id), src);
			toasts.success(`모델 ${fetched.length}개 불러옴`);
		} catch {
			/* 토스트는 call이 띄움 */
		}
	}
</script>

<svelte:window onkeydown={onKey} onpointerdown={onWindowDown} onresize={closeAll} />

<div class="picker">
	<button
		bind:this={triggerEl}
		type="button"
		class="trigger"
		class:open
		class:placeholder={!effectiveModel}
		aria-label={label}
		aria-haspopup="listbox"
		aria-expanded={open}
		disabled={!profile}
		onclick={toggleOpen}
	>
		<span class="trigger-text">{effectiveModel ? shortNameOf(effectiveModel) : "모델 선택…"}</span>
		{#if reasoning && oneffort && effort !== "auto"}
			<span class="trigger-pill">{effortLabel(effort)}</span>
		{/if}
		<ChevronDown class="chev" aria-hidden="true" />
	</button>
	<IconButton label="{label} 더 보기 (직접 입력·목록 불러오기)" size="sm" disabled={!profile} onclick={openMenu}><Ellipsis /></IconButton>
</div>

{#if open}
	<div
		bind:this={popoverEl}
		class="popover"
		role="listbox"
		aria-label={label}
		style:left="{popPos.left}px"
		style:top="{popPos.top}px"
		style:width="{popPos.width}px"
		style:max-height="{popPos.maxHeight}px"
	>
		{#if groups.length > 0}
			<div class="search-box">
				<Search class="search-ic" aria-hidden="true" />
				<input
					bind:this={searchEl}
					type="text"
					bind:value={query}
					placeholder="모델 검색…"
					aria-label="{label} 검색"
				/>
				{#if query}
					<button
						type="button"
						class="search-clear"
						aria-label="검색어 지우기"
						onclick={() => {
							query = "";
							searchEl?.focus();
						}}
					>
						<X />
					</button>
				{/if}
			</div>
		{/if}
		<div class="model-list">
			{#if groups.length === 0}
				<div class="empty">표시할 모델이 없습니다. 오른쪽 ···에서 직접 입력하거나 목록을 불러오세요.</div>
			{:else if filteredGroups.length === 0}
				<div class="empty">"{query.trim()}"와 일치하는 모델이 없습니다.</div>
			{:else}
				{#each filteredGroups as g (g.label)}
					{#if filteredGroups.length > 1}
						<div class="group-label">{g.label}</div>
					{/if}
					{#each g.items as m (m.id)}
						{@const active = effectiveModel === m.id}
						<button
							type="button"
							role="option"
							aria-selected={active}
							class="opt"
							class:active
							title={m.name === m.id ? m.id : `${m.name} (${m.id})`}
							onclick={() => selectModel(m.id)}
							onmouseenter={() => {
								if (subOpen) scheduleSubClose();
							}}
						>
							<span class="opt-name">{m.name}</span>
							{#if active}<Check class="chk" />{/if}
						</button>
					{/each}
				{/each}
			{/if}
		</div>

		{#if reasoning && oneffort}
			<div class="divider"></div>
			<button
				type="button"
				class="effort-trigger"
				class:active={subOpen}
				aria-haspopup="menu"
				aria-expanded={subOpen}
				onmouseenter={() => void openSub()}
				onmouseleave={scheduleSubClose}
				onclick={() => (subOpen ? (subOpen = false) : void openSub())}
			>
				<span class="effort-title">추론 강도</span>
				<span class="effort-val">
					<span>{effortLabel(effort)}</span>
					<ChevronRight class="sub-chev" />
				</span>
			</button>
		{/if}
	</div>
{/if}

{#if open && subOpen && reasoning && oneffort}
	<div
		bind:this={subEl}
		class="submenu"
		role="menu"
		tabindex="-1"
		aria-label="{label} 추론 강도"
		style:left="{subPos.left}px"
		style:top="{subPos.top}px"
		onmouseenter={cancelSubTimer}
		onmouseleave={scheduleSubClose}
	>
		<p class="sub-desc">추론 강도가 높을수록 더 철저하게 답하지만, 시간이 더 오래 걸리고 토큰을 더 많이 사용합니다.</p>
		<div class="sub-list">
			<button type="button" role="menuitemradio" aria-checked={effort === "auto"} class="opt" class:active={effort === "auto"} onclick={() => selectEffort("auto")}>
				<span class="opt-left">
					<span class="opt-name">자동</span>
					{#if reasoning.default}
						<span class="pill">기본 · {effortLabel(reasoning.default)}</span>
					{/if}
				</span>
				{#if effort === "auto"}<Check class="chk" />{/if}
			</button>
			{#each reasoning.levels as lvl (lvl)}
				{@const active = effort === lvl}
				{@const isRec = reasoning.default === lvl}
				{@const isHeavy = lvl === "max" || lvl === "xhigh"}
				<button type="button" role="menuitemradio" aria-checked={active} class="opt" class:active onclick={() => selectEffort(lvl)}>
					<span class="opt-left">
						<span class="opt-name">{effortLabel(lvl)}</span>
						{#if isRec}
							<span class="pill">추천</span>
						{/if}
						{#if isHeavy}
							<span class="pill warn"><TriangleAlert class="warn-ic" />높은 사용량</span>
						{/if}
					</span>
					{#if active}<Check class="chk" />{/if}
				</button>
			{/each}
		</div>
	</div>
{/if}

<ContextMenu at={menu} {items} label="모델 메뉴" onclose={() => (menu = null)} />

<style>
	.picker {
		display: flex;
		align-items: center;
		gap: 2px;
		min-width: 0;
		width: 100%;
	}
	.trigger {
		position: relative;
		display: flex;
		align-items: center;
		gap: var(--space-2);
		flex: 1;
		min-width: 0;
		height: var(--control-md);
		padding: 0 var(--space-3);
		background: var(--surface);
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-sm);
		color: var(--text);
		font: inherit;
		font-size: var(--text-sm);
		text-align: left;
		cursor: pointer;
		outline: none;
	}
	.trigger:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	.trigger:focus-visible,
	.trigger.open {
		border-color: var(--accent);
		box-shadow: 0 0 0 3px var(--focus-ring);
	}
	.trigger.placeholder .trigger-text {
		color: var(--text-muted);
	}
	.trigger-text {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.trigger-pill {
		flex-shrink: 0;
		padding: 1px 6px;
		border-radius: var(--radius-full, 999px);
		background: var(--bg-sunken);
		color: var(--text-muted);
		font-size: 11px;
		font-weight: 500;
	}
	.trigger :global(.chev) {
		width: 16px;
		height: 16px;
		flex-shrink: 0;
		color: var(--text-faint);
		transition: transform 0.12s ease;
	}
	.trigger.open :global(.chev) {
		transform: rotate(180deg);
	}

	.popover,
	.submenu {
		position: fixed;
		z-index: 1200;
		display: flex;
		flex-direction: column;
		padding: 6px;
		background: var(--surface);
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-lg);
		box-shadow: 0 10px 30px rgba(0, 0, 0, 0.16), 0 2px 8px rgba(0, 0, 0, 0.08);
	}
	.search-box {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 5px 8px;
		margin-bottom: 4px;
		border-radius: var(--radius-md);
		background: var(--bg-sunken);
		border: 1px solid var(--border);
		flex-shrink: 0;
	}
	.search-box:focus-within {
		border-color: var(--accent);
	}
	.search-box :global(.search-ic) {
		width: 14px;
		height: 14px;
		flex-shrink: 0;
		color: var(--text-faint);
	}
	.search-box input {
		flex: 1;
		min-width: 0;
		border: none;
		background: transparent;
		color: var(--text);
		font: inherit;
		font-size: var(--text-xs);
		outline: none;
	}
	.search-box input::placeholder {
		color: var(--text-faint);
	}
	.search-clear {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 16px;
		height: 16px;
		padding: 0;
		border: none;
		border-radius: var(--radius-full, 999px);
		background: transparent;
		color: var(--text-faint);
		cursor: pointer;
	}
	.search-clear:hover {
		color: var(--text);
	}
	.search-clear :global(svg) {
		width: 12px;
		height: 12px;
	}
	.model-list {
		display: flex;
		flex-direction: column;
		gap: 1px;
		overflow-y: auto;
		min-height: 0;
	}
	.group-label {
		padding: 6px 10px 2px;
		font-size: 11px;
		font-weight: 600;
		color: var(--text-faint);
	}
	.empty {
		padding: var(--space-3);
		font-size: var(--text-xs);
		color: var(--text-muted);
		line-height: 1.45;
	}
	.opt {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-2);
		width: 100%;
		padding: 7px 10px;
		border: none;
		border-radius: var(--radius-md);
		background: transparent;
		color: var(--text);
		font: inherit;
		font-size: var(--text-sm);
		text-align: left;
		cursor: pointer;
	}
	.opt:hover {
		background: var(--bg-sunken);
	}
	.opt.active {
		font-weight: 500;
	}
	.opt-left {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		min-width: 0;
	}
	.opt-name {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.opt :global(.chk) {
		width: 16px;
		height: 16px;
		flex-shrink: 0;
		color: var(--accent);
	}

	.divider {
		height: 1px;
		margin: 6px 4px;
		background: var(--border);
		flex-shrink: 0;
	}
	.effort-trigger {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-2);
		width: 100%;
		padding: 7px 10px;
		border: none;
		border-radius: var(--radius-md);
		background: transparent;
		color: var(--text);
		font: inherit;
		font-size: var(--text-sm);
		cursor: pointer;
		flex-shrink: 0;
	}
	.effort-trigger:hover,
	.effort-trigger.active {
		background: var(--bg-sunken);
	}
	.effort-title {
		font-weight: 500;
	}
	.effort-val {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		color: var(--text-muted);
		font-size: var(--text-sm);
	}
	.effort-val :global(.sub-chev) {
		width: 15px;
		height: 15px;
		color: var(--text-faint);
	}

	.submenu {
		width: 272px;
		padding: 8px;
		gap: 4px;
	}
	.sub-desc {
		margin: 0;
		padding: 4px 8px 6px;
		font-size: var(--text-xs);
		line-height: 1.45;
		color: var(--text-muted);
	}
	.sub-list {
		display: flex;
		flex-direction: column;
		gap: 1px;
	}
	.pill {
		display: inline-flex;
		align-items: center;
		gap: 3px;
		padding: 1px 7px;
		border-radius: var(--radius-full, 999px);
		background: var(--bg-sunken);
		color: var(--text-muted);
		font-size: 11px;
		font-weight: 500;
		white-space: nowrap;
	}
	.pill.warn {
		background: var(--warning-soft);
		color: var(--warning);
	}
	.pill :global(.warn-ic) {
		width: 11px;
		height: 11px;
	}
</style>
