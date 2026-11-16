// 코어 플러그인 켜짐/꺼짐 (백엔드 sys/plugins.rs의 목록). 화면은 plugins.on("phone")처럼 확인한다.
// 켜고 끄면 기능 설정에 저장되고, 백엔드가 모든 창에 feature_setting::changed를 보낸다.
import type { CorePlugin } from "$ipc/bindings";
import { call, commands } from "$shared/ipc/client";
import { onEvent } from "$shared/ipc/events";

export type PluginId = "phone" | "game" | "chat.images" | "html" | "regex" | "widget" | "translate" | "tts" | "imagegen" | "imageauto";

class Plugins {
	list = $state<CorePlugin[]>([]);
	loaded = $state(false);
	private listening = false;

	async load() {
		this.list = await call(commands.listCorePlugins(), { source: "plugins", silent: true }).catch(() => []);
		this.loaded = true;
		if (!this.listening) {
			this.listening = true;
			void onEvent<{ id: string; enabled: boolean }>("feature_setting::changed", ({ id, enabled }) => this.apply(id, enabled));
		}
	}

	/** 켜져 있는가. 아직 못 불러왔으면 기본값 (목록에 없으면 켜진 것으로) */
	on(id: PluginId): boolean {
		const p = this.list.find((x) => x.id === id);
		return p ? p.enabled : id !== "translate" && id !== "tts" && id !== "imagegen" && id !== "widget" && id !== "html";
	}

	private apply(id: string, enabled: boolean) {
		const i = this.list.findIndex((x) => x.id === id);
		if (i >= 0) this.list[i] = { ...this.list[i], enabled };
	}

	async set(id: PluginId, enabled: boolean) {
		const before = this.on(id);
		this.apply(id, enabled);
		try {
			// 위젯 창도 백엔드(update_feature_setting)가 열고 닫는다
			await call(commands.updateFeatureSetting(id, enabled), { source: "plugins" });
		} catch {
			this.apply(id, before);
		}
	}
}

export const plugins = new Plugins();
