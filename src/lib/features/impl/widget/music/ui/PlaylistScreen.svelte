<script lang="ts">
	import { _ } from "svelte-i18n";
	import { Check, ChevronLeft, EllipsisVertical, GripVertical, ListMusic, Pencil, Plus, Trash2, X } from "@lucide/svelte";
	import { handleSelectMusicFolder, changePlayMusic, playlistStore } from "../musicPlyerStore.svelte";
	import PlaylistFolderTree from "./PlaylistFolderTree.svelte";
	import PlaylistItemScreen from "./PlaylistItemScreen.svelte";
	import { sortableList } from "$/lib/action/useSortable.svelte";

	type Mode = "songs" | "playlists" | "add";
	let mode = $state<Mode>("songs");
	let showActionMenu = $state(false);
	let grab = $state(false);

	let creating = $state(false);
	let newTitle = $state("");
	let editingId = $state<string | null>(null);
	let editingTitle = $state("");

	const currentTitle = $derived(playlistStore.currentPlaylist?.title || $_("widgetExtra.defaultPlaylist"));
	const canDelete = (id: string) => id !== "default" && playlistStore.list.length > 1;

	function startCreate() {
		creating = true;
		newTitle = "";
	}
	async function submitCreate() {
		const t = newTitle.trim();
		if (!t) {
			creating = false;
			return;
		}
		await playlistStore.createPlaylist(t);
		creating = false;
		newTitle = "";
		mode = "songs";
	}
	function startRename(id: string, title: string) {
		editingId = id;
		editingTitle = title;
		showActionMenu = false;
	}
	async function submitRename() {
		if (!editingId) return;
		const id = editingId;
		const t = editingTitle.trim();
		editingId = null;
		if (t) await playlistStore.renamePlaylist(id, t);
	}
	async function removePlaylist(id: string) {
		showActionMenu = false;
		await playlistStore.deletePlaylist(id);
	}
	async function pickPlaylist(id: string) {
		if (editingId === id) return;
		await playlistStore.selectPlaylist(id);
		mode = "songs";
	}
</script>

