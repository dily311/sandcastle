<script lang="ts">
	import { _ } from "svelte-i18n";
	// prettier-ignore
	import { Heart, Menu, Pause, Play, Repeat, Repeat1, Shuffle, SkipBack, SkipForward, Volume, Volume1, Volume2, VolumeOff, VolumeX } from "@lucide/svelte";
	import Playlist from "./PlaylistScreen.svelte";
	// prettier-ignore
	import { audioStore, handleNextSong, handlePrevSong, widgetMusicSetting, playlistStore, formatAlbumArt, } from "../musicPlyerStore.svelte";
	import { handleContextMenuForWidget } from "$/lib/contextmenu/contextMenu.svelte";
	import { interactive } from "$/lib/interaction/interactive";
	import { draggable } from "$/lib/action/useDrag.svelte";
	import DefaultAlbumArt from "$/asset/widget/album_art_default.png";
	import { WIDGET_MUSIC_FEATURE_ID } from "../musicFeature.svelte";
	import { LoopMode } from "../types";
	import { draggablePosition } from "$/lib/action/useImageCrap.svelte";
	import Button from "$/components/ui/button/Button.svelte";
	import Icon from "$/components/ui/Icon.svelte";
	import { cn } from "$/lib/utils/cn";
	import IconButton from "$/components/ui/button/IconButton.svelte";
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
	class="music-player-container"
	class:resize={widgetMusicSetting.resize}
	oncontextmenu={(e) => handleContextMenuForWidget(e, WIDGET_MUSIC_FEATURE_ID)}
	{@attach interactive(WIDGET_MUSIC_FEATURE_ID)}
	{@attach draggable({
		pos: localPos,
		snap: {
			x: { offset: widgetMusicSetting.resize ? 0 : 336 },
			y: true,
			onchange: (state) => {
				if (state.x === "right" && widgetMusicSetting.isPlayListOpen) {
					widgetMusicSetting.update({ isPlayListOpen: false });
				}
			},
		},
		onDragEnd: (pos) => {
			widgetMusicSetting.update({ pos });
		},
	})}
