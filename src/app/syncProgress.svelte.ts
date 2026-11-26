// 백그라운드 파일 작업(구글 드라이브·폴더 동기화, 드라이브 데이터 삭제, SillyTavern 가져오기·내보내기) 진행률 표시기 (1-J-2, 1-J-6).
// - 하단 중앙 고정 작업 팝업(JobProgress) 대신 우측 하단 토스트 하나로 진행률을 갱신하고, 사용자가 토스트를 누르면 즉시 닫힌다.
// - 모바일(Android)에서는 백그라운드 포그라운드 서비스 알림(GenerationService)에도 진행률 막대와 퍼센트를 실시간으로 동기화한다.
import type { StProgress, SyncProgress } from "$ipc/bindings";
import { tr } from "$lib/utils/i18n";
import { onEvent } from "$shared/ipc/events";
import { pluginSetting } from "$shared/stores/aiTools.svelte";
import { toasts, type ToastProgress } from "$shared/stores/toast.svelte";
import { hasNativeDrive } from "./androidDrive";

type Native = { postMessage(msg: string): void };
const native = () => (window as unknown as { SandcastleNative?: Native }).SandcastleNative;

type ChannelKey = "sync" | "st";

interface FormattedProgress {
	title: string;
	text: string;
	toastProgress: ToastProgress;
}

interface ChannelState {
	active: boolean;
	toastId: number | null;
	dismissedByUser: boolean;
	holds: number;
	manualCounter: boolean;
	delayToastMs: number;
	delayTimer: ReturnType<typeof setTimeout> | null;
	pendingToast: { message: string; detail: string; progress: ToastProgress } | null;
	lastNativePayload: string | null;
	lastNativeSentAt: number;
	nativeTimer: ReturnType<typeof setTimeout> | null;
}

function makeChannel(): ChannelState {
	return {
		active: false,
		toastId: null,
		dismissedByUser: false,
		holds: 0,
		manualCounter: false,
		delayToastMs: 0,
		delayTimer: null,
		pendingToast: null,
		lastNativePayload: null,
		lastNativeSentAt: 0,
		nativeTimer: null,
	};
}

const channels: Record<ChannelKey, ChannelState> = {
	sync: makeChannel(),
	st: makeChannel(),
};

let watching = false;
let currentStepText = $state("");

export const syncProgressState = {
	get step() {
		return currentStepText;
	},
};

function openChannelToast(ch: ChannelState, spec: { message: string; detail: string; progress: ToastProgress }) {
	ch.toastId = toasts.show(
		{
			tone: "info",
			message: spec.message,
			detail: spec.detail,
			progress: spec.progress,
			dismissOnClick: true,
			onDismiss: () => {
				ch.dismissedByUser = true;
				ch.toastId = null;
			},
			log: false,
		},
		0,
	);
}

function postNativeForChannel(ch: ChannelState, formatted: FormattedProgress) {
	const payload = JSON.stringify({
		title: formatted.title,
		text: formatted.text,
		current: formatted.toastProgress.current ?? -1,
		total: formatted.toastProgress.total ?? 0,
		indeterminate: Boolean(formatted.toastProgress.indeterminate),
	});
	const now = Date.now();
	const isFirst = ch.lastNativePayload === null;
	const isDoneTick =
		typeof formatted.toastProgress.current === "number" &&
		typeof formatted.toastProgress.total === "number" &&
		formatted.toastProgress.total > 0 &&
		formatted.toastProgress.current >= formatted.toastProgress.total;
	ch.lastNativePayload = payload;
	if (isFirst || isDoneTick || now - ch.lastNativeSentAt >= 120) {
		if (ch.nativeTimer) {
			clearTimeout(ch.nativeTimer);
			ch.nativeTimer = null;
		}
		ch.lastNativeSentAt = now;
		native()?.postMessage(`sync:progress:${payload}`);
		return;
	}
	if (!ch.nativeTimer) {
		const wait = Math.max(16, 120 - (now - ch.lastNativeSentAt));
		ch.nativeTimer = setTimeout(() => {
			ch.nativeTimer = null;
			if (!ch.active || !ch.lastNativePayload) return;
			ch.lastNativeSentAt = Date.now();
			native()?.postMessage(`sync:progress:${ch.lastNativePayload}`);
		}, wait);
	}
}

