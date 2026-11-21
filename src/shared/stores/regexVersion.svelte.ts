// 정규식이 바뀐 판 (저장·삭제·켜고 끄기·가져오기). 열린 대화가 화면용 글을 다시 만들 때 키로 쓴다 (5-E-2)
class RegexVersion {
	n = $state(0);
	bump() {
		this.n++;
	}
}
export const regexVersion = new RegexVersion();
