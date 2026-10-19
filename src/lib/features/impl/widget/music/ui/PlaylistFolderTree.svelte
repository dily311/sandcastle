<script lang="ts">
	import { _ } from "svelte-i18n";
	import type { FolderTree, MusicMeta, SortType } from "../types";
	import PlaylistFolderNode from "./PlaylistFolderNode.svelte";
	import PlaylistItemScreen from "./PlaylistItemScreen.svelte";
	import { getSortedFiles, playlistStore, togglePlaylist } from "../musicPlyerStore.svelte";
	import { Check } from "@lucide/svelte";

	// Svelte 5 룬으로 props 정의
	let {
		tree,
		rootFiles = [],
		sortType = "name",
		level = 0,
	}: {
		tree: FolderTree;
		rootFiles?: MusicMeta[];
		sortType?: SortType;
		level?: number;
	} = $props();

	// 1. 폴더들은 이름순으로 정렬
	let sortedFolders = $derived(Array.from(tree.values()).sort((a, b) => a.name.localeCompare(b.name)));
</script>

<ul class="folder-tree" style="padding-left: {level > 0 ? '1rem' : '0'};">
	{#each sortedFolders as folder (folder.name)}
		<PlaylistFolderNode name={folder.name} content={folder} level={level + 1} />
	{/each}
	{#each getSortedFiles(rootFiles) as item}
		<li class="fileNode">
			<PlaylistItemScreen {item} handleClick={()=> togglePlaylist(item)} />
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

<style>
	ul {
		list-style: none;
		margin: 0;
		padding: 0;
		overflow: hidden;
	}
	.fileNode {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		padding: 0.6rem 0.8rem;
		border-bottom: 1px solid var(--card-border-color);
		--icon-size: 1rem;
	}
	.fileNode:last-child {
		border-bottom: none;
	}
</style>
