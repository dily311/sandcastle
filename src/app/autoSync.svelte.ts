// 자동 동기화 (CLAUDE.md 2-C-2, ADR-011). 기본 꺼짐.
// 동기화 플러그인 + 설정 > 동기화의 "자동 동기화" + 저장소(폴더 또는 구글 드라이브)가 모두 있을 때만 타이머를 만든다 (데스크톱만).
// 앱을 켜고 조금 뒤 한 번, 그 뒤 정한 간격마다. 실패는 실행 기록·동기화 기록과 로그에 남고 알림은 띄우지 않는다.
import { get } from "svelte/store";
import { _ } from "svelte-i18n";
import { call, commands } from "$shared/ipc/client";
import { plugins } from "$shared/stores/plugins.svelte";
import { pluginSetting } from "$shared/stores/aiTools.svelte";
import { logs } from "$shared/stores/log.svelte";
import { layout } from "./layout.svelte";
import { hasNativeDrive, prepareDrive, unmetered } from "./androidDrive";

const t = (k: string, values?: Record<string, string | number>) => get(_)(k, values ? { values } : undefined);

const FIRST_DELAY_MS = 20_000;
const phone = () => !layout.desktop && hasNativeDrive();
let timer: ReturnType<typeof setTimeout> | null = null;
let key = "";

function stop() {
	if (timer) clearTimeout(timer);
	timer = null;
	key = "";
}

async function runOnce() {
	try {
		// Android: Wi-Fi에서만이면 모바일 데이터일 때 건너뛰고, 폰에서 새 토큰을 받는다 (권한이 없으면 화면 없이 실패 → 기록)
		if (phone()) {
			if (pluginSetting.value?.sync?.wifiOnly && !unmetered()) {
				logs.info("sync", t("appMisc.autoSyncSkippedWifi"));
				return;
			}
			await prepareDrive(false);
		}
		const r = await call(commands.syncNow(true), { source: "sync", silent: true });
		const merged = r.merged.length;
		if (merged) logs.info("sync", t("appMisc.autoSyncMerged", { count: merged, down: r.filesDown, up: r.filesUp }));
	} catch (e) {
		logs.warn("sync", t("appMisc.autoSyncFailed", { reason: String((e as { message?: string })?.message ?? e) }));
	}
}

function schedule(delay: number, intervalMs: number) {
	timer = setTimeout(async () => {
		await runOnce();
		if (timer) schedule(intervalMs, intervalMs);
	}, delay);
}

/** AppShell이 한 번 부른다. 꺼져 있으면 설정도 읽지 않는다 */
export function watchAutoSync() {
	$effect.root(() => {
		$effect(() => {
			if (!(layout.desktop || phone()) || !plugins.loaded || !plugins.on("sync")) return stop();
			if (!pluginSetting.value) {
				void pluginSetting.load();
				return;
			}
			const s = pluginSetting.value.sync;
			// 폴더면 폴더가, 드라이브면 로그인이 있어야 (로그인이 끊기면 백엔드가 거절하고 기록에 남음 — 연결 끊기는 자동 동기화도 끔)
			if (!s?.auto || (s.target !== "drive" && (phone() || !s.folder))) return stop();
			const minutes = Math.max(5, s.intervalMinutes || 30);
			const next = `${s.target}|${s.folder}|${minutes}`;
			if (next === key) return;
			stop();
			key = next;
			schedule(FIRST_DELAY_MS, minutes * 60_000);
		});
	});
}
