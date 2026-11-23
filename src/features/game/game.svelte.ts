// 게임 목록·열린 게임·세션. 플레이·에디터는 예전 화면을 새 틀 안에서 그대로 쓴다 (gameStore 등과 같은 데이터를 공유)
import { jobs } from "$shared/stores/jobs.svelte";
import { trashedToast } from "$shared/trash";
import type {
	GamePlayerSessions_Serialize as Session,
	GameProject_Serialize as GameProject,
	Issue,
	OpenGameResponse_Serialize as OpenGame,
	Template_Serialize as Template,
} from "$ipc/bindings";
import { call, commands } from "$shared/ipc/client";
import { gameStore } from "$features/game/runtime/gameStore.svelte";

export type { GameProject, Session, Template, Issue };
const src = { source: "game" };

/** 게임 폴더 파일 주소 (open_game의 assetBaseUrl과 같은 규칙) */
export function gameAssetUrl(gameId: string, path: string | null | undefined) {
	if (!path) return null;
	if (/^(https?:|data:|blob:)/.test(path)) return path;
	const win = /Windows|Android/i.test(navigator.userAgent);
	const base = win ? `http://game.localhost/${gameId}/` : `game://localhost/${gameId}/`;
	return base + path.split("/").map(encodeURIComponent).join("/");
}

class Games {
	list = $state<GameProject[]>([]);
	templates = $state<Template[]>([]);
	loaded = $state(false);
	/** 지금 열린 게임 (세션 목록·플레이·에디터가 이 게임 기준) */
	opened = $state<OpenGame | null>(null);
	sessions = $state<Session[]>([]);

	async load() {
		const [list, templates] = await Promise.all([
			call(commands.gameList(), src).catch(() => [] as GameProject[]),
			call(commands.listTemplates(), { ...src, silent: true }).catch(() => [] as Template[]),
		]);
		this.list = list.sort((a, b) => b.updatedAt - a.updatedAt);
		this.templates = templates;
		this.loaded = true;
	}
	game(id: string | null | undefined) {
		return this.list.find((g) => g.id === id);
	}
	template(id: string) {
		return this.templates.find((t) => t.id === id);
	}

	/** 게임 열기 (백엔드의 "지금 게임"을 바꿈). 세션 목록도 다시 읽음 */
	async open(gameId: string) {
		if (this.opened?.project.id !== gameId) {
			const g = await call(commands.openGame(gameId), src);
			this.opened = g;
			gameStore.setBundle(g as never);
		}
		this.sessions = await call(commands.listSessions(), { ...src, silent: true }).catch(() => []);
		this.sessions.sort((a, b) => b.updatedAt - a.updatedAt);
		return this.opened!;
	}

	async create(title: string, templateId: string) {
		const g = await call(commands.createGame(title, templateId), src);
		this.list = [g, ...this.list];
		return g;
	}
	async import(path: string) {
		const g = await jobs.run(path.split(/[\\/]/).pop() ?? path, () => call(commands.importGame(path), src), { group: "게임 가져오기" });
		if (!g) return null;
		this.list = [g, ...this.list.filter((x) => x.id !== g.id)];
		return g;
	}
	/** dir를 주지 않으면 다운로드 폴더 (데스크톱) */
	async export(gameId: string, dir: string | null = null) {
		return call(commands.exportGame(gameId, dir), src);
	}
	/** 휴지통으로 (2-A-3). 여러 개를 한 번에 지울 때는 toast=false */
	async remove(gameId: string, toast = true) {
		const title = this.list.find((g) => g.id === gameId)?.title ?? "게임";
		const item = await jobs.run(
			title,
			async () => {
				if (this.opened?.project.id === gameId) {
					await call(commands.closeGame(), { ...src, silent: true }).catch(() => {});
					this.opened = null;
				}
				return call(commands.deleteGame(gameId), src);
			},
			{ group: "게임 지우기" },
		);
		if (!item) return;
		this.list = this.list.filter((g) => g.id !== gameId);
		if (toast) trashedToast(item, () => this.load());
	}
	async removeSession(id: string) {
		const item = await jobs.run("플레이 기록", () => call(commands.deleteSessions(id), src), { group: "플레이 기록 지우기" });
		if (!item) return;
		const gameId = this.opened?.project.id;
		this.sessions = this.sessions.filter((s) => s.id !== id);
		trashedToast(item, async () => {
			if (gameId && this.opened?.project.id === gameId) {
				this.sessions = await call(commands.listSessions(), { ...src, silent: true }).catch(() => []);
				this.sessions.sort((a, b) => b.updatedAt - a.updatedAt);
			}
		});
	}
	async validate() {
		return call(commands.validateGame(), src);
	}
}

export const games = new Games();
