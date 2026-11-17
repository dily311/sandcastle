// 커뮤니티 플러그인 호스트 (CLAUDE.md 6-D-2, ADR-012).
// 코어 플러그인 `community`가 켜져 있을 때만 시작한다: 백엔드가 켜진 플러그인을 도는 목록에 올리고, 화면은
// 플러그인마다 숨은 sandbox iframe 하나를 띄운다(PluginHost.svelte). 꺼지면 iframe·구독을 모두 내린다.
// iframe → postMessage → route(거르기) → call_user_plugin(백엔드가 다시 확인) → 답을 그 iframe에만 돌려준다.
import type { RuntimePlugin } from "$ipc/bindings";
import { call, commands, toAppError } from "$shared/ipc/client";
import { onEvent } from "$shared/ipc/events";
import { logs } from "$shared/stores/log.svelte";
import { toasts } from "$shared/stores/toast.svelte";
import { route } from "./router";

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
	private onMessage = (e: MessageEvent) => void this.handle(e);

	async start() {
		if (this.running) return;
		this.running = true;
		window.addEventListener("message", this.onMessage);
		this.unlisten = await Promise.all([
			// 설치 → 켜기처럼 잇달아 오면 한 번만 다시 시작 (도중의 호출이 "실행 중이 아님"으로 실패하지 않게)
			onEvent("user-plugins-changed", () => {
				clearTimeout(this.reloadTimer);
				this.reloadTimer = setTimeout(() => void this.reload(), 300);
			}),
			onEvent<string>("user-plugin-suspended", (id) => this.suspended(id)),
		]);
		if (!this.running) return this.cleanup(); // 그 사이 꺼짐
		await this.reload();
	}

	async stop() {
		if (!this.running) return;
		this.running = false;
		this.cleanup();
		this.plugins = [];
		this.frames.clear();
		await call(commands.stopUserPlugins(), { source: "plugins", silent: true }).catch(() => {});
	}

	private cleanup() {
		clearTimeout(this.reloadTimer);
		window.removeEventListener("message", this.onMessage);
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
		toasts.error(`플러그인 '${id}'이(가) 오류가 많아 멈췄습니다. 설정에서 다시 켤 수 있습니다.`, "PLUGIN_SUSPENDED");
	}

	private async handle(e: MessageEvent) {
		// 보낸 창으로 플러그인을 정한다 (메시지 안의 값은 믿지 않음)
		let id: string | null = null;
		let frame: HTMLIFrameElement | null = null;
		for (const [pid, el] of this.frames) {
			if (el.contentWindow && el.contentWindow === e.source) {
				id = pid;
				frame = el;
				break;
			}
		}
		if (!id || !frame) return;
		const plugin = this.plugins.find((p) => p.id === id);
		if (!plugin) return;
		const r = route(e.data, this.api, plugin.granted);
		if (!r) return;
		const reply = (msg: Record<string, unknown>) => {
			// 불투명 origin(sandbox)이라 대상 origin은 "*" — 대신 그 iframe의 창에만 보낸다
			if (this.frames.get(id) === frame) frame.contentWindow?.postMessage({ type: "sc-plugin-reply", ...msg }, "*");
		};
		if (!r.ok) {
			if (r.id !== null) reply({ id: r.id, error: { code: r.code, message: r.message } });
			logs.warn(`plugin:${id}`, r.message);
			return;
		}
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
