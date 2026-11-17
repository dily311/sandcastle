// 파일 고르기 / 저장 위치 고르기 (Tauri 대화상자). 브라우저 개발 모드에서는 안내만.
// Android는 경로 대신 content:// 주소를 주므로 앱 임시 폴더의 경로로 바꿔서 돌려준다 (1-C-2, service/picked_file.rs)
import { open, save } from "@tauri-apps/plugin-dialog";
import { commands } from "$ipc/bindings";
import { call } from "$shared/ipc/client";
import { toasts } from "$shared/stores/toast.svelte";

const isContent = (p: string) => p.startsWith("content://");

export interface Filter {
	name: string;
	extensions: string[];
}

export async function pickFiles(filters: Filter[], multiple = true): Promise<string[]> {
	try {
		const r = await open({ multiple, filters, directory: false });
		if (!r) return [];
		const picked = Array.isArray(r) ? r : [r];
		const exts = filters.flatMap((f) => f.extensions);
		return await Promise.all(picked.map((p) => (isContent(p) ? call(commands.stagePickedFile(p, exts), { source: "file" }) : p)));
	} catch (e) {
		toasts.error("파일을 고를 수 없음 (앱에서만 됨)", String(e));
		return [];
	}
}

/** 폴더 하나 고르기 (데스크톱). 취소하면 null */
export async function pickFolder(): Promise<string | null> {
	try {
		const r = await open({ directory: true, multiple: false });
		return typeof r === "string" ? r : null;
	} catch (e) {
		toasts.error("폴더를 고를 수 없음 (앱에서만 됨)", String(e));
		return null;
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

/**
 * 저장 위치를 고르고 run(경로)으로 쓴다. 취소하면 null (run은 null이 아닌 값을 돌려줄 것).
 * Android에서 content:// 주소를 고르면 임시 경로에 쓰게 한 뒤 고른 곳으로 옮긴다
 */
export async function saveTo<T>(defaultPath: string, filters: Filter[], run: (path: string) => Promise<T>): Promise<T | null> {
	const picked = await pickSavePath(defaultPath, filters);
	if (!picked) return null;
	if (!isContent(picked)) return run(picked);
	const tmp = await call(commands.prepareSaveTarget(defaultPath), { source: "file" });
	const result = await run(tmp);
	await call(commands.deliverSavedFile(tmp, picked), { source: "file" });
	return result;
}

/**
 * 폴더에 파일을 만드는 내보내기(이름은 백엔드가 정함)를 저장 위치 고르기로 (Android 전용 — 공용 다운로드 폴더에 직접 쓸 수 없음).
 * run(폴더)은 실제로 쓴 파일 경로를 돌려준다. 취소하면 null
 */
export async function saveIntoPicked(defaultName: string, filters: Filter[], run: (dir: string) => Promise<string>): Promise<string | null> {
	const picked = await pickSavePath(defaultName, filters);
	if (!picked) return null;
	const target = isContent(picked) ? await call(commands.prepareSaveTarget(defaultName), { source: "file" }) : picked;
	const dir = target.replace(/[\\/][^\\/]*$/, "");
	const written = await run(dir);
	if (isContent(picked)) await call(commands.deliverSavedFile(written, picked), { source: "file" });
	return written;
}

export const GAME_FILES: Filter = { name: "게임", extensions: ["tgame"] };
export const IMAGE_FILES: Filter = { name: "이미지", extensions: ["png", "jpg", "jpeg", "webp", "gif"] };
export const ST_FILES: Filter = { name: "SillyTavern", extensions: ["png", "json", "jsonl"] };
export const CHAT_FILES: Filter = { name: "대화 기록", extensions: ["jsonl"] };
export const PACK_FILES: Filter = { name: "캐릭터 묶음", extensions: ["zip"] };
export const JSON_FILES: Filter = { name: "JSON", extensions: ["json"] };
export const BACKUP_FILES: Filter = { name: "샌드캐슬 백업", extensions: ["scbackup"] };
export const SYNC_FILES: Filter = { name: "샌드캐슬 동기화", extensions: ["scsync"] };
