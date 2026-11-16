// 폰 화면 상태: 폰 주인(페르소나 또는 캐릭터), 연락처, 대화 미리보기·안 읽음, 화면 설정
// - 페르소나 폰: 내가 페르소나, AI가 연락처 캐릭터 (일반 대화와 같은 구조)
// - 캐릭터 폰: 내가 주인 캐릭터, AI가 연락처(다른 캐릭터나 페르소나). 엿보기로 AI가 두 사람 몫을 다 쓸 수도 있음
//   대화 meta.phone = { owner: {kind, id}, contact: {kind, id} }
import { gradientCss, gradientOf } from "$shared/ui/backgrounds";
import { fileSrc } from "$shared/ipc/file";
import { splitHtmlBlocks } from "$shared/html/blocks";
import type { Message_Serialize as Message, PhoneContact } from "$ipc/bindings";
import { call, commands } from "$shared/ipc/client";
import { library, type Conversation } from "$features/character/library.svelte";

export type { PhoneContact };
export type Kind = "persona" | "character";
export interface Who {
	kind: Kind;
	id: string;
}

export const WALLPAPERS: Record<string, string> = {
	sea: "linear-gradient(160deg, #9ad3cf 0%, #4f8fa8 55%, #2c4f6b 100%)",
	dusk: "linear-gradient(160deg, #f6c39a 0%, #c9739a 50%, #4b3d74 100%)",
	sand: "linear-gradient(160deg, #f3e3c6 0%, #d8b98a 55%, #8f7356 100%)",
	night: "linear-gradient(160deg, #3b4a6b 0%, #1e2a44 55%, #0f1424 100%)",
	forest: "linear-gradient(160deg, #b9d9a8 0%, #5f8f5a 55%, #2d4a35 100%)",
};

/** 폰 배경 값 → CSS 배경. 예전 이름(sea 등) / "preset:<id>"(공용 그라데이션) / 올린 그림 경로 */
export function wallpaperCss(value: string | null | undefined): string {
	if (!value) return WALLPAPERS.sea;
	if (WALLPAPERS[value]) return WALLPAPERS[value];
	const g = gradientOf(value);
	if (g) return gradientCss(g);
	const url = fileSrc(value);
	return url ? `center / cover no-repeat url("${url.replace(/"/g, "%22")}")` : WALLPAPERS.sea;
}

interface Prefs {
	owner: Who | null;
	display: "mockup" | "full";
	/** 주인별 배경 ("persona:id" → 배경) */
	wallpapers: Record<string, string>;
	/** 대화별 마지막으로 읽은 시각 */
	lastRead: Record<string, number>;
}
const KEY = "sc.phone";
const src = { source: "phone" };
const ownerKey = (w: Who | null) => (w ? `${w.kind}:${w.id}` : "");

function loadPrefs(): Prefs {
	const base: Prefs = { owner: null, display: "mockup", wallpapers: {}, lastRead: {} };
	try {
		const raw = JSON.parse(localStorage.getItem(KEY) ?? "{}");
		// 예전 저장값: personaId, 페르소나 id로만 된 배경
		if (raw.personaId && !raw.owner) raw.owner = { kind: "persona", id: raw.personaId };
		if (raw.wallpapers) for (const k of Object.keys(raw.wallpapers)) if (!k.includes(":")) raw.wallpapers[`persona:${k}`] = raw.wallpapers[k];
		return { ...base, ...raw };
	} catch {
		return base;
	}
}

/** 페르소나나 캐릭터 하나의 표시 정보 */
export function lookup(w: Who | null | undefined) {
	if (!w) return undefined;
	const x = w.kind === "persona" ? library.persona(w.id) : library.character(w.id);
	if (!x) return undefined;
	return { ...w, name: x.name, avatar: x.avatar, color: x.color || null, description: x.description ?? "" };
}

