# Architecture

Meering Hero의 시스템 아키텍처 문서.

## 개요

```
┌─────────────────────────────────────────────────────┐
│                    Tauri Window                      │
│  ┌───────────────────────────────────────────────┐  │
│  │           React Frontend (WebView)             │  │
│  │  SessionList │ LiveTranscript │ SummaryPanel   │  │
│  │              │                │ NoteEditor     │  │
│  │              │                │ ExportDialog   │  │
│  └──────────────┼────────────────┼────────────────┘  │
│                 │  Tauri IPC     │                    │
│  ┌──────────────┼────────────────┼────────────────┐  │
│  │           Rust Backend (Core)                   │  │
│  │  ┌─────────┐ ┌──────────┐ ┌──────────────────┐│  │
│  │  │ Session │ │ Audio    │ │ STT Pipeline     ││  │
│  │  │ Manager │ │ Capture  │ │ (whisper.cpp)    ││  │
│  │  └────┬────┘ └────┬─────┘ └────────┬─────────┘│  │
│  │       │           │                │           │  │
│  │  ┌────┴────┐ ┌────┴─────┐ ┌───────┴─────────┐│  │
│  │  │ SQLite  │ │ VAD      │ │ Claude CLI      ││  │
│  │  │ + FTS5  │ │ (Silero) │ │ (Summarization) ││  │
│  │  └─────────┘ └──────────┘ └─────────────────┘ │  │
│  │                                                │  │
│  │  ┌──────────────────────────────────────────┐  │  │
│  │  │ Dooray Wiki Client (REST API)            │  │  │
│  │  └──────────────────────────────────────────┘  │  │
│  └────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

## 레이어 구조

### Frontend (React + TypeScript)

```
src/
├── App.tsx                    # 메인 레이아웃 및 상태 관리
├── components/
│   ├── SessionList/           # 세션 목록 (좌측 사이드바)
│   ├── LiveTranscript/        # 실시간 전사 표시 (중앙)
│   ├── SummaryPanel/          # AI 요약 (우측 패널)
│   ├── NoteEditor/            # 메모 편집 (우측 패널)
│   ├── DoorayExportDialog.tsx # Dooray Wiki 내보내기
│   ├── RecordingControls.tsx  # 녹음 제어 (하단)
│   ├── NewSessionDialog.tsx   # 새 회의 생성
│   ├── Settings/              # 설정 (모델, Dooray, 오디오)
│   └── ResizeHandle.tsx       # 패널 리사이즈
├── hooks/
│   ├── useSession.ts          # 세션 CRUD 관리
│   └── useTranscript.ts       # SST 이벤트 수신
└── lib/
    └── tauri-commands.ts      # Tauri IPC 래퍼 (타입 안전)
```

**통신**: Tauri IPC (`invoke`)로 백엔드 호출. 실시간 이벤트는 `listen`으로 수신.

### Backend (Rust)

```
src-tauri/src/
├── lib.rs                     # Tauri 앱 초기화 및 커맨드 등록
├── models.rs                  # 데이터 모델 (Session, Segment, Summary, Export)
├── commands/
│   ├── session.rs             # 세션 CRUD 커맨드
│   ├── recording.rs           # 녹음 파이프라인 시작/중지
│   ├── audio.rs               # 오디오 디바이스 조회
│   ├── model.rs               # 모델 상태 조회
│   ├── summary.rs             # Claude 요약 커맨드
│   ├── export.rs              # Dooray 내보내기 커맨드
│   └── settings.rs            # 설정 로드/저장
├── audio/
│   ├── capture.rs             # cpal 오디오 캡처
│   └── vad.rs                 # VAD (에너지/Silero)
├── stt/
│   ├── engine.rs              # whisper-rs 엔진 래퍼
│   ├── pipeline.rs            # 캡처 → VAD → STT 파이프라인
│   └── postprocess.rs         # 한국어 후처리 (필러 제거)
├── llm/
│   ├── claude_cli.rs          # Claude Code CLI 호출
│   ├── templates.rs           # 요약 프롬프트 템플릿
│   └── mod.rs
├── export/
│   ├── dooray.rs              # Dooray REST API 클라이언트
│   └── renderer.rs            # 마크다운 렌더러
├── session/
│   ├── manager.rs             # 세션 비즈니스 로직
│   └── storage.rs             # SQLite 데이터 액세스
└── model/
    └── manager.rs             # STT/VAD 모델 관리 및 다운로드
```

## 핵심 파이프라인

### 녹음 및 전사 파이프라인

```
마이크 → AudioCapture → RingBuffer → VAD → SpeechSegment → WhisperEngine → Segment → DB
  (cpal)                  (ringbuf)  (silero/energy)         (whisper-rs)            (SQLite)
                                                                    │
                                                                    ├→ Event("stt:final") → Frontend
                                                                    └→ DB Insert
