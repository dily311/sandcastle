// 커뮤니티 플러그인 관리 (CLAUDE.md 6-D-8): 설치된 목록과 사용자가 하는 동작. 화면 상태(펼침·대화상자)는 컴포넌트가 가진다.
// 코어 플러그인 `community`가 꺼져 있으면 아무것도 읽지 않는다 (설정 탭이 안 보이므로 여기까지 오지 않음).
import type { PluginLogEntry, PluginPreview, UserPlugin_Serialize as UserPlugin } from "$ipc/bindings";
import { call, commands } from "$shared/ipc/client";
import { onEvent } from "$shared/ipc/events";
import { trashedToast } from "$shared/trash";

const src = { source: "community" };

class CommunityAdmin {
	list = $state<UserPlugin[]>([]);
	loaded = $state(false);
	private unlisten: (() => void) | null = null;

	async load() {
		this.list = await call(commands.listUserPlugins(), { ...src, silent: true }).catch(() => this.list);
		this.loaded = true;
	}

	/** 설치·켜기·지우기 같은 변경이 오면 목록을 다시 읽는다. 화면을 떠날 때 돌려받은 함수를 부른다 */
	async watch(): Promise<() => void> {
		const u = await onEvent("user-plugins-changed", () => void this.load());
		this.unlisten = u;
		await this.load();
		return () => {
			u();
			if (this.unlisten === u) this.unlisten = null;
		};
	}

	/** 동작이 성공하면 목록을 바로 다시 읽는다 (다른 창의 변경은 user-plugins-changed 이벤트가 알려 주지만, 내가 한 것은 이벤트를 기다리지 않는다) */
	private async after<T>(p: Promise<T | null>): Promise<T | null> {
		const r = await p;
		if (r !== null) await this.load();
		return r;
	}

	get(id: string) {
		return this.list.find((p) => p.id === id);
	}

	/** 설치하지 않고 읽어서 미리보기. 실패하면 이유를 토스트로 보이고 null */
	preview(path: string): Promise<PluginPreview | null> {
		return call(commands.previewUserPlugin(path), src).catch(() => null);
	}

	/** 미리보기에서 본 그 파일을 설치 (그 사이 바뀌었으면 백엔드가 거절). 새 플러그인은 꺼진 채 */
	install(path: string, hash: string) {
		return this.after(call(commands.installUserPlugin(path, hash), src).catch(() => null));
	}

	/** 켜기: manifest의 권한 전부에 동의한 목록을 넘긴다 (일부만으로는 백엔드가 거절) */
	enable(id: string, grant: string[]) {
		return this.after(call(commands.setUserPluginEnabled(id, true, grant), src).catch(() => null));
	}
	disable(id: string) {
		return this.after(call(commands.setUserPluginEnabled(id, false, []), src).catch(() => null));
	}

	setProcessMode(id: string, mode: "notes" | "fix") {
		return this.after(call(commands.setUserPluginProcessMode(id, mode), src).catch(() => null));
	}
	setAiAuto(id: string, on: boolean) {
		return this.after(call(commands.setUserPluginAiAuto(id, on), src).catch(() => null));
	}

	/** 지우기 = 휴지통으로. "되돌리기" 알림을 띄운다 */
	async uninstall(id: string) {
		const item = await this.after(call(commands.uninstallUserPlugin(id), src).catch(() => null));
		if (item) trashedToast(item, () => this.load());
		return item;
	}

	/** 쓸 판 고르기 (꺼진 채로 바뀜 — 다시 켜서 권한에 동의해야 함) */
	switchVersion(id: string, dir: string) {
		return this.after(call(commands.switchUserPluginVersion(id, dir), src).catch(() => null));
	}
	/** 쓰지 않는 판 폴더 지우기 (되돌릴 수 없음) */
	deleteVersion(id: string, dir: string) {
		return this.after(call(commands.deleteUserPluginVersion(id, dir), src).catch(() => null));
	}
	/** 설정 > 데이터용: 코어 플러그인 community가 꺼져 있어도 폴더는 볼 수 있다 */
	folders(): Promise<UserPlugin[]> {
		return call(commands.listPluginFolders(), { ...src, silent: true }).catch(() => []);
	}

	settings(id: string): Promise<Record<string, unknown>> {
		return call(commands.getUserPluginSettings(id), { ...src, silent: true }).then((v) => (v ?? {}) as Record<string, unknown>).catch(() => ({}));
	}
	async setSetting(id: string, key: string, value: unknown): Promise<Record<string, unknown> | null> {
		const v = await call(commands.setUserPluginSetting(id, key, value as never), src).catch(() => null);
		return v ? (v as Record<string, unknown>) : null;
	}

	log(id: string): Promise<PluginLogEntry[]> {
		return call(commands.getUserPluginLog(id), { ...src, silent: true }).catch(() => []);
	}
}

export const admin = new CommunityAdmin();
