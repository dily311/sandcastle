// 동기화 가짜 응답 (2-B-3). 브라우저 미리보기에서는 실제로 합치지 않고 숫자만 보여준다
import type { MergeReport, SyncApplied, SyncBundleInfo, SyncPreview, SyncRun, SyncRunReport, SyncStatus } from "../bindings";

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
const isSync = (args: Args) => JSON.stringify(args).includes("scsync") || JSON.stringify(args).includes('"directory":true');
function dialog(args: Args, path: string) {
	if (!isSync(args)) throw new Error("dialog 없음");
	return path;
}

const runReport = (merged: number): SyncRunReport => ({
	merged: Array.from({ length: merged }, (_, i) => ({ deviceId: status.peers[0].deviceId, createdAt: Date.now() - i * HOUR, report: { ...report, added: 3, updated: 1, conflicts: 0, branches: 0, dryRun: false } })),
	unchangedDevices: merged ? 0 : 1,
	filesDown: merged * 2,
	filesMissing: 0,
	filesUp: 1,
	tombstonesCleaned: 0,
	backup: merged ? "sync-backups/app-20261117-120000-000.db" : null,
});
const runs: SyncRun[] = [
	{ id: "r2", target: "folder", runBy: "auto", startedAt: Date.now() - 3 * HOUR, finishedAt: Date.now() - 3 * HOUR + 4000, ok: false, error: "동기화 폴더에서 읽지 못함 (devices/a81d….scsync): 다른 프로세스가 파일을 사용 중", summary: null },
	{ id: "r1", target: "folder", runBy: "user", startedAt: Date.now() - 26 * HOUR, finishedAt: Date.now() - 26 * HOUR + 6000, ok: true, error: null, summary: runReport(1) },
];

export const syncHandlers: Record<string, Handler> = {
	sync_now: async ({ auto }) => {
		const r = runReport(runs.length % 2);
		await new Promise((res) => setTimeout(res, 900));
		runs.unshift({ id: `r${runs.length + 1}`, target: "folder", runBy: auto ? "auto" : "user", startedAt: Date.now(), finishedAt: Date.now(), ok: true, error: null, summary: r });
		return r;
	},
	list_sync_runs: ({ limit }) => runs.slice(0, Number(limit)),
	"plugin:dialog|open": (args) => dialog(args, JSON.stringify(args).includes('"directory":true') ? "C:/Users/me/OneDrive/Sandcastle" : "C:/Users/me/Downloads/sandcastle-sync-20261117-1200.scsync"),
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