<div class="playlist-wrap">
	<div class="playlist-header-wrap">
		<div class="playlist-header">
			{#if mode === "songs"}
				<button
					type="button"
					aria-label={$_("widgetExtra.playlistList")}
					title={$_("widgetExtra.playlistList")}
					class="playlist header-btn"
					onclick={() => {
						showActionMenu = false;
						mode = "playlists";
					}}
				>
					<ListMusic />
				</button>
				<div class="playlist-title">
					{#if editingId === playlistStore.currentPlaylistKey}
						<input
							class="inline-input"
							bind:value={editingTitle}
							aria-label={$_("widgetExtra.playlistNameAria")}
							onkeydown={(e) => {
								if (e.key === "Enter") void submitRename();
								if (e.key === "Escape") editingId = null;
							}}
							onblur={() => void submitRename()}
						/>
					{:else}
						<button
							type="button"
							class="title-btn"
							title={$_("widgetExtra.switchPlaylistTitle")}
							onclick={() => {
								showActionMenu = false;
								mode = "playlists";
							}}
						>
							<h2>{currentTitle}</h2>
						</button>
					{/if}
				</div>
			{:else}
				<button
					type="button"
					aria-label={$_("music.playlist:goCurrentPlaylistPage")}
					class="playlist header-btn"
					onclick={() => {
						creating = false;
						editingId = null;
						mode = "songs";
					}}
				>
					<ChevronLeft />
				</button>
				<div class="playlist-title">
					<h2>{mode === "playlists" ? $_("widgetExtra.playlistsTitle") : $_("widgetExtra.addTrackTitle")}</h2>
				</div>
			{/if}
		</div>
		{#if mode === "songs"}
			<div class="playlist-action">
				<button type="button" class="header-btn" aria-label={$_("widgetExtra.playlistMenuAria")} onclick={() => (showActionMenu = !showActionMenu)}>
					<EllipsisVertical />
				</button>
				<div class:showActionMenu class="playlist-action-menu-wrap">
					<ul class="playlist-action-menu">
						<li class="menu-item">
							<button
								type="button"
								onclick={() => {
									showActionMenu = false;
									mode = "playlists";
								}}>{$_("widgetExtra.viewList")}</button
							>
						</li>
						<li class="menu-item">
							<button
								type="button"
								onclick={() => startRename(playlistStore.currentPlaylistKey, currentTitle)}>{$_("widgetExtra.rename")}</button
							>
						</li>
						{#if canDelete(playlistStore.currentPlaylistKey)}
							<li class="menu-item">
								<button
									type="button"
									class="danger"
									onclick={() => void removePlaylist(playlistStore.currentPlaylistKey)}>{$_("common.delete")}</button
								>
							</li>
						{/if}
					</ul>
				</div>
			</div>
		{/if}
	</div>

	<div class="playlist-content">
		{#if mode === "songs"}
			<div
				class="playlist-item-list"
				{@attach sortableList({
					items: playlistStore.current,
					onReorder: (items) => {
						playlistStore.updateMusicOrder(items);
					},
				})}
			>
				{#each playlistStore.current as item, index}
					<div
						data-index={index}
						class="playlist-item-wrap"
						class:playing-now={playlistStore?.currentSong?.path === item.meta.path}
					>
						<button
							type="button"
							aria-label={$_("music.playlist:change")}
							class="playlist-order-change-btn order-change-btn"
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
								onclick={() => {
									playlistStore.deleted(item.meta.path);
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
				onclick={() => {
					showActionMenu = false;
					mode = "add";
				}}
			>
				<Plus />
				<span>{$_("widgetExtra.addTrackTitle")}</span>
			</button>
		{:else if mode === "playlists"}
			<div class="playlists-list">
				{#each playlistStore.list as pl (pl.id)}
					{@const active = pl.id === playlistStore.currentPlaylistKey}
					<div class="playlist-row" class:active>
						{#if editingId === pl.id}
							<input
								class="inline-input"
								bind:value={editingTitle}
								aria-label={$_("widgetExtra.playlistNameAria")}
								onkeydown={(e) => {
									if (e.key === "Enter") void submitRename();
									if (e.key === "Escape") editingId = null;
								}}
								onblur={() => void submitRename()}
							/>
						{:else}
							<button type="button" class="playlist-row-main" onclick={() => void pickPlaylist(pl.id)}>
								<span class="playlist-row-check">
									{#if active}<Check />{/if}
								</span>
								<span class="playlist-row-title">{pl.title || $_("widgetExtra.defaultPlaylist")}</span>
							</button>
						{/if}
						<div class="playlist-action-btns">
							<button
								type="button"
								aria-label={$_("widgetExtra.rename")}
								title={$_("widgetExtra.rename")}
								onclick={() => startRename(pl.id, pl.title || "")}
							>
								<Pencil />
							</button>
							{#if canDelete(pl.id)}
								<button
									type="button"
									aria-label={$_("widgetExtra.deletePlaylistAria")}
									title={$_("widgetExtra.deletePlaylistAria")}
									class="danger"
									onclick={() => void removePlaylist(pl.id)}
								>
									<Trash2 />
								</button>
							{/if}
						</div>
					</div>
				{/each}
			</div>
			{#if creating}
				<form
					class="new-playlist-form"
					onsubmit={(e) => {
						e.preventDefault();
						void submitCreate();
					}}
				>
					<input
						class="inline-input"
						bind:value={newTitle}
						placeholder={$_("widgetExtra.newPlaylistPlaceholder")}
						aria-label={$_("widgetExtra.newPlaylistPlaceholder")}
						onkeydown={(e) => {
							if (e.key === "Escape") creating = false;
						}}
					/>
					<button type="submit" class="confirm-btn" aria-label={$_("common.create")}><Check /></button>
					<button type="button" class="confirm-btn" aria-label={$_("common.cancel")} onclick={() => (creating = false)}><X /></button>
				</form>
			{:else}
				<button type="button" class="playlistAddBtn" onclick={startCreate}>
					<Plus />
					<span>{$_("widgetExtra.newPlaylistBtn")}</span>
				</button>
			{/if}
		{:else}
			<div class="folder-bar">
				<button type="button" class="selected-folder-btn" onclick={handleSelectMusicFolder}>{$_("widgetExtra.selectFolder")}</button>
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
		border-top: 1px solid var(--border);
		overflow: hidden;
		min-height: 27em;
	}
	.playlist-header-wrap {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0.6rem;
		border-bottom: 1px solid var(--border);
		--icon-size: 1.2rem;
	}
	.playlist-header {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		min-width: 0;
		flex: 1 1 auto;
	}
	.header-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		cursor: pointer;
		color: var(--text);
	}
	.playlist-title {
		min-width: 0;
		flex: 1 1 auto;
	}
	.title-btn {
		width: 100%;
		text-align: left;
		cursor: pointer;
		color: var(--text);
	}
	.playlist-title h2 {
		font-weight: 500;
		font-size: 1rem;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.playlist-content {
		flex: 1 0 0;
		overflow: hidden auto;
		position: relative;
	}

	.playlist-content::-webkit-scrollbar {
		-webkit-appearance: none;
		width: 4px;
		background: var(--bg-sunken);
	}
	.playlist-content::-webkit-scrollbar-thumb {
		width: 4px;
		background: var(--text);
	}

	.playlistAddBtn {
		width: 100%;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.3rem;
		padding: 0.8rem 1.4rem;
		font-size: 0.8rem;
		cursor: pointer;
		color: var(--text-muted);
		--icon-size: 0.8rem;
	}

	.folder-bar {
		padding: 0.4rem 0.6rem;
	}
	.selected-folder-btn {
		padding: 0.4rem 1.4rem;
		font-size: 0.8rem;
		border-radius: 0.8rem;
		cursor: pointer;
	}

	.playlistAddBtn:hover {
		background: var(--bg-sunken);
	}
	:global(.music-player-container.resize) {
		.playlist-wrap {
			max-width: 22rem;
			width: 22rem;
			min-height: none;
			border-left: 1px solid var(--border);
			min-height: 0;
		}
	}

	.playlist-item-wrap {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.4rem;
		padding: 0.6rem;
		border-bottom: 1px solid var(--border);
		overflow: hidden;
	}
	.playlist-item-wrap.playing-now {
		background: var(--bg-sunken);
	}

	.playlist-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.4rem;
		padding: 0.5rem 0.6rem;
		border-bottom: 1px solid var(--border);
	}
	.playlist-row.active {
		background: var(--bg-sunken);
	}
	.playlist-row-main {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		flex: 1 1 auto;
		min-width: 0;
		text-align: left;
		cursor: pointer;
		color: var(--text);
		font-size: 0.9rem;
	}
	.playlist-row-check {
		width: 1rem;
		height: 1rem;
		flex-shrink: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--accent);
		--icon-size: 0.9rem;
	}
	.playlist-row-title {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.inline-input {
		flex: 1 1 auto;
		min-width: 0;
		padding: 0.25rem 0.4rem;
		border: 1px solid var(--border);
		border-radius: 0.3rem;
		background: var(--surface);
		color: var(--text);
		font-size: 0.85rem;
	}
	.new-playlist-form {
		display: flex;
		align-items: center;
		gap: 0.3rem;
		padding: 0.5rem 0.6rem;
		border-bottom: 1px solid var(--border);
	}
	.confirm-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 1.6rem;
		height: 1.6rem;
		border-radius: 0.3rem;
		color: var(--text-muted);
		cursor: pointer;
		--icon-size: 0.9rem;
	}
	.confirm-btn:hover {
		background: var(--surface-hover);
		color: var(--text);
	}

	.playlist-action-btns {
		display: flex;
		align-items: center;
		gap: 0.15rem;
		flex-shrink: 0;
	}
	.playlist-action-btns button {
		--icon-size: 0.95rem;
		width: 1.8rem;
		height: 1.8rem;
		border-radius: 100%;
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--text-muted);
		cursor: pointer;
	}
	.playlist-action-btns button:hover {
		background: var(--surface-hover);
		color: var(--text);
	}
	.playlist-action-btns button.danger:hover,
	.playlist-action-menu li button.danger {
		color: var(--danger, #e5484d);
	}

	.playlist-order-change-btn {
		cursor: grab;
		color: var(--text-muted);
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
		transition-behavior: allow-discrete;
		white-space: nowrap;
		padding: 0.4rem;
		border-radius: 0.4rem;
		border: 1px solid var(--border);
		z-index: 90;
		background: var(--surface);
	}
	.playlist-action-menu-wrap.showActionMenu {
		display: flex;
	}
	.playlist-action-menu li button {
		width: 100%;
		font-size: 0.8rem;
		padding: 0.25rem 0.6rem;
		border-bottom: 1px solid var(--border);
		cursor: pointer;
		text-align: left;
	}
	.playlist-action-menu li:last-child button {
		border-bottom: none;
	}
	.playlist-action-menu li button:hover {
		background-color: var(--surface-hover);
	}
</style>
