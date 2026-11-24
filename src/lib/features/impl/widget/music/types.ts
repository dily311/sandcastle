// ui type
export type SortType = 'name' | 'latest' | 'album';

// Rust에서 넘어오는 raw JSON 타입 (Object 형태)
export interface FolderNodeRaw {
  name: string;
  folders: Record<string, FolderNodeRaw>; // HashMap -> Record
  files: MusicMeta[];
}

export interface LibraryRootRaw {
  folders: Record<string, FolderNodeRaw>; // HashMap -> Record
  rootFiles: MusicMeta[]; // serde(rename_all = "camelCase") 반영
}

export interface FolderNode {
	name: string;
	folders: Map<string, FolderNode>;
	files: MusicMeta[];
}
export type FolderTree = Map<string, FolderNode>;
export interface LibraryRoot {
    folders: FolderTree;
    rootFiles: MusicMeta[];
}

// ===================================
// types
// ===================================
export type Pos = {x: number; y: number;};

export interface WidgetMusicSetting {
	resize: boolean; // defaultPlayer에만 적용
	isMiniPlayer: boolean;
	isImgEdit: boolean;
	isPlayListOpen: boolean; // defaultPlayer에만 적용
	pos: Pos | null;

	selectedFolder: string | null;
	currentPlaylistId: string | null;
	currentSongIndex: number;
	audio?: AudioState;
}

export interface MusicMeta {
	path: string;
	title: string;
	artist: string | null;
	album: string | null;
	albumTitle: string | null;
	albumArtist: string | null;
	trackNumber: number | null;
	genre: string | null;
	comment: string | null;
	albumArt: string | null;
	albumArtVersion?: number;
	duration: number;
	sortOrder: number;
	imgPos: Pos | null;
	updatedAt: number;
}

export interface MusicMetaInput {
	path: string;
	title: string;
	artist: string | null;
	album: string | null;
	albumTitle: string | null;
	albumArtist: string | null;
	trackNumber: number | null;
	genre: string | null;
	comment: string | null;
	albumArt: string | null;
	albumArtVersion?: number;
	duration: number;
	sortOrder: number;
	updatedAt: number;
}

export interface Playlist {
	id: string;
	title: string;
	description: string;
	coverImage: string | null;
	sortOrder: number;
	createdAt: number;
	updatedAt: number;
}

export interface PlaylistInput {
	title: string;
	description: string;
	coverImage: string | null;
	sortOrder: number;
}

export interface PlaylistItem {
	meta: MusicMeta,
	sortOrder: number, 
}

export interface AudioState {
	muted: boolean;
	volume: number;
	loopMode: LoopMode;
	isShuffle: boolean;
}

export enum LoopMode {
	// 반복 없음
	NONE,
	// 한 곡 한번만 재생
	ONCE_ONE,
	// 한 곡만 반복 루프
	LOOP_ONE,
	// 현재 플레이 리스트를 반복 재생
	LOOP_ALL,
}