/** 대화의 meta.phone */
export function phoneMeta(c: Conversation): { owner: Who; contact: Who } | null {
	const p = (c.meta as { phone?: { owner?: Who; contact?: Who } } | null)?.phone;
	return p?.owner && p.contact ? { owner: p.owner, contact: p.contact } : null;
}

export interface Preview {
	text: string;
	at: number;
	role: string;
	unread: number;
}

/** 폰 메시지 한 줄 → 말풍선 여러 개 (줄바꿈마다 하나, <cg>키</cg>는 사진) */
/** 말풍선 한 줄: 글 / 캐릭터 이미지(<cg>, text는 키) / HTML 코드 블록 / 그린 사진(text는 파일 경로) */
export type BubbleLine = { kind: "text" | "image" | "html" | "photo"; text: string };

export function bubbles(content: string): BubbleLine[] {
	// ```html``` 블록은 말풍선으로 쪼개지 않고 통째로 하나 (HtmlView로 그림)
	return splitHtmlBlocks(content).flatMap<BubbleLine>((part) => (part.kind === "html" ? [{ kind: "html", text: part.code }] : textBubbles(part.text)));
}

function textBubbles(content: string): BubbleLine[] {
	return content
		.split(/\n+/)
		.map((l) => l.trim())
		.filter(Boolean)
		.map((l) => {
			const m = /^<cg>\s*([^<]+?)\s*<\/cg>$/i.exec(l);
			return m ? { kind: "image" as const, text: m[1] } : { kind: "text" as const, text: l.replace(/<\/?cg>/gi, "") };
		});
}

/** 엿보기 중 받은 글 → (내 쪽인지, 내용) (백엔드 parse_peek와 같은 규칙) */
export function parsePeek(text: string, me: string, other: string): { mine: boolean; text: string }[] {
	const out: { mine: boolean; text: string }[] = [];
	for (const raw of text.split("\n")) {
		const line = raw.trim().replace(/^[-*•]\s*/, "");
		if (!line) continue;
		const m = /^([^:：]+)[:：]\s*(.*)$/.exec(line);
		const name = m?.[1].trim().replace(/^[*"「]+|[*"」]+$/g, "");
		const mine = name === me ? true : name === other ? false : null;
		const last = out.at(-1);
		if (mine !== null && m) {
			if (last && last.mine === mine) last.text += m[2] ? `\n${m[2]}` : "";
			else if (m[2]) out.push({ mine, text: m[2] });
		} else if (last) last.text += `\n${line}`;
		else out.push({ mine: false, text: line });
	}
	return out;
}

class PhoneState {
	prefs = $state<Prefs>(loadPrefs());
	contacts = $state<PhoneContact[]>([]);
	previews = $state<Record<string, Preview>>({});
	private contactsFor = "";

	private save() {
		try {
			localStorage.setItem(KEY, JSON.stringify(this.prefs));
		} catch {
			/* 저장 못 해도 화면은 동작 */
		}
	}

	/** 폰 주인: 저장한 값 → 시스템 기본 페르소나 → 첫 페르소나 */
	async init() {
		if (!library.loaded) await library.load();
		if (!lookup(this.prefs.owner)) {
			const conv = await call(commands.getConvSettings(), { ...src, silent: true }).catch(() => null);
			const id = library.persona(conv?.defaultPersonaId)?.id ?? library.personas[0]?.id;
			this.prefs.owner = id ? { kind: "persona", id } : null;
			this.save();
		}
		await Promise.all([this.loadContacts(), this.loadPreviews()]);
	}

	get owner() {
		return lookup(this.prefs.owner);
	}
	get isCharacterPhone() {
		return this.prefs.owner?.kind === "character";
	}
	get wallpaper() {
		return this.prefs.wallpapers[ownerKey(this.prefs.owner)] ?? "sea";
	}
	setWallpaper(id: string) {
		if (!this.prefs.owner) return;
		this.prefs.wallpapers[ownerKey(this.prefs.owner)] = id;
		this.save();
	}
	setDisplay(d: Prefs["display"]) {
		this.prefs.display = d;
		this.save();
	}
	async switchOwner(w: Who) {
		this.prefs.owner = w;
		this.save();
		await Promise.all([this.loadContacts(), this.loadPreviews()]);
	}

