// 휴지통 가짜 응답 (2-A-3). 기존 가짜 지우기 핸들러를 감싸 휴지통 항목을 돌려준다.
// 브라우저 미리보기에서는 되살리기가 목록에서 빼기만 한다(지운 데이터를 다시 넣지는 않음)
import type { TrashItem, TrashKind } from "../bindings";

type Args = Record<string, unknown>;
type Handler = (args: Args) => unknown | Promise<unknown>;

const DAY = 86_400_000;
let seq = 0;
const items: TrashItem[] = [
	{ id: "t-seed1", kind: "conversation", itemId: "old1", label: "비 오는 날의 카페", deletedAt: Date.now() - 2 * 3600_000, rows: 48, files: 0, size: 42_000 },
	{ id: "t-seed2", kind: "character", itemId: "old2", label: "레아", deletedAt: Date.now() - 3 * DAY, rows: 9, files: 1, size: 18_000 },
	{ id: "t-seed3", kind: "prompt_preset", itemId: "old3", label: "짧은 답변", deletedAt: Date.now() - 12 * DAY, rows: 8, files: 0, size: 6_000 },
];

const KINDS: [string, TrashKind, string, string][] = [
	// 커맨드, 종류, id 인자, 이름을 찾을 목록 커맨드
	["delete_character", "character", "id", "get_all_characters"],
	["delete_persona", "persona", "id", "get_all_personas"],
	["delete_conversation", "conversation", "id", "get_all_conversations"],
	["delete_worldinfo_book", "worldinfo_book", "id", "get_all_worldinfo_books"],
	["delete_prompt_preset", "prompt_preset", "id", "list_prompt_presets"],
	["delete_game", "game", "gameId", "game_list"],
	["delete_sessions", "game_session", "sessionId", ""],
];

function push(kind: TrashKind, itemId: string, label: string): TrashItem {
	const t: TrashItem = { id: `t-${++seq}`, kind, itemId, label, deletedAt: Date.now(), rows: 1, files: 0, size: 1_000 };
	items.unshift(t);
	return t;
}

export function withTrash(h: Record<string, Handler>): Record<string, Handler> {
	const out = { ...h };
	for (const [cmd, kind, arg, listCmd] of KINDS) {
		const orig = h[cmd];
		if (!orig) continue;
		out[cmd] = async (args) => {
			const id = String(args[arg]);
			const list = listCmd && h[listCmd] ? ((await h[listCmd]({})) as { id: string; name?: string; title?: string | null }[]) : [];
			const found = list.find((x) => x.id === id);
			await orig(args);
			return push(kind, id, found?.name ?? found?.title ?? (kind === "game_session" ? "플레이 기록" : id));
		};
	}
	const bulk = h.delete_conversations;
	if (bulk)
		out.delete_conversations = async (args) => {
			const n = (await bulk(args)) as number;
			for (const id of args.ids as string[]) push("conversation", id, id);
			return n;
		};
	out.list_trash = () => items.map((t) => ({ ...t }));
	out.restore_trash_item = ({ id }) => {
		const i = items.findIndex((t) => t.id === id);
		if (i < 0) throw { code: "RESOURCE_NOT_FOUND", category: "NOT_FOUND", message: "휴지통에 없는 항목", retryable: false };
		return items.splice(i, 1)[0];
	};
	out.purge_trash_item = ({ id }) => {
		const i = items.findIndex((t) => t.id === id);
		if (i >= 0) items.splice(i, 1);
		return i >= 0;
	};
	out.empty_trash = () => items.splice(0).length;
	return out;
}
