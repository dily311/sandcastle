// 이미 만들어두신 1:1 채팅의 "메시지 전송" 로직을 게임 쪽에 연결하기 위한 인터페이스입니다.
// 실제 구현은 채팅 모듈 쪽에서 만들어서 GameStage에 prop으로 넘겨주시면 됩니다.
//
// 구현하실 때 이렇게 조합해서 프롬프트를 만들면 됩니다:
//   1. characters 테이블에서 characterId로 persona/system_prompt_override 조회
//   2. memory_settings에 따라 과거 대화 기록(있다면) 구성
//   3. extraContext(게임 상태값 요약)를 시스템 프롬프트 뒤에 덧붙임
//   4. userMessage를 실제 유저 메시지로 Gemini에 전송
//
// 게임 중 나눈 대화를 메인 채팅 목록에 남길지는 선택사항입니다. 남기고 싶으시면
// conversations.mode = 'game' 정도로 태그해서 별도로 구분해두는 걸 추천해요
// (스키마에 이미 mode 컬럼이 있으니 그대로 활용하시면 됩니다).

export interface NpcResponder {
  send(args: {
    characterId: string;
    userMessage: string;
    /** 게임 상태값 등을 문자열로 요약한 추가 컨텍스트 (예: "호감도: 7, 첫 만남: 아님") */
    extraContext?: string;
    /**
     * 'structured_game'이면, 실제 프롬프트를 조립하실 때 시스템 프롬프트 뒤에
     * 아래 같은 지시문을 붙여주셔야 해요 (예시):
     *
     *   상태 변화가 있으면 <state>key += value</state> 형식으로 표시하세요.
     *   플레이어에게 선택지를 줄 땐 <choice>선택지 텍스트</choice>를 필요한 만큼 반복하세요.
     *   더 이어갈 대화가 없으면 <choice> 태그를 아예 쓰지 마세요.
     *
     * 반환값의 text는 이 태그가 그대로 포함된 원문이어야 합니다. 파싱은 게임
     * 쪽(npc/responseParser.ts)에서 처리합니다.
     */
    responseFormat?: 'structured_game';
  }): Promise<{ speakerName: string; text: string }>;
}
