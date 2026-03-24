# ADR: 사내 회의록 자동화 서비스 아키텍처 결정 기록

---

## ADR-001: 클라이언트 사이드 STT (온디바이스 처리)

**상태**: 채택

**일시**: 2026-03-23

### 맥락

회의 음성 데이터에는 프로젝트 기밀, 인프라 구성, 보안 정책 등 민감한 정보가 포함된다. 퍼블릭 STT 서비스(Tiro, Google Speech-to-Text, CLOVA Speech 등)를 사용하면 음성 데이터가 외부 서버로 전송되어 사내 보안 정책에 위배된다.

### 결정

STT 처리를 사용자의 MacBook Pro(Apple Silicon)에서 로컬로 수행한다. 서버 GPU 인프라를 별도로 구축하지 않는다.

### 검토한 대안

| 방안 | 장점 | 단점 | 결정 |
|------|------|------|------|
| **로컬 STT (MacBook)** | 데이터 외부 유출 제로, 인프라 비용 없음 | 디바이스 성능 의존, 모델 배포 필요 | **채택** |
| 사내 GPU 서버 호스팅 | 높은 동시성, 모델 중앙 관리 | GPU 서버 확보/유지 비용, 네트워크 전송 | 기각 |
| 퍼블릭 STT API | 구현 단순, 최고 정확도 | 음성 데이터 외부 전송, 보안 위배 | 기각 |

### 근거

- Apple Silicon(M1~M4)의 Neural Engine/GPU가 Whisper 모델을 실용적 속도로 처리 가능 (RTF 0.3~0.5)
- 사내 GPU 서버 확보의 불확실성 대비 즉시 시작 가능
- 음성 데이터가 로컬 머신 밖으로 나가지 않아 보안 요구사항 완전 충족

### 결과

- 각 사용자 MacBook에 STT 모델(1.5~3GB)을 배포해야 함
- MacBook 사양에 따른 모델 크기 분기 로직 필요
- 서버 GPU 인프라 의존성 제거

---

## ADR-002: Tauri v2 데스크톱 앱

**상태**: 채택

**일시**: 2026-03-23

### 맥락

로컬 STT를 구현하는 클라이언트 형태로 네이티브 데스크톱 앱, 웹 브라우저(WASM), 로컬 서버 + 웹 UI의 세 가지 방식을 검토했다.

### 결정

Tauri v2 기반 macOS 데스크톱 앱으로 구현한다.

### 검토한 대안

| 방안 | 장점 | 단점 | 결정 |
|------|------|------|------|
| **Tauri v2 데스크톱 앱** | 시스템 오디오 캡처 가능, 네이티브 성능, 오프라인 동작 | 앱 배포/업데이트 관리 필요 | **채택** |
| 웹 브라우저 WASM | 설치 불필요 | Neural Engine 미활용(2~4배 느림), 시스템 오디오 불가, large 모델 불가 | 기각 |
| 로컬 Python 서버 + 웹 UI | 웹 편의성 + 네이티브 성능 | Python 런타임 설치 관리 부담 | 기각 |

### 근거

- 시스템 오디오 캡처(ScreenCaptureKit)가 화상회의 녹음에 필수이며, 이는 네이티브 앱에서만 가능
- Rust 기반 Tauri는 whisper-rs(whisper.cpp Rust 바인딩)와 자연스럽게 통합
- Core ML 백엔드를 통한 Apple Neural Engine 활용으로 최적 성능
- React WebView로 프론트엔드 구현하여 웹 개발 생산성 유지

### 결과

- macOS 전용 앱 (Apple Silicon 필수)
- 앱 업데이트 메커니즘 필요 (Tauri 자체 업데이터 또는 Sparkle)
- 앱 번들에 VAD 모델(~2MB) 포함, STT 모델은 첫 실행 시 다운로드

---

## ADR-003: whisper.cpp + Core ML 기반 STT 엔진

**상태**: 채택

**일시**: 2026-03-23

### 맥락

온디바이스 STT 엔진으로 여러 오픈소스 모델/런타임을 검토했다.

### 결정

whisper.cpp를 Core ML 백엔드와 함께 사용하며, Rust에서 whisper-rs 바인딩을 통해 호출한다.

### 검토한 대안