function updateChannel(key: ChannelKey, formatted: FormattedProgress, delayToastMs = 0) {
	const ch = channels[key];
	ch.active = true;
	postNativeForChannel(ch, formatted);

	// 사용자가 이번 실행의 토스트를 눌러 닫았다면 백그라운드 알림만 유지하고 토스트는 다시 띄우지 않는다
	if (ch.dismissedByUser) return;

	const message = `${formatted.title} · ${formatted.text}`;
	const detail = tr("settings.syncSec.tapToHideToast");
	if (ch.toastId !== null && toasts.update(ch.toastId, { message, detail, progress: formatted.toastProgress })) {
		return;
	}
	if (delayToastMs > 0) {
		ch.pendingToast = { message, detail, progress: formatted.toastProgress };
		if (!ch.delayTimer) {
			ch.delayTimer = setTimeout(() => {
				ch.delayTimer = null;
				if (!ch.active || ch.dismissedByUser || !ch.pendingToast) return;
				openChannelToast(ch, ch.pendingToast);
				ch.pendingToast = null;
			}, delayToastMs);
		}
		return;
	}
	if (ch.delayTimer) {
		clearTimeout(ch.delayTimer);
		ch.delayTimer = null;
		ch.pendingToast = null;
	}
	openChannelToast(ch, { message, detail, progress: formatted.toastProgress });
}

function finishChannel(key: ChannelKey) {
	const ch = channels[key];
	if (key === "sync") currentStepText = "";
	if (ch.delayTimer) {
		clearTimeout(ch.delayTimer);
		ch.delayTimer = null;
	}
	if (ch.nativeTimer) {
		clearTimeout(ch.nativeTimer);
		ch.nativeTimer = null;
	}
	ch.pendingToast = null;
	ch.lastNativePayload = null;
	ch.lastNativeSentAt = 0;
	ch.active = false;
	ch.holds = 0;
	ch.manualCounter = false;
	ch.delayToastMs = 0;
	if (ch.toastId !== null) {
		toasts.dismiss(ch.toastId, false);
		ch.toastId = null;
	}
	ch.dismissedByUser = false;

	const otherKey: ChannelKey = key === "sync" ? "st" : "sync";
	const other = channels[otherKey];
	if (other.active && other.lastNativePayload) {
		native()?.postMessage(`sync:progress:${other.lastNativePayload}`);
	} else {
		native()?.postMessage("sync:stop");
	}
}