>
	<div
		class="music-player"
		oncontextmenu={(e) => {
			handleContextMenuForWidget(e, WIDGET_MUSIC_FEATURE_ID, {
				type: "music",
				data: { isNotMusic: playlistStore?.currentSong?.path !== "" },
			});
		}}
	>
		<div
			class="album-art"
			style="--pos-x: {playlistStore?.currentSong?.imgPos?.x}%; --pos-y: {playlistStore?.currentSong?.imgPos
				?.y}%;"
			oncontextmenu={(e) => {
				handleContextMenuForWidget(e, WIDGET_MUSIC_FEATURE_ID, {
					type: "albumArt",
					data: {
						isNotMusic: playlistStore?.currentSong?.path !== "",
						album_art_src: playlistStore?.currentSong?.albumArt,
					},
				});
			}}
			{@attach widgetMusicSetting.isImgEdit &&
				draggablePosition({
					pos: playlistStore?.currentSong?.imgPos ?? undefined,
					onchange: (pos) => playlistStore.updateCurrentMusicAlbumArtPos(pos),
					onEditEnd: () => {
						widgetMusicSetting.update({ isImgEdit: false });
					},
				})}
		>
			<img
				src={playlistStore?.currentSongAlbumArt
					? formatAlbumArt(
							playlistStore.currentSongAlbumArt,
							undefined,
							playlistStore.currentSong?.albumArtVersion,
						)
					: DefaultAlbumArt}
				alt=""
				draggable="false"
			/>
		</div>
		<div class="song-info-wrap">
			<div class="song-info">
				<div class="song-title-wrap">
					<h1 class="song-title">
						{playlistStore.currentSong?.title ?? $_("music.defaultTitle")}
					</h1>
				</div>
				<p class="artist">
					{playlistStore.currentSong?.artist ?? $_("music.defaultArtist")}
				</p>
			</div>

			<div class="controls">
				<div class="volume-control">
					<IconButton
						aria-label={$_("music.volume")}
						class="volume-icon"
						onclick={() => audioStore.toggleMuted()}
					>
						{#if audioStore.value.muted}
							<Icon name="VolumeX" size={22} />
						{:else if audioStore.volumeSliderValue == 0}
							<Icon name="VolumeOff" size={22} />
						{:else if audioStore.volumeSliderValue > 40}
							<Icon name="Volume2" size={22} />
						{:else if audioStore.volumeSliderValue > 0}
							<Icon name="Volume1" size={22} />
						{:else}
							<Icon name="Volume" size={22} />
						{/if}
					</IconButton>
					<div class="volume-slider-wrap">
						<input
							type="range"
							class="volume-slider base-slider"
							min="0"
							max="100"
							value={audioStore.volumeSliderValue}
							style="--current-fill: {audioStore.volumeSliderValue}%;"
							oninput={(e: Event) => {
								const value = (<HTMLInputElement>e.target).value;
								audioStore.setVolume(Number(value) / 100);
							}}
						/>
					</div>
				</div>
				<IconButton
					aria-label={audioStore.isShuffle ? $_("music.onShuffle") : $_("music.offShuffle")}
					class={cn("control-btn shuffle-btn", !audioStore.isShuffle && "shuffleOff")}
					onclick={() => audioStore.toggleShuffle()}
					disabled={audioStore.isDisabledShuffle}
				>
					<Icon name="Shuffle" size={22} />
				</IconButton>
				<IconButton
					aria-label={$_("plyer.prevSong")}
					class="control-btn prev-btn"
					onclick={() => handlePrevSong()}
					disabled={audioStore.isDisabledPrev}
				>
					<Icon name="SkipBack" size={22} />
				</IconButton>
				<IconButton
					aria-label={audioStore.paused ? $_("music.play") : $_("music.pause")}
					class="control-btn play-btn"
					onclick={() => audioStore.togglePlay()}
					disabled={!playlistStore.currentSong}
				>
					<Icon name={audioStore.paused ? "play" : "pause"} size={22} />
				</IconButton>
				<IconButton
					aria-label={$_("music.nextSong")}
					class="control-btn next-btn"
					onclick={() => handleNextSong()}
					disabled={audioStore.isDisabledNext}
				>
					<Icon name="SkipForward" size={22} />
				</IconButton>
				<IconButton
					aria-label={$_("music.loopChange")}
					aria-pressed={audioStore.loopMode !== 0}
					class="control-btn loop-btn"
					onclick={() => audioStore.toggleLoop()}
				>
					{#if audioStore.loopMode === LoopMode.LOOP_ALL}
						<Icon name="Repeat" size={22}>
							<text
								x="12"
								y="12.5"
								text-anchor="middle"
								dominant-baseline="middle"
								fill="currentColor"
								stroke="none"
								font-size="8"
								font-family="sans-serif"
							>
								A
							</text>
						</Icon>
					{:else if audioStore.loopMode === LoopMode.LOOP_ONE}
						<Icon name="Repeat1" size={22} />
					{:else if audioStore.loopMode === LoopMode.ONCE_ONE}
						<svg
							xmlns="http://w3.org"
							width="24"
							height="24"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
							stroke-linecap="round"
							stroke-linejoin="round"
							class="lucide lucide-repeat-one-straight svg-icon"
							aria-hidden="true"
						>
							<text
								x="12"
								y="11"
								text-anchor="middle"
								dominant-baseline="middle"
								fill="currentColor"
								stroke="none"
								font-size="10"
								font-family="sans-serif"
							>
								1
							</text>

							<path d="M4 19h18" />
							<path d="M17 16l4 3-3 3" />
						</svg>
					{:else}
						<svg
							xmlns="http://www.w3.org/2000/svg"
							width="24"
							height="24"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
							stroke-linecap="round"
							stroke-linejoin="round"
							class="lucide lucide-repeat-one-straight svg-icon"
							aria-hidden="true"
						>
							<text
								x="12"
								y="11"
								text-anchor="middle"
								dominant-baseline="middle"
								fill="currentColor"
								stroke="none"
								font-size="10"
								font-family="sans-serif"
							>
								A
							</text>
							<path d="M4 19h18" />
							<path d="M17 16l4 3-3 3" />
						</svg>
					{/if}
				</IconButton>
				<IconButton
					aria-label={$_("music.menu")}
					class="playlist"
					onclick={(e) => widgetMusicSetting.update({ isPlayListOpen: !widgetMusicSetting.isPlayListOpen })}
				>
					<Icon name="Menu" size={22} />
				</IconButton>
			</div>
			<div class="song-time">
				<div class="time-info">
					<span aria-label={$_("music.currentTime")} class="current-time"
						>{audioStore.displayCurrentTime}</span
					>
					<span aria-label={$_("music.duration")} class="duration">{audioStore.displayDuration}</span>
				</div>
				<input
					type="range"
					aria-label={$_("music.slider")}
					class="progress-slider base-slider"
					min="0"
					max="100"
					value={audioStore.progressSlider}
					style="--current-fill: {audioStore.progressSlider}%;"
					oninput={(e: Event) => {
						const value = (<HTMLInputElement>e.target).value;
						audioStore.seek(Number(value));
					}}
					onmousedown={(e: Event) => e.stopPropagation()}
				/>
			</div>
		</div>
	</div>
	{#if widgetMusicSetting.isPlayListOpen}
		<Playlist />
	{/if}
</div>

<style>
	.music-player-container {
		position: fixed;
		top: 0;
		left: 0;
		z-index: 10;
		box-shadow: 0 0 0 1px var(--card-border-color);
		background: var(--background-primary);
		border-radius: var(--card-radius);
		color: var(--text-color);
		cursor: default;
		overflow: hidden;
		contain: paint;
		max-width: 31rem;
		width: 100%;
		/* resize: both; */
	}
	.music-player-container.resize {
		display: flex;
		width: auto;
		max-width: none;
		.music-player {
			flex-direction: column;
			width: 18rem;
		}
		.album-art {
			border-bottom: 1px solid var(--card-border-color);
		}
		.album-art,
		.song-info-wrap {
			width: auto;
		}
	}

	.music-player {
		display: flex;
		justify-content: space-between;
		overflow: hidden;
	}
	.album-art {
		flex: 0 0 auto;
		width: 10rem;
		aspect-ratio: 1;
		overflow: hidden;
		font-size: 0;
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
		flex: 1;
		display: flex;
		flex-direction: column;
		justify-content: space-between;
		overflow: hidden;
	}
	.song-info {
		padding: 1.25rem 0.8rem;
	}
	.song-title-wrap {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.4rem;
		margin-bottom: 0.6rem;
	}
	.song-title {
		flex: 0 1 auto;
		font-size: 1.125rem;
		font-weight: 500;
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
	}

	.artist {
		color: var(--text-muted);
		font-size: 0.8rem;
		white-space: nowrap;
		text-overflow: ellipsis;
		overflow: hidden;
	}
	.controls {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0 0.8rem;
	}
	.controls button {
		cursor: pointer;
		border-radius: 100%;
		padding: 0.4rem;
		font-size: 0.8rem;
		vertical-align: middle;
	}
	.controls button:hover {
		background: var(--icon-hover);
	}

	.shuffleOff {
		opacity: 0.6;
	}

	.song-time .progress-slider {
		-webkit-appearance: none;
		width: 100%;
		height: 3px;
		background: transparent;
		cursor: pointer;
		outline: none;
		margin: 0.4rem 0 0 0;
		vertical-align: bottom;
		font-size: 0;
	}

	.song-time .progress-slider::-webkit-slider-thumb {
		-webkit-appearance: none;
		width: 8px;
		height: 3px;
		border: none;
		background: var(--fill-color);
		border-radius: 0;
		transform: none;
	}

	.song-time .progress-slider::-webkit-slider-runnable-track {
		-webkit-appearance: none;
		width: 100%;
		height: 3px;
		background:
			linear-gradient(to right, var(--fill-color), var(--fill-color)) 0% 0% / var(--current-fill) 100% no-repeat,
			var(--track-bg-color) 100% 0% / calc(100% - var(--current-fill)) 100% no-repeat;
		transition: background-size 0.1s linear;
		border: none;
		border-radius: 0;
		box-shadow: none;
	}

	.time-info {
		display: flex;
		justify-content: space-between;
		padding: 0 0.8rem;
		font-size: 0.6rem;
		color: var(--text-faint);
	}
	.volume-control {
		position: relative;
	}
	.volume-control .volume-slider-wrap {
		position: absolute;
		top: 50%;
		left: 100%;
		z-index: 1;
		transform: translate(0, -50%);
		background: var(--color-05);
		border-radius: 1.6rem;
		width: 0;
		min-width: 0;
		overflow: hidden;
		transition: width 0.3s;
	}
	.volume-control:hover .volume-slider-wrap {
		width: 100px;
		padding: 0.4rem 0.8rem;
	}
</style>