| 엔진 | 특징 | Apple Silicon 최적화 | 결정 |
|------|------|---------------------|------|
| **whisper.cpp (Core ML)** | C++ 구현, 크로스플랫폼, Core ML/Metal 네이티브 지원 | 최상 | **채택** |
| faster-whisper (CTranslate2) | Python, 4배 빠른 Whisper | Core ML 미지원, Python 의존 | 기각 |
| mlx-whisper (MLX) | Apple MLX 프레임워크 최적화 | 좋음 (Python) | 대안 |
| Vosk | 경량, CPU 동작 | 해당 없음 | 기각 (정확도 낮음) |
| NVIDIA NeMo | NVIDIA GPU 최적화 | 해당 없음 | 기각 (Apple Silicon 미지원) |

### 근거

- whisper.cpp는 Apple의 Core ML과 Metal을 네이티브로 지원하여 Apple Silicon에서 최고 성능
- Rust 바인딩(whisper-rs)이 존재하여 Tauri(Rust) 백엔드와 직접 통합 가능
- faster-whisper는 성능이 뛰어나지만 Python 의존성이 추가되고 Core ML을 지원하지 않음
- mlx-whisper는 Apple Silicon에 최적화되어 있으나 Python 런타임이 필요하여 후보로 유지

### 결과

- `whisper-rs` 크레이트를 Cargo 의존성에 추가
- Core ML 변환된 모델 파일을 별도 관리/배포
- VAD로 Silero VAD(ONNX, ~2MB)를 사용하여 발화 구간만 STT에 전달

---

## ADR-004: STT 모델 디바이스 적응형 선택

**상태**: 채택

**일시**: 2026-03-23

### 맥락

팀원들의 MacBook Pro 사양이 다양하다 (8GB ~ 32GB+ RAM, M1 ~ M4). 단일 모델을 강제하면 저사양 기기에서 성능 문제가 발생하거나, 고사양 기기에서 정확도를 낭비하게 된다.

### 결정

앱 시작 시 시스템 스펙(RAM, 칩 종류)을 감지하여 최적 모델을 자동 선택한다.

### 모델 매핑

| 시스템 RAM | 모델 | 크기 | 정확도 (한국어 CER) |
|-----------|------|------|-------------------|
| ~8GB | whisper-small | ~500MB | ~20% |
| 9~16GB | whisper-medium | ~1.5GB | ~15% |
| 32GB+ | whisper-large-v3 | ~3GB | ~10% |

### 근거

- STT 실행 중에도 사용자가 다른 업무를 수행해야 하므로 메모리 여유분 확보 필요
- 설정 UI에서 수동 오버라이드 가능하게 하여 사용자 선택권 보장
- 모델 파일은 사내 저장소(DLS S3)에서 다운로드하여 로컬 캐시

### 결과

- Model Manager 모듈이 시스템 정보 감지 및 모델 선택 담당
- 모델 파일 저장 경로: `~/Library/Application Support/meeting-app/models/`
- 최초 실행 시 모델 다운로드 UX 필요 (프로그레스 바)

---

## ADR-005: 실시간 스트리밍 STT 파이프라인

**상태**: 채택

**일시**: 2026-03-23

### 맥락

회의 중 실시간으로 전사 텍스트를 표시하려면 오디오 스트림을 청크 단위로 처리하는 파이프라인이 필요하다.

### 결정

Audio Capture → VAD → STT → Frontend 이벤트 전달의 4단계 파이프라인을 구성한다.

### 파이프라인 구조

```
[Audio Capture Manager]
    │ 30ms PCM 청크 (Ring Buffer)
    ▼
[VAD Processor — Silero VAD (ONNX)]
    │ 발화 구간 감지 (SpeechSegment)
    ├─ 짧은 구간 (<3초) → 즉시 STT
    ├─ 긴 구간 (>3초) → 3초 슬라이딩 윈도우
    ▼
[STT Engine — whisper-rs (Core ML)]
    │ TranscribeResult { text, tokens, is_partial }
    ▼
[Tauri Event Emit → Frontend]
    ├─ partial 결과 → 실시간 표시 (깜빡이는 텍스트)
    └─ final 결과 → 확정 텍스트로 교체
```

### 핵심 컴포넌트

