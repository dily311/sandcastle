// 사용자가 올린 글꼴을 앱 글꼴로 쓰기. 고른 글꼴 하나만 @font-face로 선언한다 (안 고른 글꼴은 불러오지 않음).
// 메인 창(app/theme.svelte.ts)과 위젯 창(lib/stores/themeStore.svelte.ts)이 같이 쓴다.
// tokens.css의 --font-sans가 `var(--font-user, ...)`를 맨 앞에 두므로 --font-user만 넣고 빼면 된다.
import type { UserFont } from "$ipc/bindings";
import { fileSrc } from "$shared/ipc/file";

const STYLE_ID = "sc-user-font";
const PREVIEW_ID = "sc-font-previews";

/** 글꼴 파일 경로 → CSS font-family 이름. 이름이 CSS에 들어가므로 영숫자와 -만 남기고, 같은 이름끼리 안 겹치게 파일 이름(uuid)에서 뽑는다 */
export function fontFamilyOf(path: string): string {
	const file = path.split(/[\\/]/).pop() ?? "font";
	return `SC-${file.replace(/\.[^.]+$/, "").replace(/[^a-zA-Z0-9-]/g, "")}`;
}

function formatOf(path: string): string {
	const ext = path.split(".").pop()?.toLowerCase();
	return ext === "woff2" ? "woff2" : ext === "woff" ? "woff" : ext === "otf" ? "opentype" : "truetype";
}

const faceCss = (path: string) => {
	const src = fileSrc(path);
	return src ? `@font-face{font-family:"${fontFamilyOf(path)}";src:url("${src.replace(/"/g, "%22")}") format("${formatOf(path)}");font-display:swap}` : "";
};

function styleEl(id: string): HTMLStyleElement {
	let el = document.getElementById(id) as HTMLStyleElement | null;
	if (!el) {
		el = document.createElement("style");
		el.id = id;
		document.head.appendChild(el);
	}
	return el;
}

/** 설정의 앱 글꼴을 문서에 반영. 목록에 없는 경로(지워진 글꼴)면 기본 글꼴 */
export function applyUserFont(root: HTMLElement, s: { fonts?: UserFont[]; appFont?: string | null }) {
	const chosen = s.appFont && (s.fonts ?? []).some((f) => f.path === s.appFont) ? s.appFont : null;
	if (!chosen) {
		document.getElementById(STYLE_ID)?.remove();
		root.style.removeProperty("--font-user");
		return;
	}
	styleEl(STYLE_ID).textContent = faceCss(chosen);
	root.style.setProperty("--font-user", `"${fontFamilyOf(chosen)}"`);
}

/** 글꼴 목록 화면의 미리보기용: 올린 글꼴 전부를 선언 (화면이 열려 있는 동안만). 해제 함수를 돌려줌 */
export function declarePreviewFonts(fonts: UserFont[]): () => void {
	styleEl(PREVIEW_ID).textContent = fonts.map((f) => faceCss(f.path)).join("\n");
	return () => document.getElementById(PREVIEW_ID)?.remove();
}
