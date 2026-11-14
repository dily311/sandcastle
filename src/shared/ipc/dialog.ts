// 파일 고르기 / 저장 위치 고르기 (Tauri 대화상자). 브라우저 개발 모드에서는 안내만.
import { open, save } from "@tauri-apps/plugin-dialog";
import { toasts } from "$shared/stores/toast.svelte";

export interface Filter {
	name: string;
	extensions: string[];
}

export async function pickFiles(filters: Filter[], multiple = true): Promise<string[]> {
	try {
		const r = await open({ multiple, filters, directory: false });
		if (!r) return [];
		return Array.isArray(r) ? r : [r];
	} catch (e) {
		toasts.error("파일을 고를 수 없음 (앱에서만 됨)", String(e));
		return [];
	}
}

export async function pickSavePath(defaultPath: string, filters: Filter[]): Promise<string | null> {
	try {
		return await save({ defaultPath, filters });
	} catch (e) {
		toasts.error("저장 위치를 고를 수 없음 (앱에서만 됨)", String(e));
		return null;
	}
}

export const IMAGE_FILES: Filter = { name: "이미지", extensions: ["png", "jpg", "jpeg", "webp", "gif"] };
export const ST_FILES: Filter = { name: "SillyTavern", extensions: ["png", "json", "jsonl"] };
export const CHAT_FILES: Filter = { name: "대화 기록", extensions: ["jsonl"] };
export const PACK_FILES: Filter = { name: "캐릭터 묶음", extensions: ["zip"] };
export const JSON_FILES: Filter = { name: "JSON", extensions: ["json"] };
export const BACKUP_FILES: Filter = { name: "샌드캐슬 백업", extensions: ["scbackup"] };
