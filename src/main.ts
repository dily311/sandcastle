import "$/styles/font.css";
import "$shared/ui/base.css";
import { mount } from "svelte";
import AppShell from "$app/AppShell.svelte";
import { bootstrapFeatures } from "$lib/features/feautreManager.svelte";
import { logs } from "$shared/stores/log.svelte";
import { layout } from "$app/layout.svelte";
import { watchSystemInsets } from "$app/systemInsets";
import { watchKeepAlive } from "$app/keepAlive";
import { watchAndroidBack } from "$app/androidBack.svelte";

// 앱 안에서는 브라우저 기본 우클릭 메뉴를 쓰지 않는다 (필요한 곳은 ContextMenu)
document.addEventListener("contextmenu", (e) => e.preventDefault(), false);

async function boot() {
	// 브라우저 개발 모드: 가짜 백엔드 (빌드에서는 이 분기가 통째로 빠짐)
	if (import.meta.env.DEV && !("__TAURI_INTERNALS__" in window)) await import("$ipc/dev-mock/install");

	watchSystemInsets();
	mount(AppShell, { target: document.getElementById("app")! });

	// Android: AI 작업이 도는 동안 앱이 얼지 않게 (화면 꺼짐·뒤로 가기)
	if (__IS_TAURI__ && !layout.desktop) {
		watchKeepAlive();
		// 시스템 뒤로 가기 = 앱 안의 앞 화면으로
		watchAndroidBack();
	}
	// 위젯 창 같은 부가 기능은 화면과 따로 띄운다 — 실패해도 앱은 계속. 위젯은 데스크톱 전용이라 Android에서는 시작하지 않음
	if (__IS_TAURI__ && layout.desktop) bootstrapFeatures().catch((e) => logs.error("feature", String(e)));
}

void boot();
