// 코어 플러그인의 AI 도구 (번역 · 음성 읽기 · 이미지 생성) 화면 쪽.
// - 설정: pluginSetting (백엔드 plugin-setting 저장소)
// - 메시지에 붙는 결과는 백엔드가 meta에 저장하고, 여기서는 고쳐진 메시지를 돌려준다
// - 새 AI 답변이 오면 afterReply가 자동 번역·자동 읽기를 한다 (설정에서 켰을 때)
import type { Message_Serialize as Message, PluginSetting } from "$ipc/bindings";
import { call, commands } from "$shared/ipc/client";
import { onEvent } from "$shared/ipc/events";
import { fileSrc } from "$shared/ipc/file";
import { plugins } from "./plugins.svelte";

const src = { source: "plugins" };

class PluginSettingStore {
	value = $state<PluginSetting | null>(null);
	private listening = false;

	async load() {
		this.value = await call(commands.getPluginSetting(), { ...src, silent: true }).catch(() => null);
		if (!this.listening) {
			this.listening = true;
			void onEvent<PluginSetting>("plugin_setting::changed", (v) => (this.value = v));
		}
	}

	async save(next: PluginSetting) {
		this.value = next;
		this.value = await call(commands.updatePluginSetting(next), src);
	}
}

export const pluginSetting = new PluginSettingStore();

/** 메시지 meta의 번역 (원문이 그대로일 때만 유효) */
export function translationOf(m: Message): { lang: string; text: string } | null {
	const t = (m.meta as { translation?: { lang: string; text: string; source: string } } | null)?.translation;
	return t && t.source === m.content && t.text ? { lang: t.lang, text: t.text } : null;
}

/** 메시지에 그린 그림들 */
export function generatedImagesOf(m: Message): { path: string; prompt: string }[] {
	const list = (m.meta as { generatedImages?: { path: string; prompt: string }[] } | null)?.generatedImages;
	return Array.isArray(list) ? list : [];
}

export async function translateMessage(m: Message): Promise<Message> {
	return call(commands.translateMessage(m.id), src);
}

/** 입력창: 보내기 전에 입력 언어로 번역 */
export async function translateInput(text: string): Promise<string> {
	return call(commands.translateText(text, pluginSetting.value?.translate?.inputLang ?? null), src);
}

export async function drawMessage(m: Message): Promise<Message> {
	return call(commands.generateMessageImage(m.id), src);
}

/** 글(게임 대사 등)로 그림 만들기. 채팅 AI가 장면 프롬프트로 다듬는다 */
export async function drawText(text: string, source: "game" | "manual" = "game") {
	return call(commands.generateImageFromPrompt(text, true, source), src);
}

/** 글 번역 (저장하지 않음: 게임 대사 줄 등) */
export async function translateText(text: string): Promise<string> {
	return call(commands.translateText(text, null), src);
}

// ---- 음성: 한 번에 하나만 재생 ----
let playing: HTMLAudioElement | null = null;
export const speech = $state<{ messageId: string | null; loading: boolean }>({ messageId: null, loading: false });

export function stopSpeaking() {
	playing?.pause();
	playing = null;
	speech.messageId = null;
}

export async function speak(m: { id: string; content: string }, text = m.content) {
	if (speech.messageId === m.id) return stopSpeaking();
	stopSpeaking();
	speech.messageId = m.id;
	speech.loading = true;
	try {
		const path = await call(commands.speakText(text), src);
		if (speech.messageId !== m.id) return; // 기다리는 동안 다른 걸 눌렀음
		const audio = new Audio(fileSrc(path) ?? "");
		playing = audio;
		audio.onended = () => speech.messageId === m.id && stopSpeaking();
		await audio.play();
	} catch {
		if (speech.messageId === m.id) stopSpeaking();
	} finally {
		speech.loading = false;
	}
}

/**
 * 새 AI 답변이 저장된 뒤 (대화·폰 공통). 자동 번역이 켜져 있으면 번역해 고쳐진 메시지를 돌려주고,
 * 자동 읽기가 켜져 있으면 읽기 시작한다 (번역이 있으면 번역문을 읽는다).
 */
export async function afterReply(m: Message): Promise<Message> {
	if (!pluginSetting.value) await pluginSetting.load();
	const s = pluginSetting.value;
	let out = m;
	if (s && plugins.on("translate") && s.translate?.autoTranslate) {
		out = await translateMessage(m).catch(() => m);
	}
	if (s && plugins.on("tts") && s.tts?.autoRead) {
		void speak(out, translationOf(out)?.text ?? out.content);
	}
	return out;
}