**Audio Capture Manager**
- 마이크: `cpal` 크레이트 (Core Audio 접근)
- 시스템 오디오: ScreenCaptureKit (Swift 브릿지 → Tauri 플러그인)
- 출력: 16kHz mono f32 PCM (Whisper 입력 형식)
- 리샘플링: 48kHz → 16kHz 변환 필요

**VAD Processor**
- Silero VAD ONNX 모델 (~2MB, 앱 번들에 포함)
- `ort` 크레이트 (ONNX Runtime Rust 바인딩)
- 설정: threshold 0.5, min_speech 250ms, min_silence 300ms

**STT Engine**
- `whisper-rs` 크레이트 (whisper.cpp Rust 바인딩)
- Core ML 백엔드 활성화 (`use_coreml: true`)
- Metal GPU 활성화 (`use_gpu: true`)
- 언어 고정: `ko` (한국어)
- `initial_prompt`로 도메인 용어 힌트 전달

### 근거

- VAD를 STT 앞에 배치하여 무음 구간의 불필요한 연산 방지
- partial/final 이중 출력으로 사용자에게 실시간감 제공
- 파이프라인을 별도 스레드에서 실행하여 UI 블로킹 방지

### 결과

- `pipeline.rs` 모듈이 전체 오케스트레이션 담당
- Tauri IPC 이벤트(`stt:result`)로 프론트엔드와 통신
- 별도 스레드에서 루프 실행 (메인 스레드 간섭 없음)

---

## ADR-006: Claude Code CLI 기반 LLM 요약

**상태**: 채택

**일시**: 2026-03-23

### 맥락

회의 전사 내용을 LLM으로 요약하는 방식을 결정해야 한다. 팀원들이 이미 Claude Code를 사용 중이며, 추가 비용 없이 구독 쿼터를 활용하고자 한다.

### 결정

Claude Code CLI의 비대화형 모드(`claude -p`)를 subprocess로 호출하여 요약을 수행한다.

### 검토한 대안

| 방안 | 구현 난이도 | 비용 | 응답 속도 | 데이터 보안 | 결정 |
|------|-----------|------|----------|-----------|------|
| **Claude Code CLI (`claude -p`)** | 낮음 | 무료 (구독) | ~5-15초 | 외부 전송됨 | **채택** |
| Anthropic API 키 (`sk-ant-api03-*`) | 낮음 | ~$0.05/건 | ~1-3초 | 외부 전송됨 | 대안 |
| Claude Code OAuth 토큰 직접 사용 | — | — | — | — | **불가** |
| 로컬 LLM (Ollama) | 중간 | 무료 | ~30초-3분 | 완전 로컬 | 폴백 옵션 |

### OAuth 토큰 사용 불가 사유

Claude Code 로그인 후 획득하는 OAuth 토큰(`sk-ant-oat01-*`)을 Anthropic Messages API에 직접 사용할 수 없다. API 호출 시 "OAuth authentication is currently not supported" 에러가 반환된다.

두 인증 체계는 완전히 분리되어 있다:

| 항목 | OAuth 토큰 | API 키 |
|------|-----------|--------|
| 형식 | `sk-ant-oat01-...` | `sk-ant-api03-...` |
| 발급 | `claude login` | console.anthropic.com |
| 과금 | 구독 쿼터 차감 | pay-per-use |
| Messages API | **불가** | 가능 |
| 용도 | Claude Code CLI 전용 | 범용 API |

### CLI 호출 방식

```bash
claude -p "<프롬프트>" \
  --system-prompt "<시스템 프롬프트>" \
  --output-format json \
  --max-turns 1
```

### 주의사항

**CLI 경로 문제**: Tauri GUI 앱은 셸 PATH를 상속받지 못할 수 있으므로, 앱 시작 시 `claude` 바이너리의 절대 경로를 탐색한다.

탐색 순서:
1. `/usr/local/bin/claude`
2. `/opt/homebrew/bin/claude`
3. `~/.npm-global/bin/claude`
4. `which claude` 폴백

**쿼터 관리**: 동시 요청을 큐잉하여 순차 처리하고, rate limit 에러 발생 시 사용자에게 명확히 안내한다.

**입력 길이**: 셸 인자 길이 제한이 있으므로, 전사 내용이 100KB 이상이면 stdin 파이프로 전달한다.

