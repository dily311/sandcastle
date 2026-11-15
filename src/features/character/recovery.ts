// 앱이 생성 중에 꺼져서 멈춘 채팅 응답(작업 상태 interrupted) 처리.
// 시작할 때 한 번 묻고, 사용자가 고르면 다시 만들거나(resume_task) 버린다(dismiss_task).
// 다시 만든 응답은 백엔드가 저장하고 chat:dialogue:done을 보내므로, 열린 대화는 그 이벤트로 새로 불러온다.
import type { TaskInfo_Serialize } from "$ipc/bindings";
import { call, commands } from "$shared/ipc/client";
import { confirmAction } from "$shared/stores/confirm.svelte";
import { toasts } from "$shared/stores/toast.svelte";

const src = { source: "task" };

export function interruptedChats(tasks: TaskInfo_Serialize[]): TaskInfo_Serialize[] {
	return tasks.filter((t) => t.taskType === "chat_stream" && t.status === "interrupted");
}

export async function recoverInterruptedChats() {
	const all = await call(commands.getRecoverableTasks(), { ...src, silent: true }).catch(() => []);
	const tasks = interruptedChats(all);
	if (!tasks.length) return;

	const retry = await confirmAction({
		title: "멈춘 응답",
		message: `앱이 꺼지면서 응답 ${tasks.length}개를 다 만들지 못했습니다. 다시 만들까요? 취소하면 버립니다.`,
		confirmLabel: "다시 만들기",
	});
	let failed = 0;
	for (const t of tasks) {
		try {
			await call(retry ? commands.resumeTask(t.id) : commands.dismissTask(t.id), { ...src, silent: true });
		} catch {
			failed++;
		}
	}
	if (failed) toasts.error(`멈춘 응답 ${failed}개를 처리하지 못했습니다`);
	else if (retry) toasts.info("응답을 다시 만드는 중입니다");
}
