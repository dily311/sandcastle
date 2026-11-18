<script lang="ts">
	// 이미지 페이지 = 갤러리: 만든 그림과 올린 그림 전부. 위의 "만들기"를 누르면 만들기 칸이 열린다.
	// 그림은 앱 데이터 generated/ 폴더에 있고, 같은 이름 .json에 프롬프트·출처·생성 정보가 있다 (service/plugins.rs)
	import { onMount } from "svelte";
	import { ArrowUpDown, Check, Ellipsis, Info, Plus, Copy, ExternalLink, Eye, Filter, FolderOpen, MessageSquare, Pencil, Search, Sparkles, Tag, Trash2, Upload, Wand2, X } from "@lucide/svelte";
	import type { GeneratedImage_Serialize as GeneratedImage } from "$ipc/bindings";
	import { Badge, Button, ContextMenu, Dialog, EmptyState, IconButton, Input, ItemCard, ListToolbar, SelectBar, SplitButton, Spinner, type MenuItem } from "$shared/ui";
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
	const SOURCE: Record<string, string> = {
		message: "대화",
		manual: "직접",
		game: "게임",
		phone: "폰",
		upload: "업로드",
		background: "배경",
		asset: "CG 에셋",
		avatar: "아바타",
		cover: "커버",
		persona: "페르소나",
		unknown: "기타",
	};
	/** 사용자가 다는 태그: 생성 프리셋으로 쓸지 / 그냥 올린 그림 / 배경 */
	const TAGS: { id: string; label: string }[] = [
		{ id: "preset", label: "프리셋" },
		{ id: "upload", label: "업로드" },
		{ id: "background", label: "배경" },
	];
	const tagLabel = (id: string) => TAGS.find((t) => t.id === id)?.label ?? id;
	/** 쓰이고 있는 모든 태그 (기본 태그 + 사용자가 만든 태그) */
	const allTags = $derived([...new Set([...TAGS.map((t) => t.id), ...images.flatMap((i) => i.tags)])]);
	const FILTERS = $derived([
		{ value: "all", label: `전부 ${images.length}` },
		...allTags.map((t) => ({ value: `tag:${t}`, label: `#${tagLabel(t)} ${images.filter((i) => i.tags.includes(t)).length}` })).filter((o) => !o.label.endsWith(" 0")),
		...Object.entries(SOURCE)
			.map(([value, label]) => ({ value, label: `${label} ${images.filter((i) => i.source === value).length}` }))
			.filter((o) => !o.label.endsWith(" 0")),
	]);
	// ---- 거르기 · 찾기 · 정렬 ----
	let query = $state("");
	let searchOpen = $state(false);
	let sort = $state<string>("new");
	const SORTS = [
		{ value: "new", label: "최신 순" },
		{ value: "old", label: "오래된 순" },
		{ value: "name", label: "이름 순" },
	];
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
		menu = { at: below(e), label: "거르기", items: FILTERS.map((f) => ({ label: f.label, icon: filter === f.value ? Check : undefined, onselect: () => (filter = f.value) })) };
	}
	function sortMenu(e: MouseEvent) {
		const opts = [
			{ v: "new", l: "최신 순" },
			{ v: "old", l: "오래된 순" },
			{ v: "name", l: "이름 순" },
		] as const;
		menu = { at: below(e), label: "정렬", items: opts.map((o) => ({ label: o.l, icon: sort === o.v ? Check : undefined, onselect: () => (sort = o.v) })) };
	}

	// ---- 오른쪽 클릭 메뉴 ----
	function imageMenu(e: MouseEvent, img: GeneratedImage) {
		e.preventDefault();
		const mine = owned(img);
		const canTag = taggable(img);
		menu = {
			at: { x: e.clientX, y: e.clientY },
			label: "그림 메뉴",
			items: [
				{ label: "크게 보기", icon: Eye, onselect: () => (viewing = img) },
				...(mine ? [{ label: "이름 바꾸기", icon: Pencil, onselect: () => rename(img) }] : []),
				...(canTag
					? [
							...allTags.map((t, i) => ({ label: `#${tagLabel(t)}`, icon: img.tags.includes(t) ? Check : Tag, separator: i === 0, disabled: t === "preset" && !img.tags.includes(t) && !canPreset(img), onselect: () => toggleTag(img, t) })),
							{ label: "새 태그…", icon: Plus, onselect: () => addCustomTag(img) },
						]
					: []),
				// 파일 위치 열기는 데스크톱만 (Android에는 탐색기가 없음)
				...(layout.desktop ? [{ label: "파일 위치 열기", icon: FolderOpen, separator: true, onselect: () => reveal(img) }] : []),
				...(mine ? [{ label: "삭제", icon: Trash2, danger: true, onselect: () => remove(img) }] : []),
			],
		};
	}
	/** 크게 보기 창의 … 메뉴: 다른 그림 뷰어와 같은 작업 (파일 위치·이름·삭제·정보) */
	function viewMenu(e: MouseEvent, img: GeneratedImage) {
		const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
		menu = {
			at: { x: r.right, y: r.bottom + 4, align: "end" },
			label: "그림 메뉴",
			items: [
				{ label: "정보 보기", icon: Info, onselect: () => (showInfo = true) },
				...(layout.desktop ? [{ label: "파일 위치 열기", icon: ExternalLink, onselect: () => reveal(img) }] : []),
				...(owned(img) ? [{ label: "이름 바꾸기", icon: Pencil, separator: true, onselect: () => rename(img) }] : []),
				...(owned(img) ? [{ label: "삭제", icon: Trash2, danger: true, separator: true, onselect: () => remove(img) }] : []),
			],
		};
	}
	async function rename(img: GeneratedImage) {
		const name = await promptText({ title: "이름 바꾸기", message: "갤러리에 보일 이름 (파일은 그대로)", input: { value: title(img), placeholder: "이름" }, confirmLabel: "바꾸기" });
		if (!name?.trim()) return;
		const saved = await call(commands.renameGalleryImage(img.path, name), src);
		images = images.map((i) => (i.path === img.path ? { ...i, name: saved.name } : i));
		if (viewing?.path === img.path) viewing = { ...viewing, name: saved.name };
	}
	/** 원본: 앱 안 창으로는 파일을 열 수 없어 파일 탐색기에서 보여준다 */
	async function reveal(img: GeneratedImage) {
		if (/^https?:/.test(img.path)) return void window.open(img.path, "_blank");
		await revealItemInDir(img.path).catch((e) => toasts.error("파일 위치를 열 수 없음", String(e)));
	}

	// ---- 끌어다 놓아 올리기 ----
	async function importFiles(files: File[]) {
		const paths = await Promise.all(files.filter(isImage).map((f) => storeFile(f)));
		if (!paths.length) return;
		const added = await call(commands.importGalleryImages(paths), src);
		images = [...added, ...images];
		toasts.success(`${added.length}장 업로드 됨`);
	}
	$effect(() => drops.register({ label: "갤러리에 올리기", hint: "PNG·JPG·WEBP·GIF (AI 그림이면 생성 정보도 읽음)", accept: isImage, ondrop: importFiles }));
	let viewing = $state<GeneratedImage | null>(null);

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
		if (viewing?.path === img.path) viewing = { ...viewing, tags: saved.tags };
	}
	async function addCustomTag(img: GeneratedImage) {
		const t = await promptText({ title: "태그 추가", message: "이 그림에 달 태그 (같은 태그로 거를 수 있음)", input: { value: "", placeholder: "예: 엘리안, 의상" }, confirmLabel: "추가" });
		const tag = t?.trim().replace(/^#/, "");
		if (!tag || img.tags.includes(tag)) return;
		await setTags(img, [...img.tags, tag]);
	}
	async function toggleTag(img: GeneratedImage, tag: string) {
		if (!taggable(img)) return;
		if (tag === "preset" && !img.tags.includes("preset") && !canPreset(img)) return toasts.info("프롬프트나 네거티브 프롬프트가 있는 그림만 프리셋으로 쓸 수 있음");
		const tags = img.tags.includes(tag) ? img.tags.filter((t) => t !== tag) : [...img.tags, tag];
		await setTags(img, tags);
	}

	// ---- 여러 장에 태그 달기 ----
	function bulkTagMenu(e: MouseEvent) {
		const targets = () => images.filter((i) => picked.has(i.path) && taggable(i));
		const apply = async (tag: string) => {
			const list = targets().filter((i) => !i.tags.includes(tag) && (tag !== "preset" || canPreset(i)));
			for (const i of list) await setTags(i, [...i.tags, tag]);
			toasts.success(list.length ? `${list.length}장에 #${tagLabel(tag)}` : "달 수 있는 그림 없음");
		};
		menu = {
			at: below(e),
			label: "태그 달기",
			items: [
				...allTags.map((t) => ({ label: `#${tagLabel(t)}`, icon: Tag, onselect: () => apply(t) })),
				{
					label: "새 태그…",
					icon: Plus,
					separator: true,
					onselect: async () => {
						const t = await promptText({ title: "새 태그", message: "고른 그림들에 달 태그", input: { value: "", placeholder: "예: 엘리안" }, confirmLabel: "달기" });
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
		if (!owned(img)) return toasts.info("캐릭터·페르소나의 그림은 그 화면에서 관리함");
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
		if (!(await confirmAction({ title: "그림 삭제", message: `고른 그림 ${list.length}장을 지움. 대화·배경에 쓰던 그림도 더 이상 보이지 않음. 되돌릴 수 없음`, confirmLabel: `${list.length}장 삭제`, danger: true }))) return;
		const bgs = list.filter((i) => i.source === "background");
		const gallery = list.filter((i) => i.source !== "background");
		let n = gallery.length ? await call(commands.deleteGalleryImages(gallery.map((i) => i.path)), src) : 0;
		for (const b of bgs) {
			await theme.removeBackground(b.path);
			n++;
		}
		toasts.success(`${n}장 지움`);
		stopSelecting();
		await load();
	}

	// ---- 올리기 ----
	async function upload() {
		const paths = await pickFiles([{ name: "그림", extensions: ["png", "jpg", "jpeg", "webp", "gif"] }], true);
		if (!paths.length) return;
		const added = await call(commands.importGalleryImages(paths), src);
		images = [...added, ...images];
		const withInfo = added.filter((a) => a.prompt || a.meta.length > 1).length;
		toasts.success(`${added.length}장 업로드 됨${withInfo ? ` · 생성 정보 ${withInfo}장` : ""}`);
	}

	async function remove(img: GeneratedImage) {
		if (!(await confirmAction({ title: "그림 삭제", message: "이 그림 파일을 지움. 대화에 붙어 있던 그림도 더 이상 보이지 않음", confirmLabel: "삭제", danger: true }))) return;
		if (img.source === "background") await theme.removeBackground(img.path);
		else await call(commands.deleteGeneratedImage(img.path), src);
		images = images.filter((i) => i.path !== img.path);
		if (viewing?.path === img.path) viewing = null;
	}
	async function copyText(text: string, what: string) {
		await navigator.clipboard.writeText(text).then(
			() => toasts.success(`${what} 복사함`),
			() => toasts.error("복사 못 함"),
		);
	}
	function openSource(img: GeneratedImage) {
		const conv = img.conversationId ? library.conversation(img.conversationId) : undefined;
		if (!conv) return toasts.info("원래 대화를 찾을 수 없음");
		if (conv.mode === "phone") return nav.go({ page: "phone" });
		nav.go({ page: "character", charId: conv.characterId ?? undefined, convId: conv.id });
	}
	const when = (t: number) => new Date(t).toLocaleString();

	// ---- 만들기 (위에 열리는 칸) ----
	let creating = $state(false);
	let createKey = $state(0);
	let reusePrompt = $state("");
	function reuse(img: GeneratedImage) {
		reusePrompt = img.prompt;
		createKey++;
		creating = true;
		viewing = null;
	}
</script>

<div class="page">
	<header>
		<h1>이미지</h1>
		<span class="count">{images.length}장</span>
		<span class="sp"></span>
		<ListToolbar bind:query bind:filter bind:sort bind:selecting placeholder="이름·프롬프트·태그" filters={FILTERS} sorts={SORTS} canSelect={images.some(owned)}>
			{#if plugins.on("imagegen")}
				<SplitButton label={creating ? "생성 닫기" : "새로 만들기"} onclick={() => (creating = !creating)} menuLabel="그림 올리기" items={[{ label: "파일 업로드", icon: Upload, onselect: upload }]} />
			{:else}
				<Button variant="primary" onclick={upload}>{#snippet icon()}<Upload />{/snippet}업로드</Button>
			{/if}
		</ListToolbar>
	</header>

	{#if creating && plugins.on("imagegen")}
		<section class="create-box">
			{#key createKey}
				<ImageCreate initialPrompt={reusePrompt} onmade={(img) => (images = [img, ...images])} onview={(img) => (viewing = img)} />
			{/key}
		</section>
	{/if}

	{#if loading}
		<div class="making"><Spinner /></div>
	{:else if !shown.length}
		<EmptyState title="그림 없음" description="대화의 '이 장면 그리기', 만들기, 게임 장면 그림, 업로드 한 그림이 여기 모임">
			{#snippet action()}<Button onclick={upload}>{#snippet icon()}<Upload />{/snippet}그림 올리기</Button>{/snippet}
		</EmptyState>
	{:else}
		{#if selecting}
			<SelectBar count={picked.size} unit="장" onall={() => (picked = new Set(shown.filter(owned).map((i) => i.path)))} ondelete={removePicked}>
				<Button size="sm" variant="ghost" disabled={!picked.size} onclick={bulkTagMenu}>{#snippet icon()}<Tag />{/snippet}태그 달기</Button>
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
					onclick={() => (selecting ? pick(img) : (viewing = img))}
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
			<button class="more" use:more onclick={() => (limit += BATCH)}>더 보기 <small>({shown.length - rendered.length}장 남음)</small></button>
		{/if}
	{/if}
</div>

{#if viewing}
	{@const v = viewing}
	{@const neg = negativeOf(v)}
	{@const rest = v.meta.filter((m) => m.key !== "네거티브")}
	<div class="viewer" role="dialog" aria-modal="true" aria-label="그림 크게 보기">
		<button class="scrim" aria-label="닫기" onclick={() => (viewing = null)}></button>
		<div class="box">
			<img src={fileSrc(v.path)} alt={v.prompt} data-no-zoom />
			<div class="info">
				<div class="vhead">
					<p class="vname">{title(v)}</p>
					<IconButton label="그림 메뉴" size="sm" onclick={(e) => viewMenu(e, v)}><Ellipsis /></IconButton>
				</div>
				<div class="acts">
					{#if v.prompt && plugins.on("imagegen")}<Button size="sm" onclick={() => reuse(v)}>{#snippet icon()}<Sparkles />{/snippet}이 프롬프트로 다시</Button>{/if}
					{#if v.conversationId}<Button size="sm" onclick={() => openSource(v)}>{#snippet icon()}<MessageSquare />{/snippet}대화로</Button>{/if}
				</div>
				{#if taggable(v)}
					<div class="tags" role="group" aria-label="태그">
						{#each allTags as t (t)}
							<button class="tag" class:on={v.tags.includes(t)} aria-pressed={v.tags.includes(t)} disabled={t === "preset" && !v.tags.includes(t) && !canPreset(v)} title={t === "preset" && !canPreset(v) ? "프롬프트가 없는 그림은 프리셋으로 못 씀" : undefined} onclick={() => toggleTag(v, t)}>#{tagLabel(t)}</button>
						{/each}
						<button class="tag add" onclick={() => addCustomTag(v)}>+ 태그</button>
					</div>
				{/if}
				<!-- 정보는 그림 위에 따로 뜨는 창 (… 메뉴의 "정보 보기") -->
				<Dialog open={showInfo} title="정보" size="md" onclose={() => (showInfo = false)}>
				<p class="meta"><Badge>{SOURCE[v.source] ?? v.source}</Badge> {v.model ? `${v.model} · ` : ""}{when(v.createdAt)}</p>
				{#if v.prompt}
					<div class="block">
						<div class="bh"><b>프롬프트</b><button class="mini" aria-label="프롬프트 복사" onclick={() => copyText(v.prompt, "프롬프트")}><Copy /></button></div>
						<p class="prompt">{v.prompt}</p>
					</div>
				{/if}
				{#if neg}
					<div class="block">
						<div class="bh"><b>네거티브 프롬프트</b><button class="mini" aria-label="네거티브 프롬프트 복사" onclick={() => copyText(neg, "네거티브 프롬프트")}><Copy /></button></div>
						<p class="prompt">{neg}</p>
					</div>
				{/if}
				{#if rest.length}
					<dl class="kv">
						{#each rest as m, i (`${m.key}-${i}`)}
							<dt>{m.key}</dt>
							<dd title={m.value}>{m.value}</dd>
						{/each}
					</dl>
				{:else if !v.prompt && !neg}
					<p class="meta">생성 정보 없음</p>
				{/if}
				</Dialog>
			</div>
		</div>
	</div>
{/if}
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
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
		height: 100%;
		overflow: auto;
		padding: var(--space-5) var(--space-6);
	}
	header {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		flex-wrap: wrap;
	}
	h1 {
		margin: 0;
		font-size: var(--text-xl);
	}
	.count {
		color: var(--text-faint);
		font-size: var(--text-sm);
	}
	.sp {
		flex: 1;
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
