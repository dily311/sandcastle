// Android 구글 드라이브 (CLAUDE.md 2-D, ADR-011). 폰의 Play 서비스가 권한(drive.appdata)을 주고, 이 파일이 받은
// access token을 백엔드에 넘긴다(메모리에만). refresh token은 없다 — 동기화할 때마다 폰에 다시 묻는다(이미 허락했으면 화면 없이).
// 앱(MainActivity)이 앱 주소에만 SandcastleNative·SandcastleNet을 넣어 주므로 데스크톱·브라우저에서는 hasNativeDrive()가 false
import { call, commands } from "$shared/ipc/client";

type Native = { postMessage(msg: string): void; addEventListener?(type: "message", fn: (e: { data: string }) => void): void; onmessage?: ((e: { data: string }) => void) | null };
const native = () => (window as unknown as { SandcastleNative?: Native }).SandcastleNative;
const net = () => (window as unknown as { SandcastleNet?: { unmetered(): boolean } }).SandcastleNet;

export const hasNativeDrive = () => !!native() && !!net();

let seq = 0;
let listening = false;
const waiting = new Map<string, (r: { token?: string; error?: string }) => void>();

function listen(n: Native) {
	if (listening) return;
	listening = true;
	const onMsg = (e: { data: string }) => {
		let d: { kind?: string; id?: string; token?: string; error?: string };
		try {
			d = JSON.parse(e.data);
		} catch {
			return;
		}
		if (d.kind !== "drive" || !d.id) return;
		waiting.get(d.id)?.(d);
		waiting.delete(d.id);
	};
	if (n.addEventListener) n.addEventListener("message", onMsg);
	else n.onmessage = onMsg;
}

/** 폰에 토큰을 묻는다. interactive면 처음일 때 구글 동의 화면을 띄운다 */
export function nativeDriveToken(interactive: boolean): Promise<string> {
	const n = native();
	if (!n) return Promise.reject(new Error("이 기기에서는 쓸 수 없음"));
	listen(n);
	const id = String(++seq);
	return new Promise((resolve, reject) => {
		const timer = setTimeout(() => {
			waiting.delete(id);
			reject(new Error("구글 응답이 없음"));
		}, 120_000);
		waiting.set(id, (r) => {
			clearTimeout(timer);
			if (r.token) resolve(r.token);
			else reject(new Error(explain(r.error)));
		});
		n.postMessage(`drive:authorize:${id}:${interactive ? 1 : 0}`);
	});
}

function explain(code?: string) {
	if (code === "cancelled") return "구글 로그인이 취소됨 — 직접 취소한 게 아니라면 구글 클라우드의 Android 클라이언트(패키지 이름·서명 키 SHA-1)와 테스트 사용자를 확인";
	if (code === "needs_consent") return "구글 드라이브 권한을 다시 허락해야 함 (설정 > 동기화에서 로그인)";
	// 10 = DEVELOPER_ERROR: 구글 클라우드의 Android 클라이언트(패키지 이름·서명 키 SHA-1)가 이 앱과 다름
	if (code === "api:10") return "구글이 이 앱을 알아보지 못함 — 구글 클라우드의 Android 클라이언트에 이 앱의 패키지 이름과 서명 키 SHA-1이 맞게 들어 있는지 확인";
	if (code?.startsWith("api:7")) return "네트워크에 연결할 수 없음";
	return `구글 권한을 받지 못함 (${code ?? "알 수 없음"})`;
}

/** 폰에서 토큰을 받아 백엔드에 넣는다. connect=처음 연결(동의 화면 허용, 계정을 읽고 저장소를 드라이브로) */
export async function prepareDrive(connect: boolean) {
	const token = await nativeDriveToken(connect);
	return call(commands.driveUseToken(token, connect), { source: "sync" });
}

/** 지금 연결이 데이터 요금이 들지 않는가 (모르면 true) */
export function unmetered(): boolean {
	try {
		return net()?.unmetered() ?? true;
	} catch {
		return true;
	}
}
