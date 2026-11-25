<script lang="ts">
	// 캐릭터 상세 > 갤러리 탭: 캐릭터 에셋 관리 + 이 캐릭터 대화에서 생성된 이미지 관리
	import { onMount } from "svelte";
	import { FolderArchive, ImagePlus, MessageSquare, Pencil, Trash2 } from "@lucide/svelte";
	import type { GeneratedImage_Serialize as GeneratedImage } from "$ipc/bindings";
	import { Badge, Button, EmptyState, IconButton, Input, Segmented, Spinner, Switch } from "$shared/ui";
	import { call, commands } from "$shared/ipc/client";
	import { IMAGE_FILES, pickFiles } from "$shared/ipc/dialog";
	import { fileSrc, imageSrc } from "$shared/ipc/file";
	import { confirmAction, promptText } from "$shared/stores/confirm.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	import { relative } from "$shared/time";
	import { nav } from "$app/nav.svelte";
	import { library, type CharacterImage } from "./library.svelte";

	let { charId }: { charId: string } = $props();

	let sub = $state<"assets" | "generated">("assets");
	let query = $state("");
	let genImages = $state<GeneratedImage[]>([]);
	let genLoading = $state(false);
	let genLoadedFor = "";

	const assets = $derived(library.images[charId] ?? []);
	const convIds = $derived(new Set(library.conversations.filter((c) => c.characterId === charId).map((c) => c.id)));

	const filteredAssets = $derived.by(() => {
		const q = query.trim().toLowerCase();
		if (!q) return assets;
		return assets.filter((a) => a.key.toLowerCase().includes(q) || (a.folder ?? "").toLowerCase().includes(q));
	});

	const charGenImages = $derived.by(() => {
		const q = query.trim().toLowerCase();
		return genImages
			.filter((i) => !i.readonly && ((i.conversationId && convIds.has(i.conversationId)) || i.album === `character:${charId}`))
			.filter((i) => !q || (i.name ?? "").toLowerCase().includes(q) || i.prompt.toLowerCase().includes(q))
			.sort((a, b) => b.createdAt - a.createdAt);
	});

	async function loadGenerated() {
		genLoading = true;
		try {
			genImages = await call(commands.listAllImages(), { source: "character", silent: true }).catch(() => []);
			genLoadedFor = charId;
		} finally {
			genLoading = false;
		}
	}

	onMount(() => {
		if (!library.images[charId]) void library.loadImages(charId);
		void loadGenerated();
	});

	$effect(() => {
		if (charId && genLoadedFor !== charId) {
			if (!library.images[charId]) void library.loadImages(charId);
			void loadGenerated();
		}
	});

	async function uploadAssets() {
		const paths = await pickFiles([IMAGE_FILES], true);
		if (!paths.length) return;
		await library.addImages(charId, paths);
		toasts.success(`에셋 ${paths.length}장을 올림`);
	}
	async function uploadZip() {
		const [path] = await pickFiles([{ name: "ZIP 압축 파일", extensions: ["zip"] }], false);
		if (!path) return;
		const n = await library.addImagesZip(charId, path);
		toasts.success(`ZIP에서 에셋 ${n}장을 가져옴`);
	}
	async function renameAsset(img: CharacterImage) {
		const next = await promptText({ title: "에셋 키 바꾸기", message: "대화에서 <cg>키</cg>로 쓰이는 이름", input: { value: img.key, placeholder: "예: smile" }, confirmLabel: "저장" });
		if (next?.trim() && next.trim() !== img.key) {
			await library.updateImage(img, { key: next.trim() });
		}
	}
	async function removeAsset(img: CharacterImage) {
		if (!(await confirmAction({ title: "에셋 삭제", message: `'${img.key}' 에셋을 지움`, confirmLabel: "삭제", danger: true }))) return;
		await library.removeImage(img);
	}
	async function removeGenerated(img: GeneratedImage) {
		if (!(await confirmAction({ title: "생성된 이미지 삭제", message: "이 이미지를 영구 삭제함", confirmLabel: "삭제", danger: true }))) return;
		await call(commands.deleteGeneratedImage(img.path), { source: "character" });
		genImages = genImages.filter((x) => x.path !== img.path);
		toasts.success("이미지를 지움");
	}
</script>

