// 폰 화면 상태: 폰 주인(페르소나), 연락처, 대화 미리보기·안 읽음, 화면 설정
import type { Message_Serialize as Message, PhoneContact } from "$ipc/bindings";
import { call, commands } from "$shared/ipc/client";
import { library, type Conversation } from "$features/character/library.svelte";

export type { PhoneContact };

export const WALLPAPERS: Record<string, string> = {
	sea: "linear-gradient(160deg, #9ad3cf 0%, #4f8fa8 55%, #2c4f6b 100%)",
	dusk: "linear-gradient(160deg, #f6c39a 0%, #c9739a 50%, #4b3d74 100%)",
	sand: "linear-gradient(160deg, #f3e3c6 0%, #d8b98a 55%, #8f7356 100%)",
	night: "linear-gradient(160deg, #3b4a6b 0%, #1e2a44 55%, #0f1424 100%)",
	forest: "linear-gradient(160deg, #b9d9a8 0%, #5f8f5a 55%, #2d4a35 100%)",
};

interface Prefs {
	personaId: string | null;
	display: "mockup" | "full";
	/** 페르소나별 배경 */
	wallpapers: Record<string, string>;
	/** 대화별 마지막으로 읽은 시각 */
	lastRead: Record<string, number>;
}
const KEY = "sc.phone";
const src = { source: "phone" };

function loadPrefs(): Prefs {
	const base: Prefs = { personaId: null, display: "mockup", wallpapers: {}, lastRead: {} };
	try {
		return { ...base, ...JSON.parse(localStorage.getItem(KEY) ?? "{}") };
	} catch {
		return base;
	}
}

export interface Preview {
	text: string;
	at: number;
	role: string;
	unread: number;
}

/** 폰 메시지 한 줄 → 말풍선 여러 개 (줄바꿈마다 하나, <cg>키</cg>는 사진) */
export function bubbles(content: string): { kind: "text" | "image"; text: string }[] {
	return content
		.split(/\n+/)
		.map((l) => l.trim())
		.filter(Boolean)
		.map((l) => {
			const m = /^<cg>\s*([^<]+?)\s*<\/cg>$/i.exec(l);
			return m ? { kind: "image" as const, text: m[1] } : { kind: "text" as const, text: l.replace(/<\/?cg>/gi, "") };
		});
}

class PhoneState {
	prefs = $state<Prefs>(loadPrefs());
	contacts = $state<PhoneContact[]>([]);
	previews = $state<Record<string, Preview>>({});
	private contactsFor: string | null = null;

	private save() {
		try {
			localStorage.setItem(KEY, JSON.stringify(this.prefs));
		} catch {
			/* 저장 못 해도 화면은 동작 */
		}
	}

	/** 폰 주인: 저장한 값 → 시스템 기본 → 첫 페르소나 */
	async init() {
		if (!library.loaded) await library.load();
		if (!this.prefs.personaId || !library.persona(this.prefs.personaId)) {
			const conv = await call(commands.getConvSettings(), { ...src, silent: true }).catch(() => null);
			this.prefs.personaId = library.persona(conv?.defaultPersonaId)?.id ?? library.personas[0]?.id ?? null;
			this.save();
		}
		await this.loadContacts();
		await this.loadPreviews();
	}

	get persona() {
		return library.persona(this.prefs.personaId);
	}
	get wallpaper() {
		return this.prefs.wallpapers[this.prefs.personaId ?? ""] ?? "sea";
	}
	setWallpaper(id: string) {
		if (!this.prefs.personaId) return;
		this.prefs.wallpapers[this.prefs.personaId] = id;
		this.save();
	}
	setDisplay(d: Prefs["display"]) {
		this.prefs.display = d;
		this.save();
	}
	async switchPersona(id: string) {
		this.prefs.personaId = id;
		this.save();
		await Promise.all([this.loadContacts(), this.loadPreviews()]);
	}

	/** 이 폰의 대화 (최근 순) */
	get chats(): Conversation[] {
		return library.conversations
			.filter((c) => c.mode === "phone" && c.personaId === this.prefs.personaId && library.character(c.characterId))
			.sort((a, b) => (this.previews[b.id]?.at ?? b.updatedAt) - (this.previews[a.id]?.at ?? a.updatedAt));
	}
	get unread() {
		return this.chats.reduce((n, c) => n + (this.previews[c.id]?.unread ?? 0), 0);
	}

	// ---- 연락처 ----
	async loadContacts() {
		const id = this.prefs.personaId;
		if (!id) return void (this.contacts = []);
		this.contactsFor = id;
		const list = await call(commands.listPhoneContacts(id), { ...src, silent: true }).catch(() => []);
		if (this.contactsFor === id) this.contacts = list;
	}
	async addContact(charId: string) {
		if (!this.prefs.personaId) return;
		const c = await call(commands.addPhoneContact(this.prefs.personaId, charId), src);
		if (!this.contacts.some((x) => x.characterId === charId)) this.contacts = [...this.contacts, c];
	}
	async removeContact(charId: string) {
		if (!this.prefs.personaId) return;
		await call(commands.removePhoneContact(this.prefs.personaId, charId), src);
		this.contacts = this.contacts.filter((c) => c.characterId !== charId);
	}
	async setStatus(charId: string, status: string) {
		if (!this.prefs.personaId) return;
		await call(commands.setPhoneContactStatus(this.prefs.personaId, charId, status), src);
		const c = this.contacts.find((x) => x.characterId === charId);
		if (c) c.status = status.trim();
	}

	// ---- 대화 ----
	/** 연락처에서 대화방 열기: 없으면 만들고, 연락처에도 넣음 */
	async open(charId: string): Promise<Conversation | null> {
		const pid = this.prefs.personaId;
		if (!pid) return null;
		if (!this.contacts.some((c) => c.characterId === charId)) await this.addContact(charId);
		return library.phoneConversation(pid, charId);
	}

	async loadPreviews() {
		const chats = this.chats;
		const entries = await Promise.all(
			chats.map(async (c) => [c.id, await call(commands.getMessages(c.id, 30, null), { ...src, silent: true }).catch(() => [] as Message[])] as const),
		);
		for (const [id, msgs] of entries) this.setPreview(id, msgs);
	}
	setPreview(convId: string, msgs: Message[]) {
		const last = msgs.at(-1);
		if (!last) {
			delete this.previews[convId];
			return;
		}
		const read = this.prefs.lastRead[convId] ?? 0;
		const lines = bubbles(last.content);
		const lastLine = lines.at(-1);
		this.previews[convId] = {
			text: lastLine ? (lastLine.kind === "image" ? "사진" : lastLine.text) : "",
			at: last.createdAt,
			role: last.role,
			unread: msgs.filter((m) => m.role === "assistant" && m.createdAt > read).reduce((n, m) => n + Math.max(1, bubbles(m.content).length), 0),
		};
	}
	markRead(convId: string) {
		this.prefs.lastRead[convId] = Date.now();
		this.save();
		const p = this.previews[convId];
		if (p) p.unread = 0;
	}
}

export const phone = new PhoneState();
