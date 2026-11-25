import type { MenuItem } from "$shared/ui/ContextMenu.svelte";

// 그림 크게 보기: 어느 화면에서든 그림을 누르면 앱 전체 뷰어(ImageViewerHost)가 띄운다.
// 데이터(무엇을 보나)만 여기 두고, 그리는 일은 shared/ui/ImageViewerHost.svelte가 한다.
export interface ViewedImage {
	/** 표시할 수 있는 URL (convertFileSrc 결과 등) */
	src: string;
	/** 그림 아래에 보일 이름. 없으면 src에서 파일 이름을 뽑음 */
	name?: string;
	/** 이름 바꾸기·삭제는 이 그림이 무엇인지 아는 쪽(에셋 등)만 줄 수 있다. 주지 않으면 메뉴에 나오지 않음 */
	rename?: () => void | Promise<void>;
	remove?: () => void | Promise<void>;
	/** 정보 보기에 더할 줄 (프롬프트 등) */
	info?: { label: string; value: string }[];
	/** 컨텍스트 메뉴에 추가할 동작 (예: 이 프롬프트로 다시, 태그 추가, 대화로 이동) */
	extraItems?: MenuItem[] | (() => MenuItem[]);
}

/** 그림 요소에 붙여 두는 동작: 눌러서 크게 볼 때(imageZoom) 뷰어가 가져다 쓴다. `use:viewerActions={{ rename, remove }}` */
const actionsOf = new WeakMap<Element, Pick<ViewedImage, "name" | "rename" | "remove" | "info">>();
export function viewerActions(node: Element, actions: Pick<ViewedImage, "name" | "rename" | "remove" | "info">) {
	actionsOf.set(node, actions);
	return {
		update: (a: typeof actions) => void actionsOf.set(node, a),
		destroy: () => void actionsOf.delete(node),
	};
}
export const viewerActionsOf = (node: Element) => actionsOf.get(node);

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
	/** 열려 있는 그림의 이름 등을 바꿈 (이름 바꾸기 뒤) */
	patch(p: Partial<ViewedImage>) {
		if (this.current) this.current = { ...this.current, ...p };
	}
	close() {
		this.current = null;
	}
}

export const imageViewer = new ImageViewerStore();