### 근거

- 팀원 전원이 Claude Code 구독(Pro/Max) 보유 → 추가 비용 없음
- API 키 발급/관리 불필요
- subprocess 오버헤드(3~5초)는 회의 종료 후 일괄 요약 시나리오에서 허용 가능

### 결과

- LLM Provider 추상화 (`enum LlmProvider { ClaudeCodeCli, AnthropicApi, Ollama }`)
- 설정에서 사용자가 provider 전환 가능
- CLI 상태 체크 기능 (`check_claude_status` 커맨드)

---

## ADR-007: 서버리스 아키텍처 + Dooray Wiki 저장

**상태**: 채택

**일시**: 2026-03-23

### 맥락

초기 설계에서는 사내 서버(Spring Boot WebFlux + MongoDB)를 두고 노트 저장, 요약, 공유, 검색을 처리하려 했으나, 인프라 관리 부담과 서비스 목적(팀 내부 회의록)을 고려하여 재검토했다.

### 결정

사내 백엔드 서버를 두지 않는다. 모든 처리는 로컬 MacBook에서 수행하고, 최종 결과물만 Dooray Wiki에 내보내기한다.

### 변경 전 vs 변경 후

```
[변경 전]
MacBook (STT) → 사내 서버 (저장/요약/공유/검색) → Dooray Wiki

[변경 후]
MacBook (STT + 요약 + 로컬 저장) → Dooray Wiki (최종 내보내기)
```

### 근거

- STT가 로컬로 내려간 시점에서 서버의 역할이 크게 줄어듦
- LLM 요약도 Claude Code CLI로 로컬 처리 가능
- 팀 공유는 Dooray Wiki가 이미 담당하고 있어 별도 공유 인프라 불필요
- 서버 운영/유지 부담 제거

### 결과

- 로컬 SQLite로 세션 데이터 관리 (오프라인 열람 가능)
- Dooray Wiki API를 통한 내보내기 모듈 구현
- 전문 검색(Elasticsearch 등)은 포기하고 SQLite FTS로 대체

---

## ADR-008: Dooray Wiki 내보내기 구조

**상태**: 채택

**일시**: 2026-03-23

### 맥락

회의 종료 후 전사 내용, LLM 요약, 사용자 노트를 Dooray Wiki에 저장해야 한다.

### 결정

Dooray Wiki API를 통해 사용자가 지정한 위치에 단일 Wiki 페이지를 생성한다. 페이지 본문에 요약, 노트, 전체 전사 내용(접기)을 마크다운으로 구성한다.

### 내보내기 플로우

```
[회의 종료]
    ↓
[전사 결과 확인/편집]
    ↓
[LLM 요약 생성] ← Claude Code CLI
    ↓
[요약 결과 확인/편집]
    ↓
[내보내기]
  ├─ Dooray 프로젝트 선택
  ├─ 상위 Wiki 페이지 선택 (트리 탐색)
  └─ 템플릿 선택
    ↓
[Wiki 페이지 생성 완료]
  └─ 생성된 URL 표시 → 브라우저에서 열기
```

### Wiki 페이지 구성

```markdown
# {제목}

**일시**: {날짜 시간}
**참석자**: {참석자 목록}
**소요시간**: {분}분

---

## 요약

{LLM 요약 내용}

---

## 노트

{사용자 편집 노트}

---

<details>
<summary>전체 전사 내용 (펼치기)</summary>

{타임스탬프 포함 전체 전사}

</details>
```

### API 호출

```
POST {dooray_base_url}/wiki/v2/pages
Header: Authorization: dooray-api {token}
Body: { parentId, title, body }
```

### 근거

- 전사 내용을 `<details>` 태그로 접어서 페이지 가독성 유지
- 요약이 최상단에 위치하여 빠른 파악 가능
- 기존 Dooray Wiki 구조에 자연스럽게 통합

### 결과

- Dooray API 토큰을 macOS Keychain에 저장
- Wiki 페이지 트리 탐색 UI 구현 필요
- Dooray API 스펙 사전 확인 필요

---

## ADR-009: 로컬 데이터 저장소 (SQLite)

**상태**: 채택

**일시**: 2026-03-23

