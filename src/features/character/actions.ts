// 확인·파일 대화상자가 끼는 대화 동작 (카드·메뉴·패널이 같이 씀)
import { TRASH_HINT } from "$shared/trash";
import { nav } from "$app/nav.svelte";
import { exportPicker, type ExportFormat } from "$app/exportFormats.svelte";
import { call, commands } from "$shared/ipc/client";
import { PACK_FILES, saveTo } from "$shared/ipc/dialog";
import { confirmAction } from "$shared/stores/confirm.svelte";
import { toasts } from "$shared/stores/toast.svelte";
import { library, type Character, type Conversation } from "./library.svelte";

export type ChatFormat = "jsonl" | "txt" | "zip";
export const CHAT_FORMATS: ExportFormat<ChatFormat>[] = [
	{ id: "jsonl", label: "SillyTavern 채팅 (.jsonl)", desc: "SillyTavern에서 바로 불러올 수 있음" },
	{ id: "txt", label: "텍스트 (.txt)", desc: "읽기용. 이름과 대사만" },
	{ id: "zip", label: "캐릭터 묶음 (.zip)", desc: "캐릭터·이미지와 함께, 샌드캐슬끼리 옮길 때" },
];
export type CharacterFormat = "zip" | "png" | "json";
export const CHARACTER_FORMATS: ExportFormat<CharacterFormat>[] = [
	{ id: "zip", label: "캐릭터 묶음 (.zip)", desc: "대화·이미지·로어북까지 모두, 샌드캐슬끼리 옮길 때" },
	{ id: "png", label: "SillyTavern 카드 (.png)", desc: "아바타 그림에 카드를 넣음. 로어북 포함" },
	{ id: "json", label: "SillyTavern 카드 (.json)", desc: "카드 데이터만. 로어북 포함" },
];

export const convActions = {
	async remove(conv: Conversation) {
		const ok = await confirmAction({
			title: "대화 삭제",
			message: `'${conv.title}'와 메시지 ${conv.messageCount}개를 휴지통으로 옮김. ${TRASH_HINT}`,
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
		const n = await saveTo(`${ch?.name ?? "chat"} - ${conv.title}.${format}`, [filter], (path) => call(commands.exportConversation(conv.id, format, path), { source: "character" }));
		if (n == null) return;
		toasts.success(`메시지 ${n}개 내보냄`);
	},

	/** 고른 형식으로 내보내기 */
	exportFormat(conv: Conversation, format: ChatFormat) {
		return format === "zip" ? convActions.export(conv) : convActions.exportAs(conv, format);
	},
	/** 메뉴의 '내보내기…': 누른 자리에 형식 목록 */
	exportMenu(conv: Conversation) {
		exportPicker.open("대화 내보내기", "sc.export.chat", CHAT_FORMATS, (f: ChatFormat) => void convActions.exportFormat(conv, f));
	},

	/** 대화 하나를 캐릭터 묶음(zip)으로 내보내기 */
	async export(conv: Conversation) {
		const ch = library.character(conv.characterId);
		const r = await saveTo(`${ch?.name ?? "chat"} - ${conv.title}.zip`, [PACK_FILES], (path) => call(commands.exportCharacterPack([conv.characterId], [conv.id], path), { source: "character" }));
		if (r == null) return;
		toasts.success("내보냄");
	},
};

export const charActions = {
	async export(c: Character, format: CharacterFormat) {
		if (format === "zip") {
			const r = await saveTo(`${c.name}.zip`, [PACK_FILES], (path) => call(commands.exportCharacterPack([c.id], null, path), { source: "character" }));
			if (r == null) return;
		} else {
			const filter = format === "png" ? { name: "SillyTavern 카드", extensions: ["png"] } : { name: "SillyTavern 카드", extensions: ["json"] };
			const r = await saveTo(`${c.name}.${format}`, [filter], async (path) => {
				await call(commands.exportCharacterCard(c.id, format, path), { source: "character" });
				return true;
			});
			if (!r) return;
		}
		toasts.success("내보냄");
	},
	exportMenu(c: Character) {
		exportPicker.open("캐릭터 내보내기", "sc.export.character", CHARACTER_FORMATS, (f: CharacterFormat) => void charActions.export(c, f));
	},
};
