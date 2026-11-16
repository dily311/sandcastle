<script lang="ts">
	// 이미지 페이지 = 갤러리: 만든 그림과 올린 그림 전부. 위의 "만들기"를 누르면 만들기 칸이 열린다.
	// 그림은 앱 데이터 generated/ 폴더에 있고, 같은 이름 .json에 프롬프트·출처·생성 정보가 있다 (service/plugins.rs)
	import { onMount } from "svelte";
	import { Copy, ExternalLink, MessageSquare, RefreshCw, Sparkles, Trash2, Upload, Wand2, X } from "@lucide/svelte";
	import type { GeneratedImage } from "$ipc/bindings";
	import { Badge, Button, EmptyState, IconButton, Select, Spinner } from "$shared/ui";
	import { call, commands } from "$shared/ipc/client";
	import { fileSrc } from "$shared/ipc/file";
	import { pickFiles } from "$shared/ipc/dialog";
	import { confirmAction } from "$shared/stores/confirm.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	import { plugins } from "$shared/stores/plugins.svelte";
	import { nav } from "$app/nav.svelte";
	import { library } from "$features/character/library.svelte";
	import ImageCreate from "./ImageCreate.svelte";

	const src = { source: "images" };

	let images = $state<GeneratedImage[]>([]);
	let loading = $state(true);
	let filter = $state("all");
	async function load() {
		loading = true;
		images = await call(commands.listGeneratedImages(), { ...src, silent: true }).catch(() => []);
		loading = false;
	}
	onMount(() => {
		void load();
		if (!library.loaded) void library.load();
	});
	const SOURCE: Record<string, string> = { message: "대화", manual: "직접", game: "게임", phone: "폰", upload: "업로드", unknown: "기타" };
	const FILTERS = $derived([
		{ value: "all", label: `전부 ${images.length}` },
		...Object.entries(SOURCE)
			.map(([value, label]) => ({ value, label: `${label} ${images.filter((i) => i.source === value).length}` }))
			.filter((o) => !o.label.endsWith(" 0")),
	]);
	const shown = $derived(filter === "all" ? images : images.filter((i) => i.source === filter));
	let viewing = $state<GeneratedImage | null>(null);

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
		await call(commands.deleteGeneratedImage(img.path), src);
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
		<Select bind:value={filter} options={FILTERS} aria-label="출처로 거르기" />
		<IconButton label="새로 고침" size="sm" onclick={load}><RefreshCw /></IconButton>
		<Button size="sm" onclick={upload}>{#snippet icon()}<Upload />{/snippet}업로드</Button>
		{#if plugins.on("imagegen")}
			<Button size="sm" variant={creating ? "secondary" : "primary"} onclick={() => (creating = !creating)}>{#snippet icon()}{#if creating}<X />{:else}<Wand2 />{/if}{/snippet}{creating ? "생성 탭 닫기" : "생성하기"}</Button>
		{/if}
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
		<div class="grid">
			{#each shown as img (img.createdAt)}
				<figure>
					<button class="thumb" onclick={() => (viewing = img)}><img src={fileSrc(img.path)} alt={img.prompt} loading="lazy" /></button>
					<figcaption>
						<Badge>{SOURCE[img.source] ?? img.source}</Badge>
						<span class="p">{img.prompt || "(프롬프트 없음)"}</span>
					</figcaption>
				</figure>
			{/each}
		</div>
	{/if}
</div>

{#if viewing}
	{@const v = viewing}
	<div class="viewer" role="dialog" aria-modal="true" aria-label="그림 크게 보기">
		<button class="scrim" aria-label="닫기" onclick={() => (viewing = null)}></button>
		<div class="box">
			<img src={fileSrc(v.path)} alt={v.prompt} />
			<div class="info">
				<p class="meta"><Badge>{SOURCE[v.source] ?? v.source}</Badge> {v.model ? `${v.model} · ` : ""}{when(v.createdAt)}</p>
				{#if v.prompt}
					<div class="block">
						<div class="bh"><b>프롬프트</b><button class="mini" onclick={() => copyText(v.prompt, "프롬프트")}><Copy /></button></div>
						<p class="prompt">{v.prompt}</p>
					</div>
				{/if}
				{#if v.meta?.length}
					<dl class="kv">
						{#each v.meta as m, i (m.key)}
							<dt>{m.key}</dt>
							<dd title={m.value}>{m.value}</dd>
						{/each}
					</dl>
				{:else if !v.prompt}
					<p class="meta">생성 정보 없음</p>
				{/if}
				<div class="acts">
					{#if v.prompt && plugins.on("imagegen")}<Button size="sm" onclick={() => reuse(v)}>{#snippet icon()}<Sparkles />{/snippet}이 프롬프트로 다시</Button>{/if}
					{#if v.conversationId}<Button size="sm" onclick={() => openSource(v)}>{#snippet icon()}<MessageSquare />{/snippet}대화로</Button>{/if}
					<Button size="sm" variant="ghost" onclick={() => window.open(fileSrc(v.path) ?? "", "_blank")}>{#snippet icon()}<ExternalLink />{/snippet}원본</Button>
					<span class="sp"></span>
					<Button size="sm" variant="ghost" onclick={() => remove(v)}>{#snippet icon()}<Trash2 />{/snippet}삭제</Button>
				</div>
			</div>
		</div>
	</div>
{/if}

<style>
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
		grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
		gap: var(--space-3);
	}
	figure {
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: 6px;
	}
	.thumb {
		padding: 0;
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		overflow: hidden;
		background: var(--bg-sunken);
		aspect-ratio: 3 / 4;
		cursor: zoom-in;
	}
	.thumb img {
		width: 100%;
		height: 100%;
		object-fit: cover;
		display: block;
	}
	figcaption {
		display: flex;
		align-items: flex-start;
		gap: 6px;
		font-size: var(--text-xs);
		color: var(--text-muted);
	}
	figcaption .p {
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}
	.viewer {
		position: fixed;
		inset: 0;
		z-index: 200;
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
		max-width: 1200px;
		max-height: 90vh;
		padding: var(--space-4);
		border-radius: var(--radius-lg);
		background: var(--surface);
		box-shadow: var(--shadow-lg);
	}
	.box > img {
		max-width: 58vw;
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
    	max-width: 25vw;
		min-width: 0;
		overflow: auto;
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
		margin-top: auto;
	}
	@media (max-width: 720px) {
		.page {
			padding: var(--space-4);
		}
		.box {
			flex-direction: column;
			overflow: auto;
		}
		.box > img {
			max-width: none;
		}
		.info {
			max-width: none;
		}
	}
</style>
