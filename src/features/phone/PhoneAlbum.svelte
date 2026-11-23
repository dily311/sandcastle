<script lang="ts">
	// 폰 사진 앱: 이 폰에서 만든 그림 — 카메라로 만든 것(앨범 = 폰 주인)과 이 폰 채팅방에서 그린 것
	import { jobs } from "$shared/stores/jobs.svelte";
	import { onMount } from "svelte";
	import { ChevronLeft, Trash2, X } from "@lucide/svelte";
	import type { GeneratedImage } from "$ipc/bindings";
	import { call, commands } from "$shared/ipc/client";
	import { fileSrc } from "$shared/ipc/file";
	import { confirmAction } from "$shared/stores/confirm.svelte";

	interface Props {
		/** 폰 주인 앨범 키 ("persona:<id>") */
		album: string;
		/** 이 폰의 채팅방 id들 */
		chatIds: string[];
		onback: () => void;
		/** 카메라 열기 */
		oncamera?: () => void;
	}
	let { album, chatIds, onback, oncamera }: Props = $props();

	let all = $state<GeneratedImage[]>([]);
	let loading = $state(true);
	onMount(async () => {
		all = await call(commands.listGeneratedImages(), { source: "phone", silent: true }).catch(() => []);
		loading = false;
	});
	const photos = $derived(all.filter((i) => i.album === album || (i.conversationId && chatIds.includes(i.conversationId))));
	let viewing = $state<GeneratedImage | null>(null);

	async function remove(img: GeneratedImage) {
		if (!(await confirmAction({ title: "사진 삭제", message: "이 그림 파일을 지움 (이미지 갤러리에서도 사라짐)", confirmLabel: "삭제", danger: true }))) return;
		const ok = await jobs.run(img.path.split(/[\\/]/).pop() ?? "사진", () => call(commands.deleteGeneratedImage(img.path), { source: "phone" }).then(() => true), { group: "사진 지우기" });
		if (!ok) return;
		all = all.filter((i) => i.path !== img.path);
		viewing = null;
	}
</script>

{#if viewing}
	{@const v = viewing}
	<div class="viewer">
		<div class="vbar">
			<button class="plain" aria-label="닫기" onclick={() => (viewing = null)}><X /></button>
			<span class="grow"></span>
			<button class="plain" aria-label="삭제" onclick={() => remove(v)}><Trash2 /></button>
		</div>
		<img src={fileSrc(v.path)} alt={v.prompt} />
		{#if v.prompt}<p class="cap">{v.prompt}</p>{/if}
	</div>
{:else}
	<div class="app-bar">
		<button class="plain" aria-label="뒤로" onclick={onback}><ChevronLeft /></button>
		<h2 class="grow">사진</h2>
		{#if oncamera}<button class="link" onclick={oncamera}>카메라</button>{/if}
	</div>
	{#if loading}
		<p class="none">불러오는 중…</p>
	{:else if !photos.length}
		<p class="none">아직 사진 없음<br />카메라로 찍거나 채팅방에서 '이 장면 그리기'를 쓰면 여기 모임</p>
	{:else}
		<div class="grid">
			{#each photos as p (p.path)}
				<button class="ph" onclick={() => (viewing = p)}><img src={fileSrc(p.path)} alt={p.prompt} loading="lazy" /></button>
			{/each}
		</div>
	{/if}
{/if}

<style>
	.app-bar {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 8px 18px 12px;
	}
	.app-bar h2 {
		font-size: 22px;
	}
	.grow {
		flex: 1;
	}
	.plain {
		display: grid;
		place-items: center;
		padding: 0;
		border: none;
		background: none;
		color: inherit;
		cursor: pointer;
	}
	.plain :global(svg) {
		width: 22px;
		height: 22px;
	}
	.link {
		padding: 0;
		border: none;
		background: none;
		color: var(--accent);
		font: inherit;
		font-size: 14px;
		cursor: pointer;
	}
	.none {
		padding: 40px 24px;
		text-align: center;
		color: #8a8a8a;
		font-size: 13px;
		line-height: 1.6;
	}
	.grid {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 2px;
		overflow: auto;
	}
	.ph {
		aspect-ratio: 1;
		padding: 0;
		border: none;
		background: #eee;
		cursor: pointer;
	}
	.ph img {
		width: 100%;
		height: 100%;
		object-fit: cover;
		display: block;
	}
	.viewer {
		display: flex;
		flex-direction: column;
		height: 100%;
		background: #000;
		color: #fff;
	}
	.vbar {
		display: flex;
		align-items: center;
		padding: 8px 14px;
	}
	.viewer img {
		flex: 1;
		min-height: 0;
		width: 100%;
		object-fit: contain;
	}
	.cap {
		max-height: 30%;
		overflow: auto;
		padding: 10px 16px 16px;
		font-size: 12px;
		color: #ccc;
	}
</style>
