// 실행 기록의 오류를 로그 탭에도 (설정 > 관리 > 로그). 작업이 실패하거나 다시 시도할 때 한 줄씩.
// 앱이 켜져 있는 동안만 듣는다 (로그 탭은 이번 실행의 기록)
// 예산 한도로 막힌 작업은 알림도 띄운다: 사용자 실행이면 "이번만 계속" (ADR-007 — 다시 실행은 사용자가 누름)
import type { TaskErrorInfo } from "$ipc/bindings";
import { call, commands } from "$shared/ipc/client";
import { onEvent } from "$shared/ipc/events";
import { logs } from "./log.svelte";
import { toasts } from "./toast.svelte";

const FEATURE: Record<string, string> = { chat: "대화", phone: "폰", game: "게임", translate: "번역", tts: "음성", imagegen: "이미지", settings: "설정" };

type Ev = { type: string; data: { taskId: string; attemptNo?: number; error?: TaskErrorInfo } };

let started = false;

export function watchTaskErrors() {
	if (started) return;
	started = true;
	void onEvent<Ev>("task-event", async (e) => {
		if (e.type === "completed" || e.type === "failed") void checkBudget();
		if (e.type !== "failed" && e.type !== "retrying") return;
		const err = e.data.error;
		const task = await call(commands.getTask(e.data.taskId), { source: "task", silent: true }).catch(() => null);
		const who = task ? `${FEATURE[task.feature ?? ""] ?? (task.feature?.startsWith("plugin:") ? "플러그인" : task.feature) ?? "작업"}${task.label ? ` · ${task.label}` : ""}` : "작업";
		const auto = task?.runBy === "auto" ? " (자동)" : "";
		const details = { taskId: e.data.taskId, code: err?.code, retryable: err?.retryable, attempt: e.data.attemptNo, feature: task?.feature, runBy: task?.runBy };
		if (e.type === "failed" && err?.code === "BUDGET_EXCEEDED") budgetToast(err.message, task?.runBy !== "auto");
		if (e.type === "failed") logs.error("task", `${who}${auto} 실패: ${err?.message ?? "알 수 없는 오류"}`, details);
		else logs.warn("task", `${who}${auto} 다시 시도 (${e.data.attemptNo}번째): ${err?.message ?? ""}`, details);
	});
}

function budgetToast(message: string, canOverride: boolean) {
	toasts.show(
		{
			tone: "warning",
			message,
			detail: canOverride ? "한도는 설정 > 관리 > 실행 기록에서 바꿈" : undefined,
			action: canOverride
				? {
						label: "이번만 계속",
						run: () =>
							void call(commands.allowBudgetOnce(), { source: "budget" }).then(() => toasts.info("2분 안에 다시 실행하면 이번 한 번은 한도를 넘어 실행함")),
					}
				: undefined,
		},
		10000,
	);
}

// 한도의 80%에 새로 닿으면 한 번 알림 (작업이 끝날 때 묻는다, 요금 없음). 몰려도 한 번만
const METRIC: Record<string, string> = { tokens: "토큰", characters: "음성 글자", images: "그림" };
let budgetTimer: ReturnType<typeof setTimeout> | undefined;
function checkBudget() {
	clearTimeout(budgetTimer);
	budgetTimer = setTimeout(async () => {
		const list = await call(commands.takeBudgetWarnings(), { source: "budget", silent: true }).catch(() => []);
		for (const w of list) {
			const m = w.meter;
			const when = m.period === "day" ? "오늘" : "이번 달";
			const p = Math.round((m.used / m.limit) * 100);
			toasts.show({ tone: "warning", message: `${w.profileName ?? "API"} ${when} ${METRIC[m.metric] ?? m.metric} 한도의 ${p}%를 씀 (${m.used.toLocaleString()} / ${m.limit.toLocaleString()})`, detail: m.used >= m.limit ? "자동 실행은 이제 막힘" : undefined }, 8000);
		}
	}, 500);
}
