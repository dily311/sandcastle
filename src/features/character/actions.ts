// 확인·파일 대화상자가 끼는 대화 동작 (카드·메뉴·패널이 같이 씀)
import { nav } from "$app/nav.svelte";
import { call, commands } from "$shared/ipc/client";
import { PACK_FILES, pickSavePath } from "$shared/ipc/dialog";
import { confirmAction } from "$shared/stores/confirm.svelte";
import { toasts } from "$shared/stores/toast.svelte";
import { library, type Conversation } from "./library.svelte";

export const convActions = {
	async remove(conv: Conversation) {
		const ok = await confirmAction({
			title: "대화 삭제",
			message: `'${conv.title}'와 메시지 ${conv.messageCount}개를 지움. 되돌릴 수 없음`,
			confirmLabel: "삭제",
			danger: true,
		});
		if (!ok) return;
		await library.remove(conv.id);
		// 보고 있던 대화를 지웠으면 캐릭터 화면으로
		const cur = nav.current;
		if (cur.page === "character" && cur.convId === conv.id) nav.replace({ page: "character", charId: conv.characterId });
	},

	/** 대화 하나를 SillyTavern 채팅(jsonl) 또는 읽기용 글(txt)로 */
	async exportAs(conv: Conversation, format: "jsonl" | "txt") {
		const ch = library.character(conv.characterId);
		const filter = format === "jsonl" ? { name: "SillyTavern 채팅", extensions: ["jsonl"] } : { name: "텍스트", extensions: ["txt"] };
		const path = await pickSavePath(`${ch?.name ?? "chat"} - ${conv.title}.${format}`, [filter]);
		if (!path) return;
		const n = await call(commands.exportConversation(conv.id, format, path), { source: "character" });
		toasts.success(`메시지 ${n}개 내보냄`);
	},

	/** 대화 하나를 캐릭터 묶음(zip)으로 내보내기 */
	async export(conv: Conversation) {
		const ch = library.character(conv.characterId);
		const path = await pickSavePath(`${ch?.name ?? "chat"} - ${conv.title}.zip`, [PACK_FILES]);
		if (!path) return;
		await call(commands.exportCharacterPack([conv.characterId], [conv.id], path), { source: "character" });
		toasts.success("내보냄");
	},
};
