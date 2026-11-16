import type { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { commands } from "$ipc/bindings";
import type { FeatureRuntime } from "../../types";

const WIDGET_FEATURE_ID = "widget";
class RegisterWidgetWindowFeature implements FeatureRuntime {
	id = WIDGET_FEATURE_ID;
	enabled = $state(false);
	defaultEnabled = false;
	private win?: WebviewWindow;

	// 위젯 창은 백엔드만 열고 닫는다 (설정을 바꿀 때 update_feature_setting, 앱을 켤 때 lib.rs).
	// 여기서도 열면 백엔드와 동시에 열려 창이 두 개가 되던 버그가 있었다
	init() {
		this.enabled = true;
	}

	shown() {
        this.enabled = true;
		void commands.toggleWidget();
	}

	disable() {
        this.enabled = false;
		void commands.toggleWidget();
	}

	destroy() {
		this.enabled = false;
	}
}
export const registerWidgetWindowFeature = new RegisterWidgetWindowFeature();
