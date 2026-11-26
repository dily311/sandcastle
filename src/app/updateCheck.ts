// 새 버전 확인 (CLAUDE.md 1-E-2, service/update.rs). 내려받거나 설치하지 않는다 — 릴리스 페이지를 열어 주기만.
// 앱을 켤 때 확인은 설정(checkUpdatesOnStart, 기본 꺼짐)을 켰을 때만
import { get } from "svelte/store";
import { _ } from "svelte-i18n";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { UpdateInfo } from "$ipc/bindings";
import { call, commands } from "$shared/ipc/client";
import { toasts } from "$shared/stores/toast.svelte";

const t = (k: string, values?: Record<string, string | number>) => get(_)(k, values ? { values } : undefined);

export async function checkForUpdate(silent = false): Promise<UpdateInfo | null> {
	return call(commands.checkForUpdate(), { source: "update", silent }).catch(() => null);
}

export function openRelease(u: UpdateInfo) {
	if (u.url) void openUrl(u.url).catch((e) => toasts.error(t("appMisc.openReleaseErr"), String(e)));
}

/** 설정이 켜져 있을 때만, 시작하고 조금 뒤 한 번 (화면을 먼저 띄운다) */
export function checkOnStart(enabled: boolean) {
	if (!enabled) return;
	setTimeout(async () => {
		const u = await checkForUpdate(true);
		if (u?.newer) toasts.show({ tone: "info", message: t("appMisc.updateNewToast", { latest: u.latest ?? "", current: u.current }), action: { label: t("appMisc.viewReleaseBtn"), run: () => openRelease(u) } }, 12_000);
	}, 5000);
}
