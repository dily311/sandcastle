// 그림 크게 보기: 어느 화면에서든 그림을 누르면 앱 전체 뷰어(ImageViewerHost)가 띄운다.
// 데이터(무엇을 보나)만 여기 두고, 그리는 일은 shared/ui/ImageViewerHost.svelte가 한다.
export interface ViewedImage {
	/** 표시할 수 있는 URL (convertFileSrc 결과 등) */
	src: string;
	/** 그림 아래에 보일 이름. 없으면 src에서 파일 이름을 뽑음 */
	name?: string;
}

/** src(URL)에서 사람이 읽을 파일 이름과 디스크 경로를 뽑는다. 파일이 아닌 그림(data:, blob:)은 이름만 비움 */
export function describeSrc(src: string): { name: string; path: string | null } {
	if (/^(data|blob):/i.test(src)) return { name: "", path: null };
	let pathname = src;
	try {
		pathname = new URL(src, window.location.href).pathname;
	} catch {
		/* 상대 경로면 그대로 */
	}
	let decoded = pathname;
	try {
		decoded = decodeURIComponent(pathname);
	} catch {
		/* 깨진 % 표기면 그대로 */
	}
	const last = decoded.split(/[\\/]/).filter(Boolean).pop() ?? "";
	// convertFileSrc가 만든 주소(asset / *.localhost)만 디스크 경로로 본다. 인터넷 주소는 경로가 아님
	const local = /^(asset:|https?:\/\/asset\.localhost)/i.test(src);
	// 인터넷 주소의 끝마디는 파일 이름이 아닐 수 있어(…/png?text=) 그림 확장자가 있을 때만 이름으로 씀
	const name = local || /\.(png|jpe?g|webp|gif|avif|bmp|svg)$/i.test(last) ? last : "";
	const path = local ? decoded.replace(/^\/(?=[a-z]:)/i, "") : null;
	return { name, path };
}

class ImageViewerStore {
	current = $state<ViewedImage | null>(null);

	open(img: ViewedImage) {
		this.current = img;
	}
	close() {
		this.current = null;
	}
}

export const imageViewer = new ImageViewerStore();
