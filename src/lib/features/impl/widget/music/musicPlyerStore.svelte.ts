import { convertFileSrc } from "@tauri-apps/api/core";
import { widgetCommands } from "$/ipc/commands";
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
const defaultMusicWidgetSetting: WidgetMusicSetting = {
	resize: false,
	isMiniPlayer: false,
	isImgEdit: false,
	isPlayListOpen: false,
	pos: null,
	selectedFolder: null,
	currentPlaylistId: "default",
	currentSongIndex: 0,
};

const defaultState = {
	muted: false,
	volume: 0.7,
	loopMode: LoopMode.NONE,
	isShuffle: false,
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
		this.#setting = await widgetCommands.getWidgetMusicSettings();

		// 연관 스토어 초기화
		await playlistStore.init(
			this.#setting.selectedFolder,
			this.#setting.currentPlaylistId,
			this.#setting.currentSongIndex,
		);
		await audioStore.init();
	}

	update(patch: Partial<WidgetMusicSetting>) {
		const isIdentical = Object.entries(patch).every(
			([key, value]) => this.#setting[key as keyof WidgetMusicSetting] === value,
		);
		if (isIdentical) return;

		this.#setting = { ...this.#setting, ...patch };
		widgetCommands.saveWidgetMusicSettings({ patch: patch });
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
		this.#store = await widgetCommands.getAllPlaylist();
		if (currentPlaylistId) {
			this.#currentPlaylistId = currentPlaylistId;
			this.#currentPlaylist = await widgetCommands.getPlaylistMusic({ playlistId: currentPlaylistId });
		}
		if (this.#currentPlaylist.length > currentSongIndex) {
			this.#currentSongIndex = currentSongIndex;
		}
		if (selectedFolder) {
			const raw = await widgetCommands.getSelectedFolderMusics({ folderPath: selectedFolder });
			this.#selectedFolderMusics = convertLibraryRoot(raw);
		}
	}

	async updateCurrentIndex(index: number) {
		this.#currentSongIndex = index;
		await widgetCommands.saveWidgetMusicSettings({ patch: { currentSongIndex: index } });
	}

	/** 저장 X 그냥 순서만 변경하는 경우에 사용 */
	updateCurrentPlaylist(patch: PlaylistItem[]) {
		this.#currentPlaylist = patch;
	}

	async appendCurrentPlaylist(item: PlaylistItem) {
		this.#currentPlaylist.push(item);
		await widgetCommands.appendPlaylistMusic({
			playlistId: this.currentPlaylistKey,
			musicSrc: item.meta.path,
			sortOrder: this.#currentPlaylist.length,
		});
	}
	async removeCurrentPlaylist(music_src: string) {
		this.#currentPlaylist = this.#currentPlaylist.filter((v) => v.meta.path !== music_src);
		await widgetCommands.removePlaylistMusic({ playlistId: this.currentPlaylistKey, musicSrc: music_src });
	}
	/** 현재 음악데이터만 업데이트 */
	async updateCurrentSongData(patch: Partial<MusicMeta>) {
		const changeData = { ...this.currentSong, ...patch };
		this.#currentPlaylist.map((v) => (v.meta.path == changeData.path ? changeData : v));
		await widgetCommands.updateMusicMeta({ input: changeData });
	}

	async updateCurrentMusicAlbumArtPos(pos: Pos) {
		const changeData = { ...this.currentSong, imgPos: pos };
		const changedPlaylist = this.#currentPlaylist.map((v) =>
			v.meta.path == changeData.path ? { meta: changeData, sortOrder: this.currentSongIndex } : v,
		);
		this.updateCurrentPlaylist(changedPlaylist);
		await widgetCommands.updateMusicAlbumArtPos({ path: changeData.path, pos });
	}

	async updateMusicOrder(musicSrc: string, sortOrder: number) {
		const item = this.#currentPlaylist.find((v) => v.meta.path === musicSrc);
		const changeData = { ...item, sortOrder };
		this.#currentPlaylist.map((v) => (v.meta.path == changeData.meta?.path ? changeData : v));
		await widgetCommands.updateMusicOrder({ playlistId: this.#currentPlaylistId, musicSrc, sortOrder });
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
		if (this.#currentSongIndex === this.current.length - 1) {
			audioStore.setMusic(music);
		}

		// 업데이트
		this.appendCurrentPlaylist({
			meta: music,
			sortOrder: this.#currentPlaylist.length,
		});
	};

	deleted = async (path: string, index?: number) => {
		// 현재 플레이 중인 음악과 같을 경우 멈추기
		if (this.currentSong?.path === path) {
			audioStore.pause();
		}

		// 업데이트
		await this.removeCurrentPlaylist(path);

		// 인덱스 갱신
		let changeIndex = this.#currentSongIndex;
		// 현재 인덱스보다 바뀌는 배열이 더 작다면 -1 처리
		// 만약 삭제하려는 인덱스가 현재 인덱스보다 작은 경우 -1 처리
		if (this.current.length < changeIndex || (changeIndex > this.currentSong.sortOrder)) {
			changeIndex -= 1;
			// 바뀔 인덱스가 음수인 경우 음악을 세팅하지 않고 그대로 리턴
			if (changeIndex < 0) {
				this.#currentSongIndex = 0;
				return;
			}

			this.#currentSongIndex = changeIndex;
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

	async init() {
		const data = localStorage.getItem("WIDGET.AUDIO_STATE");
		if (data) {
			this.#audioState = JSON.parse(data);
		}
		const audio = new Audio();
		audio.volume = this.#audioState.volume;
		audio.addEventListener("timeupdate", () => (this.#currentTime = audio.currentTime));
		audio.addEventListener("loadedmetadata", () => (this.#duration = audio.duration));
		audio.addEventListener("volumechange", () => this.update({ volume: audio.volume }));
		audio.addEventListener("play", () => (this.#paused = false));
		audio.addEventListener("pause", () => (this.#paused = true));
		audio.addEventListener("ended", () => handleAudioEnded());
		this.#audioElement = audio;
		this.setMusic(playlistStore.currentSong);
	}

	set(newValue: AudioState) {
		this.#audioState = newValue;
	}

	// 저장은 여기서만
	update(state: Partial<AudioState>) {
		this.#audioState = { ...this.#audioState, ...state };
		localStorage.setItem("WIDGET.AUDIO_STATE", JSON.stringify(this.#audioState));
	}

	setMusic = (playMusic: MusicMeta) => {
		if (!playMusic) return console.error("없는 음악을 세팅하려함");
		if (!this.#audioElement) return console.error("오디오 el이 없음");

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
			console.warn("오디오 재생 실패", e);
			//TODO: 파일 존재여부 확인 로직 필요
		});
		this.#paused = false;
	};

	pause = () => {
		this.#audioElement.pause();
		this.#paused = true;
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
	audioStore.setMusic(currentMusic.meta);
	// 멈춰 있지 않았으면 플레이
	if (!audioStore.paused) {
		audioStore.play();
	}
	playlistStore.updateCurrentIndex(changePlayIndex);
}

export function handleNextSong(autoPlay?: boolean) {
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
	const raw = await widgetCommands.getSelectedFolderMusics({ folderPath });
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

	const album_art = await widgetCommands.setAlbumCover({
		filePath: playlistStore.currentSong.path,
		imgPath: selected,
	});

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

// export const groupedByFolder = (musicList: MusicMeta[]): LibraryRoot => {
// 	const rootTree: FolderTree = new Map();
// 	const rootFiles: MusicMeta[] = [];

// 	for (const file of musicList) {
// 		// 선택된 폴더 루트 경로 제거
// 		const rootPath = widgetMusicSetting.selectedFolder ? widgetMusicSetting.selectedFolder + "\\" : "";
// 		const path = file.path.replace(rootPath, "");
// 		const parts = path.split("\\");

// 		const fileName = parts.pop()!;
// 		const folderParts = parts; // 하위 폴더 경로들

// 		// 만약 하위 폴더 경로가 전혀 없다면 루트 레벨 파일입니다!
// 		if (folderParts.length === 0) {
// 			rootFiles.push(file);
// 			continue;
// 		}

// 		let currentFolders = rootTree;
// 		let currentNode: FolderNode | null = null;

// 		// 하위 폴더 생성 및 탐색
// 		for (const folderName of folderParts) {
// 			if (!currentFolders.has(folderName)) {
// 				currentFolders.set(folderName, {
// 					name: folderName,
// 					folders: new Map(),
// 					files: [],
// 				});
// 			}
// 			currentNode = currentFolders.get(folderName)!;
// 			currentFolders = currentNode.folders;
// 		}

// 		// 마지막 폴더 노드의 files 배열에 추가
// 		if (currentNode) {
// 			currentNode.files.push(file);
// 		}
// 	}

// 	return { folders: rootTree, rootFiles };
// };
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