```

**스레딩 모델**:
- **메인 스레드**: Tauri 이벤트 루프 + IPC
- **오디오 스레드**: cpal 콜백 (고정 우선순위)
- **STT 스레드**: `std::thread::spawn`으로 분리, `Arc<Mutex>` 동기화

**오디오 처리**:
1. cpal로 디바이스 기본 설정으로 캡처
2. 16kHz mono f32로 리샘플링
3. VAD로 음성 구간 감지
4. 최소 0.3초 이상의 오디오만 처리 (노이즈 방지)
5. `no_speech_prob > 0.6`인 세그먼트 제거

### 요약 파이프라인

```
Segments(DB) → 타임스탬프 포맷팅 → Claude CLI → 마크다운 요약 → Summary(DB)
                 [MM:SS] text         (-p --bare)
```

**Claude CLI 호출**:
- `--strict-mcp-config` + 빈 MCP config으로 MCP 서버 비활성화
- `--output-format text` + `--max-turns 1`
- 커스텀 시스템 프롬프트 지원 (설정에서 변경 가능)
- 100KB 초과 시 stdin으로 전달
- 120초 타임아웃

### 내보내기 파이프라인

```
Session + Segments + Summary + Notes → Markdown Renderer → Dooray Wiki API
                                         (renderer.rs)        (POST/PUT)
```

**마크다운 구조**: 회의 정보 → 요약 → 노트 → 전사본(`<details>` 접기)

## 데이터베이스 스키마

```sql
sessions (
  id TEXT PRIMARY KEY,          -- UUID v4
  title TEXT, started_at TEXT, ended_at TEXT,
  duration_secs INTEGER,
  participants TEXT,            -- JSON: ["name1", "name2"]
  context_hint TEXT,            -- 전문용어 힌트
  notes TEXT, status TEXT,      -- created | recording | ended
  audio_path TEXT, model_used TEXT,
  created_at TEXT, updated_at TEXT
)

segments (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  session_id TEXT REFERENCES sessions(id),
  text TEXT, start_ms INTEGER, end_ms INTEGER,
  is_final BOOLEAN, speaker TEXT,
  created_at TEXT
)
-- FTS5 인덱스: segments_fts(text)

summaries (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  session_id TEXT, template TEXT, content TEXT,
  provider TEXT,                -- "ClaudeCodeCli"
  cost_usd REAL, duration_ms INTEGER,
  created_at TEXT
)

exports (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  session_id TEXT, summary_id INTEGER,
  target TEXT,                  -- "dooray_wiki"
  target_url TEXT, exported_at TEXT
)
```

**저장 위치**: `~/Library/Application Support/meeting-app/db/sessions.db`

## 모델 관리

**모델 저장 경로**: `~/Library/Application Support/meeting-app/models/`

| 모델 | 파일 | 소스 | 용도 |
|------|------|------|------|
| Silero VAD | `silero_vad.onnx` | GitHub (snakers4) | 음성 구간 감지 |
| Whisper Small | `ggml-small.bin` | HuggingFace (ggerganov) | STT |
| Whisper Medium | `ggml-medium.bin` | HuggingFace | STT (추천) |
| Whisper Large | `ggml-large-v3-turbo.bin` | HuggingFace | STT (고정확도) |

**자동 선택 로직**: 시스템 RAM 기반 (`sysinfo` 크레이트)
- 8GB → Small
- 16GB → Medium
- 32GB+ → Large

## 설정 파일

**경로**: `~/Library/Application Support/meeting-app/settings.json`

```json
{
  "dooray_base_url": "https://your-org.dooray.com",
  "dooray_token": "dooray-api-token",
  "stt_model": null,           // null = auto, "small" | "medium" | "large"
  "audio_device": null,        // null = system default
  "vad_mode": null,            // null = "energy", "silero"
  "summary_prompt": null       // null = default prompt
}
```

## 자동 업데이트

**메커니즘**: `tauri-plugin-updater` + GitHub Releases

```
앱 시작 / 수동 확인
    │
    ▼
GET /releases/latest/download/latest.json
    │
    ▼
버전 비교 (current vs latest)
    │
    ├─ 동일 → "최신 버전입니다"
    │
    └─ 새 버전 → .app.tar.gz 다운로드
                      │
                      ▼
                 .sig 검증 (pubkey)
                      │
                      ▼
                 앱 교체 + 재시작
```

## 외부 의존성

| 의존성 | 용도 | 연결 방식 |
|--------|------|-----------|
| whisper.cpp | STT 엔진 | whisper-rs (FFI) |
| ONNX Runtime | Silero VAD | ort 크레이트 |
| CoreML | whisper 가속 | whisper-rs feature |
| Claude Code CLI | 요약 | 프로세스 스폰 |
| Dooray API | 위키 내보내기 | REST (reqwest) |
| SQLite | 데이터 저장 | rusqlite (bundled) |

## 보안 고려사항

- **음성 데이터**: 로컬에서만 처리, 외부 전송 없음
- **Claude CLI**: `--strict-mcp-config`으로 MCP 서버 차단
- **Dooray 토큰**: 로컬 settings.json에 저장 (앱 데이터 디렉토리)
- **업데이트**: 서명 검증 (minisign pubkey)으로 무결성 보장
