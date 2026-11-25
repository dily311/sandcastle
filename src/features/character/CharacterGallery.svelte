<script lang="ts">
	import { _ } from "svelte-i18n";
	// 캐릭터 상세 > 갤러리 탭: 캐릭터 에셋 관리 + 이 캐릭터 대화에서 생성된 이미지 관리
	import { onMount } from "svelte";
	import { FolderArchive, ImagePlus, MessageSquare, Pencil, Trash2 } from "@lucide/svelte";
	import type { GeneratedImage_Serialize as GeneratedImage } from "$ipc/bindings";
	import { Badge, Button, EmptyState, IconButton, ListToolbar, Segmented, Spinner, Switch } from "$shared/ui";
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
	let assetFilter = $state("all");
	let assetSort = $state("order");
	let genSort = $state("new");
	let genImages = $state<GeneratedImage[]>([]);
	let genLoading = $state(false);
	let genLoadedFor = "";

	const assets = $derived(library.images[charId] ?? []);
	const convIds = $derived(new Set(library.conversations.filter((c) => c.characterId === charId).map((c) => c.id)));
	const folders = $derived([...new Set(assets.map((a) => a.folder).filter(Boolean))].sort((a, b) => a.localeCompare(b, "ko")));
	const ASSET_FILTERS = $derived([
		{ value: "all", label: $_("charGallery.filterAll", { values: { count: assets.length } }) },
		{ value: "on", label: $_("charGallery.filterOn") },
		{ value: "off", label: $_("charGallery.filterOff") },
		...(folders.length ? [{ value: "folder:", label: $_("charGallery.filterCommon") }, ...folders.map((f) => ({ value: `folder:${f}`, label: f }))] : []),
	]);
	const ASSET_SORTS = $derived([
		{ value: "order", label: $_("charGallery.sortOrder") },
		{ value: "name", label: $_("charGallery.sortKey") },
		{ value: "new", label: $_("charGallery.sortRecent") },
	]);
	const GEN_SORTS = $derived([
		{ value: "new", label: $_("charGallery.genSortNew") },
		{ value: "old", label: $_("charGallery.genSortOld") },
		{ value: "name", label: $_("charGallery.genSortName") },
	]);

	const filteredAssets = $derived.by(() => {
		const q = query.trim().toLowerCase();
		const list = assets.filter((a) => {
			if (assetFilter === "on" && !a.enabled) return false;
			if (assetFilter === "off" && a.enabled) return false;
			if (assetFilter.startsWith("folder:") && (a.folder ?? "") !== assetFilter.slice(7)) return false;
			return !q || a.key.toLowerCase().includes(q) || (a.folder ?? "").toLowerCase().includes(q);
		});
		if (assetSort === "name") return [...list].sort((a, b) => a.key.localeCompare(b.key, "ko"));
		if (assetSort === "new") return [...list].sort((a, b) => b.createdAt - a.createdAt);
		return list;
	});

	const charGenImages = $derived.by(() => {
		const q = query.trim().toLowerCase();
		const list = genImages
			.filter((i) => !i.readonly && ((i.conversationId && convIds.has(i.conversationId)) || i.album === `character:${charId}`))
			.filter((i) => !q || (i.name ?? "").toLowerCase().includes(q) || i.prompt.toLowerCase().includes(q));
		if (genSort === "old") return [...list].sort((a, b) => a.createdAt - b.createdAt);
		if (genSort === "name") return [...list].sort((a, b) => (a.name || a.prompt || "").localeCompare(b.name || b.prompt || "", "ko"));
		return [...list].sort((a, b) => b.createdAt - a.createdAt);
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
		toasts.success($_("charGallery.uploadedToast", { values: { count: paths.length } }));
	}
	async function uploadZip() {
		const [path] = await pickFiles([{ name: $_("charGallery.zipFileLabel"), extensions: ["zip"] }], false);
		if (!path) return;
		const n = await library.addImagesZip(charId, path);
		toasts.success($_("charGallery.zipImportedToast", { values: { count: n } }));
	}
	async function renameAsset(img: CharacterImage) {
		const next = await promptText({ title: $_("charGallery.renameTitle"), message: $_("charGallery.renameMsg"), input: { value: img.key, placeholder: $_("charGallery.renamePlaceholder") }, confirmLabel: $_("common.save") });
		if (next?.trim() && next.trim() !== img.key) {
			await library.updateImage(img, { key: next.trim() });
		}
	}
	async function removeAsset(img: CharacterImage) {
		if (!(await confirmAction({ title: $_("charGallery.deleteAssetTitle"), message: $_("charGallery.deleteAssetMsg", { values: { key: img.key } }), confirmLabel: $_("common.delete"), danger: true }))) return;
		await library.removeImage(img);
	}
	async function removeGenerated(img: GeneratedImage) {
		if (!(await confirmAction({ title: $_("charGallery.deleteGenTitle"), message: $_("charGallery.deleteGenMsg"), confirmLabel: $_("common.delete"), danger: true }))) return;
		await call(commands.deleteGeneratedImage(img.path), { source: "character" });
		genImages = genImages.filter((x) => x.path !== img.path);
		toasts.success($_("charGallery.deletedImgToast"));
	}
</script>

<div class="gallery-wrap">
	<div class="bar">
		<Segmented
			label={$_("charGallery.kindAria")}
			bind:value={sub}
			options={[
				{ value: "assets", label: $_("charGallery.tabAssets", { values: { count: assets.length } }) },
				{ value: "generated", label: $_("charGallery.tabGenerated", { values: { count: charGenImages.length } }) },
			]}
		/>
		<div class="right">
			{#if sub === "assets"}
				<ListToolbar bind:query bind:filter={assetFilter} bind:sort={assetSort} placeholder={$_("charGallery.searchAssetPlaceholder")} filters={ASSET_FILTERS} sorts={ASSET_SORTS}>
					<Button size="sm" variant="secondary" onclick={uploadZip}>{#snippet icon()}<FolderArchive />{/snippet}ZIP</Button>
					<Button size="sm" variant="primary" onclick={uploadAssets}>{#snippet icon()}<ImagePlus />{/snippet}{$_("charGallery.addAssetBtn")}</Button>
				</ListToolbar>
			{:else}
				<ListToolbar bind:query bind:sort={genSort} placeholder={$_("charGallery.searchGenPlaceholder")} sorts={GEN_SORTS} />
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
								<Switch checked={img.enabled} label={$_("charGallery.enableAria", { values: { key: img.key } })} hideLabel onchange={(v) => library.updateImage(img, { enabled: v })} />
							</div>
							<div class="bot-row">
								{#if img.folder}<Badge>{img.folder}</Badge>{:else}<span></span>{/if}
								<div class="acts">
									<IconButton size="sm" label={$_("charGallery.renameKeyAria")} onclick={() => renameAsset(img)}><Pencil /></IconButton>
									<IconButton size="sm" variant="danger" label={$_("common.delete")} onclick={() => removeAsset(img)}><Trash2 /></IconButton>
								</div>
							</div>
						</div>
					</div>
				{/each}
			</div>
		{:else}
			<EmptyState title={assets.length ? $_("charGallery.noResults") : $_("charGallery.noAssetsTitle")} description={$_("charGallery.noAssetsDesc")} />
		{/if}
	{:else if genLoading && !genImages.length}
		<div class="center"><Spinner /></div>
	{:else if charGenImages.length}
		<div class="grid">
			{#each charGenImages as img (img.path)}
				<div class="card">
					<div class="thumb">
						<img src={fileSrc(img.path)} alt={img.name || img.prompt || $_("charGallery.genAlt")} loading="lazy" />
					</div>
					<div class="meta">
						<b title={img.name || img.prompt}>{img.name || img.prompt || $_("charGallery.genAlt")}</b>
						<div class="bot-row">
							<small>{relative(img.createdAt)}</small>
							<div class="acts">
								{#if img.conversationId}
									<IconButton size="sm" label={$_("charGallery.goConvAria")} onclick={() => nav.go({ page: "character", charId, convId: img.conversationId ?? undefined })}><MessageSquare /></IconButton>
								{/if}
								<IconButton size="sm" variant="danger" label={$_("common.delete")} onclick={() => removeGenerated(img)}><Trash2 /></IconButton>
							</div>
						</div>
					</div>
				</div>
			{/each}
		</div>
	{:else}
		<EmptyState title={query.trim() ? $_("charGallery.noResults") : $_("charGallery.noGenTitle")} description={$_("charGallery.noGenDesc")} />
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