### 맥락

서버가 없으므로 회의 세션 데이터를 로컬에 보관해야 한다.

### 결정

SQLite를 로컬 저장소로 사용한다.

### 스키마

```sql
CREATE TABLE sessions (
    id              TEXT PRIMARY KEY,      -- UUID
    title           TEXT NOT NULL,
    started_at      TEXT NOT NULL,         -- ISO 8601
    ended_at        TEXT,
    duration_secs   INTEGER,
    participants    TEXT,                  -- JSON array
    context_hint    TEXT,                  -- 맥락 힌트
    status          TEXT NOT NULL,         -- recording, completed, exported
    audio_path      TEXT,                  -- 원본 녹음 파일 경로
    model_used      TEXT,                  -- whisper-medium 등
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);

CREATE TABLE segments (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id      TEXT NOT NULL REFERENCES sessions(id),
    text            TEXT NOT NULL,
    start_ms        INTEGER NOT NULL,
    end_ms          INTEGER NOT NULL,
    is_final        BOOLEAN NOT NULL DEFAULT 1,
    speaker         TEXT,                  -- 화자 라벨 (Phase 2)
    created_at      TEXT NOT NULL
);

CREATE TABLE summaries (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id      TEXT NOT NULL REFERENCES sessions(id),
    template        TEXT NOT NULL,         -- MeetingMinutes, DailyScrum 등
    content         TEXT NOT NULL,         -- 마크다운 요약 내용
    provider        TEXT NOT NULL,         -- ClaudeCodeCli, Ollama 등
    cost_usd        REAL,
    duration_ms     INTEGER,
    created_at      TEXT NOT NULL
);

CREATE TABLE exports (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id      TEXT NOT NULL REFERENCES sessions(id),
    summary_id      INTEGER REFERENCES summaries(id),
    target          TEXT NOT NULL,         -- dooray_wiki
    target_url      TEXT,                  -- 생성된 Wiki 페이지 URL
    exported_at     TEXT NOT NULL
);

-- 전문 검색 인덱스
CREATE VIRTUAL TABLE segments_fts USING fts5(text, content=segments, content_rowid=id);
```

### 근거

- Rust에서 `rusqlite` 크레이트로 직접 접근 가능
- 별도 프로세스/서버 불필요 (파일 기반)
- FTS5로 전문 검색 지원
- macOS FileVault 암호화에 의존하여 별도 암호화 불필요

### 저장 경로

```
~/Library/Application Support/meeting-app/
├── db/
│   └── sessions.db          -- SQLite 데이터베이스
├── models/
│   ├── whisper-medium/      -- STT 모델
│   └── silero_vad.onnx      -- VAD 모델
└── audio/
    └── {session-id}.wav     -- 원본 녹음 파일 (선택적 보존)
```

---

## ADR-010: 프로젝트 구조

**상태**: 채택

**일시**: 2026-03-23

### 디렉토리 구조

