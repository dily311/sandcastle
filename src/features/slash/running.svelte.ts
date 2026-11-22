// 대화마다 돌고 있는 슬래시 스크립트 (5-E-20-6-8): 입력칸의 멈추기 버튼·빠른 답장 줄이 본다.
// 멈추면 다음 커맨드부터 돌지 않고(엔진이 커맨드·바퀴마다 확인), 생성 중인 /gen·대필·대화 응답은 취소한다 (받은 데까지 저장).
import { call, commands } from "$shared/ipc/client";
import { ghost } from "$features/ghost/ghost.svelte";
import type { ChatSession } from "$features/character/session.svelte";

class SlashRuns {
	running = $state<Record<string, boolean>>({});
	private stops: Record<string, boolean> = {};

	isRunning(convId: string) {
		return !!this.running[convId];
	}
	start(convId: string) {
		this.stops[convId] = false;
		this.running[convId] = true;
	}
	end(convId: string) {
		delete this.running[convId];
		delete this.stops[convId];
	}
	stopped(convId: string) {
		return !!this.stops[convId];
	}
	stop(convId: string, session?: ChatSession) {
		if (!this.running[convId]) return;
		this.stops[convId] = true;
		void call(commands.slashCancel(convId), { source: "slash", silent: true }).catch(() => {});
		ghost.cancel(convId);
		if (session?.busy) session.cancel();
	}
}

export const slashRuns = new SlashRuns();
