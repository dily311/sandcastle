// 어느 화면의 그림이든 눌러서 크게 보기: window에 클릭 하나만 걸어 두고 img를 찾아 뷰어를 띄운다.
// 화면마다 따로 붙이지 않아도 새 화면의 그림이 저절로 대상이 된다. 빼려면 그림이나 그 부모에 data-no-zoom.
import { imageViewer } from "$shared/stores/imageViewer.svelte";

/** 눌러도 크게 보지 않는 자리: 링크·버튼처럼 이미 다른 동작이 있는 곳, 드래그 손잡이, 뷰어 자신 */
const BLOCKED = "a, button, summary, label, [role='button'], [role='link'], [role='menuitem'], [draggable='true'], [data-no-zoom], .image-viewer";
/** 이보다 작게 그려진 그림은 아이콘·아바타 조각으로 보고 건너뜀 */
const MIN_SIZE = 40;

export function zoomTarget(target: EventTarget | null): HTMLImageElement | null {
	if (!(target instanceof HTMLImageElement)) return null;
	if (target.closest(BLOCKED)) return null;
	if (!target.currentSrc && !target.src) return null;
	const r = target.getBoundingClientRect();
	if (r.width < MIN_SIZE || r.height < MIN_SIZE) return null;
	return target;
}

/** 앱 시작 때 한 번. 해제 함수를 돌려줌 */
export function installImageZoom(): () => void {
	const onclick = (e: MouseEvent) => {
		if (e.defaultPrevented || e.button !== 0) return;
		const img = zoomTarget(e.target);
		if (!img) return;
		imageViewer.open({ src: img.currentSrc || img.src, name: img.dataset.name || undefined });
	};
	window.addEventListener("click", onclick);
	return () => window.removeEventListener("click", onclick);
}