```
meeting-app/
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── src/
│   │   ├── main.rs                     -- 앱 진입점, Tauri Builder 설정
│   │   │
│   │   ├── commands/                   -- Tauri IPC 커맨드 정의
│   │   │   ├── mod.rs
│   │   │   ├── session.rs              -- start/stop/list_sessions
│   │   │   ├── audio.rs               -- set_audio_source, get_audio_devices
│   │   │   ├── summary.rs             -- check_claude_status, summarize_session
│   │   │   ├── export.rs              -- export_to_dooray_wiki
│   │   │   ├── model.rs               -- download_model, get_model_status
│   │   │   └── settings.rs            -- 설정 CRUD
│   │   │
│   │   ├── audio/                      -- 오디오 캡처 및 전처리
│   │   │   ├── mod.rs
│   │   │   ├── capture.rs             -- AudioCaptureManager (cpal + ScreenCaptureKit)
│   │   │   ├── vad.rs                 -- VadProcessor (Silero ONNX)
│   │   │   └── resampler.rs           -- 샘플레이트 변환 (48kHz → 16kHz)
│   │   │
│   │   ├── stt/                        -- 음성 인식 엔진
│   │   │   ├── mod.rs
│   │   │   ├── engine.rs              -- SttEngine (whisper-rs + Core ML)
│   │   │   ├── pipeline.rs            -- Capture → VAD → STT 오케스트레이션
│   │   │   └── postprocess.rs         -- 후처리 (필러 제거, 용어 교정)
│   │   │
│   │   ├── llm/                        -- LLM 요약
│   │   │   ├── mod.rs
│   │   │   ├── client.rs              -- LlmProvider enum, 추상화 레이어
│   │   │   ├── claude_cli.rs          -- Claude Code CLI subprocess 호출
│   │   │   ├── ollama.rs              -- Ollama 로컬 LLM (폴백)
│   │   │   └── templates.rs           -- 요약 프롬프트 템플릿
│   │   │
│   │   ├── export/                     -- 외부 내보내기
│   │   │   ├── mod.rs
│   │   │   ├── dooray.rs              -- Dooray Wiki API 클라이언트
│   │   │   └── renderer.rs            -- Wiki 마크다운 렌더링
│   │   │
│   │   ├── session/                    -- 세션 관리
│   │   │   ├── mod.rs
│   │   │   ├── manager.rs             -- SessionManager
│   │   │   └── storage.rs             -- SQLite 저장/조회
│   │   │
│   │   ├── model/                      -- STT 모델 관리
│   │   │   └── manager.rs             -- ModelManager (다운로드, 버전, 선택)
│   │   │
│   │   └── swift_bridge/              -- macOS 네이티브 브릿지
│   │       └── screen_capture.rs      -- ScreenCaptureKit Swift 연동
│   │
│   ├── resources/
│   │   └── silero_vad.onnx            -- VAD 모델 (앱 번들 포함)
│   │
│   └── models/                         -- 다운로드된 STT 모델 캐시
│
├── src/                                -- Frontend (React + TypeScript)
│   ├── App.tsx
│   ├── components/
│   │   ├── MenuBar/                   -- 시스템 트레이 UI
│   │   ├── LiveTranscript/            -- 실시간 전사 화면
│   │   │   ├── TranscriptLine.tsx     -- 개별 발화 라인
│   │   │   ├── PartialText.tsx        -- 인식 중 텍스트
│   │   │   └── SpeakerLabel.tsx       -- 화자 라벨
│   │   ├── NoteEditor/                -- 노트 편집기
│   │   ├── SummaryPanel/              -- LLM 요약 패널
│   │   ├── ExportDialog/              -- Dooray Wiki 내보내기 다이얼로그
│   │   ├── SessionList/               -- 과거 세션 목록
│   │   └── Settings/                  -- 설정 (모델, 오디오, LLM 등)
│   ├── hooks/
│   │   ├── useAudioCapture.ts         -- 오디오 캡처 제어
│   │   ├── useTranscript.ts           -- 실시간 전사 이벤트 수신
│   │   ├── useSession.ts              -- 세션 상태 관리
│   │   └── useSummary.ts              -- 요약 요청/결과 관리
│   └── lib/
│       └── tauri-commands.ts          -- Tauri invoke 래퍼
│
├── package.json
└── README.md
```

### 기술 스택 요약

| 레이어 | 기술 |
|--------|------|
| **앱 프레임워크** | Tauri v2 |
| **백엔드 (Rust)** | whisper-rs, ort, cpal, rusqlite, reqwest, tokio, serde |
| **프론트엔드** | React, TypeScript, Tailwind CSS |
| **STT 엔진** | whisper.cpp (Core ML 백엔드) |
| **VAD** | Silero VAD (ONNX) |
| **LLM** | Claude Code CLI (`claude -p`) |
| **로컬 DB** | SQLite (rusqlite + FTS5) |
| **외부 연동** | Dooray Wiki API |
| **Swift 브릿지** | ScreenCaptureKit (시스템 오디오) |

### Cargo 주요 의존성

```toml
[dependencies]
tauri = { version = "2", features = ["tray-icon"] }
whisper-rs = { version = "0.12", features = ["coreml"] }
ort = "2"                          # ONNX Runtime (Silero VAD)
cpal = "0.15"                      # 오디오 캡처
rusqlite = { version = "0.31", features = ["bundled", "fts5"] }
reqwest = { version = "0.12", features = ["json"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v4", "serde"] }
thiserror = "1"
ringbuf = "0.4"                    # Lock-free ring buffer
```