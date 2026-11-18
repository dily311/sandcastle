// 동기화 가짜 응답 (2-B-3). 브라우저 미리보기에서는 실제로 합치지 않고 숫자만 보여준다
import type { MergeReport, SyncApplied, SyncBackup, SyncBundleInfo, SyncConflict, SyncPreview, SyncRun, SyncRunReport, SyncStatus } from "../bindings";

type Args = Record<string, unknown>;
type Handler = (args: Args) => unknown;

const HOUR = 3600_000;
const status: SyncStatus = {
	deviceId: "3f9a1c2e-7b44-4d1a-9c0e-5a6b7c8d9e0f",
	peers: [{ deviceId: "a81d00c4-1e2f-4a3b-8c9d-0e1f2a3b4c5d", lastMergeAt: Date.now() - 26 * HOUR, merges: 3, theirAck: Date.now() - 30 * HOUR, stale: false, excluded: false },
		{ deviceId: "c2b4d6f8-0a1b-4c3d-8e5f-6a7b8c9d0e1f", lastMergeAt: Date.now() - 120 * 24 * HOUR, merges: 2, theirAck: null, stale: true, excluded: false },
	],
	openConflicts: 3,
	pendingRollback: null,
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
	merged: Array.from({ length: merged }, (_, i) => ({ deviceId: status.peers[0].deviceId, createdAt: Date.now() - i * HOUR, report: { ...report, added: 3, updated: 1, conflicts: 0, branches: 0, dryRun: false }, returnedAfterExclusion: false })),
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

const peer = "a81d00c4-1e2f-4a3b-8c9d-0e1f2a3b4c5d";
let conflicts: SyncConflict[] = [
	{
		id: "k1",
		kind: "edit",
		tbl: "characters",
		key: '["sera"]',
		label: "세라피나",
		deviceId: peer,
		createdAt: Date.now() - 2 * HOUR,
		diff: [
			{ column: "name", local: "세라피나", remote: "세라" },
			{ column: "description", local: "숲의 수호자. 상처를 치유한다.", remote: "숲의 수호자. 상처를 치유하고 길 잃은 이를 돕는다." },
		],
		remote: null,
	},
	{ id: "k2", kind: "branch", tbl: "conversations", key: '["c1"]', label: "숲에서 깨어남", deviceId: peer, createdAt: Date.now() - 2 * HOUR, diff: [], remote: { branchId: "c1", fromSeq: 9 } },
	{ id: "k3", kind: "unique", tbl: "worldinfo_entries", key: '["e9"]', label: "엘도리아 숲", deviceId: peer, createdAt: Date.now() - 5 * HOUR, diff: [], remote: { id: "e9", comment: "엘도리아 숲", content: "고대의 숲." } },
];
const backups: SyncBackup[] = [
	{ name: "app-20261117-120000-000.db", createdAt: Date.now() - 2 * HOUR, bytes: 3_400_000 },
	{ name: "app-20261116-090000-000.db", createdAt: Date.now() - 27 * HOUR, bytes: 3_300_000 },
];

const drive = { available: true, connected: false, email: null as string | null };

export const syncHandlers: Record<string, Handler> = {
	drive_status: () => ({ ...drive }),
	drive_login: async () => {
		await new Promise((r) => setTimeout(r, 1200)); // 브라우저 로그인 흉내
		Object.assign(drive, { connected: true, email: "me@example.com" });
		return { ...drive };
	},
	drive_logout: ({ deleteRemote }) => {
		Object.assign(drive, { connected: false, email: null });
		return deleteRemote ? 42 : 0;
	},
	list_sync_conflicts: () => conflicts,
	resolve_sync_conflict: ({ id }) => {
		conflicts = conflicts.filter((c) => c.id !== id);
		status.openConflicts = conflicts.length;
		return null;
	},
	list_sync_backups: () => backups,
	request_sync_rollback: ({ name }) => ((status.pendingRollback = String(name)), null),
	cancel_sync_rollback: () => ((status.pendingRollback = null), null),
	sync_now: async ({ auto }) => {
		const r = runReport(runs.length % 2);
		await new Promise((res) => setTimeout(res, 900));
		runs.unshift({ id: `r${runs.length + 1}`, target: "folder", runBy: auto ? "auto" : "user", startedAt: Date.now(), finishedAt: Date.now(), ok: true, error: null, summary: r });
		return r;
	},
	list_sync_runs: ({ limit }) => runs.slice(0, Number(limit)),
	// 커뮤니티 플러그인 설치(zip 고르기)는 예제 묶음 경로를 돌려준다 (미리보기 가짜 응답이 그 경로와 상관없이 예제를 보여 줌)
	"plugin:dialog|open": (args) => (JSON.stringify(args).includes("플러그인 묶음") ? "C:/Users/me/Downloads/word-count-1.0.0.zip" : dialog(args, JSON.stringify(args).includes('"directory":true') ? "C:/Users/me/OneDrive/Sandcastle" : "C:/Users/me/Downloads/sandcastle-sync-20261117-1200.scsync")),
	"plugin:dialog|save": (args) => dialog(args, "C:/Users/me/Downloads/sandcastle-sync.scsync"),
	get_sync_status: () => status,
	set_sync_peer_excluded: ({ deviceId, excluded }) => {
		const p = status.peers.find((x) => x.deviceId === deviceId);
		if (p) p.excluded = Boolean(excluded);
		return null;
	},
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
