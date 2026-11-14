import type { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { commands } from "$ipc/bindings";
import type { FeatureRuntime } from "../../types";

const WIDGET_FEATURE_ID = "widget";
class RegisterWidgetWindowFeature implements FeatureRuntime {
	id = WIDGET_FEATURE_ID;
	enabled = $state(false);
	defaultEnabled = false;
	private win?: WebviewWindow;

	async init() {
		this.enabled = true;
		await commands.openWidget();
	}

	shown() {
        this.enabled = true;
		void commands.toggleWidget();
	}

	disable() {
        this.enabled = false;
		void commands.toggleWidget();
	}

	async destroy() {
        this.enabled = false;
		await commands.closeWidget();
	}
}
export const registerWidgetWindowFeature = new RegisterWidgetWindowFeature();
