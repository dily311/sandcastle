<script lang="ts">
	import { _ } from "svelte-i18n";
	import { interactive } from "$/lib/interaction/interactive";
	import { draggable } from "$/lib/action/useDrag.svelte";
	import { GripVertical, Pause, Play, SkipForward } from "@lucide/svelte";
	import {
		audioStore,
		handleNextSong,
		widgetMusicSetting,
		playlistStore,
		formatAlbumArt,
	} from "../musicPlyerStore.svelte";
	import { handleContextMenuForWidget } from "$/lib/contextmenu/contextMenu.svelte";
	import DefaultAlbumArt from "$/asset/widget/album_art_default.png";
	import { WIDGET_MUSIC_FEATURE_ID } from "../musicFeature.svelte";
	import { LoopMode } from "../types";
	import IconButton from "$shared/ui/IconButton.svelte";
	import Icon from "$shared/ui/Icon.svelte";

	let moveEl = $state<HTMLElement>();
	let localPos = $state({
		x: widgetMusicSetting.pos?.x ?? 10,
		y: widgetMusicSetting.pos?.y ?? 10,
	});
	$effect(() => {
        if (widgetMusicSetting.pos) {
            localPos.x = widgetMusicSetting.pos.x;
            localPos.y = widgetMusicSetting.pos.y;
        }
    });
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	class="mini-player"
	bind:this={moveEl}
	oncontextmenu={(e) => handleContextMenuForWidget(e, WIDGET_MUSIC_FEATURE_ID, { type: "mini_player" })}
	{@attach interactive(WIDGET_MUSIC_FEATURE_ID)}
>
	<div class="move-btn-wrap">
		<button
			type="button"
			aria-label={$_("music.loopChange")}
			aria-pressed={audioStore.loopMode !== 0}
			class="control-btn loop-btn {LoopMode[audioStore.loopMode]}"
			onclick={() => audioStore.toggleLoop()}
		></button>
		<button
			type="button"
			aria-label={$_("music.dragHandle")}
			class="control-btn move-btn"
			{@attach draggable({
				target: moveEl,
				pos: localPos,
				snap: { x: { offset: 230, sides: ["right"] }, y: { offset: 0 } },
				onDragEnd: (pos) => {
					widgetMusicSetting.update({ pos });
				},
			})}
		>
			<GripVertical size={18} />
		</button>
	</div>
	<div class="song-info-wrap">
		<div
			class="album-art"
			style="--pos-x: {playlistStore?.currentSong?.imgPos?.x}%; --pos-y: {playlistStore?.currentSong?.imgPos
				?.y}%;"
		>
			<img
				src={playlistStore.currentSong?.albumArt
					? formatAlbumArt(
							playlistStore.currentSong?.albumArt,
							512,
							playlistStore.currentSong?.albumArtVersion,
						)
					: DefaultAlbumArt}
				alt=""
				draggable="false"
			/>
		</div>
		<div class="song-info">
			<h2 class="song-title">{playlistStore.currentSong?.title ?? $_("music.defaultTitle")}</h2>
			<p class="artist">{playlistStore.currentSong?.artist ?? $_("music.defaultArtist")}</p>
		</div>
	</div>
	<div class="controls">
		<IconButton
			label={audioStore.paused ? $_("music.play") : $_("music.pause")}
			class="control-btn play-btn"
			onclick={() => audioStore.togglePlay()}
			disabled={!playlistStore.currentSong}
		>
			<Icon name={audioStore.paused ? "play" : "pause"} size={20} />
		</IconButton>
		<IconButton
			label={$_("music.nextSong")}
			class="control-btn next-btn"
			onclick={() => handleNextSong()}
			disabled={audioStore.isDisabledNext}
		>
			<Icon name={"SkipForward"} size={20} />
		</IconButton>
	</div>
</div>

<style>
	.mini-player {
		position: fixed;
		top: 0;
		left: 0;
		display: flex;
		align-items: center;
		gap: 0.2rem;
		width: 300px;
		border-radius: var(--radius-xs);
		padding: 0.2rem;
		background: var(--surface);
		backdrop-filter: blur(20px);
		box-shadow: 0 0 0 1px var(--border);
		cursor: default;
		z-index: 900;
	}
	.move-btn-wrap {
		display: flex;
		flex-direction: column;
		justify-content: center;
		align-items: center;
		gap: 0.2rem;
	}
	.loop-btn {
		width: 0.6rem;
		height: 0.6rem;
		background: var(--text-muted);
		border-radius: 100%;
	}
	.loop-btn.LOOP_ALL {
		background: var(--danger);
	}
	.loop-btn.ONCE_ONE {
		background: var(--info);
	}
	.loop-btn.LOOP_ONE {
		background: var(--success);
	}
	.move-btn {
		color: var(--text-muted);
		font-size: 0.8rem;
		padding: 0;
		cursor: grab;
	}
	:global(.drag) {
		cursor: grabbing;
	}

	.album-art {
		width: 2.6rem;
		height: 2.6rem;
		border-radius: 0.2rem;
		font-size: 0;
		overflow: hidden;
	}
	.album-art img {
		width: 100%;
		height: 100%;
		object-fit: cover;
		object-position: var(--pos-x, 50%) var(--pos-y, 50%);
		-webkit-user-select: none; /* Safari */
		-ms-user-select: none; /* IE 10+ and Edge */
		user-select: none; /* Standard syntax */
	}
	.song-info-wrap {
		display: flex;
		justify-content: center;
		align-items: center;
		gap: 0.5rem;
		overflow: hidden;
		flex: 1 1 0;
	}

	.song-info {
		display: flex;
		flex-direction: column;
		flex: 1 1 0;
		white-space: nowrap;
		overflow: hidden;
	}
	.song-info h2 {
		font-size: 0.9rem;
		margin-bottom: 0.225rem;
		font-weight: 500;
		text-overflow: ellipsis;
		white-space: nowrap;
		overflow: hidden;
	}
	.song-info p {
		font-size: 0.7rem;
		color: var(--text-muted);
		text-overflow: ellipsis;
		white-space: nowrap;
		overflow: hidden;
	}

	.controls {
		display: flex;
		margin-right: 0.225rem;
	}

	.play-btn:hover {
		background: var(--surface-hover);
		color: var(--text);
	}
</style>
