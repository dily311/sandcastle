// 파일 끌어다 놓기: 화면마다 "무엇을 받는지"를 등록하고, 앱 전체 덮개(DropLayer)가 받아서 넘긴다.
//   $effect(() => drops.register({ label: "캐릭터 가져오기", hint: "PNG·JSON 카드, ZIP 묶음", accept: (f) => ..., ondrop: async (files) => ... }))
// 웹뷰의 드롭은 파일 경로 없이 내용만 주므로, 경로가 필요한 가져오기는 storeFile로 디스크에 먼저 쓴다.
import { untrack } from "svelte";
import { call, commands } from "$shared/ipc/client";

export interface DropTarget {
	/** 덮개에 크게 보일 말 */
	label: string;
	/** 받을 수 있는 파일 설명 */
	hint?: string;
	accept: (f: File) => boolean;
	ondrop: (files: File[]) => void | Promise<void>;
	/** 겹칠 때 높은 것이 이김 (화면 안의 더 구체적인 부분, 예: 대화 > 캐릭터 화면). 같으면 나중 것 */
	priority?: number;
}

class Drops {
	/** priority가 높은 것, 같으면 나중에 등록한 것이 받음. 해제하면 다음 것이 다시 받음
	 *  (Svelte는 자식의 effect를 먼저 돌리므로 등록 순서만으로는 안쪽 화면이 이기지 못함) */
	// 목록은 보통 배열에 두고 바뀐 횟수만 반응형으로: 여러 화면의 effect가 한 번에 돌 때
	// $state 배열을 통째로 바꿔 끼우면 서로의 등록을 덮어쓸 수 있음
	private stack: { id: number; t: DropTarget }[] = [];
	private version = $state(0);
	private seq = 0;
	/** 파일을 끌고 들어온 중 */
	dragging = $state(false);

	get target(): DropTarget | null {
		void this.version;
		let best: { id: number; t: DropTarget } | null = null;
		for (const x of this.stack) if (!best || (x.t.priority ?? 0) >= (best.t.priority ?? 0)) best = x;
		return best?.t ?? null;
	}

	register(t: DropTarget) {
		const id = ++this.seq;
		this.stack.push({ id, t });
		// 화면의 $effect 안에서 불리므로 그 effect가 version을 따라가지 않게
		untrack(() => this.version++);
		return () => {
			const i = this.stack.findIndex((x) => x.id === id);
			if (i >= 0) this.stack.splice(i, 1);
			untrack(() => this.version++);
		};
	}
}

export const drops = new Drops();

export const extOf = (f: File) => f.name.split(".").pop()?.toLowerCase() ?? "";
export const isImage = (f: File) => ["png", "jpg", "jpeg", "webp", "gif"].includes(extOf(f));

function readAsDataUrl(f: File): Promise<string> {
	return new Promise((resolve, reject) => {
		const r = new FileReader();
		r.onload = () => resolve(String(r.result));
		r.onerror = () => reject(r.error);
		r.readAsDataURL(f);
	});
}

/** 놓은 파일을 디스크에 저장하고 경로를 받음. keep: 메시지 첨부처럼 계속 둘 파일 */
export async function storeFile(f: File, keep = false): Promise<string> {
	const data = await readAsDataUrl(f);
	return call(commands.storeDroppedFile(f.name, data, keep), { source: "drop" });
}
