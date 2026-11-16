// 끌어서 순서 바꾸기 (HTML 드래그 앤 드롭, 라이브러리 없음). 홈 블록·로어북 항목이 같이 쓴다.
//   const sort = new Sortable((from, to) => ...);
//   <li {...sort.item(i)} class:dragging={sort.from === i} class:over={sort.over === i}>
// 키보드로도 옮길 수 있게 화면 쪽에서 위/아래 버튼을 따로 두는 것을 권한다.

/** 배열에서 from을 빼서 to 자리에 넣은 새 배열 */
export function moveItem<T>(list: readonly T[], from: number, to: number): T[] {
	if (from === to || from < 0 || from >= list.length || to < 0 || to >= list.length) return [...list];
	const next = [...list];
	const [x] = next.splice(from, 1);
	next.splice(to, 0, x);
	return next;
}

let groups = 0;

export class Sortable {
	from = $state<number | null>(null);
	over = $state<number | null>(null);
	enabled = $state(true);
	/** 한 화면에 목록이 여러 개일 때 손가락으로 끌어도 다른 목록에 놓이지 않게 */
	private group = `s${++groups}`;

	constructor(private onmove: (from: number, to: number) => void) {}

	/**
	 * 손잡이(⋮⋮)에 펼쳐 넣을 속성들 — 터치 화면용 (1-C-3).
	 * Android WebView는 HTML 드래그를 손가락으로 시작하지 못하므로, 손잡이를 누른 채 움직이면 포인터 이벤트로 옮긴다.
	 * 손잡이는 touch-action: none이라 누른 동안 화면이 스크롤되지 않는다
	 */
	handle(i: number) {
		return {
			style: "touch-action: none",
			onpointerdown: (e: PointerEvent) => {
				if (!this.enabled || e.pointerType === "mouse") return;
				e.preventDefault();
				this.from = i;
				this.over = i;
				const move = (ev: PointerEvent) => {
					const el = document.elementFromPoint(ev.clientX, ev.clientY)?.closest<HTMLElement>(`[data-sort-group="${this.group}"]`);
					const at = el?.dataset.sortIndex;
					if (at !== undefined) this.over = Number(at);
				};
				const end = (ev: PointerEvent) => {
					window.removeEventListener("pointermove", move);
					window.removeEventListener("pointerup", end);
					window.removeEventListener("pointercancel", end);
					const from = this.from;
					const to = this.over;
					this.from = this.over = null;
					if (ev.type === "pointerup" && from !== null && to !== null && from !== to) this.onmove(from, to);
				};
				window.addEventListener("pointermove", move);
				window.addEventListener("pointerup", end);
				window.addEventListener("pointercancel", end);
			},
		};
	}

	/** 끌 수 있는 요소에 펼쳐 넣을 속성들 */
	item(i: number) {
		return {
			"data-sort-group": this.group,
			"data-sort-index": i,
			draggable: this.enabled,
			ondragstart: (e: DragEvent) => {
				if (!this.enabled) return;
				// 입력칸 안에서 글을 고르는 중이면 끌지 않는다
				if ((e.target as Element | null)?.closest?.("input, textarea, [contenteditable]")) return e.preventDefault();
				this.from = i;
				e.dataTransfer?.setData("text/plain", String(i));
				if (e.dataTransfer) e.dataTransfer.effectAllowed = "move";
			},
			ondragover: (e: DragEvent) => {
				if (this.from === null) return;
				e.preventDefault();
				if (e.dataTransfer) e.dataTransfer.dropEffect = "move";
				this.over = i;
			},
			ondragleave: () => {
				if (this.over === i) this.over = null;
			},
			ondrop: (e: DragEvent) => {
				e.preventDefault();
				const from = this.from;
				this.from = this.over = null;
				if (from !== null && from !== i) this.onmove(from, i);
			},
			ondragend: () => {
				this.from = this.over = null;
			},
		};
	}
}
