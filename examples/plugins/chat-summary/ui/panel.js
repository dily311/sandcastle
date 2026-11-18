// 패널은 저장된 요약만 보여 준다 (AI를 부르지 않음 → 열 때마다 요금이 들지 않음)
(async () => {
	const text = document.getElementById("text");
	const meta = document.getElementById("meta");
	try {
		const ctx = await sandcastle.app.context();
		if (!ctx.conversationId) {
			text.textContent = "대화를 열면 요약을 보여 줍니다";
			return;
		}
		const saved = await sandcastle.storage.get("summary:" + ctx.conversationId);
		if (!saved) {
			text.textContent = "아직 요약이 없습니다. 타이틀바의 플러그인 명령에서 '대화 요약하기'를 누르세요 (AI를 부르므로 요금이 들 수 있습니다)";
			return;
		}
		text.textContent = saved.text;
		meta.textContent = saved.messages + "개 메시지 · " + (saved.model || "") + " · " + new Date(saved.at).toLocaleString();
	} catch (e) {
		text.textContent = "읽지 못했습니다: " + (e && e.message ? e.message : e);
	}
})();
