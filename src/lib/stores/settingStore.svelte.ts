import type { AppSettings } from "$/lib/type/settingTypes";
import { settingCommands } from "$/ipc/commands";

export const defaultSetting: AppSettings = {
	autoUpdate: true,
	language: "ko",
	sendOnEnter: false,
	showTimestamps: true,
	selectedPreset: "default",
	selectedApiProfile: null,
	selectedCharacterId: null,
	selectedPersonaId: "default",
	lastPlayedGameId: null,
};

class AppSettingStore {
	setting = $state<AppSettings>(defaultSetting);

	get autoUpdate() {return this.setting.autoUpdate};
	get language() {return this.setting.language};
	get sendOnEnter() {return this.setting.sendOnEnter};
	get showTimestamps() {return this.setting.showTimestamps};
	get selectedPreset() {return this.setting.selectedPreset};
	get selectedApiProfile() {return this.setting.selectedApiProfile};
	get selectedCharacterId() {return this.setting.selectedCharacterId};
	get selectedPersonaId() {return this.setting.selectedPersonaId};
	get lastPlayedGameId() {return this.setting.lastPlayedGameId};

	set(newValue: AppSettings) {
		this.setting = newValue;
	}

	async update(patch: Partial<AppSettings>) {
		const isIdentical = Object.entries(patch).every(
			([key, value]) => this.setting[key as keyof AppSettings] == value,
		);
		if (isIdentical) return;

		this.setting = { ...this.setting, ...patch };
		await settingCommands.updateAppSetting({ patch });
	}

	async init() {
		// 리스너 등록
		await settingCommands.listen_appSettingsChanged((c) => this.set(c));

		// 세팅 불러오기
		const appSetting = await settingCommands.getAppSetting();
		this.set(appSetting);
	}
}

export const appSetting = new AppSettingStore();