<div class="gallery-wrap">
	<div class="bar">
		<Segmented
			label="갤러리 종류"
			bind:value={sub}
			options={[
				{ value: "assets", label: `에셋 관리 ${assets.length}` },
				{ value: "generated", label: `생성된 이미지 ${charGenImages.length}` },
			]}
		/>
		<div class="right">
			<div class="search">
				<Input bind:value={query} placeholder={sub === "assets" ? "키·폴더 검색" : "이름·프롬프트 검색"} aria-label="갤러리 검색" type="search" />
			</div>
			{#if sub === "assets"}
				<Button size="sm" variant="secondary" onclick={uploadZip}>{#snippet icon()}<FolderArchive />{/snippet}ZIP</Button>
				<Button size="sm" variant="primary" onclick={uploadAssets}>{#snippet icon()}<ImagePlus />{/snippet}에셋 추가</Button>
			{/if}
		</div>
	</div>

	{#if sub === "assets"}
		{#if filteredAssets.length}
			<div class="grid">
				{#each filteredAssets as img (img.id)}
					<div class="card" class:off={!img.enabled}>
						<div class="thumb">
							{#if imageSrc(img)}
								<img src={imageSrc(img)} alt={img.key} loading="lazy" />
							{/if}
						</div>
						<div class="meta">
							<div class="top-row">
								<b title={img.key}>{img.key}</b>
								<Switch checked={img.enabled} label="{img.key} 사용" hideLabel onchange={(v) => library.updateImage(img, { enabled: v })} />
							</div>
							<div class="bot-row">
								{#if img.folder}<Badge>{img.folder}</Badge>{:else}<span></span>{/if}
								<div class="acts">
									<IconButton size="sm" label="키 바꾸기" onclick={() => renameAsset(img)}><Pencil /></IconButton>
									<IconButton size="sm" label="삭제" onclick={() => removeAsset(img)}><Trash2 /></IconButton>
								</div>
							</div>
						</div>
					</div>
				{/each}
			</div>
		{:else}
			<EmptyState title={assets.length ? "검색 결과 없음" : "등록된 에셋 없음"} description="에셋 추가 또는 ZIP으로 캐릭터 CG·표정 이미지를 올려 관리할 수 있음" />
		{/if}
	{:else if genLoading && !genImages.length}
		<div class="center"><Spinner /></div>
	{:else if charGenImages.length}
		<div class="grid">
			{#each charGenImages as img (img.path)}
				<div class="card">
					<div class="thumb">
						<img src={fileSrc(img.path)} alt={img.name || img.prompt || "생성된 이미지"} loading="lazy" />
					</div>
					<div class="meta">
						<b title={img.name || img.prompt}>{img.name || img.prompt || "생성된 이미지"}</b>
						<div class="bot-row">
							<small>{relative(img.createdAt)}</small>
							<div class="acts">
								{#if img.conversationId}
									<IconButton size="sm" label="대화로 이동" onclick={() => nav.go({ page: "character", charId, convId: img.conversationId ?? undefined })}><MessageSquare /></IconButton>
								{/if}
								<IconButton size="sm" label="삭제" onclick={() => removeGenerated(img)}><Trash2 /></IconButton>
							</div>
						</div>
					</div>
				</div>
			{/each}
		</div>
	{:else}
		<EmptyState title={query.trim() ? "검색 결과 없음" : "생성된 이미지 없음"} description="이 캐릭터와 나눈 대화에서 생성된 이미지가 여기에 모임" />
	{/if}
</div>

<style>
	.gallery-wrap {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
		margin-top: var(--space-4);
	}
	.bar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-3);
		flex-wrap: wrap;
	}
	.right {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		flex-wrap: wrap;
	}
	.search {
		width: min(220px, 45vw);
	}
	.center {
		display: grid;
		place-items: center;
		padding: var(--space-8);
	}
	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(170px, 1fr));
		gap: var(--space-3);
	}
	.card {
		display: flex;
		flex-direction: column;
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		background: var(--surface);
		overflow: hidden;
	}
	.card.off {
		opacity: 0.55;
	}
	.thumb {
		aspect-ratio: 3 / 4;
		background: var(--bg-sunken);
		overflow: hidden;
	}
	.thumb img {
		width: 100%;
		height: 100%;
		object-fit: cover;
		display: block;
	}
	.meta {
		display: flex;
		flex-direction: column;
		gap: 4px;
		padding: var(--space-2) var(--space-3);
		min-width: 0;
	}
	.top-row,
	.bot-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-1);
		min-width: 0;
	}
	.meta b {
		font-size: var(--text-sm);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.meta small {
		font-size: var(--text-xs);
		color: var(--text-faint);
	}
	.acts {
		display: flex;
		align-items: center;
		gap: 2px;
		flex: none;
	}
</style>
