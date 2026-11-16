// 실행 기록의 오류를 로그 탭에도 (설정 > 관리 > 로그). 작업이 실패하거나 다시 시도할 때 한 줄씩.
// 앱이 켜져 있는 동안만 듣는다 (로그 탭은 이번 실행의 기록)
import type { TaskErrorInfo } from "$ipc/bindings";
import { call, commands } from "$shared/ipc/client";
import { onEvent } from "$shared/ipc/events";
import { logs } from "./log.svelte";

const FEATURE: Record<string, string> = { chat: "대화", phone: "폰", game: "게임", translate: "번역", tts: "음성", imagegen: "이미지", settings: "설정" };

type Ev = { type: string; data: { taskId: string; attemptNo?: number; error?: TaskErrorInfo } };

let started = false;

export function watchTaskErrors() {
	if (started) return;
	started = true;
	void onEvent<Ev>("task-event", async (e) => {
		if (e.type !== "failed" && e.type !== "retrying") return;
		const err = e.data.error;
		const task = await call(commands.getTask(e.data.taskId), { source: "task", silent: true }).catch(() => null);
		const who = task ? `${FEATURE[task.feature ?? ""] ?? task.feature ?? "작업"}${task.label ? ` · ${task.label}` : ""}` : "작업";
		const auto = task?.runBy === "auto" ? " (자동)" : "";
		const details = { taskId: e.data.taskId, code: err?.code, retryable: err?.retryable, attempt: e.data.attemptNo, feature: task?.feature, runBy: task?.runBy };
		if (e.type === "failed") logs.error("task", `${who}${auto} 실패: ${err?.message ?? "알 수 없는 오류"}`, details);
		else logs.warn("task", `${who}${auto} 다시 시도 (${e.data.attemptNo}번째): ${err?.message ?? ""}`, details);
	});
}
