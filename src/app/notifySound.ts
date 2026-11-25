// 백엔드 알림(notify:play-sound)에 맞춰 커스텀 알림음을 재생한다.
import type { NotifySoundPayload } from "$ipc/bindings";
import { onEvent } from "$shared/ipc/events";
import { fileSrc } from "$shared/ipc/file";
import { logs } from "$shared/stores/log.svelte";

let current: HTMLAudioElement | null = null;
let started = false;

export function playNotifySound(path: string, volume: number | null | undefined = 0.8) {
	const src = fileSrc(path);
	if (!src) return;
	if (current) {
		current.pause();
		current.currentTime = 0;
	}
	const v = volume ?? 0.8;
	const audio = new Audio(src);
	audio.volume = Math.max(0, Math.min(1, Number.isFinite(v) ? v : 0.8));
	current = audio;
	void audio.play().catch((e) => logs.warn("notify", `알림음 재생 실패: ${String(e)}`));
}

export function watchNotifySound() {
	if (started) return;
	started = true;
	void onEvent<NotifySoundPayload>("notify:play-sound", (payload) => {
		if (payload?.path) playNotifySound(payload.path, payload.volume);
	});
}
