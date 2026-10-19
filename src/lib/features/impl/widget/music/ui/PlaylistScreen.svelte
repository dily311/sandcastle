<script lang="ts">
	import { _ } from "svelte-i18n";
	import { Check, ChevronLeft, EllipsisVertical, GripVertical, Plus, X } from "@lucide/svelte";
	import { handleSelectMusicFolder, changePlayMusic, playlistStore } from "../musicPlyerStore.svelte";
	import PlaylistFolderTree from "./PlaylistFolderTree.svelte";
	import PlaylistItemScreen from "./PlaylistItemScreen.svelte";

	let currentPageMode = $state(true);
	let showActionMenu = $state(false);
	let grab = $state(false);
</script>

<div class="playlist-wrap">
	<div class="playlist-header-wrap">
		<div class="playlist-header">
			<button
				type="button"
				aria-label={currentPageMode
					? $_("music.playlist:goChosesMusicPage")
					: $_("music.playlist:goCurrentPlaylistPage")}
				class="playlist"
				onclick={() => (currentPageMode = !currentPageMode)}
			>
				<ChevronLeft />
			</button>
			<div class="playlist-title">
				<h2>
					{currentPageMode ? "Playlist title" : "Music list"}
				</h2>
			</div>
		</div>
		<div class="playlist-action">
			<button type="button" onclick={() => (showActionMenu = !showActionMenu)}>
				<EllipsisVertical />
			</button>
			<div class:showActionMenu class="playlist-action-menu-wrap">
				<ul class="playlist-action-menu">
					<li class="menu-item"><button>정렬</button></li>
				</ul>
			</div>
		</div>
	</div>
	<div class="playlist-content">
		{#if currentPageMode}
			<div class="playlist-item-list">
				{#each playlistStore.current as item, index}
					<div
						class="playlist-item-wrap"
						class:playing-now={playlistStore?.currentSong?.path === item.meta.path}
					>
						<button
							type="button"
							aria-label={$_("music.playlist:change")}
							class="playlist-order-change-btn"
							class:grab
							draggable="true"
							onclick={() => playlistStore.changeOrder(item, index)}
						>
							<GripVertical />
						</button>
						<PlaylistItemScreen
							item={item.meta}
							{index}
							handleClick={() => changePlayMusic(index, item.meta)}
						/>
						<div class="playlist-action-btns">
							<button
								type="button"
								aria-label={$_("music.playlist:remove")}
								class="playlist-remove-btn"
								onclick={(e) => {
									playlistStore.deleted(item.meta.path, index);
								}}
							>
								<X />
							</button>
						</div>
					</div>
				{/each}
			</div>
			<button
				type="button"
				aria-label={$_("music.playlist:goChosesMusicPage")}
				class="playlistAddBtn"
				onclick={() => (currentPageMode = !currentPageMode)}
			>
				<Plus />
				<span>곡 추가</span>
			</button>
		{:else}
			<div>
				<button class="selected-folder-btn" onclick={handleSelectMusicFolder}>폴더 선택</button>
			</div>
			<div class="playlist-item-list">
				<PlaylistFolderTree
					tree={playlistStore.selectedFolderMusics?.folders}
					rootFiles={playlistStore.selectedFolderMusics?.rootFiles}
				/>
			</div>
		{/if}
	</div>
</div>

<style>
	.playlist-wrap {
		flex: 0 1 auto;
		width: 100%;
		display: flex;
		flex-direction: column;
		max-width: none;
		border-top: 1px solid var(--card-border-color);
		overflow: hidden;
		min-height: 27em;
	}
	.playlist-header-wrap {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0.6rem;
		border-bottom: 1px solid var(--card-border-color);
		--icon-size: 1.2rem;
	}
	.playlist-header {
		display: flex;
		align-items: center;
		gap: 0.4rem;
	}
	.playlist-title h2 {
		font-weight: 500;
		font-size: 1rem;
	}

	.playlist-content {
		flex: 1 0 0;
		overflow: hidden auto;
		position: relative;
	}

	.playlist-content::-webkit-scrollbar {
		-webkit-appearance: none;
		width: 4px;
		background: var(--color-10);
	}
	.playlist-content::-webkit-scrollbar-thumb {
		width: 4px;
		background: var(--color-100);
	}

	.playlistAddBtn {
		width: 100%;
		padding: 0.8rem 1.4rem;
		font-size: 0.8rem;
		cursor: pointer;
		color: var(--icon-color);
		--icon-size: 0.8rem;
	}

	.selected-folder-btn {
		padding: 0.4rem 1.4rem;
		font-size: 0.8rem;
		border-radius: 0.8rem;
		cursor: pointer;
	}

	.playlistAddBtn:hover {
		background: var(--color-10);
	}
	:global(.music-player-container.resize) {
		.playlist-wrap {
			max-width: 22rem;
			width: 22rem;
			min-height: none;
			border-left: 1px solid var(--card-border-color);
			min-height: 0;
		}
	}

	.playlist-item-wrap {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.4rem;
		padding: 0.6rem;
		border-bottom: 1px solid var(--card-border-color);
		overflow: hidden;
	}
	.playlist-item-wrap.playing-now {
		background: var(--color-10);
	}

	.playlist-item-index {
		flex: 0 0 1.2rem;
		/* width: 1.2rem; */
		text-align: center;
		color: var(--text-muted);
		font-size: 0.8rem;
	}

	.playlist-action-btns button {
		--icon-size: 1rem;
		width: 1.8rem;
		height: 1.8rem;
		border-radius: 100%;
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--icon-color);
		cursor: pointer;
	}
	.playlist-action-btns button:hover {
		background: var(--icon-hover-bg);
	}

	.playlist-order-change-btn {
		cursor: grab;
		color: var(--icon-color);
	}
	.playlist-order-change-btn.grab {
		cursor: grabbing;
	}

	.playlist-action {
		position: relative;
	}
	.playlist-action-menu-wrap {
		position: absolute;
		right: 0;
		transform: translateY(0.8rem);
		display: none;
		white-space: nowrap;
		padding: 0.4rem;
		border-radius: 0.4rem;
		border: 1px solid var(--card-border-color);
		/* overflow: hidden; */
		z-index: 90;
		background: var(--background-primary);
	}
	.playlist-action-menu-wrap.showActionMenu {
		display: flex;
	}
	.playlist-action-menu li button {
		width: 100%;
		font-size: 0.8rem;
		padding: 0.2rem 0.6rem;
		border-bottom: 1px solid var(--border-color);
		cursor: pointer;
	}
	.playlist-action-menu li:last-child button {
		border-bottom: none;
	}
	.playlist-action-menu li button:hover {
		background-color: var(--button-hover-bg);
	}
</style>
