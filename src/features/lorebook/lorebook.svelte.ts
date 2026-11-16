// 로어북(월드인포): 책 목록, 책별 항목, 쓰는 곳. 로어북 화면과 대화 패널이 같이 씀
import { moveItem } from "$shared/dnd/sortable.svelte";
import type { WorldInfoBook_Serialize as Book, WorldInfoEntry as Entry, WorldInfoEntryInput_Deserialize as EntryInput } from "$ipc/bindings";
import { call, commands } from "$shared/ipc/client";
import { JSON_FILES, saveTo } from "$shared/ipc/dialog";
import { confirmAction } from "$shared/stores/confirm.svelte";
import { toasts } from "$shared/stores/toast.svelte";
import { library } from "$features/character/library.svelte";
import { exportPicker, type ExportFormat } from "$app/exportFormats.svelte";

export type BookFormat = "json" | "txt";
export const BOOK_FORMATS: ExportFormat<BookFormat>[] = [
	{ id: "json", label: "SillyTavern 월드인포 (.json)", desc: "SillyTavern이나 다른 앱에서 불러올 수 있음" },
	{ id: "txt", label: "텍스트 (.txt)", desc: "읽기용. 제목·키워드·내용" },
];

export type { Book, Entry, EntryInput };

const src = { source: "lorebook" };
const DRAFT = "draft:";

/** 목록에 보일 항목 이름: 제목, 없으면 키워드 */
export const entryTitle = (e: Pick<Entry, "comment" | "keys">) => e.comment.trim() || e.keys.join(", ") || "(제목 없음)";
export const isDraft = (e: Pick<Entry, "id">) => e.id.startsWith(DRAFT);
/** 키워드 없이 상시도 아니면 저장할 수 없음 (어디서도 걸리지 않음) */
export const isValid = (e: Pick<Entry, "keys" | "constant">) => e.constant || e.keys.length > 0;

class Lorebook {
	books = $state<Book[]>([]);
	counts = $state<Record<string, { entries: number; enabled: number }>>({});
	entries = $state<Record<string, Entry[]>>({});
	loaded = $state(false);

	async load() {
		const [books, counts] = await Promise.all([call(commands.getAllWorldinfoBooks(), src), call(commands.worldinfoEntryCounts(), src)]);
		this.books = books.sort((a, b) => a.name.localeCompare(b.name, "ko"));
		this.counts = Object.fromEntries(counts.map((c) => [c.bookId, { entries: c.entries, enabled: c.enabled }]));
		this.loaded = true;
	}
	book(id: string | null | undefined) {
		return id ? this.books.find((b) => b.id === id) : undefined;
	}

	async loadEntries(bookId: string, force = false) {
		if (this.entries[bookId] && !force) return this.entries[bookId];
		const list = await call(commands.getWorldinfoEntries(bookId), src);
		this.entries[bookId] = sortEntries(list);
		return this.entries[bookId];
	}

	// ---- 책 ----
	async create(name: string, description: string | null = null) {
		const b = await call(commands.createWorldinfoBook({ name, description }), src);
		this.books = [...this.books, b].sort((a, c) => a.name.localeCompare(c.name, "ko"));
		this.entries[b.id] = [];
		this.counts[b.id] = { entries: 0, enabled: 0 };
		return b;
	}
	async update(id: string, patch: { name: string; description: string | null }) {
		await call(commands.updateWorldinfoBook(id, { name: patch.name, description: patch.description }), src);
		const b = this.book(id);
		if (b) Object.assign(b, patch, { updatedAt: Date.now() });
	}
	async remove(b: Book) {
		const n = this.counts[b.id]?.entries ?? 0;
		const ok = await confirmAction({ title: "로어북 삭제", message: `'${b.name}'와 항목 ${n}개를 지움. 이 책을 쓰던 캐릭터·페르소나·대화에서도 빠짐`, confirmLabel: "삭제", danger: true });
		if (!ok) return false;
		await call(commands.deleteWorldinfoBook(b.id), src);
		this.books = this.books.filter((x) => x.id !== b.id);
		delete this.entries[b.id];
		await library.load();
		return true;
	}
	async export(b: Book, format: BookFormat = "json") {
		const filter = format === "json" ? JSON_FILES : { name: "텍스트", extensions: ["txt"] };
		const n = await saveTo(`${b.name}.${format}`, [filter], (path) => call(commands.exportWorldinfoBook(b.id, format, path), src));
		if (n == null) return;
		toasts.success(`항목 ${n}개 내보냄`);
	}
	exportMenu(b: Book) {
		exportPicker.open("로어북 내보내기", "sc.export.lorebook", BOOK_FORMATS, (f: BookFormat) => void this.export(b, f));
	}
	/** SillyTavern 월드인포 JSON 가져오기. 가져온 책을 돌려줌 */
	async importPath(path: string) {
		const r = await call(commands.importSillytavern(path, null, null, null), src);
		await this.load();
		for (const w of r.warnings) toasts.info(w);
		if (r.kind !== "worldInfo") {
			toasts.info("로어북 파일이 아님. 캐릭터·프리셋은 각 화면에서 가져옴");
			await library.load();
			return null;
		}
		toasts.success(`'${r.book.name}' 가져옴 · 항목 ${r.entryCount}개`);
		return this.book(r.book.id) ?? null;
	}

