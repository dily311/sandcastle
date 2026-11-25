import "$/styles/font.css";
import "$shared/ui/base.css";
import "$lib/utils/i18n";
import { mount } from "svelte";
import AppShell from "$app/AppShell.svelte";
import { bootstrapWidgetFeatures } from "$lib/features/widgetFeatures.svelte";
import { logs } from "$shared/stores/log.svelte";
import { layout } from "$app/layout.svelte";
import { watchSystemInsets } from "$app/systemInsets";
import { watchKeepAlive } from "$app/keepAlive";
import { watchAndroidBack } from "$app/androidBack.svelte";

// 입력칸이나 텍스트가 선택된 상태에서는 브라우저/OS 기본 컨텍스트 메뉴(복사·잘라내기·붙여넣기·번역 등)를 허용하고,
// 그 외 빈 화면·UI 크롬에서만 기본 우클릭 메뉴를 막는다 (필요한 곳은 자체 ContextMenu 사용)
document.addEventListener(
	"contextmenu",
	(e) => {
		const target = e.target as Element | null;
		if (target?.closest("input, textarea, [contenteditable='true']")) return;
		if (window.getSelection()?.toString().trim()) return;
		e.preventDefault();
	},
	false,
);

async function boot() {
	// 브라우저 개발 모드: 가짜 백엔드 (빌드에서는 이 분기가 통째로 빠짐)
	if (import.meta.env.DEV && !("__TAURI_INTERNALS__" in window)) await import("$ipc/dev-mock/install");

	watchSystemInsets();
	const target = document.getElementById("app")!;
	document.getElementById("boot-splash")?.remove();
	mount(AppShell, { target });

	// Android: AI 작업이 도는 동안 앱이 얼지 않게 (화면 꺼짐·뒤로 가기)
	if (__IS_TAURI__ && !layout.desktop) {
		watchKeepAlive();
		// 시스템 뒤로 가기 = 앱 안의 앞 화면으로
		watchAndroidBack();
	}
	// 위젯 창 같은 부가 기능은 화면과 따로 띄운다 — 실패해도 앱은 계속. 위젯은 데스크톱 전용이라 Android에서는 시작하지 않음
	if (__IS_TAURI__ && layout.desktop) bootstrapWidgetFeatures().catch((e) => logs.error("feature", String(e)));
}

void boot();
