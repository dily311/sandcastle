<script lang="ts">
	// 이미지 페이지 = 갤러리: 만든 그림과 올린 그림 전부. 위의 "만들기"를 누르면 만들기 칸이 열린다.
	// 그림은 앱 데이터 generated/ 폴더에 있고, 같은 이름 .json에 프롬프트·출처·생성 정보가 있다 (service/plugins.rs)
	import { jobs } from "$shared/stores/jobs.svelte";
	import { _ } from "svelte-i18n";
	import { getAppLocale } from "$lib/utils/i18n";
	import { onMount } from "svelte";
	import { ArrowUpDown, Check, Ellipsis, Info, Plus, Copy, ExternalLink, Eye, Filter, FolderOpen, MessageSquare, Pencil, Search, Sparkles, Tag, Trash2, Upload, Wand2, X } from "@lucide/svelte";
	import type { GeneratedImage_Serialize as GeneratedImage } from "$ipc/bindings";
	import { Badge, Button, ContextMenu, EmptyState, IconButton, ItemCard, ListToolbar, SelectBar, SplitButton, Spinner, type MenuItem } from "$shared/ui";
	import { imageViewer } from "$shared/stores/imageViewer.svelte";
	import { call, commands } from "$shared/ipc/client";
	import { fileSrc } from "$shared/ipc/file";
	import { pickFiles } from "$shared/ipc/dialog";
	import { confirmAction, promptText } from "$shared/stores/confirm.svelte";
	import { drops, isImage, storeFile } from "$app/drop.svelte";
	import { revealItemInDir } from "@tauri-apps/plugin-opener";
	import { layout } from "$app/layout.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	import { plugins } from "$shared/stores/plugins.svelte";
	import { nav } from "$app/nav.svelte";
	import { library } from "$features/character/library.svelte";
	import ImageCreate from "./ImageCreate.svelte";
	import { theme } from "$app/theme.svelte";

	const src = { source: "images" };

	let images = $state<GeneratedImage[]>([]);
	let loading = $state(true);
	let filter = $state("all");
	async function load() {
		loading = true;
		images = await call(commands.listAllImages(), { ...src, silent: true }).catch(() => []);
		loading = false;
	}
	onMount(() => {
		void load();
		if (!library.loaded) void library.load();
	});
	const SOURCE = $derived<Record<string, string>>({
		message: $_("imagesPage.source.message"),
		manual: $_("imagesPage.source.manual"),
		game: $_("imagesPage.source.game"),
		phone: $_("imagesPage.source.phone"),
		upload: $_("imagesPage.source.upload"),
		background: $_("imagesPage.source.background"),
		asset: $_("imagesPage.source.asset"),
		avatar: $_("imagesPage.source.avatar"),
		cover: $_("imagesPage.source.cover"),
		persona: $_("imagesPage.source.persona"),
		unknown: $_("imagesPage.source.unknown"),
	});
	/** 사용자가 다는 태그: 생성 프리셋으로 쓸지 / 그냥 올린 그림 */
	const TAGS = $derived<{ id: string; label: string }[]>([
		{ id: "preset", label: $_("imagesPage.tagPreset") },
		{ id: "upload", label: $_("imagesPage.tagUpload") },
	]);
	const tagLabel = (id: string) => TAGS.find((t) => t.id === id)?.label ?? id;
	/** 쓰이고 있는 모든 태그 (기본 태그 + 사용자가 만든 태그) */
	const allTags = $derived([...new Set([...TAGS.map((t) => t.id), ...images.flatMap((i) => i.tags)])]);
	const FILTERS = $derived([
		{ value: "all", label: $_("imagesPage.filterAll", { values: { count: images.length } }) },
		...allTags.map((t) => ({ value: `tag:${t}`, label: `#${tagLabel(t)} ${images.filter((i) => i.tags.includes(t)).length}` })).filter((o) => !o.label.endsWith(" 0")),
		...Object.entries(SOURCE)
			.map(([value, label]) => ({ value, label: `${label} ${images.filter((i) => i.source === value).length}` }))
			.filter((o) => !o.label.endsWith(" 0")),
	]);
	// ---- 거르기 · 찾기 · 정렬 ----
	let query = $state("");
	let searchOpen = $state(false);
	let sort = $state<string>("new");
	const SORTS = $derived([
		{ value: "new", label: $_("imagesPage.sortNew") },
		{ value: "old", label: $_("imagesPage.sortOld") },
		{ value: "name", label: $_("imagesPage.sortName") },
	]);
	const filtered = $derived(
		filter === "all" ? images : filter.startsWith("tag:") ? images.filter((i) => i.tags.includes(filter.slice(4))) : images.filter((i) => i.source === filter),
	);
	const shown = $derived.by(() => {
		const q = query.trim().toLowerCase();
		const list = q ? filtered.filter((i) => title(i).toLowerCase().includes(q) || i.prompt.toLowerCase().includes(q) || i.tags.some((t) => t.includes(q))) : filtered;
		const by = sort === "name" ? (a: GeneratedImage, b: GeneratedImage) => title(a).localeCompare(title(b), "ko") : sort === "old" ? (a: GeneratedImage, b: GeneratedImage) => a.createdAt - b.createdAt : (a: GeneratedImage, b: GeneratedImage) => b.createdAt - a.createdAt;
		return [...list].sort(by);
	});
	// ---- 많이 있을 때: 처음엔 BATCH장만 그리고 끝에 닿으면 더 (1-D-5) ----
	// 3000장을 한 번에 그리면 화면이 몇 초 멈췄다. 고르기·모두 선택·찾기는 전체(shown)를 그대로 쓴다
	const BATCH = 120;
	let limit = $state(BATCH);
	const rendered = $derived(shown.slice(0, limit));
	// 거르기·정렬·찾기를 바꾸면 처음부터
	$effect(() => {
		void filter;
		void sort;
		void query;
		limit = BATCH;
	});
	/** 목록 끝 표시가 보이면 더 그린다 */
	function more(node: HTMLElement) {
		const io = new IntersectionObserver((es) => {
			if (es.some((e) => e.isIntersecting)) limit += BATCH;
		}, { rootMargin: "600px" });
		io.observe(node);
		return { destroy: () => io.disconnect() };
	}
	let menu = $state<{ at: { x: number; y: number; align?: "start" | "end" }; items: MenuItem[]; label: string } | null>(null);
	const below = (e: MouseEvent) => {
		const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
		return { x: r.right, y: r.bottom + 4, align: "end" as const };
	};
	function filterMenu(e: MouseEvent) {
		menu = { at: below(e), label: $_("common.filter"), items: FILTERS.map((f) => ({ label: f.label, icon: filter === f.value ? Check : undefined, onselect: () => (filter = f.value) })) };
	}
	function sortMenu(e: MouseEvent) {
		const opts = [
			{ v: "new", l: $_("imagesPage.sortNew") },
			{ v: "old", l: $_("imagesPage.sortOld") },
			{ v: "name", l: $_("imagesPage.sortName") },
		] as const;
		menu = { at: below(e), label: $_("common.sort"), items: opts.map((o) => ({ label: o.l, icon: sort === o.v ? Check : undefined, onselect: () => (sort = o.v) })) };
	}

	// ---- 오른쪽 클릭 메뉴 ----
	function imageMenu(e: MouseEvent, img: GeneratedImage) {
		e.preventDefault();
		const mine = owned(img);
		const canTag = taggable(img);
		menu = {
			at: { x: e.clientX, y: e.clientY },
			label: $_("imagesPage.imgMenuAria"),
			items: [
				{ label: $_("imagesPage.viewLarge"), icon: Eye, onselect: () => openViewer(img) },
				...(mine ? [{ label: $_("imagesPage.rename"), icon: Pencil, onselect: () => rename(img) }] : []),
				...(img.prompt && plugins.on("imagegen") ? [{ label: $_("imagesPage.reusePrompt"), icon: Sparkles, onselect: () => reuse(img) }] : []),
				...(img.conversationId ? [{ label: $_("imagesPage.goConv"), icon: MessageSquare, onselect: () => openSource(img) }] : []),
				...(canTag
					? [
							{
								label: $_("imagesPage.tagsMenu"),
								icon: Tag,
								separator: true,
								children: [
									...allTags.map((t) => ({ label: `#${tagLabel(t)}`, checked: img.tags.includes(t), disabled: t === "preset" && !img.tags.includes(t) && !canPreset(img), onselect: () => toggleTag(img, t) })),
									{ label: $_("imagesPage.newTagMenu"), icon: Plus, separator: true, onselect: () => addCustomTag(img) },
								],
							},
						]
					: []),
				// 파일 위치 열기는 데스크톱만 (Android에는 탐색기가 없음)
				...(layout.desktop ? [{ label: $_("imagesPage.revealFile"), icon: FolderOpen, separator: true, onselect: () => reveal(img) }] : []),
				...(mine ? [{ label: $_("common.delete"), icon: Trash2, danger: true, onselect: () => remove(img) }] : []),
			],
		};
	}
	function metaKeyLabel(key: string): string {
		const map: Record<string, string> = {
			크기: $_("finalPolish.imgMetaSize"),
			단계: $_("finalPolish.imgMetaSteps"),
			"프롬프트 반영": $_("finalPolish.imgMetaCfg"),
			샘플러: $_("finalPolish.imgMetaSampler"),
			시드: $_("finalPolish.imgMetaSeed"),
			"노이즈 스케줄": $_("finalPolish.imgMetaNoise"),
			SMEA: $_("finalPolish.imgMetaSmea"),
		};
		return map[key] ?? key;
	}
	function imageInfoRows(img: GeneratedImage) {
		const neg = negativeOf(img);
		const rest = img.meta.filter((m) => m.key !== "네거티브");
		return [
			{ label: $_("imagesPage.infoSource"), value: SOURCE[img.source] ?? img.source },
			...(img.model ? [{ label: $_("imagesPage.infoModel"), value: img.model }] : []),
			{ label: $_("imagesPage.infoDate"), value: when(img.createdAt) },
			...(img.tags.length ? [{ label: $_("imagesPage.infoTags"), value: img.tags.map((t) => `#${tagLabel(t)}`).join(" ") }] : []),
			...(img.prompt ? [{ label: $_("imagesPage.infoPrompt"), value: img.prompt }] : []),
			...(neg ? [{ label: $_("imagesPage.infoNegative"), value: neg }] : []),
			...rest.map((m) => ({ label: metaKeyLabel(m.key), value: m.value })),
		];
	}
	function openViewer(img: GeneratedImage) {
		const srcUrl = fileSrc(img.path);
		if (!srcUrl) return;
		const latest = () => images.find((x) => x.path === img.path) ?? img;
		imageViewer.open({
			src: srcUrl,
			name: title(img),
			info: imageInfoRows(img),
			rename: owned(img) ? () => rename(latest()) : undefined,
			remove: owned(img) ? () => remove(latest()) : undefined,
			extraItems: () => {
				const cur = latest();
				const canTag = taggable(cur);
				return [
					...(cur.prompt && plugins.on("imagegen") ? [{ label: $_("imagesPage.reusePrompt"), icon: Sparkles, onselect: () => reuse(cur) }] : []),
					...(cur.conversationId ? [{ label: $_("imagesPage.goConv"), icon: MessageSquare, onselect: () => (imageViewer.close(), openSource(cur)) }] : []),
					...(canTag
						? [
								{
									label: $_("imagesPage.tagsMenu"),
									icon: Tag,
									children: [
										...allTags.map((t) => ({ label: `#${tagLabel(t)}`, checked: cur.tags.includes(t), disabled: t === "preset" && !cur.tags.includes(t) && !canPreset(cur), onselect: () => toggleTag(cur, t) })),
										{ label: $_("imagesPage.newTagMenu"), icon: Plus, separator: true, onselect: () => addCustomTag(cur) },
									],
								},
							]
						: []),
				];
			},
		});
	}
	async function rename(img: GeneratedImage) {
		const name = await promptText({ title: $_("imagesPage.rename"), message: $_("imagesPage.renameMsg"), input: { value: title(img), placeholder: $_("imagesPage.namePlaceholder") }, confirmLabel: $_("imagesPage.renameConfirm") });
		if (!name?.trim()) return;
		const saved = await call(commands.renameGalleryImage(img.path, name), src);
		images = images.map((i) => (i.path === img.path ? { ...i, name: saved.name } : i));
		const updated = images.find((i) => i.path === img.path);
		if (updated && imageViewer.current?.src === fileSrc(img.path)) {
			imageViewer.patch({ name: title(updated), info: imageInfoRows(updated) });
		}
	}
	/** 원본: 앱 안 창으로는 파일을 열 수 없어 파일 탐색기에서 보여준다 */
	async function reveal(img: GeneratedImage) {
		if (/^https?:/.test(img.path)) return void window.open(img.path, "_blank");
		await revealItemInDir(img.path).catch((e) => toasts.error($_("imagesPage.revealErr"), String(e)));
	}

	// ---- 끌어다 놓아 올리기 ----
	async function importFiles(files: File[]) {
		const paths = await Promise.all(files.filter(isImage).map((f) => storeFile(f)));
		if (!paths.length) return;
		const added = await jobs.run($_("imagesPage.imagesCountJob", { values: { count: paths.length } }), () => call(commands.importGalleryImages(paths), src), { group: $_("imagesPage.uploadJobGroup") });
		if (!added) return;
		images = [...added, ...images];
		toasts.success($_("imagesPage.uploadedToast", { values: { count: added.length } }));
	}
	$effect(() => drops.register({ label: $_("imagesPage.dropLabel"), hint: $_("imagesPage.dropHint"), accept: isImage, ondrop: importFiles }));

	/** 그림 아래 이름: 원래 파일 이름 → 프롬프트 → 파일 이름 */
	const title = (img: GeneratedImage) => img.name || img.prompt || img.path.split(/[\\/]/).pop() || "";
	/** 다른 곳(캐릭터·페르소나·에셋)의 그림은 원래 화면에서 관리 */
	const owned = (img: GeneratedImage) => !img.readonly;
	const negativeOf = (img: GeneratedImage) => img.meta.find((m) => m.key === "네거티브")?.value ?? "";

	// ---- 태그 ----
	/** 태그를 달 수 있는 그림: 갤러리에서 만든·올린 그림 (배경 목록·CG 에셋·아바타는 원래 자리에서 관리) */
	const taggable = (img: GeneratedImage) => !img.readonly && img.source !== "background" && img.source !== "asset";
	/** #프리셋은 프롬프트나 네거티브 프롬프트가 있는 그림만 (생성 정보가 없으면 프리셋으로 쓸 게 없음) */
	const canPreset = (img: GeneratedImage) => !!img.prompt.trim() || !!negativeOf(img).trim();
	async function setTags(img: GeneratedImage, tags: string[]) {
		const saved = await call(commands.setImageTags(img.path, tags), src);
		images = images.map((i) => (i.path === img.path ? { ...i, tags: saved.tags } : i));
		const updated = images.find((i) => i.path === img.path);
		if (updated && imageViewer.current?.src === fileSrc(img.path)) {
			imageViewer.patch({ info: imageInfoRows(updated) });
		}
	}
	async function addCustomTag(img: GeneratedImage) {
		const t = await promptText({ title: $_("imagesPage.addTagTitle"), message: $_("imagesPage.addTagMsg"), input: { value: "", placeholder: $_("imagesPage.addTagPlaceholder") }, confirmLabel: $_("imagesPage.addBtn") });
		const tag = t?.trim().replace(/^#/, "");
		if (!tag || img.tags.includes(tag)) return;
		await setTags(img, [...img.tags, tag]);
	}
	async function toggleTag(img: GeneratedImage, tag: string) {
		if (!taggable(img)) return;
		if (tag === "preset" && !img.tags.includes("preset") && !canPreset(img)) return toasts.info($_("imagesPage.presetNeedPrompt"));
		const tags = img.tags.includes(tag) ? img.tags.filter((t) => t !== tag) : [...img.tags, tag];
		await setTags(img, tags);
	}

	// ---- 여러 장에 태그 달기 ----
	function bulkTagMenu(e: MouseEvent) {
		const targets = () => images.filter((i) => picked.has(i.path) && taggable(i));
		const apply = async (tag: string) => {
			const list = targets().filter((i) => !i.tags.includes(tag) && (tag !== "preset" || canPreset(i)));
			for (const i of list) await setTags(i, [...i.tags, tag]);
			toasts.success(list.length ? $_("imagesPage.bulkTaggedToast", { values: { count: list.length, tag: tagLabel(tag) } }) : $_("imagesPage.bulkTagNoneToast"));
		};
		menu = {
			at: below(e),
			label: $_("imagesPage.bulkTagLabel"),
			items: [
				...allTags.map((t) => ({ label: `#${tagLabel(t)}`, icon: Tag, onselect: () => apply(t) })),
				{
					label: $_("imagesPage.newTagMenu"),
					icon: Plus,
					separator: true,
					onselect: async () => {
						const t = await promptText({ title: $_("imagesPage.newTagTitle"), message: $_("imagesPage.newTagMsg"), input: { value: "", placeholder: $_("imagesPage.newTagPlaceholder") }, confirmLabel: $_("imagesPage.applyTagBtn") });
						const tag = t?.trim().replace(/^#/, "");
						if (tag) await apply(tag);
					},
				},
			],
		};
	}

	// ---- 정보 보기 (크게 볼 때 기본은 그림만) ----
	let showInfo = $state(false);

	// ---- 여러 장 고르기 · 단체 삭제 ----
	let selecting = $state(false);
	// 선택 모드를 끄면(아이콘을 다시 누르는 등) 고른 것도 푼다
	$effect(() => {
		if (!selecting) picked = new Set();
	});
	let picked = $state<Set<string>>(new Set());
	function pick(img: GeneratedImage) {
		if (!owned(img)) return toasts.info($_("imagesPage.readonlyToast"));
		const next = new Set(picked);
		if (next.has(img.path)) next.delete(img.path);
		else next.add(img.path);
		picked = next;
	}
	function stopSelecting() {
		selecting = false;
		picked = new Set();
	}
	async function removePicked() {
		const list = images.filter((i) => picked.has(i.path));
		if (!list.length) return;
		if (!(await confirmAction({ title: $_("imagesPage.deleteTitle"), message: $_("imagesPage.deleteManyMsg", { values: { count: list.length } }), confirmLabel: $_("imagesPage.deleteManyBtn", { values: { count: list.length } }), danger: true }))) return;
		const bgs = list.filter((i) => i.source === "background");
		const gallery = list.filter((i) => i.source !== "background");
		const n = await jobs.run(
			$_("imagesPage.imagesCountJob", { values: { count: list.length } }),
			async () => {
				let n = gallery.length ? await call(commands.deleteGalleryImages(gallery.map((i) => i.path)), src) : 0;
				for (const b of bgs) {
					await theme.removeBackground(b.path);
					n++;
				}
				return n;
			},
			{ group: $_("imagesPage.deleteJobGroup") },
		);
		if (n !== undefined) toasts.success($_("imagesPage.deletedCountToast", { values: { count: n } }));
		stopSelecting();
		await load();
	}

	// ---- 올리기 ----
	async function upload() {
		const paths = await pickFiles([{ name: $_("imagesPage.imageFileLabel"), extensions: ["png", "jpg", "jpeg", "webp", "gif"] }], true);
		if (!paths.length) return;
		const added = await jobs.run($_("imagesPage.imagesCountJob", { values: { count: paths.length } }), () => call(commands.importGalleryImages(paths), src), { group: $_("imagesPage.uploadJobGroup") });
		if (!added) return;
		images = [...added, ...images];
		const withInfo = added.filter((a) => a.prompt || a.meta.length > 1).length;
		toasts.success(withInfo ? $_("imagesPage.uploadedWithInfo", { values: { count: added.length, info: withInfo } }) : $_("imagesPage.uploadedToast", { values: { count: added.length } }));
	}

	async function remove(img: GeneratedImage) {
		if (!(await confirmAction({ title: $_("imagesPage.deleteTitle"), message: $_("imagesPage.deleteOneMsg"), confirmLabel: $_("common.delete"), danger: true }))) return;
		const name = img.path.split(/[\\/]/).pop() ?? $_("imagesPage.imageFileLabel");
		const ok = await jobs.run(name, async () => (img.source === "background" ? theme.removeBackground(img.path) : call(commands.deleteGeneratedImage(img.path), src)).then(() => true), { group: $_("imagesPage.deleteJobGroup") });
		if (!ok) return;
		images = images.filter((i) => i.path !== img.path);
		if (imageViewer.current?.src === fileSrc(img.path)) imageViewer.close();
	}
	async function copyText(text: string, what: string) {
		await navigator.clipboard.writeText(text).then(
			() => toasts.success($_("imagesPage.copiedWhatToast", { values: { what } })),
			() => toasts.error($_("imagesPage.copyFailedToast")),
		);
	}
	function openSource(img: GeneratedImage) {
		const conv = img.conversationId ? library.conversation(img.conversationId) : undefined;
		if (!conv) return toasts.info($_("imagesPage.sourceConvNotFound"));
		if (conv.mode === "phone") return nav.go({ page: "phone" });
		nav.go({ page: "character", charId: conv.characterId ?? undefined, convId: conv.id });
	}
	const when = (t: number) => new Date(t).toLocaleString(getAppLocale() === "en" ? "en-US" : "ko-KR");

	// ---- 만들기 (위에 열리는 칸) ----
	let creating = $state(false);
	let createKey = $state(0);
	let reusePrompt = $state("");
	function reuse(img: GeneratedImage) {
		reusePrompt = img.prompt;
		createKey++;
		creating = true;
		imageViewer.close();
	}
</script>

<div class="page">
	<div class="inner">
		<header class="head">
			<div class="title-wrap">
				<h1>{$_("nav.images")}</h1>
				<span class="count">{images.length}{$_("common.unitImage")}</span>
			</div>
			<ListToolbar bind:query bind:filter bind:sort bind:selecting placeholder={$_("imagesPage.searchPlaceholder")} filters={FILTERS} sorts={SORTS} canSelect={images.some(owned)}>
				{#if plugins.on("imagegen")}
					<SplitButton label={creating ? $_("common.close") : $_("common.create")} onclick={() => (creating = !creating)} menuLabel={$_("common.import")} items={[{ label: $_("imagesPage.uploadFileMenu"), icon: Upload, onselect: upload }]} />
				{:else}
					<Button variant="primary" onclick={upload}>{#snippet icon()}<Upload />{/snippet}{$_("common.import")}</Button>
				{/if}
			</ListToolbar>
		</header>

		{#if creating && plugins.on("imagegen")}
			<section class="create-box">
				{#key createKey}
					<ImageCreate initialPrompt={reusePrompt} onmade={(img) => (images = [img, ...images])} onview={(img) => openViewer(img)} />
				{/key}
			</section>
		{/if}

		{#if loading}
			<div class="making"><Spinner /></div>
		{:else if !shown.length}
			<EmptyState title={$_("imagesPage.emptyTitle")} description={$_("imagesPage.emptyDesc")}>
				{#snippet action()}<Button onclick={upload}>{#snippet icon()}<Upload />{/snippet}{$_("imagesPage.uploadBtn")}</Button>{/snippet}
			</EmptyState>
		{:else}
			{#if selecting}
				<SelectBar count={picked.size} unit={$_("common.unitImage")} onall={() => (picked = new Set(shown.filter(owned).map((i) => i.path)))} ondelete={removePicked}>
					<Button size="sm" variant="ghost" disabled={!picked.size} onclick={bulkTagMenu}>{#snippet icon()}<Tag />{/snippet}{$_("imagesPage.bulkTagLabel")}</Button>
				</SelectBar>
			{/if}
			<div class="grid">
				{#each rendered as img (img.path)}
					<ItemCard
						title={title(img)}
						image={fileSrc(img.path)}
						ratio="3 / 4"
						{selecting}
						selected={picked.has(img.path)}
						locked={!owned(img)}
						onclick={() => (selecting ? pick(img) : openViewer(img))}
						oncontextmenu={(e) => imageMenu(e, img)}
					>
						{#snippet badges()}
							<Badge>{SOURCE[img.source] ?? img.source}</Badge>
							{#each img.tags.filter((t) => t !== "upload" || img.source !== "upload") as t (t)}<Badge tone="accent">#{tagLabel(t)}</Badge>{/each}
						{/snippet}
					</ItemCard>
				{/each}
			</div>
			{#if rendered.length < shown.length}
				<!-- 끝에 닿으면 저절로, 안 되면 눌러서 -->
				<button class="more" use:more onclick={() => (limit += BATCH)}>{$_("imagesPage.showMore")} <small>{$_("imagesPage.remainingCount", { values: { count: shown.length - rendered.length } })}</small></button>
			{/if}
		{/if}
	</div>
</div>

{#if menu}<ContextMenu at={menu.at} label={menu.label} items={menu.items} onclose={() => (menu = null)} />{/if}

<style>
	.more {
		display: block;
		margin: var(--space-5) auto;
		padding: var(--space-2) var(--space-5);
		border: 1px solid var(--border);
		border-radius: var(--radius-full, 999px);
		background: var(--surface);
		color: var(--text-muted);
		font: inherit;
		font-size: var(--text-sm);
		cursor: pointer;
	}
	.more small {
		color: var(--text-faint);
	}
	.page {
		--selectbar-max: var(--page-wide);
		height: 100%;
		overflow: auto;
		padding: var(--space-6) var(--space-8);
	}
	.inner {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
		max-width: var(--page-wide);
		margin: 0 auto;
	}
	.head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-4);
		flex-wrap: wrap;
	}
	.title-wrap {
		display: flex;
		align-items: baseline;
		gap: var(--space-2);
	}
	.head h1 {
		white-space: nowrap;
	}
	.count {
		color: var(--text-faint);
		font-size: var(--text-sm);
		font-variant-numeric: tabular-nums;
	}
	.create-box {
		padding: var(--space-5);
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		background: var(--surface);
	}
	.making {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		color: var(--text-muted);
		font-size: var(--text-sm);
	}
	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(min(180px, 45%), 1fr));
		gap: var(--space-3);
	}
	.vhead {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-2);
	}
	.vname {
		margin: 0;
		font-weight: 600;
		overflow-wrap: anywhere;
	}
	.tags {
		display: flex;
		gap: 6px;
		flex-wrap: wrap;
	}
	.tag {
		padding: 3px 10px;
		border: 1px solid var(--border);
		border-radius: var(--radius-full);
		background: transparent;
		color: var(--text-muted);
		font: inherit;
		font-size: var(--text-xs);
		cursor: pointer;
	}
	.tag:disabled {
		opacity: 0.45;
		cursor: not-allowed;
	}
	.tag.add {
		border-style: dashed;
	}
	.tag.on {
		border-color: var(--accent);
		background: var(--accent-soft);
		color: var(--accent-text);
	}
	.viewer {
		position: fixed;
		inset: 0;
		z-index: var(--z-viewer);
		display: grid;
		place-items: center;
		padding: var(--space-5);
	}
	.scrim {
		position: absolute;
		inset: 0;
		border: none;
		background: rgb(0 0 0 / 0.6);
	}
	.box {
		position: relative;
		display: flex;
		gap: var(--space-4);
		width: 90vw;
		max-width: var(--page-xwide);
		max-height: 90vh;
		padding: var(--space-4);
		border-radius: var(--radius-lg);
		background: var(--surface);
		box-shadow: var(--shadow-lg);
		flex-direction: column;
		overflow: auto;
	}
	.box > img {
		width: 100%;
		height: 100%;
		max-height: calc(90vh - 2 * var(--space-4));
		border-radius: var(--radius-md);
		overflow: hidden;
		object-fit: contain;
	}
	.info {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		width: 100%;
		min-width: 0;
	}

	.img_meta_wrap {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
	}
	.meta {
		margin: 0;
		font-size: var(--text-xs);
		color: var(--text-faint);
	}
	.block {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}
	.bh {
		display: flex;
		align-items: center;
		justify-content: space-between;
		font-size: var(--text-xs);
		color: var(--text-muted);
	}
	.mini {
		display: grid;
		place-items: center;
		width: 24px;
		height: 24px;
		padding: 0;
		border: none;
		border-radius: var(--radius-sm);
		background: transparent;
		color: var(--text-muted);
		cursor: pointer;
	}
	.mini:hover {
		background: var(--surface-hover);
	}
	.mini :global(svg) {
		width: 14px;
		height: 14px;
	}
	.prompt {
		margin: 0;
		max-height: 180px;
		overflow: auto;
		padding: var(--space-2);
		border-radius: var(--radius-sm);
		background: var(--bg-sunken);
		font-size: var(--text-sm);
		white-space: pre-wrap;
	}
	.kv {
		display: grid;
		grid-template-columns: max-content minmax(0, 1fr);
		gap: 4px var(--space-3);
		margin: 0;
		font-size: var(--text-xs);
	}
	.kv dt {
		color: var(--text-muted);
	}
	.kv dd {
		margin: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		display: -webkit-box;
		-webkit-line-clamp: 3;
		line-clamp: 3;
		-webkit-box-orient: vertical;
		word-break: break-all;
	}
	.acts {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2);
	}
	@media (max-width: 720px) {
		.page {
			padding: var(--space-4);
		}
	}
</style>
