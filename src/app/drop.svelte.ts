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
	/** 주면 이 영역 안에 놓은 파일만 받는다 (예: 대화의 읽는 영역, 오른쪽 패널). 영역 밖에서는 영역 없는 대상이 받는다 */
	zone?: () => Element | null | undefined;
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
	/** 끌고 있는 포인터 아래의 요소 (영역 있는 대상을 고르는 데 씀) */
	over = $state<Element | null>(null);

	/** 포인터가 el 위일 때 받을 대상: 그 자리를 영역으로 가진 것이 먼저, 없으면 영역 없는 것 */
	targetAt(el: Element | null): DropTarget | null {
		void this.version;
		let best: DropTarget | null = null;
		let bestZoned = false;
		for (const x of this.stack) {
			const zone = x.t.zone;
			const zoned = !!zone;
			if (zone && !(el && zone()?.contains(el))) continue;
			if (!best || (zoned && !bestZoned) || (zoned === bestZoned && (x.t.priority ?? 0) >= (best.priority ?? 0))) {
				best = x.t;
				bestZoned = zoned;
			}
		}
		return best;
	}

	get target(): DropTarget | null {
		return this.targetAt(this.over);
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

function readAsDataUrl(f: Blob): Promise<string> {
	return new Promise((resolve, reject) => {
		const r = new FileReader();
		r.onload = () => resolve(String(r.result));
		r.onerror = () => reject(r.error);
		r.readAsDataURL(f);
	});
}

/** avif까지 (테마 배경·폰 배경·글 카드 배경 버튼이 받는 형식) */
export const isImageAvif = (f: File) => isImage(f) || extOf(f) === "avif";
export const isAudio = (f: File) => ["mp3", "wav", "ogg", "flac", "m4a", "aac"].includes(extOf(f));
export const isFont = (f: File) => ["ttf", "otf", "woff", "woff2"].includes(extOf(f));

export interface PathDrop {
	label: string;
	hint?: string;
	/** 받을 파일. 기본은 그림 */
	accept?: (f: File) => boolean;
	/** false면 여러 개를 놓아도 첫 파일만 (버튼이 파일 하나만 고르는 곳). 기본 true */
	multiple?: boolean;
	/** 이보다 큰 파일은 받지 않음 (놓은 파일은 통째로 메모리에 읽혀 디스크에 쓰이므로 큰 파일은 버튼으로) */
	maxBytes?: number;
	priority?: number;
	/** 파일 고르기 버튼과 같은 처리: 디스크 경로를 받음 */
	onpaths: (paths: string[]) => void | Promise<void>;
}

/** 업로드 버튼이 있는 곳의 끌어다 놓기. 버튼과 같은 처리를 경로로 넘긴다. $effect 안에서: $effect(() => dropPaths({...})) */
export function dropPaths(t: PathDrop) {
	const base = t.accept ?? isImage;
	return drops.register({
		label: t.label,
		hint: t.hint,
		priority: t.priority,
		accept: (f) => base(f) && (t.maxBytes === undefined || f.size <= t.maxBytes),
		ondrop: async (files) => {
			const use = t.multiple === false ? files.slice(0, 1) : files;
			await t.onpaths(await Promise.all(use.map((f) => storeFile(f))));
		},
	});
}

/** 이보다 큰 파일은 조각으로 나눠 보냄 (한 번에 읽어 base64로 IPC에 실으면 수백 MB에서 화면이 얼어붙음) */
const CHUNK = 8 * 1024 * 1024;

/** 놓은 파일을 디스크에 저장하고 경로를 받음. keep: 메시지 첨부처럼 계속 둘 파일 */
export async function storeFile(f: File, keep = false): Promise<string> {
	const src = { source: "drop" };
	if (keep || f.size <= CHUNK) return call(commands.storeDroppedFile(f.name, await readAsDataUrl(f), keep, null), src);
	let path = "";
	for (let at = 0; at < f.size; at += CHUNK) {
		const data = await readAsDataUrl(f.slice(at, at + CHUNK));
		path = await call(commands.storeDroppedFile(f.name, data, false, at === 0 ? null : path), src);
	}
	return path;
}
