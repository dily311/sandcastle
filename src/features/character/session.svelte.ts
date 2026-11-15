// 열린 대화 하나: 메시지 목록, 보내기·다시 생성·멈추기(스트리밍), 스와이프, 수정·숨김·챕터·삭제
// 메시지 저장은 백엔드(chat_send / chat_regenerate)가 하고, 여기서는 결과만 반영한다.
import { Channel } from "@tauri-apps/api/core";
import type { Message_Serialize, StreamPayload } from "$ipc/bindings";
import { call, commands } from "$shared/ipc/client";
import { onEvent } from "$shared/ipc/events";
import { library } from "./library.svelte";
import { afterReply } from "$shared/stores/aiTools.svelte";

export type Message = Message_Serialize;

export interface Swipe {
	content: string;
	meta?: Record<string, unknown>;
	createdAt?: number;
	updatedAt?: number;
}
export interface MessageMeta {
	sender?: { type?: string; name?: string; id?: string };
	info?: { model?: string; apiProfileId?: string; presetId?: string; genStarted?: number; genFinished?: number; cancelled?: boolean };
	images?: { key: string; imageId: string; path: string }[];
	swipes?: Swipe[];
	swipeId?: number;
	hidden?: boolean;
	/** 이 메시지부터 새 챕터 (챕터 제목) */
	chapter?: string;
	/** 붙인 이미지 (파일 경로) */
	attachments?: string[];
	[k: string]: unknown;
}

export function metaOf(m: Message): MessageMeta {
	return (m.meta && typeof m.meta === "object" ? m.meta : {}) as MessageMeta;
}

const src = { source: "chat" };

// 열려 있는 대화들. 이 화면이 기다리지 않은 응답(앱 재시작 후 다시 만든 응답 등)이 저장되면 새로 불러온다.
const open = new Set<WeakRef<ChatSession>>();
let listening = false;
function watchDone() {
	if (listening) return;
	listening = true;
	void onEvent<{ conversationId: string }>("chat:dialogue:done", ({ conversationId }) => {
		for (const ref of open) {
			const s = ref.deref();
			if (!s) open.delete(ref);
			else if (s.convId === conversationId && !s.busy) void s.load().catch(() => {});
		}
		void library.refresh(conversationId);
	});
}

/** 진행 중인 응답 */
interface Live {
	/** send: 새 응답 / regenerate: 기존 메시지 다시 만들기 / peek: 폰 엿보기 (AI가 두 사람 몫을 모두 씀) */
	kind: "send" | "regenerate" | "peek";
	text: string;
	/** 다시 만드는 메시지 */
	targetId?: string;
	/** 저장 전 보여줄 내 입력 */
	userText?: string;
	/** 저장 전 보여줄 첨부 이미지 (미리보기 주소) */
	userImages?: string[];
}

export class ChatSession {
	messages = $state<Message[]>([]);
	loading = $state(true);
	live = $state<Live | null>(null);
	/** 화면용 글 (표시용 정규식을 거치고 {{char}}·{{user}}를 이름으로). 없으면 원문을 보임 */
	display = $state<Record<string, string>>({});
	private displaySig = "";

	constructor(readonly convId: string) {
		open.add(new WeakRef(this));
		watchDone();
	}

	/** 메시지가 바뀌면 화면용 글을 다시 받음 (화면의 $effect에서 부름) */
	async refreshDisplay(key = "") {
		const list = this.messages;
		const sig = key + "#" + list.map((m) => `${m.id}:${m.updatedAt}:${m.content.length}`).join("|");
		if (sig === this.displaySig) return;
		this.displaySig = sig;
		const out = await call(commands.displayMessages(this.convId, list.map((m) => ({ role: m.role, content: m.content }))), { ...src, silent: true }).catch(() => null);
		if (!out || sig !== this.displaySig) return;
		this.display = Object.fromEntries(list.map((m, i) => [m.id, out[i]]));
	}

	get busy() {
		return this.live !== null;
	}
	get last() {
		return this.messages.at(-1);
	}

	async load() {
		this.loading = true;
		try {
			this.messages = await call(commands.getMessages(this.convId, 2000, null), src);
		} finally {
			this.loading = false;
		}
	}

	private channel() {
		const ch = new Channel<StreamPayload>();
		ch.onmessage = (p) => {
			if (this.live) this.live.text += p.chunk;
		};
		return ch;
	}

	/** 백엔드가 고친 메시지를 반영 (번역·그림 등 플러그인 결과) */
	update(m: Message) {
		this.replace(m);
	}

	private replace(m: Message) {
		const i = this.messages.findIndex((x) => x.id === m.id);
		if (i >= 0) this.messages[i] = m;
		else this.messages.push(m);
	}

