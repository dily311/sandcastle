<script lang="ts">
	import { _ } from "svelte-i18n";
	import { Check, Folder, FolderOpen } from "@lucide/svelte";
	import type { FolderNode, MusicMeta } from "../types";
	import PlaylistFolderTree from "./PlaylistFolderTree.svelte";
	import PlaylistItemScreen from "./PlaylistItemScreen.svelte";
	import { getSortedFiles, playlistStore, togglePlaylist } from "../musicPlyerStore.svelte";

	let { name, content, level }: { name: string; content: FolderNode; level: number } = $props();

	let isOpen = $state(false);
</script>

<li class="folderNode">
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div class="folderTitle" class:isOpen onclick={() => (isOpen = !isOpen)}>
		{#if isOpen}
			<FolderOpen />
		{:else}
			<Folder />
		{/if}
		<span class="label">{name}</span>
	</div>

	{#if isOpen}
		<ul class="folderContent">
			<!-- 하위 폴더 구조를 위해 부모 재귀 호출 -->
			<PlaylistFolderTree tree={content.folders} level={level + 1} />
			{#each getSortedFiles(content.files) as item}
				<li class="fileNode" style="padding-left: {(level + 1) * 1}rem;">
					<PlaylistItemScreen {item} handleClick={() => togglePlaylist(item)} />
					<div class="playlist-action-btns">
						{#if playlistStore.isExists(item.path)}
							<button type="button" aria-label={$_("music.playlist:include")}>
								<Check />
							</button>
						{/if}
					</div>
				</li>
			{/each}
		</ul>
	{/if}
</li>

<style>
	.folderNode {
		flex: 1;
		overflow: hidden;
	}

	.folderTitle {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		height: 2rem;
		padding: 0.6rem;
		/* border-bottom: 1px solid var(--border); */
		position: sticky;
		top: 0;
		z-index: 99;
		background: var(--surface);
		cursor: pointer;
		--icon-size: 1rem;
	}
	.folderTitle .label {
		font-size: 0.8rem;
		color: var(--text);
	}
	.folderContent {
		display: flex;
		flex-direction: column;
	}

	.fileNode {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		padding: 0.6rem;
		border-bottom: 1px solid var(--border);
	}
	.fileNode:last-child {
		border-bottom: none;
	}
</style>
