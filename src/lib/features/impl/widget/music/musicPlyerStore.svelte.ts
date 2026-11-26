import { onEvent } from "$shared/ipc/events";
import { convertFileSrc } from "@tauri-apps/api/core";
import { commands } from "$ipc/bindings";
import { open } from "@tauri-apps/plugin-dialog";
import {
	type WidgetMusicSetting,
	type Playlist,
	type MusicMeta,
	type AudioState,
	LoopMode,
	type Pos,
	type PlaylistItem,
	type SortType,
	type FolderTree,
	type FolderNode,
	type LibraryRoot,
	type FolderNodeRaw,
	type LibraryRootRaw,
} from "./types";

// 상수 & 기본 설정
const defaultState: AudioState = {
	muted: false,
	volume: 0.7,
	loopMode: LoopMode.NONE,
	isShuffle: false,
};

const defaultMusicWidgetSetting: WidgetMusicSetting = {
	resize: false,
	isMiniPlayer: false,
	isImgEdit: false,
	isPlayListOpen: false,
	pos: null,
	selectedFolder: null,
	currentPlaylistId: "default",
	currentSongIndex: 0,
	audio: defaultState,
};
// ===================================
// 음악 플레이어
// ===================================
// -----------------------------------
// 전체 세팅
// -----------------------------------
class WidgetMusicSettingStore {
	#setting = $state<WidgetMusicSetting>(defaultMusicWidgetSetting);

	async init() {
		// 설정 값 불러오기
		this.#setting = (await commands.getWidgetMusicSettings()) as unknown as WidgetMusicSetting;

		// 연관 스토어 초기화
		await playlistStore.init(
			this.#setting.selectedFolder,
			this.#setting.currentPlaylistId,
			this.#setting.currentSongIndex,
		);
		await audioStore.init(this.#setting.audio);
		// 설정 화면에서 바꾼 것 따라가기 (5-E-21). 저장은 이미 됐으므로 다시 저장하지 않는다
		void onEvent<WidgetMusicSetting>("widget_music::changed", (next) => {
			const folderChanged = next.selectedFolder !== this.#setting.selectedFolder;
			this.#setting = { ...this.#setting, ...next };
			if (next.audio) audioStore.syncFromSetting(next.audio);
			if (folderChanged) void playlistStore.init(next.selectedFolder ?? null, next.currentPlaylistId ?? null, next.currentSongIndex ?? 0);
		});
	}

	update(patch: Partial<WidgetMusicSetting>) {
		const isIdentical = Object.entries(patch).every(
			([key, value]) => this.#setting[key as keyof WidgetMusicSetting] === value,
		);
		if (isIdentical) return;

		this.#setting = { ...this.#setting, ...patch };
		void commands.saveWidgetMusicSettings(patch as never);
	}

	toggleResize() {
		this.update({ resize: !widgetMusicSetting.resize });
	}

	toggleMini() {
		this.update({ isMiniPlayer: !widgetMusicSetting.isMiniPlayer });
	}

	onImgEdit() {
		this.update({ isImgEdit: true });
	}

	get value() {
		return this.#setting;
	}
	get resize() {
		return this.#setting.resize;
	}
	get isImgEdit() {
		return this.#setting.isImgEdit;
	}
	get isMiniPlayer() {
		return this.#setting.isMiniPlayer;
	}
	get isPlayListOpen() {
		return this.#setting.isPlayListOpen;
	}
	get pos() {
		return this.#setting.pos;
	}
	get selectedFolder() {
		return this.#setting.selectedFolder;
	}
}

export const widgetMusicSetting = new WidgetMusicSettingStore();

// -----------------------------------
// 플레이리스트
// -----------------------------------
class PlaylistStore {
	#store = $state<Playlist[]>([]);
	#currentPlaylistId = $state<string>("default");
	#currentPlaylist = $state<PlaylistItem[]>([]);
	#currentSongIndex = $state<number>(0);

	// 현재 선택된 폴더 내의 음악만 표시
	#selectedFolderMusics = $state<LibraryRoot>({ folders: new Map(), rootFiles: [] });
	#sortType = $state<SortType>("name");

