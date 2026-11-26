// Android: AI 작업이 도는 동안만 포그라운드 서비스를 켜 둔다 (CLAUDE.md 1-C-2, gen/android … background/GenerationService.kt).
// 화면이 꺼지거나 앱이 뒤로 가도 응답 생성이 끊기지 않게. 켜져 있는 동안 알림 줄에 진행 상태가 보인다(Observability).
// 동기화·드라이브 삭제 진행률은 syncProgress.svelte.ts가 따로 updateSync/stopSync로 다룬다.
import { tr } from "$lib/utils/i18n";
import { call, commands } from "$shared/ipc/client";
import { onEvent } from "$shared/ipc/events";

type Native = { postMessage(msg: string): void };
const native = () => (window as unknown as { SandcastleNative?: Native }).SandcastleNative;

const START = new Set(["queued", "started"]);
const END = new Set(["completed", "failed", "cancelled", "interrupted"]);
let running = false;
let stopTimer: ReturnType<typeof setTimeout> | undefined;

function start() {
	clearTimeout(stopTimer);
	if (running) return;
	running = true;
	native()?.postMessage(`keepalive:start:${tr("appMisc.keepAliveGenerating")}`);
}

/** 작업이 끝나면 잠깐 기다렸다가(이어지는 작업이 있을 수 있음) 남은 작업이 없을 때만 끈다 */
function scheduleStop() {
	clearTimeout(stopTimer);
	stopTimer = setTimeout(async () => {
		const active = await call(commands.listActiveTasks(), { source: "keep-alive", silent: true }).catch(() => []);
		if (active.length || !running) return;
		running = false;
		native()?.postMessage("keepalive:stop");
	}, 2000);
}

export function watchKeepAlive() {
	if (!native()) return;
	void onEvent<{ type: string; data?: { taskType?: string } }>("task-event", (e) => {
		if (e.data?.taskType === "background_sync") return;
		if (START.has(e.type)) start();
		else if (END.has(e.type)) scheduleStop();
	});
	// 페이지를 다시 연 경우: 이미 도는 작업이 있으면 켜 두고, 없으면 혹시 켜져 있던 것을 끈다
	void call(commands.listActiveTasks(), { source: "keep-alive", silent: true })
		.then((a) => (a.some((t) => t.taskType !== "background_sync") ? start() : native()?.postMessage("keepalive:stop")))
		.catch(() => {});
}
