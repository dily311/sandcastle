// Android 뒤로 가기 (CLAUDE.md 1-C-2): 앱 안의 이동 기록(nav)을 브라우저 기록과 맞춰,
// 시스템 뒤로 가기가 앱을 닫는 대신 앞 화면으로 가게 한다 (WebView가 뒤로 갈 기록이 있으면 goBack을 부름).
// 첫 화면에서 뒤로 가기를 누르면 그때는 앱이 닫힌다. 데스크톱은 쓰지 않는다 (타이틀바·마우스 옆 버튼이 따로 처리)
import { nav } from "./nav.svelte";

/** 우리가 pushState로 쌓은 기록 수 (= nav.index와 맞춘다) */
let pushed = 0;
/** 앱 안에서 뒤로 갔을 때 브라우저 기록을 따라 줄이는 중 (그때 오는 popstate는 무시) */
let syncing = 0;

function sync(index: number) {
	while (pushed < index) {
		history.pushState({ sc: pushed + 1 }, "");
		pushed++;
	}
	if (pushed > index) {
		syncing += pushed - index;
		history.go(index - pushed);
		pushed = index;
	}
}

export function watchAndroidBack() {
	$effect.root(() => {
		$effect(() => sync(nav.index));
	});
	window.addEventListener("popstate", async () => {
		if (syncing > 0) {
			syncing--;
			return;
		}
		// 시스템 뒤로 가기: 기록 하나가 이미 빠졌다
		pushed = Math.max(0, pushed - 1);
		const before = nav.index;
		await nav.back();
		// 저장 안 한 편집이 있어 머무르기로 했으면 빠진 기록을 되돌린다
		if (nav.index === before) sync(nav.index);
	});
}
