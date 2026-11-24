// 앱 기본 설정 (Enter로 보내기, 시각 표시, 기본 API·프리셋 등). 여러 화면이 같이 읽는다.
import type { AppSetting_Serialize as AppSetting } from "$ipc/bindings";
import { call, commands } from "$shared/ipc/client";

export type { AppSetting };

class AppSettingStore {
	value = $state<AppSetting | null>(null);
	private loading: Promise<void> | null = null;

	/** 처음 한 번만 읽음 (여러 곳에서 불러도 됨) */
	load() {
		this.loading ??= call(commands.getAppSetting(), { source: "settings", silent: true })
			.then((v) => void (this.value = v))
			.catch(() => void (this.loading = null));
		return this.loading;
	}

	async update(patch: Partial<AppSetting>) {
		if (this.value) this.value = { ...this.value, ...patch };
		this.value = await call(commands.updateAppSetting(patch), { source: "settings" });
	}

	get sendOnEnter() {
		return this.value?.sendOnEnter ?? true;
	}
}

export const appSetting = new AppSettingStore();
