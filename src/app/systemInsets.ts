// Android: 상태 표시줄 높이를 CSS 변수(--native-inset-top)로 (CLAUDE.md 1-C-2).
// 키보드 때문에 WebView 아래를 비우면 env(safe-area-inset-top)이 0이 되므로 앱(MainActivity.kt)이 숫자를 알려 준다.
// 데스크톱·브라우저에는 SandcastleInsets가 없어 아무것도 하지 않는다
type NativeInsets = { top(): number };

function apply() {
	const n = (window as unknown as { SandcastleInsets?: NativeInsets }).SandcastleInsets;
	if (!n) return;
	document.documentElement.style.setProperty("--native-inset-top", `${n.top()}px`);
}

export function watchSystemInsets() {
	if (!(window as unknown as { SandcastleInsets?: NativeInsets }).SandcastleInsets) return;
	apply();
	window.addEventListener("sc-insets", apply);
	window.addEventListener("resize", apply);
}
