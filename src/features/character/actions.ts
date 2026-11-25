// 확인·파일 대화상자가 끼는 대화 동작 (카드·메뉴·패널이 같이 씀)
import { get } from "svelte/store";
import { _ } from "svelte-i18n";
import { trashHint } from "$shared/trash";
import { nav } from "$app/nav.svelte";
import { exportPicker, type ExportFormat } from "$app/exportFormats.svelte";
import { call, commands } from "$shared/ipc/client";
import { PACK_FILES, saveTo } from "$shared/ipc/dialog";
import { confirmAction } from "$shared/stores/confirm.svelte";
import { toasts } from "$shared/stores/toast.svelte";
import { library, type Character, type Conversation } from "./library.svelte";

const t = (key: string, values?: Record<string, string | number>) => get(_)(key, values ? { values } : undefined);

export type ChatFormat = "jsonl" | "jsonl-img" | "txt" | "zip";
export function getChatFormats(tr: (k: string) => string = t): ExportFormat<ChatFormat>[] {
	return [
		{ id: "jsonl", label: tr("uiPolish.exportJsonlLabel"), desc: tr("uiPolish.exportJsonlDesc") },
		{ id: "jsonl-img", label: tr("uiPolish.exportJsonlImgLabel"), desc: tr("uiPolish.exportJsonlImgDesc") },
		{ id: "txt", label: tr("uiPolish.exportTxtLabel"), desc: tr("uiPolish.exportTxtDesc") },
		{ id: "zip", label: tr("uiPolish.exportZipChatLabel"), desc: tr("uiPolish.exportZipChatDesc") },
	];
}
export type CharacterFormat = "zip" | "png" | "json";
export function getCharacterFormats(tr: (k: string) => string = t): ExportFormat<CharacterFormat>[] {
	return [
		{ id: "zip", label: tr("uiPolish.exportZipCharLabel"), desc: tr("uiPolish.exportZipCharDesc") },
		{ id: "png", label: tr("uiPolish.exportPngLabel"), desc: tr("uiPolish.exportPngDesc") },
		{ id: "json", label: tr("uiPolish.exportJsonLabel"), desc: tr("uiPolish.exportJsonDesc") },
	];
}

export const convActions = {
	async remove(conv: Conversation) {
		const ok = await confirmAction({
			title: t("uiPolish.deleteConvTitle"),
			message: t("uiPolish.deleteConvMsg", { title: conv.title, count: conv.messageCount, hint: trashHint() }),
			confirmLabel: t("common.delete"),
			danger: true,
		});
		if (!ok) return;
		await library.remove(conv.id);
		// 보고 있던 대화를 지웠으면 캐릭터 화면으로
		const cur = nav.current;
		if (cur.page === "character" && cur.convId === conv.id) nav.replace({ page: "character", charId: conv.characterId });
	},

	/** 대화 하나를 SillyTavern 채팅(jsonl) 또는 읽기용 글(txt)로 */
	async exportAs(conv: Conversation, format: "jsonl" | "jsonl-img" | "txt") {
		const ch = library.character(conv.characterId);
		const ext = format === "txt" ? "txt" : "jsonl";
		const filter = ext === "jsonl" ? { name: t("uiPolish.exportChatFilterJsonl"), extensions: ["jsonl"] } : { name: t("uiPolish.exportChatFilterTxt"), extensions: ["txt"] };
		const n = await saveTo(`${ch?.name ?? "chat"} - ${conv.title}.${ext}`, [filter], (path) => call(commands.exportConversation(conv.id, format, path), { source: "character" }));
		if (n == null) return;
		toasts.success(t("uiPolish.exportMsgCountToast", { count: n }));
	},

	/** 고른 형식으로 내보내기 */
	exportFormat(conv: Conversation, format: ChatFormat) {
		return format === "zip" ? convActions.export(conv) : convActions.exportAs(conv, format);
	},
	/** 메뉴의 '내보내기…': 누른 자리에 형식 목록 */
	exportMenu(conv: Conversation) {
		exportPicker.open(t("uiPolish.exportChatTitle"), "sc.export.chat", getChatFormats(), (f: ChatFormat) => void convActions.exportFormat(conv, f));
	},

	/** 대화 하나를 캐릭터 묶음(zip)으로 내보내기 */
	async export(conv: Conversation) {
		const ch = library.character(conv.characterId);
		const r = await saveTo(`${ch?.name ?? "chat"} - ${conv.title}.zip`, [PACK_FILES], (path) => call(commands.exportCharacterPack([conv.characterId], [conv.id], path), { source: "character" }));
		if (r == null) return;
		toasts.success(t("uiPolish.exportedToast"));
	},
};

export const charActions = {
	async export(c: Character, format: CharacterFormat) {
		if (format === "zip") {
			const r = await saveTo(`${c.name}.zip`, [PACK_FILES], (path) => call(commands.exportCharacterPack([c.id], null, path), { source: "character" }));
			if (r == null) return;
		} else {
			const filter = format === "png" ? { name: t("uiPolish.exportCharFilterCard"), extensions: ["png"] } : { name: t("uiPolish.exportCharFilterCard"), extensions: ["json"] };
			const r = await saveTo(`${c.name}.${format}`, [filter], async (path) => {
				await call(commands.exportCharacterCard(c.id, format, path), { source: "character" });
				return true;
			});
			if (!r) return;
		}
		toasts.success(t("uiPolish.exportedToast"));
	},
	exportMenu(c: Character) {
		exportPicker.open(t("uiPolish.exportCharTitle"), "sc.export.character", getCharacterFormats(), (f: CharacterFormat) => void charActions.export(c, f));
	},
};
