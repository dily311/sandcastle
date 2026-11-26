// 동기화·드라이브 데이터 삭제 진행률 표시기 (1-J-2).
// - 하단 중앙 고정 작업 팝업(JobProgress) 대신 우측 하단 토스트 하나로 진행률을 갱신하고, 사용자가 토스트를 누르면 즉시 닫힌다.
// - 모바일(Android)에서는 백그라운드 포그라운드 서비스 알림(GenerationService)에도 진행률 막대와 퍼센트를 실시간으로 동기화한다.
import type { SyncProgress } from "$ipc/bindings";
import { tr } from "$lib/utils/i18n";
import { onEvent } from "$shared/ipc/events";
import { pluginSetting } from "$shared/stores/aiTools.svelte";
import { toasts, type ToastProgress } from "$shared/stores/toast.svelte";
import { hasNativeDrive } from "./androidDrive";

type Native = { postMessage(msg: string): void };
const native = () => (window as unknown as { SandcastleNative?: Native }).SandcastleNative;

let watching = false;
let toastId: number | null = null;
let dismissedByUser = false;
let currentStepText = $state("");

export const syncProgressState = {
	get step() {
		return currentStepText;
	},
};

function formatProgress(p: SyncProgress): { title: string; text: string; toastProgress: ToastProgress } {
	const isDrive = hasNativeDrive() || pluginSetting.value?.sync?.target === "drive";
	const isWipe = p.phase === "wiping";
	const title = isWipe
		? tr("settings.syncSec.driveWipeProgressTitle")
		: isDrive
			? tr("settings.syncSec.driveSyncProgressTitle")
			: tr("settings.syncSec.syncProgressTitle");

	const cur = p.current ?? undefined;
	const tot = p.total ?? undefined;
	const hasCounts = typeof cur === "number" && typeof tot === "number" && tot > 0;
	const pct = hasCounts ? Math.min(100, Math.round((cur * 100) / tot)) : 0;

	let text = p.step;
	if (p.phase === "checking") text = tr("settings.syncSec.stepChecking");
	else if (p.phase === "downloading" && hasCounts) text = tr("settings.syncSec.stepDownloading", { current: cur, total: tot, pct });
	else if (p.phase === "merging") text = tr("settings.syncSec.stepMerging");
	else if (p.phase === "uploading") {
		text = hasCounts ? tr("settings.syncSec.stepUploadingFiles", { current: cur, total: tot, pct }) : tr("settings.syncSec.stepUploadingBundle");
	} else if (p.phase === "cleaning") text = tr("settings.syncSec.stepCleaning");
	else if (p.phase === "wiping") {
		text = hasCounts ? tr("settings.syncSec.stepWiping", { current: cur, total: tot, pct }) : tr("settings.syncSec.stepWipingPreparing");
	}

	const toastProgress: ToastProgress = hasCounts ? { current: cur, total: tot } : { indeterminate: true };
	return { title, text, toastProgress };
}

function clearProgress() {
	currentStepText = "";
	if (toastId !== null) {
		toasts.dismiss(toastId, false);
		toastId = null;
	}
	dismissedByUser = false;
	native()?.postMessage("sync:stop");
}

export function watchSyncProgress() {
	if (watching) return;
	watching = true;
	void onEvent<SyncProgress>("sync::progress", (p) => {
		if (p.phase === "done") {
			clearProgress();
			return;
		}
		const { title, text, toastProgress } = formatProgress(p);
		currentStepText = text;

		// 모바일 알림창에 진행률 실시간 반영
		native()?.postMessage(
			`sync:progress:${JSON.stringify({
				title,
				text,
				current: toastProgress.current ?? -1,
				total: toastProgress.total ?? 0,
				indeterminate: Boolean(toastProgress.indeterminate),
			})}`,
		);

		// 사용자가 이번 실행의 토스트를 눌러 닫았다면 백그라운드 알림만 유지하고 토스트는 다시 띄우지 않는다
		if (dismissedByUser) return;

		const message = `${title} · ${text}`;
		const detail = tr("settings.syncSec.tapToHideToast");
		if (toastId !== null && toasts.update(toastId, { message, detail, progress: toastProgress })) {
			return;
		}
		toastId = toasts.show(
			{
				tone: "info",
				message,
				detail,
				progress: toastProgress,
				dismissOnClick: true,
				onDismiss: () => {
					dismissedByUser = true;
					toastId = null;
				},
				log: false,
			},
			0,
		);
	});
}
