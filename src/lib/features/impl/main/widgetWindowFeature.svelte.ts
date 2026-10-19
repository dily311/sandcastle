import type { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { widgetCommands } from "$/ipc/commands";
import type { FeatureRuntime } from "../../types";

const WIDGET_FEATURE_ID = "widget";
class RegisterWidgetWindowFeature implements FeatureRuntime {
	id = WIDGET_FEATURE_ID;
	enabled = $state(false);
	defaultEnabled = false;
	private win?: WebviewWindow;

	async init() {
		this.enabled = true;
		await widgetCommands.openWidgetWindow();
	}

	shown() {
        this.enabled = true;
		widgetCommands.toggleWidgetWindow();
	}

	disable() {
        this.enabled = false;
		widgetCommands.toggleWidgetWindow();
	}

	async destroy() {
        this.enabled = false;
		await widgetCommands.closeWidgetWindow();
	}
}
export const registerWidgetWindowFeature = new RegisterWidgetWindowFeature();
