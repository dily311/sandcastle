<script lang="ts">
	// 배경 고르기: 없음 · 사용자 이미지(모달에서 저장된 이미지/에셋/생성 이미지 선택) · 이미지 올리기 · 기본 그라데이션
	// 값: null(없음) / "preset:<id>" / 이미지 경로
	import { _ } from "svelte-i18n";
	import { Ban, Camera, ChevronDown, Images, Search, Upload, X } from "@lucide/svelte";
	import type { GeneratedImage_Serialize as GeneratedImage } from "$ipc/bindings";
	import { call, commands } from "$shared/ipc/client";
	import { fileSrc } from "$shared/ipc/file";
	import Button from "./Button.svelte";
	import Dialog from "./Dialog.svelte";
	import Input from "./Input.svelte";
	import Segmented from "./Segmented.svelte";
	import Spinner from "./Spinner.svelte";
	import { GRADIENTS, gradientCss } from "./backgrounds";

	export interface PickerImageTab {
		id: string;
		label: string;
		images: { path: string; label?: string }[];
	}

	interface Props {
		value: string | null;
		/** 올려 둔 배경 이미지 경로들 (제거 가능한 목록) */
		images: string[];
		/** 추가 이미지 탭 (예: 이미지 카드에서 캐릭터 에셋·대화 생성 이미지) */
		extraTabs?: PickerImageTab[];
		onpick: (value: string | null) => void;
		/** 이미지 올리기 (없으면 올리기 칸을 숨김) */
		onupload?: () => void;
		/** 올린 이미지 지우기 (없으면 지우기 버튼 숨김) */
		onremove?: (path: string) => void;
		/** "없음" 칸 보이기 */
		allowNone?: boolean;
		label?: string;
	}
	let { value, images, extraTabs = [], onpick, onupload, onremove, allowNone = true, label }: Props = $props();
	const effectiveLabel = $derived(label ?? $_("settings.sharedUi.bgLabel"));

	let expanded = $state(false);
	let row: HTMLDivElement | undefined = $state();
	let overflowing = $state(false);
	const isCustomPath = $derived(!!value && !value.startsWith("preset:") && !["sea", "dusk", "sand", "night", "forest"].includes(value));
	const count = $derived(1 + (onupload ? 1 : 0) + (allowNone ? 1 : 0) + (isCustomPath ? 1 : 0) + GRADIENTS.length);

	// 사용자 이미지 모달
	let modalOpen = $state(false);
	let loading = $state(false);
	let savedImages = $state<GeneratedImage[]>([]);
	let query = $state("");
	let tagFilter = $state<string>("all");
	let activeTab = $state("saved");
	const removableSet = $derived(new Set(images));
	const allTags = $derived(
		[...new Set(savedImages.flatMap((i) => i.tags))].sort((a, b) => a.localeCompare(b, "ko")),
	);
	const TAG_LABELS = $derived<Record<string, string>>({ preset: $_("settings.dataSec.counts.promptPresets"), upload: $_("settings.sharedUi.tagUpload") });
	const tagLabel = (t: string) => TAG_LABELS[t] ?? t;

	async function openLibrary() {
		modalOpen = true;
		query = "";
		tagFilter = "all";
		if (extraTabs.length && !extraTabs.some((t) => t.id === activeTab) && activeTab !== "saved") {
			activeTab = extraTabs[0].id;
		}
		loading = true;
		try {
			savedImages = await call(commands.listAllImages(), { source: "bg-picker", silent: true }).catch(() => []);
		} finally {
			loading = false;
		}
	}

	const allTabs = $derived<{ id: string; label: string; items: { path: string; label: string; tags: string[]; removable: boolean }[] }[]>([
		...extraTabs.map((t) => ({
			id: t.id,
			label: `${t.label} ${t.images.length}`,
			items: t.images.map((x) => ({ path: x.path, label: x.label ?? x.path.split(/[\\/]/).pop() ?? "", tags: [], removable: removableSet.has(x.path) })),
		})),
		{
			id: "saved",
			label: $_("settings.sharedUi.savedImagesTab"),
			items: (() => {
				const seen = new Set<string>();
				const out: { path: string; label: string; tags: string[]; removable: boolean }[] = [];
				for (const img of savedImages) {
					if (seen.has(img.path)) continue;
					seen.add(img.path);
					out.push({ path: img.path, label: img.name || img.prompt || img.path.split(/[\\/]/).pop() || "", tags: img.tags ?? [], removable: removableSet.has(img.path) });
				}
				for (const p of [...images].reverse()) {
					if (seen.has(p)) continue;
					seen.add(p);
					out.push({ path: p, label: p.split(/[\\/]/).pop() ?? $_("settings.sharedUi.bgLabel"), tags: [], removable: true });
				}
				return out;
			})(),
		},
	]);

	const currentTab = $derived(allTabs.find((t) => t.id === activeTab) ?? allTabs[allTabs.length - 1]);
	const shownItems = $derived.by(() => {
		const q = query.trim().toLowerCase();
		const list = currentTab?.items ?? [];
		return list.filter((i) => {
			if (currentTab?.id === "saved" && tagFilter !== "all" && !i.tags.includes(tagFilter)) return false;
			return !q || i.label.toLowerCase().includes(q) || i.tags.some((t) => t.toLowerCase().includes(q));
		});
	});

	function choose(path: string) {
		onpick(path);
		modalOpen = false;
	}

	$effect(() => {
		void count;
		const el = row;
		if (!el || expanded) return;
		let frame = 0;
		const check = () => {
			cancelAnimationFrame(frame);
			frame = requestAnimationFrame(() => (overflowing = el.scrollWidth > el.clientWidth + 1));
		};
		check();
		const ro = new ResizeObserver(check);
		ro.observe(el);
		return () => {
			cancelAnimationFrame(frame);
			ro.disconnect();
		};
	});
