// 로컬 파일 경로 → 화면에 쓸 수 있는 URL
import { convertFileSrc } from "@tauri-apps/api/core";

export function fileSrc(path: string | null | undefined): string | null {
	if (!path) return null;
	if (/^(https?:|data:|blob:|asset:)/i.test(path)) return path;
	return convertFileSrc(path);
}
