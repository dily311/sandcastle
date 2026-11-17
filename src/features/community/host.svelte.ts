// 커뮤니티 플러그인 호스트 (CLAUDE.md 6-D-2, ADR-012).
// 코어 플러그인 `community`가 켜져 있을 때만 시작한다: 백엔드가 켜진 플러그인을 도는 목록에 올리고, 화면은
// 플러그인마다 숨은 sandbox iframe 하나를 띄운다(PluginHost.svelte). 꺼지면 iframe·구독을 모두 내린다.
// iframe → postMessage → route(거르기) → call_user_plugin(백엔드가 다시 확인) → 답을 그 iframe에만 돌려준다.
import type { RuntimePlugin } from "$ipc/bindings";
import { call, commands, toAppError } from "$shared/ipc/client";
import { onEvent } from "$shared/ipc/events";
import { logs } from "$shared/stores/log.svelte";
import { toasts } from "$shared/stores/toast.svelte";
import { addReplyHook } from "$shared/stores/aiTools.svelte";
import { postProcess } from "./postprocess";
import { gather } from "./promptHook";
import type { PromptHookRequest } from "$ipc/bindings";
import { route } from "./router";
import { Puzzle } from "@lucide/svelte";
import { panel } from "$app/panel.svelte";
import { nav } from "$app/nav.svelte";
import { isPluginTab, panelTabId } from "./panels";

type Unlisten = () => void;

class PluginHost {
	/** 띄울 플러그인. 다시 시작할 때마다 gen이 올라 iframe을 새로 만든다(새 코드) */
	plugins = $state<RuntimePlugin[]>([]);
	gen = $state(0);
	private api: Record<string, string | null> = {};
	private frames = new Map<string, HTMLIFrameElement>();
	private unlisten: Unlisten[] = [];
	private running = false;
	private reloadTimer: ReturnType<typeof setTimeout> | undefined;
	private eventSeq = 0;
	private waiting = new Map<string, { resolve: (v: unknown) => void; reject: (e: Error) => void; timer: ReturnType<typeof setTimeout> }>();
	private onMessage = (e: MessageEvent) => void this.handle(e);
	/** 패널 iframe → 플러그인 (6-D-6). 패널도 같은 권한으로 API를 부른다 */
	private panelFrames = new Map<HTMLIFrameElement, string>();
	/** 사용자가 그 플러그인을 마지막으로 직접 쓴 때 (명령을 누름·패널을 누름) — 6-D-7의 실행 주체 판단에 쓴다 */
	readonly userActedAt = new Map<string, number>();
	private onBlur = () =>
		setTimeout(() => {
			const el = document.activeElement;
			if (el instanceof HTMLIFrameElement && this.panelFrames.has(el)) this.userActedAt.set(this.panelFrames.get(el)!, Date.now());
		}, 0);

	async start() {
		if (this.running) return;
		this.running = true;
		window.addEventListener("message", this.onMessage);
		window.addEventListener("blur", this.onBlur);
		// 새 AI 응답의 후처리 (6-D-4)
		const unhook = addReplyHook((m) => postProcess(m, this.plugins, (id, ev, data) => this.emit(id, ev, data)));
		this.unlisten = await Promise.all([
			// 설치 → 켜기처럼 잇달아 오면 한 번만 다시 시작 (도중의 호출이 "실행 중이 아님"으로 실패하지 않게)
			onEvent("user-plugins-changed", () => {
				clearTimeout(this.reloadTimer);
				this.reloadTimer = setTimeout(() => void this.reload(), 300);
			}),
			onEvent<string>("user-plugin-suspended", (id) => this.suspended(id)),
			// 보내기 직전 프롬프트 훅 (6-D-5): 늦거나 실패해도 답은 꼭 보낸다(백엔드가 덜 기다리게)
			onEvent<PromptHookRequest>("user-plugin-prompt", (req) => void this.answerPrompt(req)),
		]);
		this.unlisten.push(unhook);
		if (!this.running) return this.cleanup(); // 그 사이 꺼짐
		await this.reload();
	}

	async stop() {
		if (!this.running) return;
		this.running = false;
		this.cleanup();
		this.plugins = [];
		this.frames.clear();
		this.panelFrames.clear();
		this.syncPanels();
		await call(commands.stopUserPlugins(), { source: "plugins", silent: true }).catch(() => {});
	}

	private cleanup() {
		clearTimeout(this.reloadTimer);
		window.removeEventListener("message", this.onMessage);
		window.removeEventListener("blur", this.onBlur);
		for (const u of this.unlisten) u();
		this.unlisten = [];
	}

	/** 켜진 플러그인 목록을 다시 받아 iframe을 새로 띄운다 */
	async reload() {
		if (!this.running) return;
		try {
			const r = await call(commands.startUserPlugins(), { source: "plugins", silent: true });
			if (!this.running) return;
			this.api = Object.fromEntries(Object.entries(r.api).map(([k, v]) => [k, v ?? null]));
			this.frames.clear();
			this.plugins = r.plugins;
			this.gen++;
		} catch {
			this.plugins = [];
		}
		this.syncPanels();
	}

	/** 오른쪽 패널에 플러그인 탭을 올린다 (도는 플러그인의 것만) */
	private syncPanels() {
		panel.plugins = this.plugins.flatMap((p) => p.panels.map((x) => ({ id: panelTabId(p.id, x.id), label: `${x.title} · ${p.name}`, icon: Puzzle })));
		// 보고 있던 플러그인 탭이 사라지면 화면의 첫 탭으로
		if (isPluginTab(panel.tab) && !panel.plugins.some((t) => t.id === panel.tab)) panel.tab = panel.tabs[0]?.id ?? "";
	}

