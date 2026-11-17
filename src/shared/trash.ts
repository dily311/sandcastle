// 휴지통 (CLAUDE.md 2-A-3). 지우기 커맨드는 항목을 휴지통으로 옮기고 TrashItem을 돌려준다.
// 지운 직후 "되돌리기" 토스트를 띄우고, 누르면 되살린다. 휴지통 목록은 설정 > 데이터 > 휴지통
import type { TrashItem, TrashKind } from "$ipc/bindings";
import { call, commands } from "$shared/ipc/client";
import { toasts } from "$shared/stores/toast.svelte";

export const TRASH_KIND_LABEL: Record<TrashKind, string> = {
	character: "캐릭터",
	persona: "페르소나",
	conversation: "대화",
	worldinfo_book: "월드인포",
	prompt_preset: "프리셋",
	game: "게임",
	game_session: "플레이 기록",
};

/** 휴지통에서 되살린다. 실패하면(연결된 항목이 없음 등) call이 이유를 토스트로 보여 준다 */
export async function restoreFromTrash(id: string): Promise<TrashItem | null> {
	return call(commands.restoreTrashItem(id), { source: "trash" }).catch(() => null);
}

/** 지운 뒤 "휴지통으로 옮김 · 되돌리기" 알림. 되살린 뒤 목록을 다시 읽을 함수를 넘긴다 */
export function trashedToast(item: TrashItem, onRestored?: () => unknown) {
	toasts.show(
		{
			tone: "success",
			message: `${TRASH_KIND_LABEL[item.kind]} "${item.label}"을(를) 휴지통으로 옮김`,
			action: {
				label: "되돌리기",
				run: async () => {
					if (!(await restoreFromTrash(item.id))) return;
					toasts.success(`"${item.label}" 되살림`);
					await onRestored?.();
				},
			},
		},
		6000,
	);
}

/** 확인 대화상자 문구: 지우기가 휴지통으로 간다는 것 */
export const TRASH_HINT = "휴지통으로 옮김. 설정 > 데이터 > 휴지통에서 되살릴 수 있음";