</script>

<div class="picker" class:expanded role="radiogroup" aria-label={effectiveLabel}>
	<div class="tiles" bind:this={row}>
		{#if onupload}
			<button type="button" class="tile upload" title={$_("settings.sharedUi.uploadNewImg")} aria-label={$_("settings.sharedUi.uploadNewImg")} onclick={onupload}><Camera /></button>
		{/if}
		<button type="button" class="tile library" class:on={isCustomPath} title={$_("settings.sharedUi.pickFromUserImages")} aria-label={$_("settings.sharedUi.pickFromUserImages")} onclick={openLibrary}>
			{#if isCustomPath && value}
				<img src={fileSrc(value)} alt="" loading="lazy" />
				<span class="lib-badge" aria-hidden="true"><Images /></span>
			{:else}
				<Images />
			{/if}
		</button>
		{#if allowNone}
			<button type="button" class="tile none" class:on={!value} role="radio" aria-checked={!value} title={$_("common.none")} aria-label={$_("settings.sharedUi.noBgAria")} onclick={() => onpick(null)}><Ban /></button>
		{/if}
		{#each GRADIENTS as g (g.id)}
			{@const v = `preset:${g.id}`}
			{@const gname = $_(`uiPolish.gradients.${g.id}`, { default: g.name })}
			<button type="button" class="tile" class:on={value === v} role="radio" aria-checked={value === v} title={gname} aria-label={gname} style:background={gradientCss(g)} onclick={() => onpick(v)}></button>
		{/each}
	</div>
	{#if overflowing || expanded}
		<button type="button" class="more" aria-expanded={expanded} onclick={() => (expanded = !expanded)}>
			{expanded ? $_("settings.sharedUi.collapse") : $_("settings.sharedUi.expand")}<ChevronDown class={expanded ? "up" : ""} />
		</button>
	{/if}
</div>

<Dialog open={modalOpen} title={$_("settings.sharedUi.pickUserImgTitle")} size="lg" onclose={() => (modalOpen = false)}>
	<div class="modal-head">
		{#if allTabs.length > 1}
			<Segmented label={$_("settings.sharedUi.imgSourceAria")} bind:value={activeTab} options={allTabs.map((t) => ({ value: t.id, label: t.label }))} />
		{/if}
		<div class="modal-tools">
			<div class="modal-search">
				<Input bind:value={query} placeholder={$_("settings.sharedUi.searchImgPlaceholder")} aria-label={$_("settings.sharedUi.searchImgAria")} type="search">
					{#snippet leading()}<Search />{/snippet}
				</Input>
			</div>
			{#if onupload}
				<Button size="sm" variant="secondary" onclick={onupload}>{#snippet icon()}<Upload />{/snippet}{$_("settings.sharedUi.uploadNewImg")}</Button>
			{/if}
		</div>
	</div>

	{#if currentTab?.id === "saved" && allTags.length}
		<div class="tag-chips" role="tablist" aria-label={$_("settings.sharedUi.tagFilterAria")}>
			<button type="button" class="tag-chip" class:on={tagFilter === "all"} onclick={() => (tagFilter = "all")}>{$_("common.all")}</button>
			{#each allTags as t (t)}
				<button type="button" class="tag-chip" class:on={tagFilter === t} onclick={() => (tagFilter = tagFilter === t ? "all" : t)}>#{tagLabel(t)}</button>
			{/each}
		</div>
	{/if}

	{#if loading && !shownItems.length}
		<div class="modal-center"><Spinner /></div>
	{:else if shownItems.length}
		<div class="modal-grid">
			{#each shownItems.slice(0, 180) as item (item.path)}
				<div class="modal-item" class:on={value === item.path}>
					<button type="button" class="modal-thumb" title={item.label} onclick={() => choose(item.path)}>
						<img src={fileSrc(item.path)} alt={item.label} loading="lazy" />
						{#if item.label}<span class="modal-cap">{item.label}</span>{/if}
					</button>
					{#if onremove && item.removable}
						<button type="button" class="del" aria-label={$_("settings.sharedUi.deleteBgAria")} title={$_("common.delete")} onclick={() => onremove(item.path)}><X /></button>
					{/if}
				</div>
			{/each}
		</div>
	{:else}
		<p class="modal-empty">{$_("settings.sharedUi.noSelectableImg")}</p>
	{/if}

	{#snippet footer()}
		{#if allowNone && value}
			<Button variant="ghost" onclick={() => ((onpick(null)), (modalOpen = false))}>{$_("settings.sharedUi.noBgAria")}</Button>
		{/if}
		<Button variant="secondary" onclick={() => (modalOpen = false)}>{$_("common.close")}</Button>
	{/snippet}
</Dialog>

<style>
	.picker {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		min-width: 0;
		max-width: 100%;
	}
	.picker.expanded {
		flex-direction: column;
		align-items: stretch;
	}
	.tiles {
		display: flex;
		gap: 8px;
		min-width: 0;
		flex: 1;
		overflow: hidden;
		padding: 8px 4px;
	}
	.expanded .tiles {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(48px, 1fr));
		grid-auto-rows: minmax(0, auto);
		gap: var(--space-2);
		overflow: visible;
		max-height: 260px;
		overflow-y: auto;
	}
	.tile {
		position: relative;
		flex: 0 0 auto;
		display: grid;
		place-items: center;
		width: 48px;
		height: 48px;
		padding: 0;
		border: none;
		border-radius: var(--radius-md);
		background: var(--surface-active);
		color: var(--text-muted);
		cursor: pointer;
		overflow: hidden;
		box-shadow: 0 0 0 1px var(--border);
	}
	.expanded .tile {
		width: 100%;
		height: auto;
		aspect-ratio: 1;
	}
	.tile:hover {
		filter: brightness(1.05);
	}
	.tile.on {
		box-shadow:
			0 0 0 2px var(--surface),
			0 0 0 4px var(--accent);
	}
	.tile img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}
	.tile :global(svg) {
		width: 18px;
		height: 18px;
	}
	.library {
		background: var(--accent-soft);
		color: var(--accent-text);
	}
	.lib-badge {
		position: absolute;
		right: 3px;
		bottom: 3px;
		display: grid;
		place-items: center;
		width: 18px;
		height: 18px;
		border-radius: 5px;
		background: rgb(0 0 0 / 0.6);
		color: #fff;
	}
	.lib-badge :global(svg) {
		width: 11px;
		height: 11px;
	}
	.upload {
		background: var(--text-faint);
		color: var(--surface);
	}
	.del {
		position: absolute;
		top: 4px;
		right: 4px;
		display: grid;
		place-items: center;
		width: 20px;
		height: 20px;
		padding: 0;
		border: none;
		border-radius: 50%;
		background: rgb(0 0 0 / 0.65);
		color: #fff;
		cursor: pointer;
	}
	.del :global(svg) {
		width: 11px;
		height: 11px;
	}
	.more {
		display: inline-flex;
		align-items: center;
		gap: 2px;
		flex-shrink: 0;
		align-self: center;
		padding: 4px 8px;
		border: none;
		border-radius: var(--radius-sm);
		background: transparent;
		color: var(--text-muted);
		font: inherit;
		font-size: var(--text-xs);
		cursor: pointer;
	}
	.expanded .more {
		align-self: flex-end;
	}
	.more:hover {
		background: var(--surface-hover);
	}
	.more :global(svg) {
		width: 14px;
		height: 14px;
		transition: transform var(--duration-fast) var(--ease);
	}
	.more :global(.up) {
		transform: rotate(180deg);
	}
	.modal-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-3);
		flex-wrap: wrap;
		margin-bottom: var(--space-3);
	}
	.modal-tools {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		flex: 1;
		justify-content: flex-end;
		min-width: 200px;
	}
	.modal-search {
		width: min(240px, 100%);
	}
	.modal-center {
		display: grid;
		place-items: center;
		padding: var(--space-8);
	}
	.modal-empty {
		padding: var(--space-8);
		text-align: center;
		color: var(--text-faint);
		font-size: var(--text-sm);
	}
	.modal-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(112px, 1fr));
		gap: var(--space-2);
		max-height: 55vh;
		overflow-y: auto;
		padding: 2px;
	}
	.modal-item {
		position: relative;
		border-radius: var(--radius-md);
		overflow: hidden;
		border: 2px solid transparent;
	}
	.modal-item.on {
		border-color: var(--accent);
	}
	.modal-thumb {
		position: relative;
		display: block;
		width: 100%;
		aspect-ratio: 1;
		padding: 0;
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		background: var(--bg-sunken);
		overflow: hidden;
		cursor: pointer;
	}
	.modal-thumb img {
		width: 100%;
		height: 100%;
		object-fit: cover;
		display: block;
	}
	.modal-cap {
		position: absolute;
		inset: auto 0 0 0;
		padding: 3px 6px;
		background: linear-gradient(to top, rgb(0 0 0 / 0.72), transparent);
		color: #fff;
		font-size: 11px;
		text-align: left;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.tag-chips {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
		margin-bottom: var(--space-3);
	}
	.tag-chip {
		padding: 3px 10px;
		border: 1px solid var(--border);
		border-radius: var(--radius-full);
		background: var(--surface);
		color: var(--text-muted);
		font: inherit;
		font-size: var(--text-xs);
		cursor: pointer;
	}
	.tag-chip:hover {
		border-color: var(--border-strong);
		color: var(--text);
	}
	.tag-chip.on {
		border-color: var(--accent);
		background: var(--accent-soft);
		color: var(--accent-text);
		font-weight: 600;
	}
</style>
