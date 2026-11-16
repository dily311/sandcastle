// 새 버전 확인 (CLAUDE.md 1-E-2, service/update.rs). 내려받거나 설치하지 않는다 — 릴리스 페이지를 열어 주기만.
// 앱을 켤 때 확인은 설정(checkUpdatesOnStart, 기본 꺼짐)을 켰을 때만
import { openUrl } from "@tauri-apps/plugin-opener";
import type { UpdateInfo } from "$ipc/bindings";
import { call, commands } from "$shared/ipc/client";
import { toasts } from "$shared/stores/toast.svelte";

export async function checkForUpdate(silent = false): Promise<UpdateInfo | null> {
	return call(commands.checkForUpdate(), { source: "update", silent }).catch(() => null);
}

export function openRelease(u: UpdateInfo) {
	if (u.url) void openUrl(u.url).catch((e) => toasts.error("릴리스 페이지를 열 수 없음", String(e)));
}

/** 설정이 켜져 있을 때만, 시작하고 조금 뒤 한 번 (화면을 먼저 띄운다) */
export function checkOnStart(enabled: boolean) {
	if (!enabled) return;
	setTimeout(async () => {
		const u = await checkForUpdate(true);
		if (u?.newer) toasts.show({ tone: "info", message: `새 버전 ${u.latest}이 나옴 (지금 ${u.current})`, action: { label: "릴리스 보기", run: () => openRelease(u) } }, 12_000);
	}, 5000);
}
