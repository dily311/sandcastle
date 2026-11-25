// 실행 기록의 오류를 로그 탭에도 (설정 > 관리 > 로그). 작업이 실패하거나 다시 시도할 때 한 줄씩.
// 앱이 켜져 있는 동안만 듣는다 (로그 탭은 이번 실행의 기록)
// 예산 한도로 막힌 작업은 알림도 띄운다: 사용자 실행이면 "이번만 계속" (ADR-007 — 다시 실행은 사용자가 누름)
import { get } from "svelte/store";
import { _ } from "svelte-i18n";
import type { TaskErrorInfo } from "$ipc/bindings";
import { localizeBackendError, localizeTaskFeature } from "$lib/utils/backendI18n";
import { call, commands } from "$shared/ipc/client";
import { onEvent } from "$shared/ipc/events";
import { logs } from "./log.svelte";
import { toasts } from "./toast.svelte";

const t = (key: string, values?: Record<string, string | number>) => get(_)(key, values ? { values } : undefined);

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
		const who = task ? `${localizeTaskFeature(task.feature)}${task.label ? ` · ${task.label}` : ""}` : t("tasks.feature.default");
		const auto = task?.runBy === "auto" ? t("tasks.auto") : "";
		const message = err ? localizeBackendError({ code: err.code, message: err.message }) : t("errors.UNKNOWN");
		const details = { taskId: e.data.taskId, code: err?.code, retryable: err?.retryable, attempt: e.data.attemptNo, feature: task?.feature, runBy: task?.runBy };
		if (e.type === "failed" && err?.code === "BUDGET_EXCEEDED") budgetToast(message, task?.runBy !== "auto");
		if (e.type === "failed") logs.error("task", t("tasks.failed", { who, auto, message }), details);
		else logs.warn("task", t("tasks.retrying", { who, auto, attempt: e.data.attemptNo ?? 1, message }), details);
	});
}

function budgetToast(message: string, canOverride: boolean) {
	toasts.show(
		{
			tone: "warning",
			message,
			detail: canOverride ? t("tasks.budgetDetail") : undefined,
			action: canOverride
				? {
						label: t("tasks.budgetContinueOnce"),
						run: () =>
							void call(commands.allowBudgetOnce(), { source: "budget" }).then(() => toasts.info(t("tasks.budgetOverrideToast"))),
					}
				: undefined,
		},
		10000,
	);
}

// 한도의 80%에 새로 닿으면 한 번 알림 (작업이 끝날 때 묻는다, 요금 없음). 몰려도 한 번만
let budgetTimer: ReturnType<typeof setTimeout> | undefined;
function checkBudget() {
	clearTimeout(budgetTimer);
	budgetTimer = setTimeout(async () => {
		const list = await call(commands.takeBudgetWarnings(), { source: "budget", silent: true }).catch(() => []);
		for (const w of list) {
			const m = { usedUsd: w.meter.usedUsd ?? 0, limitUsd: w.meter.limitUsd ?? 0 };
			const p = Math.round((m.usedUsd / m.limitUsd) * 100);
			toasts.show(
				{
					tone: "warning",
					message: t("tasks.budgetWarning", { profile: w.profileName ?? "API", pct: p, used: m.usedUsd.toFixed(2), limit: m.limitUsd.toFixed(2) }),
					detail: m.usedUsd >= m.limitUsd ? t("tasks.budgetAutoBlocked") : undefined,
				},
				8000,
			);
		}
	}, 500);
}