function formatSyncProgress(p: SyncProgress): FormattedProgress {
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

export function formatStProgress(p: StProgress): FormattedProgress {
	const title = p.kind === "export" ? tr("settings.dataSec.stExportProgressTitle") : tr("settings.dataSec.stImportProgressTitle");
	const cur = p.current ?? undefined;
	const tot = p.total ?? undefined;
	const hasCounts = typeof cur === "number" && typeof tot === "number" && tot > 0;
	const pct = hasCounts ? Math.min(100, Math.round((cur * 100) / tot)) : 0;

	let text = tr("settings.dataSec.stStepPreparing");
	if (p.phase === "extracting" && hasCounts) text = tr("settings.dataSec.stStepExtracting", { current: cur, total: tot, pct });
	else if (p.phase === "books" && hasCounts) text = tr("settings.dataSec.stStepBooks", { current: cur, total: tot, pct });
	else if (p.phase === "characters" && hasCounts) text = tr("settings.dataSec.stStepCharacters", { current: cur, total: tot, pct });
	else if (p.phase === "chats" && hasCounts) text = tr("settings.dataSec.stStepChats", { current: cur, total: tot, pct });
	else if (p.phase === "presets" && hasCounts) text = tr("settings.dataSec.stStepPresets", { current: cur, total: tot, pct });
	else if (p.phase === "regex" && hasCounts) text = tr("settings.dataSec.stStepRegex", { current: cur, total: tot, pct });
	else if (p.phase === "personas" && hasCounts) text = tr("settings.dataSec.stStepPersonas", { current: cur, total: tot, pct });
	else if (p.phase === "files" && hasCounts) text = tr("settings.dataSec.stStepFiles", { current: cur, total: tot, pct });
	else if (p.phase === "finishing") {
		text = hasCounts ? tr("settings.dataSec.stStepFinishingCount", { current: cur, total: tot, pct }) : tr("settings.dataSec.stStepFinishing");
	}

	const toastProgress: ToastProgress = hasCounts ? { current: cur, total: tot } : { indeterminate: true };
	return { title, text, toastProgress };
}

export interface StProgressBeginOptions {
	kind: "import" | "export";
	initialPhase?: string;
	initialTotal?: number;
	manualCounter?: boolean;
	delayToastMs?: number;
}

/** 프런트에서 SillyTavern 가져오기·내보내기 구간(Android 임시 복사·다중 파일 포함)을 시작한다 */
export function beginStProgress(opts: StProgressBeginOptions) {
	const ch = channels.st;
	ch.holds += 1;
	if (opts.manualCounter) ch.manualCounter = true;
	ch.delayToastMs = opts.delayToastMs ?? 0;
	const total = opts.initialTotal && opts.initialTotal > 0 ? opts.initialTotal : null;
	const formatted = formatStProgress({
		kind: opts.kind,
		phase: opts.initialPhase ?? "preparing",
		current: total !== null ? 0 : null,
		total,
	});
	updateChannel("st", formatted, ch.delayToastMs);
}

/** 프런트에서 직접 단계·개수를 갱신한다 (다중 파일 순차 가져오기, Android 저장 마무리 등) */
export function stepStProgress(p: StProgress) {
	updateChannel("st", formatStProgress(p), channels.st.delayToastMs);
}

/** 프런트에서 열었던 SillyTavern 진행 구간을 닫는다 */
export function endStProgress() {
	const ch = channels.st;
	if (ch.holds > 1) {
		ch.holds -= 1;
		return;
	}
	finishChannel("st");
}

export async function runWithStProgress<T>(
	opts: StProgressBeginOptions,
	fn: (step: (phase: string, current?: number, total?: number) => void) => Promise<T>,
): Promise<T> {
	beginStProgress(opts);
	try {
		return await fn((phase, current, total) => {
			stepStProgress({
				kind: opts.kind,
				phase,
				current: current ?? null,
				total: total ?? null,
			});
		});
	} finally {
		endStProgress();
	}
}

export function handleSyncProgressEvent(p: SyncProgress) {
	if (p.phase === "done") {
		finishChannel("sync");
		return;
	}
	const formatted = formatSyncProgress(p);
	currentStepText = formatted.text;
	updateChannel("sync", formatted, 0);
}

export function handleStProgressEvent(p: StProgress) {
	const ch = channels.st;
	if (p.phase === "done") {
		if (ch.holds === 0) finishChannel("st");
		return;
	}
	// 다중 파일 루프가 파일 개수로 진행률을 세고 있을 때는 개별 파일 내부 카운터로 막대를 덮어쓰지 않는다
	if (ch.manualCounter) return;
	// 백엔드에서 실제 개수(total > 1)가 올라오면 짧은 지연 대기 없이 바로 토스트를 띄운다
	const delay = typeof p.total === "number" && p.total > 1 ? 0 : ch.delayToastMs;
	updateChannel("st", formatStProgress(p), delay);
}

/** 테스트 전용 초기화 */
export function resetBgProgressForTest() {
	finishChannel("sync");
	finishChannel("st");
}

export function watchSyncProgress() {
	if (watching) return;
	watching = true;
	void onEvent<SyncProgress>("sync::progress", handleSyncProgressEvent);
	void onEvent<StProgress>("st::progress", handleStProgressEvent);
}
