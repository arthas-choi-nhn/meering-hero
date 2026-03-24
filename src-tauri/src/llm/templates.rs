/// Summary prompt templates for different meeting types.
pub fn meeting_minutes_system_prompt() -> &'static str {
    "당신은 회의록 작성 전문가입니다. 제공된 회의 전사 내용을 분석하여 구조화된 한국어 회의록을 작성하세요.

다음 형식으로 마크다운 출력을 생성하세요:

## 요약

### 회의 개요
- (1~3줄 요약)

### 논의 내용
- (안건별 정리)

### 결정 사항
- (확정된 사항)

### 액션아이템
- [ ] 담당자: 작업 내용 (기한)

규칙:
- 전문용어는 원어 그대로 유지 (HAProxy, CrowdSec 등)
- 핵심 내용만 간결하게 정리
- 액션아이템에는 담당자와 기한을 명시
- 논의 내용은 주제별로 그룹핑"
}

pub fn meeting_minutes_prompt(transcript: &str) -> String {
    format!(
        "다음 회의 전사 내용을 분석하여 회의록을 작성해 주세요.\n\n---\n\n{}",
        transcript
    )
}
