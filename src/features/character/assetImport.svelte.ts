// 캐릭터 에셋 넣기 작업의 진행 상태. 화면(오른쪽 패널)이 아니라 앱에 두어서,
// 다른 화면으로 나가도 작업이 이어지고 진행·결과가 계속 보인다 (끝나면 결과 토스트).
import { get } from "svelte/store";
import { _ } from "svelte-i18n";
import { toasts } from "$shared/stores/toast.svelte";

class AssetImport {
	/** 지금 하는 일 (비면 쉬는 중) */
	busy = $state("");
	private toastId = 0;

	private show(label: string) {
		if (this.toastId) toasts.dismiss(this.toastId);
		// 끝날 때까지 떠 있는 토스트 (다른 화면에서도 진행이 보이게)
		this.toastId = toasts.show({ tone: "info", message: label, log: false }, 0);
	}

	/** 하나씩만 한다. 도중에 또 오면 거절. `set`으로 단계를 바꿔 알림 */
	async run<T>(label: string, work: (set: (label: string) => void) => Promise<T>): Promise<T | undefined> {
		if (this.busy) return void toasts.info(get(_)("chatExtra.assetBusy"));
		this.busy = label;
		this.show(label);
		try {
			return await work((l) => {
				this.busy = l;
				this.show(l);
			});
		} finally {
			this.busy = "";
			toasts.dismiss(this.toastId);
			this.toastId = 0;
		}
	}
}

export const assetImport = new AssetImport();
