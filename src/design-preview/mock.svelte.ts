// 미리보기용 가짜 데이터 (실제 화면에서는 바인딩 커맨드로 대체)
export interface MockConv {
	id: string;
	title: string;
	charId: string;
	count: number;
	ago: number; // 분
	pinned: boolean;
	folderId: string | null;
	imported?: boolean;
}
export interface MockFolder {
	id: string;
	charId: string;
	name: string;
}

export const characters = [
	{ id: "sera", name: "Seraphina", intro: "숲의 수호자", convs: 3 },
	{ id: "aria", name: "Aria", intro: "왕국 기사단 견습생", convs: 1 },
	{ id: "narr", name: "나레이터", intro: "모든 이야기의 목소리", convs: 1 },
];

let seq = 10;
export const store = $state({
	convs: [
		{ id: "a", title: "숲에서 깨어남", charId: "sera", count: 12, ago: 2, pinned: true, folderId: "f1" },
		{ id: "b", title: "Branch #1", charId: "sera", count: 5, ago: 300, pinned: false, folderId: "f1" },
		{ id: "d", title: "Seraphina - 2026-03-18@13h32m35s", charId: "sera", count: 12, ago: 9000, pinned: false, folderId: "f2", imported: true },
		{ id: "f", title: "늑대 무리 습격", charId: "sera", count: 64, ago: 4000, pinned: false, folderId: null },
		{ id: "c", title: "성 앞의 약속", charId: "aria", count: 48, ago: 1500, pinned: false, folderId: null },
		{ id: "e", title: "밤의 이야기", charId: "narr", count: 30, ago: 40, pinned: false, folderId: null },
	] as MockConv[],
	folders: [
		{ id: "f1", charId: "sera", name: "본편" },
		{ id: "f2", charId: "sera", name: "가져온 것" },
	] as MockFolder[],
});

export const convActions = {
	rename(id: string, title: string) {
		const c = store.convs.find((c) => c.id === id);
		if (c && title.trim()) c.title = title.trim();
	},
	togglePin(id: string) {
		const c = store.convs.find((c) => c.id === id);
		if (c) c.pinned = !c.pinned;
	},
	duplicate(id: string) {
		const c = store.convs.find((c) => c.id === id);
		if (c) store.convs.push({ ...c, id: `x${seq++}`, title: `${c.title} (복제)`, ago: 0, pinned: false });
	},
	remove(id: string) {
		store.convs = store.convs.filter((c) => c.id !== id);
	},
	move(id: string, folderId: string | null) {
		const c = store.convs.find((c) => c.id === id);
		if (c) c.folderId = folderId;
	},
};

export const folderActions = {
	create(charId: string, name: string): string | null {
		if (!name.trim()) return null;
		const id = `f${seq++}`;
		store.folders.push({ id, charId, name: name.trim() });
		return id;
	},
	rename(id: string, name: string) {
		const f = store.folders.find((f) => f.id === id);
		if (f && name.trim()) f.name = name.trim();
	},
	/** 폴더만 지우고 대화는 폴더 밖으로 */
	remove(id: string) {
		store.folders = store.folders.filter((f) => f.id !== id);
		for (const c of store.convs) if (c.folderId === id) c.folderId = null;
	},
};

export function relative(min: number): string {
	if (min < 1) return "방금";
	if (min < 60) return `${min}분 전`;
	if (min < 60 * 24) return `${Math.floor(min / 60)}시간 전`;
	if (min < 60 * 24 * 30) return `${Math.floor(min / 60 / 24)}일 전`;
	return `${Math.floor(min / 60 / 24 / 30)}달 전`;
}

export function absolute(min: number): string {
	return new Date(Date.now() - min * 60_000).toLocaleString("ko-KR", { dateStyle: "full", timeStyle: "short" });
}
