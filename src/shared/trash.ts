// 휴지통 (CLAUDE.md 2-A-3). 지우기 커맨드는 항목을 휴지통으로 옮기고 TrashItem을 돌려준다.
// 지운 직후 "되돌리기" 토스트를 띄우고, 누르면 되살린다. 휴지통 목록은 설정 > 데이터 > 휴지통
import { get } from "svelte/store";
import { _ } from "svelte-i18n";
import { jobs } from "$shared/stores/jobs.svelte";
import type { TrashItem, TrashKind } from "$ipc/bindings";
import { call, commands } from "$shared/ipc/client";
import { toasts } from "$shared/stores/toast.svelte";

const t = (key: string, values?: Record<string, string | number>) => get(_)(key, values ? { values } : undefined);

export const TRASH_KIND_LABEL: Record<TrashKind, string> = new Proxy({} as Record<TrashKind, string>, {
	get(_target, prop: string) {
		return t(`settings.trash.kind.${prop}`);
	},
});

/** 휴지통에서 되살린다. 실패하면(연결된 항목이 없음 등) call이 이유를 토스트로 보여 준다 */
export async function restoreFromTrash(id: string): Promise<TrashItem | null> {
	const item = await jobs.run(t("settings.trash.trashItem"), () => call(commands.restoreTrashItem(id), { source: "trash" }), { group: t("settings.trash.restoreGroup") }).catch(() => null);
	return item ?? null;
}

/** 지운 뒤 "휴지통으로 옮김 · 되돌리기" 알림. 되살린 뒤 목록을 다시 읽을 함수를 넘긴다 */
export function trashedToast(item: TrashItem, onRestored?: () => unknown) {
	toasts.show(
		{
			tone: "success",
			message: t("settings.trash.trashedToast", { kind: TRASH_KIND_LABEL[item.kind], label: item.label }),
			action: {
				label: t("settings.trash.undo"),
				run: async () => {
					if (!(await restoreFromTrash(item.id))) return;
					toasts.success(t("settings.trash.restoredToast", { label: item.label }));
					await onRestored?.();
				},
			},
		},
		6000,
	);
}

/** 확인 대화상자 문구: 지우기가 휴지통으로 간다는 것 */
export function trashHint(): string {
	return t("settings.trash.hint");
}
