import { contextMenuRegistry } from "$/lib/contextmenu/registry";
import { contextActionRegistry } from "$/lib/contextmenu/actions";
import { audioStore, handleChangeAlbumCover, handleNextSong, handlePrevSong, widgetMusicSetting, playlistStore } from "./musicPlyerStore.svelte";
import type { FeatureRuntime } from "$/lib/features/types";
import MusicPlayer from "./ui/MusicPlayer.svelte";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { widgetFeatures } from "$/lib/features/widgetFeatures.svelte";
import { interactiveManager } from "$/lib/interaction/interactive-registry";
import { visibleFor } from "$/lib/contextmenu/contextMenu.svelte";

export const WIDGET_MUSIC_FEATURE_ID = "widget.music";

class RegisterMusicFeature implements FeatureRuntime {
	id = WIDGET_MUSIC_FEATURE_ID;
	enabled = $state(false);
	defaultEnabled = false;
	component = MusicPlayer;
	private unsubscriber?: () => void;

	async init() {
		if (!this.enabled) return;
		this.enabled = true;
		this.unsubscriber = registerMusicMenuFeature();
	}

	shown() {
		this.enabled = true;
	}

	disable() {
		this.enabled = false;
		audioStore.stop();
		interactiveManager.unregisterInteractiveHitbox(this.id);
	}

	destroy() {
		this.enabled = false;
		audioStore.stop();
		this.unsubscriber?.();
		this.unsubscriber = undefined;

		interactiveManager.unregisterInteractiveHitbox(this.id);
		// 제거
		widgetFeatures.unregister(this.id);
	}
}

export const registerWidgetMusic = new RegisterMusicFeature();

function registerMusicMenuFeature() {
	const unregisterMenu = contextMenuRegistry.register({
		featureId: WIDGET_MUSIC_FEATURE_ID,
		priority: 10,
		interactive: true,

		items: [
			{
				id: "music.controls",
				label: "",
				controls: [
					{
						id: "music.prev",
						icon: "SkipBack",
						label: "music.prevSong",
						action: "music.prev",
						disabled: () => playlistStore.current.length === 0 || (audioStore.isDisabledPrev && Math.round(audioStore.currentTime) <= 10),
						keepOpen: true,
					},
					{
						id: "music.togglePlay",
						icon: () => (audioStore.paused ? "Play" : "Pause"),
						label: "music.play",
						action: "music.togglePlay",
						disabled: () => playlistStore.current.length === 0,
						keepOpen: true,
					},
					{
						id: "music.next",
						icon: "SkipForward",
						label: "music.nextSong",
						action: "music.next",
						disabled: () => playlistStore.current.length === 0 || audioStore.isDisabledNext,
						keepOpen: true,
					},
				],
			},
			{
				id: "music.controlsSeparator",
				label: "",
				separator: true,
			},
			{
				id: "music.playlists",
				label: "music.context:playlists",
				icon: "ListMusic",
				children: () =>
					playlistStore.list.map((pl) => ({
						id: `music.playlist.${pl.id}`,
						label: pl.title || "기본 플레이리스트",
						rawLabel: true,
						isChecked: () => playlistStore.currentPlaylistKey === pl.id,
						onselect: () => playlistStore.selectPlaylist(pl.id),
					})),
			},
			{
				id: "music.changeLayout",
				label: "music.context:changeLayout",
				icon: widgetMusicSetting.resize ? "Maximize2" : "Minimize2",
				action: "music.changeLayout",
				visible: () => !widgetMusicSetting.isMiniPlayer,
				isChecked: () => widgetMusicSetting.resize,
			},
			{
				id: "music.miniPlayer",
				label: "music.context:miniPlayer",
				icon: "PictureInPicture2",
				action: "music.miniPlayer",
				isChecked: (context) => visibleFor("mini_player")(context),
			},
			{
				id: "music.open",
				label: "music.context:open",
				icon: "FolderOpen",
				action: "music.open",
				visible: (context) => visibleFor("music", "albumArt")(context),
				disabled: (context) => Boolean(context?.data?.["isNotMusic"]),
			},
			{
				id: "music.albumArtGroup",
				label: "music.context:albumArtGroup",
				icon: "Image",
				children: [
					{
						id: "music.changeAlbumArt",
						label: "music.context:changeAlbumArt",
						icon: "Image",
						action: "music.changeAlbumArt",
						visible: (context) => visibleFor("albumArt")(context),
						disabled: (context) => Boolean(context?.data?.["isNotMusic"]),
					},
					{
						id: "music.imgEdit",
						label: "music.context:imgEdit",
						icon: "Move",
						action: "music.imgEdit",
						visible: (context) => visibleFor("albumArt")(context) && context?.data?.["album_art_src"] !== null,
					},
				]
			},

			{
				id: "music.separator",
				label: "",
				separator: true,
			},
			{
				id: "music.quit",
				label: "music.context:quit",
				icon: "X",
				action: "music.quit",
			},
		],
	});

	const unregisterPrev = contextActionRegistry.register("music.prev", () => {
		handlePrevSong();
	});
	const unregisterTogglePlay = contextActionRegistry.register("music.togglePlay", () => {
		audioStore.togglePlay();
	});
	const unregisterNext = contextActionRegistry.register("music.next", () => {
		handleNextSong();
	});
	const unregisterChangeLayout = contextActionRegistry.register("music.changeLayout", () => {
		widgetMusicSetting.toggleResize();
	});
	const unregisterMiniPlayer = contextActionRegistry.register("music.miniPlayer", () => {
		widgetMusicSetting.toggleMini();
	});
	const unregisterOpen = contextActionRegistry.register("music.open", async () => {
		const file_src = playlistStore.currentSong.path;
		if (!file_src || file_src == "") return;
		await revealItemInDir(file_src);
	});
	const unregisterChangeAlbumArt = contextActionRegistry.register("music.changeAlbumArt", () => {
		handleChangeAlbumCover();
	});
	const unregisterImgEdit = contextActionRegistry.register("music.imgEdit", () => {
		widgetMusicSetting.onImgEdit();
	});
	const unregisterQuit = contextActionRegistry.register("music.quit", () => {
		registerWidgetMusic.destroy();
	});

	return () => {
		unregisterMenu();
		unregisterPrev();
		unregisterTogglePlay();
		unregisterNext();
		unregisterChangeLayout();
		unregisterMiniPlayer();
		unregisterOpen();
		unregisterChangeAlbumArt();
		unregisterImgEdit();
		unregisterQuit();
	};
}
