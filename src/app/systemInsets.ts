// Android: 상태 표시줄·하단 탐색 막대 높이를 CSS 변수(--native-inset-top / --native-inset-bottom)로 (CLAUDE.md 1-C-2).
// - 위: 키보드 때문에 WebView 아래를 비우면 env(safe-area-inset-top)이 0이 되므로 앱(MainActivity.kt)이 숫자를 알려 준다.
// - 아래: 키보드는 앱이 비우고, 탐색 막대 높이만 알려 준다 (키보드가 열려 있으면 0). env(safe-area-inset-bottom)과 겹쳐 두 번 비우지 않게 이 값만 쓴다 (--safe-bottom)
// 데스크톱·브라우저에는 SandcastleInsets가 없어 아무것도 하지 않는다
type NativeInsets = { top(): number; bottom?(): number };

function native() {
	return (window as unknown as { SandcastleInsets?: NativeInsets }).SandcastleInsets;
}

function apply() {
	const n = native();
	if (!n) return;
	const root = document.documentElement.style;
	root.setProperty("--native-inset-top", `${n.top()}px`);
	// 예전 앱(bottom 없음)은 아래를 앱이 비웠으므로 0
	root.setProperty("--native-inset-bottom", `${n.bottom?.() ?? 0}px`);
}

export function watchSystemInsets() {
	if (!native()) return;
	apply();
	window.addEventListener("sc-insets", apply);
	window.addEventListener("resize", apply);
}
