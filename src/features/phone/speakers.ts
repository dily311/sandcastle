// 폰 단체 대화 (5-E-19-1): 한 답장 안의 `이름: 대사` 줄을 말한 사람별 말풍선 묶음으로 나눈다.
// 규칙은 일반 대화의 다인봇(5-F-2, format.ts SAY)과 같다. 이름 없는 줄은 바로 앞 사람의 말에 붙고,
// 처음부터 이름이 없으면 대화 상대(main)의 말이다.
import { SAY } from "$features/character/format";

export interface SpeakerPart {
	speaker: string;
	text: string;
}

export function splitSpeakers(content: string, main: string): SpeakerPart[] {
	const out: SpeakerPart[] = [];
	for (const raw of content.split("\n")) {
		const line = raw.trim();
		if (!line) continue;
		const m = SAY.exec(line);
		const speaker = m ? m[1].trim().replace(/^\*+|\*+$/g, "") : null;
		const text = m ? line.slice(m[0].length) : line;
		const last = out.at(-1);
		if (speaker && last?.speaker !== speaker) {
			out.push({ speaker, text });
		} else if (last) {
			last.text += `\n${text}`;
		} else {
			out.push({ speaker: speaker ?? main, text });
		}
	}
	return out.map((p) => ({ ...p, text: p.text.trim() })).filter((p) => p.text);
}
