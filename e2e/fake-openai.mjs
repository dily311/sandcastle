// E2E용 가짜 OpenAI 호환 공급자 (요금 없음). 127.0.0.1의 빈 포트에서 뜬다.
// 모델 목록과 채팅(스트리밍·한 번에)만 흉내 낸다
import http from "node:http";

export const REPLY = "E2E 가짜 공급자의 응답입니다.";

/** 서버를 띄우고 { url, close, calls }를 돌려준다 */
export async function startFakeOpenAI() {
	const calls = [];
	const server = http.createServer(async (req, res) => {
		let body = "";
		for await (const c of req) body += c;
		calls.push({ method: req.method, url: req.url, auth: req.headers.authorization ?? "" });
		if (req.url.endsWith("/models")) {
			res.writeHead(200, { "content-type": "application/json" });
			return res.end(JSON.stringify({ data: [{ id: "fake-model" }] }));
		}
		if (req.url.endsWith("/chat/completions")) {
			const j = JSON.parse(body || "{}");
			if (j.stream) {
				res.writeHead(200, { "content-type": "text/event-stream" });
				for (const ch of REPLY.match(/.{1,5}/gsu)) res.write(`data: ${JSON.stringify({ choices: [{ delta: { content: ch } }] })}\n\n`);
				res.write(`data: ${JSON.stringify({ choices: [{ delta: {}, finish_reason: "stop" }], usage: { prompt_tokens: 10, completion_tokens: 5 } })}\n\n`);
				return res.end("data: [DONE]\n\n");
			}
			res.writeHead(200, { "content-type": "application/json" });
			return res.end(JSON.stringify({ choices: [{ message: { role: "assistant", content: REPLY }, finish_reason: "stop" }], usage: { prompt_tokens: 3, completion_tokens: 2 } }));
		}
		res.writeHead(404);
		res.end();
	});
	await new Promise((r) => server.listen(0, "127.0.0.1", r));
	const { port } = server.address();
	return { url: `http://127.0.0.1:${port}/v1`, calls, close: () => new Promise((r) => server.close(r)) };
}
