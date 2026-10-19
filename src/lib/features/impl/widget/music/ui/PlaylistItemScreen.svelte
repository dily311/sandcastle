<script lang="ts">
	import { formatAlbumArt } from "../musicPlyerStore.svelte";
	import type { MusicMeta } from "../types";
	import DefaultAlbumArt from "$/asset/widget/album_art_default.png";

	let { item, handleClick }: { item: MusicMeta; index?: number; handleClick?: () => void } = $props();
</script>

<button class="playlist-item" onclick={(e) => handleClick?.()}>
	<div class="albumArt" style="--pos-x: {item?.imgPos?.x}%; --pos-y: {item?.imgPos?.y}%;">
		<img
			src={item?.albumArt ? formatAlbumArt(item.albumArt, 128, item?.albumArtVersion) : DefaultAlbumArt}
			alt=""
			loading="lazy"
		/>
	</div>
	<div class="musicInfo">
		<div class="title">
			{item?.title ?? "Song Title"}
		</div>
		<div class="artist">
			{item?.artist ?? "Artist"}
		</div>
	</div>
</button>

<style>
	/* 노래정보 */
	.playlist-item {
		flex: 0 1 auto;
		width: 100%;
		display: flex;
		align-items: center;
		gap: 0.6rem;
		overflow: hidden;
		text-align: unset;
		cursor: pointer;
	}

	.albumArt {
		flex-shrink: 0;
		width: 2rem;
		height: 2rem;
		aspect-ratio: 1;
		overflow: hidden;
		border-radius: 0.4rem;
	}
	.albumArt img {
		width: 100%;
		height: 100%;
		object-fit: cover;
		object-position: var(--pos-x, 50%) var(--pos-y, 50%);
		-webkit-user-select: none; /* Safari */
		-ms-user-select: none; /* IE 10+ and Edge */
		user-select: none; /* Standard syntax */
	}

	.musicInfo {
		flex: 1 1 auto;
		overflow: hidden;
	}
	.title {
		font-size: 0.8rem;
		margin-bottom: 0.125rem;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.artist {
		font-size: 0.725rem;
		color: var(--text-muted);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
</style>