	/** 이 대화의 상대 (연락처) */
	contactOf(c: Conversation): Who | null {
		const m = phoneMeta(c);
		if (m) return m.contact;
		return c.characterId ? { kind: "character", id: c.characterId } : null;
	}

	/** 이 폰의 대화 (최근 순) */
	get chats(): Conversation[] {
		const o = this.prefs.owner;
		if (!o) return [];
		return library.conversations
			.filter((c) => {
				if (c.mode !== "phone") return false;
				const m = phoneMeta(c);
				const mine = o.kind === "character" ? m?.owner.kind === "character" && m.owner.id === o.id : !m && c.personaId === o.id;
				return mine && !!lookup(this.contactOf(c));
			})
			.sort((a, b) => (this.previews[b.id]?.at ?? b.updatedAt) - (this.previews[a.id]?.at ?? a.updatedAt));
	}
	get unread() {
		return this.chats.reduce((n, c) => n + (this.previews[c.id]?.unread ?? 0), 0);
	}

	// ---- 연락처 ----
	async loadContacts() {
		const o = this.prefs.owner;
		if (!o) return void (this.contacts = []);
		const key = ownerKey(o);
		this.contactsFor = key;
		const list = await call(commands.listPhoneContacts(o.kind, o.id), { ...src, silent: true }).catch(() => []);
		if (this.contactsFor === key) this.contacts = list;
	}
	hasContact(w: Who) {
		return this.contacts.some((c) => c.contactKind === w.kind && c.contactId === w.id);
	}
	async addContact(w: Who) {
		const o = this.prefs.owner;
		if (!o) return;
		const c = await call(commands.addPhoneContact(o.kind, o.id, w.kind, w.id), src);
		if (!this.hasContact(w)) this.contacts = [...this.contacts, c];
	}
	async removeContact(w: Who) {
		const o = this.prefs.owner;
		if (!o) return;
		await call(commands.removePhoneContact(o.kind, o.id, w.kind, w.id), src);
		this.contacts = this.contacts.filter((c) => !(c.contactKind === w.kind && c.contactId === w.id));
	}
	async setStatus(w: Who, status: string) {
		const o = this.prefs.owner;
		if (!o) return;
		await call(commands.setPhoneContactStatus(o.kind, o.id, w.kind, w.id, status), src);
		const c = this.contacts.find((x) => x.contactKind === w.kind && x.contactId === w.id);
		if (c) c.status = status.trim();
	}

	// ---- 대화 ----
	/** 연락처에서 대화방 열기: 없으면 만들고, 연락처에도 넣음 */
	async open(w: Who): Promise<Conversation | null> {
		const o = this.prefs.owner;
		if (!o) return null;
		if (!this.hasContact(w)) await this.addContact(w);
		if (o.kind === "persona") {
			if (w.kind !== "character") return null;
			return library.phoneConversation(o.id, w.id);
		}
		// 캐릭터 폰: 대화는 주인 캐릭터에 붙고, 상대는 meta에 적음
		const found = library.conversations.find((c) => {
			const m = phoneMeta(c);
			return c.mode === "phone" && m?.owner.kind === "character" && m.owner.id === o.id && m.contact.kind === w.kind && m.contact.id === w.id;
		});
		if (found) return found;
		const conv = await call(
			commands.createConversation({
				title: lookup(w)?.name ?? "폰 대화",
				characterId: o.id,
				personaId: library.personas[0]?.id ?? "default",
				worldInfoIds: [],
				mode: "phone",
				meta: { phone: { owner: { kind: o.kind, id: o.id }, contact: { kind: w.kind, id: w.id } } },
			}),
			src,
		);
		library.conversations = [conv, ...library.conversations];
		return conv;
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
		const lastLine = bubbles(last.content).at(-1);
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
