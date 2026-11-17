// 동기화 가짜 응답 (2-B-3). 브라우저 미리보기에서는 실제로 합치지 않고 숫자만 보여준다
import type { MergeReport, SyncApplied, SyncBundleInfo, SyncPreview, SyncStatus } from "../bindings";

type Args = Record<string, unknown>;
type Handler = (args: Args) => unknown;

const HOUR = 3600_000;
const status: SyncStatus = {
	deviceId: "3f9a1c2e-7b44-4d1a-9c0e-5a6b7c8d9e0f",
	peers: [{ deviceId: "a81d00c4-1e2f-4a3b-8c9d-0e1f2a3b4c5d", lastMergeAt: Date.now() - 26 * HOUR, merges: 3 }],
	openConflicts: 1,
};
const report: MergeReport = {
	added: 12,
	updated: 4,
	unchanged: 230,
	deleted: 1,
	keptLocalEdits: 0,
	restored: 1,
	conflicts: 2,
	branches: 1,
	skipped: 0,
	settingsApplied: true,
	dryRun: true,
};

/** 파일 대화상자: 브라우저에는 없으므로 동기화 파일(.scsync)일 때만 가짜 경로를 돌려준다 (다른 파일은 지금처럼 "앱에서만 됨") */
const isSync = (args: Args) => JSON.stringify(args).includes("scsync");
function dialog(args: Args, path: string) {
	if (!isSync(args)) throw new Error("dialog 없음");
	return path;
}

export const syncHandlers: Record<string, Handler> = {
	"plugin:dialog|open": (args) => dialog(args, "C:/Users/me/Downloads/sandcastle-sync-20261117-1200.scsync"),
	"plugin:dialog|save": (args) => dialog(args, "C:/Users/me/Downloads/sandcastle-sync.scsync"),
	get_sync_status: () => status,
	export_sync_bundle: ({ includeFiles }): SyncBundleInfo => ({
		deviceId: status.deviceId,
		createdAt: Date.now(),
		appVersion: "0.1.0",
		rows: 248,
		tombstones: 3,
		files: 41,
		blobs: includeFiles ? 41 : 0,
		bytes: includeFiles ? 8_400_000 : 120_000,
	}),
	preview_sync_bundle: (): SyncPreview => ({
		bundle: { deviceId: status.peers[0].deviceId, createdAt: Date.now() - 2 * HOUR, appVersion: "0.1.0", rows: 250, tombstones: 2, files: 44, blobs: 3, bytes: 2_100_000 },
		report,
		files: { added: 3, missing: 1, keptLocal: 0 },
	}),
	apply_sync_bundle: (): SyncApplied => {
		status.peers[0] = { ...status.peers[0], lastMergeAt: Date.now(), merges: status.peers[0].merges + 1 };
		status.openConflicts += report.conflicts + report.branches;
		return { report: { ...report, dryRun: false }, files: { added: 3, missing: 1, keptLocal: 0 }, backup: "sync-backups/app-20261117-120000-000.db" };
	},
};