	// ---- 항목 ----
	/** 아직 저장하지 않은 새 항목 (키워드를 넣으면 저장됨) */
	addDraft(bookId: string): Entry {
		const e: Entry = { id: `${DRAFT}${Date.now().toString(36)}`, bookId, keys: [], secondaryKeys: [], content: "", enabled: true, priority: 100, constant: false, comment: "" };
		this.entries[bookId] = [e, ...(this.entries[bookId] ?? [])];
		return e;
	}
	async saveEntry(e: Entry): Promise<Entry> {
		const input: EntryInput = { ...e, id: isDraft(e) ? null : e.id };
		const saved = await call(commands.upsertWorldinfoEntry(input), { ...src, silent: true });
		const list = this.entries[e.bookId] ?? [];
		const i = list.findIndex((x) => x.id === e.id);
		if (i >= 0) list[i] = saved;
		else list.unshift(saved);
		this.recount(e.bookId);
		return saved;
	}
	async duplicate(e: Entry) {
		if (isDraft(e)) return;
		await this.saveEntry({ ...e, id: `${DRAFT}${Date.now().toString(36)}`, comment: e.comment ? `${e.comment} 사본` : "" });
	}
	async removeEntry(e: Entry) {
		if (!isDraft(e)) await call(commands.deleteWorldinfoEntry(e.id), src);
		this.entries[e.bookId] = (this.entries[e.bookId] ?? []).filter((x) => x.id !== e.id);
		this.recount(e.bookId);
	}
	/**
	 * 끌어서 순서 바꾸기: 순서 = 우선순위(높은 것이 위). 옮긴 항목만 새 이웃 사이 값으로 바꾸고,
	 * 사이에 빈 값이 없으면 전부 10 간격으로 다시 매긴다. 바뀐 항목만 저장한다
	 */
	async reorder(bookId: string, from: number, to: number) {
		const list = this.entries[bookId];
		if (!list) return;
		const next = moveItem(list, from, to);
		const changed = reprioritize(next, to);
		this.entries[bookId] = next;
		for (const e of changed) if (!isDraft(e)) await this.saveEntry(e);
	}

	private recount(bookId: string) {
		const list = (this.entries[bookId] ?? []).filter((e) => !isDraft(e));
		this.counts[bookId] = { entries: list.length, enabled: list.filter((e) => e.enabled).length };
	}

	/** 이 책을 쓰는 곳 */
	usage(bookId: string) {
		return {
			characters: library.characters.filter((c) => c.worldInfoIds.includes(bookId)),
			personas: library.personas.filter((p) => p.worldInfoIds.includes(bookId)),
			conversations: library.conversations.filter((c) => c.worldInfoIds.includes(bookId)),
		};
	}
}

/**
 * to 자리로 옮겨진 목록의 우선순위를 순서에 맞춘다 (제자리에서 고친다). 값이 바뀐 항목을 돌려준다.
 * 옮긴 항목만 이웃 사이 값으로, 자리가 없으면 전부 (개수-순번)*10
 */
export function reprioritize(list: Entry[], to: number): Entry[] {
	const prev = list[to - 1];
	const nextE = list[to + 1];
	const cur = list[to];
	let p: number | null = null;
	if (!prev && nextE) p = nextE.priority + 10;
	else if (prev && !nextE) p = prev.priority - 10;
	else if (prev && nextE && prev.priority - nextE.priority >= 2) p = Math.floor((prev.priority + nextE.priority) / 2);
	if (p !== null || !prev) {
		if (p !== null && cur.priority !== p) {
			cur.priority = p;
			return [cur];
		}
		return [];
	}
	const changed: Entry[] = [];
	list.forEach((e, i) => {
		const want = (list.length - i) * 10;
		if (e.priority !== want) {
			e.priority = want;
			changed.push(e);
		}
	});
	return changed;
}

/** 우선순위 높은 것부터, 같으면 제목 순 */
export function sortEntries(list: Entry[]) {
	return [...list].sort((a, b) => b.priority - a.priority || entryTitle(a).localeCompare(entryTitle(b), "ko"));
}

export const lorebook = new Lorebook();
