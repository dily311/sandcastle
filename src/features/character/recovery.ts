// 앱이 생성 중에 꺼져서 멈춘 채팅 응답(작업 상태 interrupted) 처리.
// 시작할 때 한 번 묻고, 사용자가 고르면 다시 만들거나(resume_task) 버린다(dismiss_task).
// 다시 만든 응답은 백엔드가 저장하고 chat:dialogue:done을 보내므로, 열린 대화는 그 이벤트로 새로 불러온다.
import { get } from "svelte/store";
import { _ } from "svelte-i18n";
import type { TaskInfo_Serialize } from "$ipc/bindings";
import { call, commands } from "$shared/ipc/client";
import { confirmAction } from "$shared/stores/confirm.svelte";
import { toasts } from "$shared/stores/toast.svelte";

const src = { source: "task" };

export function interruptedChats(tasks: TaskInfo_Serialize[]): TaskInfo_Serialize[] {
	return tasks.filter((t) => t.taskType === "chat_stream" && t.status === "interrupted" && t.feature !== "rewrite");
}

export async function recoverInterruptedChats() {
	const all = await call(commands.getRecoverableTasks(), { ...src, silent: true }).catch(() => []);
	// 앱이 꺼지며 끊긴 자동 재생성은 이어서 만들지 않고 조용히 버린다 (사용자가 시킨 것이 아니라 정책이 낸 작업)
	for (const t of all.filter((x) => x.taskType === "chat_stream" && x.status === "interrupted" && x.feature === "rewrite")) {
		await call(commands.dismissTask(t.id), { ...src, silent: true }).catch(() => {});
	}
	const tasks = interruptedChats(all);
	if (!tasks.length) return;

	const tr = get(_);
	const retry = await confirmAction({
		title: tr("chatExtra.recovery.title"),
		message: tr("chatExtra.recovery.message", { values: { count: tasks.length } }),
		confirmLabel: tr("chatExtra.recovery.confirm"),
	});
	let failed = 0;
	for (const t of tasks) {
		try {
			await call(retry ? commands.resumeTask(t.id) : commands.dismissTask(t.id), { ...src, silent: true });
		} catch {
			failed++;
		}
	}
	if (failed) toasts.error(get(_)("chatExtra.recovery.failedToast", { values: { count: failed } }));
	else if (retry) toasts.info(get(_)("chatExtra.recovery.resumingToast"));
}
