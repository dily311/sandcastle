// 앱 기본 설정 (Enter로 보내기, 언어, 기본 API·프리셋 등). 여러 화면이 같이 읽는다.
import type { AppSetting_Serialize as AppSetting } from "$ipc/bindings";
import { getAppLocale, hasSavedLocaleInStorage, onLocalePersist, setAppLocale } from "$lib/utils/i18n";
import { call, commands } from "$shared/ipc/client";
import { onEvent } from "$shared/ipc/events";

export type { AppSetting };

class AppSettingStore {
	value = $state<AppSetting | null>(null);
	private loading: Promise<void> | null = null;
	private listening = false;

	constructor() {
		onLocalePersist((next) => {
			if (this.value && this.value.language === next) return;
			void this.update({ language: next }).catch(() => {});
		});
	}

	/** 처음 한 번만 읽고 `app_setting::changed` 이벤트와 화면 언어를 동기화한다 */
	load() {
		if (!this.listening) {
			this.listening = true;
			void onEvent<AppSetting>("app_setting::changed", (v) => {
				this.value = v;
				if (v.language && v.language !== getAppLocale()) {
					void setAppLocale(v.language, { persistSetting: false });
				}
			});
		}
		this.loading ??= call(commands.getAppSetting(), { source: "settings", silent: true })
			.then(async (v) => {
				this.value = v;
				const hadLocal = hasSavedLocaleInStorage();
				const cur = getAppLocale();
				if (!hadLocal && v.language && v.language !== cur) {
					await setAppLocale(v.language, { persistSetting: false });
				} else if (v.language !== cur) {
					// 처음 설치(v.language === "")이거나 localStorage(E2E·이전 실행)에 고른 언어가 있으면 백엔드에도 맞춤
					this.value = await call(commands.updateAppSetting({ language: cur }), { source: "settings", silent: true }).catch(() => v);
				}
			})
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
