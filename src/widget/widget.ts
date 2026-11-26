import "$/styles/font.css";
import "$shared/ui/base.css";
import "./widget.css";
import Widget from "./WidgetApp.svelte";
import { mount } from "svelte";
import { initI18n } from "$/lib/utils/i18n";
import { appSetting } from "$shared/stores/appSetting.svelte";

window.addEventListener("keydown", (e) => {
	// Ctrl + P 또는 Cmd + P (인쇄 단축키) 감지
	if ((e.ctrlKey || e.metaKey) && (e.key === "p" || e.key === "P")) {
		e.preventDefault();
	}
});

async function boot() {
	// 브라우저 개발 모드: 메인 창과 같은 가짜 백엔드 (빌드에서는 빠짐)
	if (import.meta.env.DEV && !("__TAURI_INTERNALS__" in window)) await import("$ipc/dev-mock/install");
	await initI18n();
	void appSetting.load();
	mount(Widget, { target: document.getElementById("app")! });
}

void boot();