	attachPanel(pluginId: string, el: HTMLIFrameElement) {
		this.panelFrames.set(el, pluginId);
	}
	detachPanel(el: HTMLIFrameElement) {
		this.panelFrames.delete(el);
	}

	/** 지금 화면 (플러그인의 app.context). id만 준다 */
	private context() {
		const r = nav.current as { page: string; convId?: string; charId?: string; gameId?: string };
		return { page: r.page, conversationId: r.convId ?? null, characterId: r.charId ?? null };
	}

	/** 명령 실행 — 사용자가 메뉴에서 누를 때만 부른다 */
	async runCommand(pluginId: string, commandId: string) {
		this.userActedAt.set(pluginId, Date.now());
		try {
			await this.emit(pluginId, "command", { id: commandId, context: this.context() }, 10_000);
		} catch (e) {
			toasts.error(`플러그인 명령 실패: ${e instanceof Error ? e.message : String(e)}`, "PLUGIN_COMMAND");
		}
	}

	/** PluginHost.svelte가 iframe을 만들고 지울 때 */
	attach(id: string, el: HTMLIFrameElement | null) {
		if (el) this.frames.set(id, el);
		else this.frames.delete(id);
	}

	private suspended(id: string) {
		if (!this.plugins.some((p) => p.id === id)) return;
		this.plugins = this.plugins.filter((p) => p.id !== id);
		this.frames.delete(id);
		this.syncPanels();
		toasts.error(`플러그인 '${id}'이(가) 오류가 많아 멈췄습니다. 설정에서 다시 켤 수 있습니다.`, "PLUGIN_SUSPENDED");
	}

	private async answerPrompt(req: PromptHookRequest) {
		const { answers, errors } = await gather(
			req,
			this.plugins.map((p) => p.id),
			(id, ev, data, ms) => this.emit(id, ev, data, ms),
		);
		for (const e of errors) logs.warn(`plugin:${e.pluginId}`, `프롬프트 훅 실패: ${e.message}`);
		await commands.answerPluginPrompt(req.requestId, answers).catch(() => {});
	}

	/** 도는 플러그인 (설치 순서) */
	get active(): readonly RuntimePlugin[] {
		return this.plugins;
	}

	/** 플러그인 하나에 이벤트를 보내고 답을 기다린다 (시간 초과·멈춤이면 거절). 핸들러가 없으면 null */
	emit(pluginId: string, event: string, data: unknown, timeoutMs = 5000): Promise<unknown> {
		const frame = this.frames.get(pluginId);
		if (!frame?.contentWindow) return Promise.reject(new Error(`플러그인이 실행 중이 아님: ${pluginId}`));
		const id = ++this.eventSeq;
		const key = `${pluginId}:${id}`;
		return new Promise((resolve, reject) => {
			const timer = setTimeout(() => {
				this.waiting.delete(key);
				reject(new Error(`응답 없음 (${timeoutMs / 1000}초): ${event}`));
			}, timeoutMs);
			this.waiting.set(key, { resolve, reject, timer });
			frame.contentWindow!.postMessage({ type: "sc-plugin-event", id, event, data }, "*");
		});
	}

	private async handle(e: MessageEvent) {
		// 보낸 창으로 플러그인을 정한다 (메시지 안의 값은 믿지 않음)
		let id: string | null = null;
		let frame: HTMLIFrameElement | null = null;
		let isPanel = false;
		for (const [pid, el] of this.frames) {
			if (el.contentWindow && el.contentWindow === e.source) {
				id = pid;
				frame = el;
				break;
			}
		}
		if (!id) {
			for (const [el, pid] of this.panelFrames) {
				if (el.contentWindow && el.contentWindow === e.source) {
					id = pid;
					frame = el;
					isPanel = true;
					break;
				}
			}
		}
		if (!id || !frame) return;
		const plugin = this.plugins.find((p) => p.id === id);
		if (!plugin) return;
		// 호스트가 보낸 이벤트(후처리 등)의 답
		const d = e.data as { type?: unknown; id?: unknown; ok?: unknown; error?: { message?: unknown } } | null;
		if (d && d.type === "sc-plugin-event-reply" && typeof d.id === "number" && !isPanel) {
			const key = `${id}:${d.id}`;
			const w = this.waiting.get(key);
			if (w) {
				this.waiting.delete(key);
				clearTimeout(w.timer);
				if (d.error) w.reject(new Error(String(d.error.message ?? "오류")));
				else w.resolve(d.ok ?? null);
			}
			return;
		}
		const r = route(e.data, this.api, plugin.granted);
		if (!r) return;
		const reply = (msg: Record<string, unknown>) => {
			// 불투명 origin(sandbox)이라 대상 origin은 "*" — 대신 그 iframe의 창에만 보낸다
			const alive = isPanel ? this.panelFrames.has(frame) : this.frames.get(id) === frame;
			if (alive) frame.contentWindow?.postMessage({ type: "sc-plugin-reply", ...msg }, "*");
		};
		if (!r.ok) {
			if (r.id !== null) reply({ id: r.id, error: { code: r.code, message: r.message } });
			logs.warn(`plugin:${id}`, r.message);
			return;
		}
		// 화면 상태는 백엔드가 모르므로 호스트가 답한다 (권한 확인은 위의 route에서 끝남)
		if (r.call.method === "app.context") return reply({ id: r.call.id, ok: this.context() });
		try {
			const ok = await commands.callUserPlugin(id, r.call.method, r.call.args as never);
			reply({ id: r.call.id, ok });
		} catch (err) {
			const ae = toAppError(err);
			reply({ id: r.call.id, error: { code: ae.code, message: ae.message } });
		}
	}
}

export const pluginHost = new PluginHost();
