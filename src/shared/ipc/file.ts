// 로컬 파일 경로 → 화면에 쓸 수 있는 URL
import { convertFileSrc } from "@tauri-apps/api/core";

/** 캐릭터 에셋 그림 주소. 같은 키로 다시 올리면 파일 경로가 그대로라 브라우저가 옛 그림을 보여주므로 바뀐 때(updatedAt)를 붙인다 */
export function imageSrc(img: { path: string; thumbnailPath?: string | null; updatedAt: number }, thumb = false): string | null {
	const src = fileSrc(thumb ? (img.thumbnailPath ?? img.path) : img.path);
	return src && `${src}${src.includes("?") ? "&" : "?"}v=${img.updatedAt}`;
}

export function fileSrc(path: string | null | undefined): string | null {
	if (!path) return null;
	if (/^(https?:|data:|blob:|asset:)/i.test(path)) return path;
	return convertFileSrc(path);
}
