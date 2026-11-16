<script lang="ts">
	// 이미지 생성 플러그인 페이지: 직접 만들기 + 만든 그림 전부 모아 보기(갤러리)
	// 그림은 앱 데이터의 generated/ 폴더에 있고, 같은 이름 .json에 프롬프트·출처가 있다 (service/plugins.rs)
	import { onMount } from "svelte";
	import { Copy, ExternalLink, ImagePlus, MessageSquare, RefreshCw, Sparkles, Trash2, Wand2, X } from "@lucide/svelte";
	import type { GeneratedImage } from "$ipc/bindings";
	import { Badge, Button, EmptyState, IconButton, Segmented, Spinner, Switch, Textarea } from "$shared/ui";
	import { call, commands } from "$shared/ipc/client";
	import { fileSrc } from "$shared/ipc/file";
	import { confirmAction } from "$shared/stores/confirm.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	import { nav } from "$app/nav.svelte";
	import { library } from "$features/character/library.svelte";
	import { pluginSetting } from "$shared/stores/aiTools.svelte";

	let { tab: routeTab }: { tab?: "create" | "gallery" } = $props();
	const tab = $derived(routeTab ?? "create");
	const src = { source: "imagegen" };

	// ---- 갤러리 ----
	let images = $state<GeneratedImage[]>([]);
	let loading = $state(true);
	let filter = $state<"all" | "message" | "manual" | "game">("all");
	async function load() {
		loading = true;
		images = await call(commands.listGeneratedImages(), { ...src, silent: true }).catch(() => []);
		loading = false;
	}
	onMount(() => {
		void load();
		if (!pluginSetting.value) void pluginSetting.load();
		if (!library.loaded) void library.load();
	});
	const shown = $derived(filter === "all" ? images : images.filter((i) => i.source === filter));
	const SOURCE: Record<string, string> = { message: "대화", manual: "직접", game: "게임", unknown: "기타" };
	let viewing = $state<GeneratedImage | null>(null);

	async function remove(img: GeneratedImage) {
		if (!(await confirmAction({ title: "그림 삭제", message: "이 그림 파일을 지움. 대화에 붙어 있던 그림도 더 이상 보이지 않음", confirmLabel: "삭제", danger: true }))) return;
		await call(commands.deleteGeneratedImage(img.path), src);
		images = images.filter((i) => i.path !== img.path);
		if (viewing?.path === img.path) viewing = null;
	}
	async function copyPrompt(img: GeneratedImage) {
		await navigator.clipboard.writeText(img.prompt).then(
			() => toasts.success("프롬프트 복사함"),
			() => toasts.error("복사 못 함"),
		);
	}
	function openSource(img: GeneratedImage) {
		const conv = img.conversationId ? library.conversation(img.conversationId) : undefined;
		if (!conv) return toasts.info("원래 대화를 찾을 수 없음");
		if (conv.mode === "phone") return nav.go({ page: "phone" });
		nav.go({ page: "character", charId: conv.characterId, convId: conv.id });
	}
	const when = (t: number) => new Date(t).toLocaleString();

	// ---- 만들기 ----
	let prompt = $state("");
	let refine = $state(false);
	let making = $state(false);
	let made = $state<GeneratedImage[]>([]);
	async function make(text = prompt) {
		if (!text.trim() || making) return;
		making = true;
		try {
			const img = await call(commands.generateImageFromPrompt(text, refine, "manual"), src);
			made = [img, ...made];
			images = [img, ...images];
		} finally {
			making = false;
		}
	}
	function reuse(img: GeneratedImage) {
		prompt = img.prompt;
		refine = false;
		nav.replace({ page: "images", tab: "create" });
	}
	const setting = $derived(pluginSetting.value?.imageGen);
	const profile = $derived(library.apis.find((a) => a.id === setting?.apiProfileId));
</script>