	async init(selectedFolder: string | null, currentPlaylistId: string | null, currentSongIndex: number) {
		this.#store = (await commands.getAllPlaylist()) as unknown as Playlist[];
		const targetId =
			currentPlaylistId && this.#store.some((p) => p.id === currentPlaylistId)
				? currentPlaylistId
				: (this.#store[0]?.id ?? currentPlaylistId ?? "default");
		if (targetId) {
			this.#currentPlaylistId = targetId;
			this.#currentPlaylist = (await commands.getPlaylistEntry(targetId)) as unknown as PlaylistItem[];
		}
		this.#currentSongIndex = this.#currentPlaylist.length > currentSongIndex && currentSongIndex >= 0 ? currentSongIndex : 0;
		if (selectedFolder) {
			const raw = (await commands.getSelectFolderMusicData(selectedFolder)) as unknown as LibraryRootRaw;
			this.#selectedFolderMusics = convertLibraryRoot(raw);
		}
	}

	async selectPlaylist(id: string) {
		if (!id || id === this.#currentPlaylistId) return;
		const wasPlaying = !audioStore.paused;
		const entries = (await commands.getPlaylistEntry(id)) as unknown as PlaylistItem[];
		this.#currentPlaylistId = id;
		this.#currentPlaylist = entries;
		this.#currentSongIndex = 0;
		widgetMusicSetting.update({ currentPlaylistId: id, currentSongIndex: 0 });
		if (entries.length > 0) {
			audioStore.setMusic(entries[0].meta);
			if (wasPlaying) audioStore.play();
		} else {
			audioStore.stop();
		}
	}

	async createPlaylist(title: string): Promise<Playlist | null> {
		const trimmed = title.trim();
		if (!trimmed) return null;
		const created = (await commands.createPlaylist({
			title: trimmed,
			description: "",
			coverImage: null,
			sortOrder: this.#store.length,
		})) as unknown as Playlist;
		this.#store = [...this.#store, created];
		await this.selectPlaylist(created.id);
		return created;
	}

	async renamePlaylist(id: string, title: string) {
		const trimmed = title.trim();
		if (!trimmed) return;
		const existing = this.#store.find((p) => p.id === id);
		if (!existing) return;
		await commands.updatePlaylist(id, {
			title: trimmed,
			description: existing.description ?? "",
			coverImage: existing.coverImage ?? null,
			sortOrder: existing.sortOrder ?? 0,
		});
		this.#store = this.#store.map((p) => (p.id === id ? { ...p, title: trimmed } : p));
	}

	async deletePlaylist(id: string) {
		if (id === "default" || this.#store.length <= 1) return;
		await commands.deletePlaylist(id);
		this.#store = this.#store.filter((p) => p.id !== id);
		if (this.#currentPlaylistId === id) {
			const fallback = this.#store[0]?.id ?? "default";
			await this.selectPlaylist(fallback);
		}
	}

	async updateCurrentIndex(index: number) {
		this.#currentSongIndex = index;
		await commands.saveWidgetMusicSettings({ currentSongIndex: index });
	}

	/** 저장 X 그냥 순서만 변경하는 경우에 사용 */
	updateCurrentPlaylist(patch: PlaylistItem[]) {
		this.#currentPlaylist = patch;
	}

	async appendCurrentPlaylist(item: PlaylistItem) {
		this.#currentPlaylist.push(item);
		await commands.appendPlaylistEntry(this.currentPlaylistKey, item.meta.path, this.#currentPlaylist.length);
	}
	async removeCurrentPlaylist(music_src: string) {
		this.#currentPlaylist = this.#currentPlaylist.filter((v) => v.meta.path !== music_src);
		await commands.removePlaylistEntry(this.currentPlaylistKey, music_src);
	}
	/** 현재 음악데이터만 업데이트 */
	async updateCurrentSongData(patch: Partial<MusicMeta>) {
		const changeData = { ...this.currentSong, ...patch };
		this.#currentPlaylist.map((v) => (v.meta.path == changeData.path ? changeData : v));
		await commands.updateMusicMeta(changeData as never);
	}

	async updateCurrentMusicAlbumArtPos(pos: Pos) {
		const changeData = { ...this.currentSong, imgPos: pos };
		const changedPlaylist = this.#currentPlaylist.map((v) =>
			v.meta.path == changeData.path ? { meta: changeData, sortOrder: this.currentSongIndex } : v,
		);
		this.updateCurrentPlaylist(changedPlaylist);
		await commands.updateMusicAlbumArtPos(changeData.path, pos as never);
	}

	async updateMusicOrder(input: PlaylistItem[]) {
		if (this.#currentPlaylist === input) return;
		this.updateCurrentPlaylist(input);
		await commands.updatePlaylistMusicOrder(this.#currentPlaylistId, input as never);
	}

	setSelectedFolderMusics(newValue: LibraryRoot) {
		this.#selectedFolderMusics = newValue;
	}

	isExists = (path: string): boolean => {
		return this.current.some((v) => v.meta.path === path);
	};

	add = (music: MusicMeta) => {
		if (this.isExists(music.path)) return;
		// 새로 추가되는 거면 순서가 마지막에 오도록
		music.sortOrder = this.#currentPlaylist.length;
		// 초기값인 경우 바로 세팅
		if (this.#currentSongIndex === this.current.length) {
			audioStore.setMusic(music);
		}

		// 업데이트
		this.appendCurrentPlaylist({
			meta: music,
			sortOrder: this.#currentPlaylist.length,
		});
	};

	deleted = async (path: string) => {
		const deletedIndex = this.#currentPlaylist.findIndex((v) => v.meta.path === path);
		if (deletedIndex === -1) return;

		const isCurrent = deletedIndex === this.#currentSongIndex;
		if (isCurrent) {
			audioStore.pause();
		}

		await this.removeCurrentPlaylist(path);

		const newLen = this.#currentPlaylist.length;
		let nextIndex = this.#currentSongIndex;
		if (deletedIndex < this.#currentSongIndex) {
			// 현재 재생 곡보다 앞의 곡을 지웠으면, 같은 곡을 계속 가리키도록 인덱스만 보정
			nextIndex = Math.max(0, this.#currentSongIndex - 1);
		} else if (nextIndex >= newLen) {
			// 마지막 곡을 지워 인덱스가 범위를 벗어나면 마지막 유효 인덱스로
			nextIndex = Math.max(0, newLen - 1);
		}

		this.#currentSongIndex = nextIndex;
		await commands.saveWidgetMusicSettings({ currentSongIndex: nextIndex });

		if (isCurrent) {
			const nextSong = this.#currentPlaylist[nextIndex]?.meta;
			if (nextSong) audioStore.setMusic(nextSong);
			else audioStore.stop();
		}
	};

	shuffle = (isStartIndex: boolean = false) => {
		const playIndex = this.#currentSongIndex;
		const newPlaylist = this.#currentPlaylist;

		// 현재 재생 중인 곡의 실제 데이터(또는 고유 ID)를 미리 저장해 둡니다.
		const currentSong = this.current[playIndex];

		const index = isStartIndex ? playIndex + 1 : 0;
		const len = this.current.length - index;
		const orderArr = Array.from(new Array(len), (_, i) => i + index);

		// 피셔-예이츠(Fisher-Yates) 셔플 알고리즘
		for (let i = len; i > 0; i--) {
			const random = Math.floor(Math.random() * i);
			const value = orderArr[i - 1];
			orderArr[i - 1] = orderArr[random];
			orderArr[random] = value;
		}

		const startArr = Array.from(new Array(index).keys());
		const playOrder = [...startArr, ...orderArr];

		playOrder.forEach((newOrder, index) => {
			this.current[index].sortOrder = newOrder;
		});

		// 플레이리스트 정렬 적용
		const sortedPlaylist = newPlaylist.sort((a, b) => a.sortOrder - b.sortOrder);
		this.updateCurrentPlaylist(sortedPlaylist);

		// 정렬된 배열에서 방금 전 재생 중이던 곡의 새로운 인덱스를 찾아 currentIndex를 갱신
		if (currentSong) {
			const newCurrentIndex = this.current.findIndex((song) => song === currentSong); // 또는 id 비교
			if (newCurrentIndex !== -1) {
				this.#currentSongIndex = newCurrentIndex;
			}
		}
	};

	changeOrder = (item: PlaylistItem, index: number) => {};

	// getter
	get list() {
		return this.#store;
	}
	get currentPlaylist(): Playlist | undefined {
		return this.#store.find((p) => p.id === this.#currentPlaylistId);
	}
	get current() {
		return this.#currentPlaylist;
	}
	get currentSongIndex() {
		return this.#currentSongIndex;
	}
	get currentPlaylistKey() {
		return this.#currentPlaylistId;
	}
	get selectedFolderMusics() {
		return this.#selectedFolderMusics;
	}
	get currentSong(): MusicMeta {
		return this.#currentPlaylist[this.#currentSongIndex]?.meta;
	}
	get currentSongAlbumArt() {
		return this.currentSong?.albumArt;
	}
	get sortType() {
		return this.#sortType;
	}
}

export const playlistStore = new PlaylistStore();

// -----------------------------------
// 오디오 상태관리 스토어
// -----------------------------------
class AudioStateStore {
	// 오디오 el
	#audioElement = $state<HTMLAudioElement>(new Audio());
	#audioState = $state<AudioState>(defaultState);
	#paused = $state(true);
	#currentTime = $state(0);
	#duration = $state(0);

	async init(saved?: Partial<AudioState>) {
		// 이전에 재생 중이던 오디오 인스턴스가 남지 않도록 먼저 정리
		this.stop();
		// 예전 localStorage 값이 남아 있으면 한 번 옮겨 담고 제거
		let legacy: Partial<AudioState> | null = null;
		try {
			const raw = localStorage.getItem("WIDGET.AUDIO_STATE");
			if (raw) {
				legacy = JSON.parse(raw) as Partial<AudioState>;
				localStorage.removeItem("WIDGET.AUDIO_STATE");
			}
		} catch {
			/* ignore */
		}
		this.#audioState = { ...defaultState, ...(saved ?? {}), ...(legacy ?? {}) };
		if (legacy) {
			widgetMusicSetting.update({ audio: { ...this.#audioState } });
		}
		const audio = new Audio();
		audio.volume = this.#audioState.volume;
		audio.muted = this.#audioState.muted;
		audio.addEventListener("timeupdate", () => (this.#currentTime = audio.currentTime));
		audio.addEventListener("loadedmetadata", () => (this.#duration = audio.duration));
		audio.addEventListener("volumechange", () => this.update({ volume: audio.volume, muted: audio.muted }));
		audio.addEventListener("play", () => (this.#paused = false));
		audio.addEventListener("pause", () => (this.#paused = true));
		audio.addEventListener("ended", () => handleAudioEnded());
		this.#audioElement = audio;
		this.setMusic(playlistStore.currentSong);
	}

	syncFromSetting(next: Partial<AudioState>) {
		const merged = { ...this.#audioState, ...next };
		this.#audioState = merged;
		if (this.#audioElement) {
			if (this.#audioElement.volume !== merged.volume) this.#audioElement.volume = merged.volume;
			if (this.#audioElement.muted !== merged.muted) this.#audioElement.muted = merged.muted;
		}
	}

	set(newValue: AudioState) {
		this.#audioState = newValue;
	}

	// 저장은 위젯 설정(widget.json)으로
	update(state: Partial<AudioState>) {
		const isIdentical = Object.entries(state).every(
			([key, value]) => this.#audioState[key as keyof AudioState] === value,
		);
		if (isIdentical) return;
		this.#audioState = { ...this.#audioState, ...state };
		widgetMusicSetting.update({ audio: { ...this.#audioState } });
	}

	setMusic = (playMusic: MusicMeta) => {
		if (!playMusic) return console.error("[MusicPlayer] Missing music metadata");
		if (!this.#audioElement) return console.error("[MusicPlayer] Missing audio element");

		const newSrc = convertFileSrc(playMusic.path);
		if (this.#audioElement.src != newSrc) {
			this.#audioElement.src = newSrc;
			this.#audioElement.currentTime = 0;
			this.#duration = playMusic.duration;
			this.#currentTime = 0;
		}
	};
	autoSetMusic() {
		const music = playlistStore.currentSong;
		this.setMusic(music);
	}
	isCurrentMusic = (music_src: string) => {
		return this.#audioElement.src === music_src;
	};

	play = () => {
		this.#audioElement.play().catch((e) => {
			console.warn("[MusicPlayer] Audio playback failed", e);
			//TODO: 파일 존재여부 확인 로직 필요
		});
		this.#paused = false;
	};

	pause = () => {
		this.#audioElement.pause();
		this.#paused = true;
	};

	stop = () => {
		this.#audioElement.pause();
		this.#audioElement.currentTime = 0;
		this.#audioElement.removeAttribute("src");
		this.#audioElement.load();
		this.#paused = true;
		this.#currentTime = 0;
	};

	togglePlay = () => {
		if (this.#paused) {
			this.play();
		} else {
			this.pause();
		}
	};

	setMuted = (muted: boolean) => {
		this.#audioElement.muted = muted;
		this.update({ muted: muted });
	};

	toggleMuted = () => {
		this.#audioElement.muted = !this.#audioElement.muted;
		this.update({ muted: this.#audioElement.muted });
	};

	toggleLoop = () => {
		let change = this.#audioState.loopMode + 1;
		if (this.#audioState.loopMode > 2) {
			change = 0;
		}
		this.update({ loopMode: change });
	};

	toggleShuffle = () => {
		const isShuffle = !this.#audioState.isShuffle;
		this.update({ isShuffle: isShuffle });
		if (isShuffle) playlistStore.shuffle();
	};

	setVolume = (value: number) => {
		this.#audioElement.volume = value;
		this.update({ volume: this.#audioElement.volume });
	};

	seek = (value: number) => {
		const time = (Number(value) / 100) * this.#duration;
		this.#audioElement.currentTime = time;
		this.#currentTime = time;
	};

	// getter
	get value() {
		return this.#audioState;
	}
	get loopMode() {
		return this.#audioState.loopMode;
	}
	get isShuffle() {
		return this.#audioState.isShuffle;
	}
	get paused() {
		return this.#paused;
	}
	get currentTime() {
		return this.#currentTime;
	}
	get displayDuration() {
		return formatTime(this.#duration);
	}
	get displayCurrentTime() {
		return formatTime(this.#currentTime);
	}
	get progressSlider() {
		return this.#duration === 0 ? 0 : (this.#currentTime / this.#duration) * 100;
	}
	get volumeSliderValue() {
		return this.#audioState.volume * 100;
	}
	get isDisabledPrev() {
		return playlistStore.currentSongIndex === 0;
	}
	get isDisabledNext() {
		return (
			this.#audioState.loopMode !== LoopMode.LOOP_ALL &&
			playlistStore.currentSongIndex >= playlistStore.current.length - 1
		);
	}
	get isDisabledShuffle() {
		return (
			!(playlistStore.current.length > 1) ||
			this.#audioState.loopMode === LoopMode.ONCE_ONE ||
			this.#audioState.loopMode === LoopMode.LOOP_ONE
		);
	}
}

export const audioStore = new AudioStateStore();

// ===================================
// 사용함수
// ===================================

// -----------------------------------
// 음악 플레이
// -----------------------------------

export function handlePrevSong() {
	if (!playlistStore.current.length) return;
	// 현재 재생되고 있다면 처음으로 돌아가도록
	if (Math.round(audioStore.currentTime) > 10) {
		audioStore.seek(0);
		return;
	}

	// 아니라면 이전 곡 재생
	let changePlayIndex = playlistStore.currentSongIndex;

	// 현재 인덱스가 0이고 만약 loop가 참이라면, 마지막 리스트로 돌아가도록
	if (Array.isArray(playlistStore.current) && audioStore.loopMode === LoopMode.LOOP_ALL && changePlayIndex == 0) {
		changePlayIndex = playlistStore.current.length - 1;
	} else if (changePlayIndex > 0) {
		changePlayIndex--;
	}

	// 음악 세팅
	const currentMusic = playlistStore.current[changePlayIndex];
	if (!currentMusic) return;
	audioStore.setMusic(currentMusic.meta);
	// 멈춰 있지 않았으면 플레이
	if (!audioStore.paused) {
		audioStore.play();
	}
	playlistStore.updateCurrentIndex(changePlayIndex);
}

export function handleNextSong(autoPlay?: boolean) {
	if (!playlistStore.current.length) return;
	let playIndex = playlistStore.currentSongIndex;

	// 플레이리스트 길이에 도달했으면 루프가 아닌 이상 다음 곡 재생되지 않게 함
	if (playlistStore.current.length - 1 == playIndex && audioStore.loopMode !== LoopMode.LOOP_ALL) return;

	if (playlistStore.current.length - 1 > playIndex) {
		// 현재 플레이리스트 길이보다 인덱스가 작은 경우 ++하기
		playlistStore.updateCurrentIndex(++playIndex);
	} else {
		// 현재 플레이리스트 길이보다 인덱스가 크고, 루프가 참인 경우 초기값으로 돌아가기
		playlistStore.updateCurrentIndex(0);
		// 셔플 기능을 켜둔 경우 루프 종료시 자동 셔플
		if (audioStore.isShuffle) playlistStore.shuffle(false);
	}

	// 음악 세팅
	audioStore.setMusic(playlistStore.currentSong);

	// 멈춰 있지 않았으면 플레이
	if (!audioStore.paused || autoPlay) {
		audioStore.play();
	}
}

export function handleAudioEnded() {
	if (audioStore.loopMode === LoopMode.ONCE_ONE) return;
	if (audioStore.loopMode === LoopMode.LOOP_ONE) return audioStore.play();

	handleNextSong(true);
}

/** 클릭 이벤트로 바꾸는 음악 */
export function changePlayMusic(index: number, music: MusicMeta) {
	// index 허용범위: this.currentPlaylist.length > index >= 0
	if (index < 0 || playlistStore.current?.length < index) return;

	// 음악 세팅
	audioStore.setMusic(music);
	if (!audioStore.paused) {
		audioStore.play();
	}
	playlistStore.updateCurrentIndex(index);
}

// -----------------------------------
// 음악 데이터를 받아오는 처리
// -----------------------------------
export async function handleSelectMusicFolder() {
	// 유저가 폴더 선택
	const selected = await open({
		directory: true,
		multiple: false,
	});
	if (!selected) return;
	await setSelectedMusicFolder(selected);

	widgetMusicSetting.update({ selectedFolder: selected });
}

export async function setSelectedMusicFolder(folderPath: string) {
	const raw = (await commands.getSelectFolderMusicData(folderPath)) as unknown as LibraryRootRaw;
	// 스토어 등록
	playlistStore.setSelectedFolderMusics(convertLibraryRoot(raw));
	return raw;
}

export async function handleChangeAlbumCover() {
	const selectedIndex = playlistStore.currentSongIndex;
	const selectedSong = playlistStore.currentSong;
	// 앨범아트에 추가할 이미지 선택
	const selected = await open({
		directory: false,
		multiple: false,
		filters: [
			{
				name: "Images",
				extensions: ["png", "jpg", "jpeg"],
			},
		],
	});
	if (!selected || !playlistStore.currentSong) return;

	const album_art = await commands.setAlbumArt(playlistStore.currentSong.path, selected);

	// 앨범아트 변경된 경로 반영
	// 캐싱된거 사용하는 이유: 다음 음악으로 자동으로 넘어가는 경우 인덱스가 꼬임
	const changedPlaylist = playlistStore.current;
	changedPlaylist[selectedIndex] = {
		...selectedSong,
		meta: { ...selectedSong, albumArt: album_art, albumArtVersion: Date.now() },
	};

	playlistStore.updateCurrentPlaylist(changedPlaylist);
}

export const togglePlaylist = (music: MusicMeta) => {
	if (playlistStore.isExists(music.path)) {
		playlistStore.deleted(music.path);
	} else {
		playlistStore.add(music);
	}
};

// -----------------------------------
// 그룹화
// -----------------------------------
function convertFolderNode(raw: FolderNodeRaw): FolderNode {
	const convertedFolders = new Map<string, FolderNode>();

	// 내부 folders 객체의 키-값을 순회하며 재귀 변환
	for (const [key, value] of Object.entries(raw.folders)) {
		convertedFolders.set(key, convertFolderNode(value));
	}

	return {
		name: raw.name,
		files: raw.files,
		folders: convertedFolders,
	};
}

function convertLibraryRoot(raw: LibraryRootRaw): LibraryRoot {
	const convertedFolders = new Map<string, FolderNode>();

	for (const [key, value] of Object.entries(raw.folders)) {
		convertedFolders.set(key, convertFolderNode(value));
	}

	return {
		rootFiles: raw.rootFiles,
		folders: convertedFolders,
	};
}

// -----------------------------------
// 파일 정렬
// -----------------------------------
export const getSortedFiles = (files: MusicMeta[]) => {
	return [...files].sort((a, b) => {
		switch (playlistStore.sortType) {
			case "latest":
				return (b.updatedAt || 0) - (a.updatedAt || 0);
			case "album":
				return (a.album || "").localeCompare(b.album || "");
			case "name":
			default:
				return a.path.localeCompare(b.path);
		}
	});
};
// -----------------------------------
// Utils
// -----------------------------------
function formatTime(seconds: number) {
	const hour = Math.floor(seconds / 60 / 60);
	const minutes = Math.floor(seconds / 60 - hour * 60);
	const secs = Math.floor(seconds % 60);

	// 숫자 앞에 0 추가
	const hh = String(hour).padStart(2, "0");
	const mm = String(minutes).padStart(2, "0");
	const ss = String(secs).padStart(2, "0");

	return `${hour != 0 ? hh + ":" : ""}${mm}:${ss}`;
}

export function formatAlbumArt(src: string, size?: number, version?: number) {
	let changeSrc = src;
	if (size) {
		const target = "album_arts\\";
		changeSrc = src.includes(target) ? src.replace(target, `${target}${size}\\`) : src;
	}

	const updatedSrc = convertFileSrc(changeSrc);
	return `${updatedSrc}?v=${version ?? 0}`;
}