	/** 보내기. 빈 입력(첨부도 없음)이면 저장 없이 이어서 응답. attachments: 디스크에 저장된 이미지 경로 */
	async send(text: string, attachments: string[] = [], previews: string[] = []) {
		if (this.busy) return;
		this.live = { kind: "send", text: "", userText: text.trim() || undefined, userImages: previews.length ? previews : undefined };
		try {
			const turn = await call(commands.chatSend(this.convId, text, attachments, this.channel()), src);
			if (turn.user) this.messages.push(turn.user);
			this.messages.push(turn.assistant);
			void afterReply(turn.assistant).then((m) => m !== turn.assistant && this.replace(m));
		} catch {
			// 입력은 응답 실패와 상관없이 저장돼 있을 수 있다
			await this.load().catch(() => {});
		} finally {
			this.live = null;
			void library.refresh(this.convId);
		}
	}

	/** 마지막 AI 메시지 다시 만들기 (swipe: 스와이프로 쌓기, 아니면 스와이프 버리고 교체) */
	async regenerate(swipe: boolean) {
		const last = this.last;
		if (this.busy || !last || last.role !== "assistant") return;
		this.live = { kind: "regenerate", text: "", targetId: last.id };
		try {
			const turn = await call(commands.chatRegenerate(this.convId, swipe, this.channel()), src);
			this.replace(turn.assistant);
			void afterReply(turn.assistant).then((m) => m !== turn.assistant && this.replace(m));
		} catch {
			/* 토스트로 알림 */
		} finally {
			this.live = null;
			void library.refresh(this.convId);
		}
	}

	/** 폰 엿보기: 두 사람 메시지를 AI가 이어서 씀 (여러 메시지가 한 번에 저장됨) */
	async peek() {
		if (this.busy) return;
		this.live = { kind: "peek", text: "" };
		try {
			const saved = await call(commands.phonePeek(this.convId, this.channel()), src);
			this.messages.push(...saved);
		} catch {
			// 멈췄을 때 받은 데까지 저장됐을 수 있음
			await this.load().catch(() => {});
		} finally {
			this.live = null;
			void library.refresh(this.convId);
		}
	}

	cancel() {
		void call(commands.chatCancel(this.convId), { ...src, silent: true }).catch(() => {});
	}

	private async save(m: Message, content: string, meta: MessageMeta) {
		await call(commands.updateMessage(m.id, content, meta as never), src);
		this.replace({ ...m, content, meta: meta as never, updatedAt: Date.now() });
	}

	/** 스와이프 이동. 마지막 메시지에서 끝을 넘기면 새로 만든다 */
	async swipe(m: Message, dir: -1 | 1) {
		const meta = metaOf(m);
		const swipes = meta.swipes ?? [];
		const cur = meta.swipeId ?? 0;
		const next = cur + dir;
		if (next < 0) return;
		if (next >= swipes.length) {
			if (m.id === this.last?.id) await this.regenerate(true);
			return;
		}
		const s = swipes[next];
		await this.save(m, s.content, { ...meta, ...(s.meta ?? {}), swipes, swipeId: next });
	}

	async edit(m: Message, content: string) {
		if (content === m.content) return;
		const meta = metaOf(m);
		if (meta.swipes?.length) {
			const swipes = meta.swipes.map((s, i) => (i === (meta.swipeId ?? 0) ? { ...s, content, updatedAt: Date.now() } : s));
			await this.save(m, content, { ...meta, swipes });
		} else {
			await this.save(m, content, meta);
		}
	}

	async setHidden(m: Message, hidden: boolean) {
		const meta = { ...metaOf(m) };
		if (hidden) meta.hidden = true;
		else delete meta.hidden;
		await this.save(m, m.content, meta);
	}

	/** 챕터 시작 표시 (null이면 지움) */
	async setChapter(m: Message, title: string | null) {
		const meta = { ...metaOf(m) };
		if (title?.trim()) meta.chapter = title.trim();
		else delete meta.chapter;
		await this.save(m, m.content, meta);
	}

	/** 여러 메시지 한 번에 지우기 */
	async removeMany(ids: string[]) {
		if (!ids.length) return 0;
		const n = await call(commands.deleteMessages(ids), src);
		const gone = new Set(ids);
		this.messages = this.messages.filter((x) => !gone.has(x.id));
		void library.refresh(this.convId);
		return n;
	}

	async remove(m: Message) {
		await call(commands.deleteMessage(m.id), src);
		this.messages = this.messages.filter((x) => x.id !== m.id);
		void library.refresh(this.convId);
	}

	async sentPrompt(m: Message) {
		return call(commands.getMessagePrompt(m.id), src);
	}
}