<div class="page">
	<header>
		<h1>이미지</h1>
		<Segmented
			label="이미지 화면"
			value={tab}
			options={[
				{ value: "create", label: "만들기" },
				{ value: "gallery", label: `갤러리 ${images.length || ""}`.trim() },
			]}
			onchange={(v) => nav.replace({ page: "images", tab: v as "create" | "gallery" })}
		/>
		<span class="sp"></span>
		<Button size="sm" variant="ghost" onclick={() => nav.go({ page: "settings", tab: "imagegen" })}>설정</Button>
	</header>

	{#if tab === "create"}
		<section class="create">
			<p class="faint">
				{#if profile}{profile.name} · {setting?.model} · {setting?.size}{:else}이미지 API가 없음 — <button class="link" onclick={() => nav.go({ page: "settings", tab: "imagegen" })}>설정에서 고르기</button>{/if}
			</p>
			<Textarea bind:value={prompt} rows={4} autosize maxRows={12} placeholder="그릴 장면. NovelAI는 태그(1girl, silver hair, forest, night…)가 잘 먹힘" aria-label="그릴 프롬프트" />
			<div class="bar">
				<Switch bind:checked={refine} label="AI가 프롬프트 다듬기" description="짧게 써도 장면 프롬프트로 정리함" />
				<span class="sp"></span>
				<Button variant="primary" loading={making} disabled={!prompt.trim()} onclick={() => make()}>{#snippet icon()}<Wand2 />{/snippet}그리기</Button>
			</div>
			{#if setting?.stylePrompt}<p class="faint">화풍 태그가 끝에 붙음: {setting.stylePrompt}</p>{/if}
			{#if making}<div class="making"><Spinner /> 그리는 중… (수십 초 걸릴 수 있음)</div>{/if}
			{#if made.length}
				<div class="grid">
					{#each made as img (img.path)}
						<figure>
							<button class="thumb" onclick={() => (viewing = img)}><img src={fileSrc(img.path)} alt={img.prompt} /></button>
							<figcaption>{img.prompt}</figcaption>
						</figure>
					{/each}
				</div>
			{/if}
		</section>
	{:else}
		<section class="gallery">
			<div class="bar">
				<Segmented
					label="출처"
					value={filter}
					options={[
						{ value: "all", label: "전부" },
						{ value: "message", label: "대화" },
						{ value: "manual", label: "직접" },
						{ value: "game", label: "게임" },
					]}
					onchange={(v) => (filter = v as typeof filter)}
				/>
				<span class="sp"></span>
				<IconButton label="새로 고침" size="sm" onclick={load}><RefreshCw /></IconButton>
			</div>
			{#if loading}
				<div class="making"><Spinner /></div>
			{:else if !shown.length}
				<EmptyState title="그림 없음" description="대화 메시지의 '이 장면 그리기'나 만들기 탭에서 그린 그림이 여기에 모임">
					{#snippet action()}<Button onclick={() => nav.replace({ page: "images", tab: "create" })}>{#snippet icon()}<ImagePlus />{/snippet}만들기</Button>{/snippet}
				</EmptyState>
			{:else}
				<div class="grid">
					{#each shown as img (img.path)}
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
		</section>
	{/if}
</div>

{#if viewing}
	{@const v = viewing}
	<div class="viewer" role="dialog" aria-modal="true" aria-label="그림 크게 보기">
		<button class="scrim" aria-label="닫기" onclick={() => (viewing = null)}></button>
		<div class="box">
			<img src={fileSrc(v.path)} alt={v.prompt} />
			<div class="info">
				<p class="meta"><Badge>{SOURCE[v.source] ?? v.source}</Badge> {v.model} · {when(v.createdAt)}</p>
				<p class="prompt">{v.prompt}</p>
				<div class="acts">
					<Button size="sm" onclick={() => copyPrompt(v)}>{#snippet icon()}<Copy />{/snippet}프롬프트 복사</Button>
					<Button size="sm" onclick={() => (reuse(v), (viewing = null))}>{#snippet icon()}<Sparkles />{/snippet}이 프롬프트로 다시</Button>
					{#if v.conversationId}<Button size="sm" onclick={() => openSource(v)}>{#snippet icon()}<MessageSquare />{/snippet}대화로</Button>{/if}
					<Button size="sm" variant="ghost" onclick={() => window.open(fileSrc(v.path) ?? "", "_blank")}>{#snippet icon()}<ExternalLink />{/snippet}원본</Button>
					<span class="sp"></span>
					<Button size="sm" variant="ghost" onclick={() => remove(v)}>{#snippet icon()}<Trash2 />{/snippet}삭제</Button>
				</div>
			</div>
			<IconButton label="닫기" size="sm" onclick={() => (viewing = null)}><X /></IconButton>
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
		gap: var(--space-3);
	}
	h1 {
		margin: 0;
		font-size: var(--text-xl);
	}
	.sp {
		flex: 1;
	}
	.create,
	.gallery {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		max-width: 1100px;
	}
	.bar {
		display: flex;
		align-items: center;
		gap: var(--space-3);
	}
	.faint {
		margin: 0;
		font-size: var(--text-sm);
		color: var(--text-faint);
	}
	.link {
		padding: 0;
		border: none;
		background: none;
		color: var(--accent-text);
		font: inherit;
		cursor: pointer;
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
		max-width: 1100px;
		max-height: 90vh;
		padding: var(--space-4);
		border-radius: var(--radius-lg);
		background: var(--surface);
		box-shadow: var(--shadow-lg);
	}
	.box > img {
		max-width: 60vw;
		max-height: calc(90vh - 2 * var(--space-4));
		border-radius: var(--radius-md);
		object-fit: contain;
	}
	.info {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		width: 320px;
		min-width: 0;
	}
	.meta {
		margin: 0;
		font-size: var(--text-xs);
		color: var(--text-faint);
	}
	.prompt {
		margin: 0;
		overflow: auto;
		font-size: var(--text-sm);
		white-space: pre-wrap;
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
			max-width: 100%;
		}
		.info {
			width: auto;
		}
	}
</style>
