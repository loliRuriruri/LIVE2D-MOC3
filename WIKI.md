# LIVE2D-MOC3 (Live2D Project Recovery Tool)

> 이 문서는 나무위키식으로 정리한 프로젝트 개요 문서이다. 정확한 기술 문서는
> `README.md`와 `docs/` 디렉터리를 참고할 것.

|  |  |
|---|---|
| 개발자 | loliRuriruri |
| 저장소 | https://github.com/loliRuriruri/LIVE2D-MOC3 |
| 언어 | Rust (2021 edition, MSRV 1.75) |
| 현재 버전 | **0.1.0-alpha** (EXPERIMENTAL) |
| 라이선스 | 미정 (프로젝트 결정 대기) |
| 상태 | 개발 단계 (AGENT.5.3까지 PASS) |
| Cubism 호환성 | **NOT TESTED** |
| 실사용 호환성 | **UNVALIDATED** |

## 1. 개요

Live2D Cubism의 **런타임 파일(.moc3)**을 읽어서, 그 안에 실제로 저장된 데이터만으로
**에디터 프로젝트(.cmo3)**를 재구성해 주는 커맨드 라인 도구이다.

핵심 원칙은 다음과 같다.

* **복원 추론 금지** — 저장된 값만 사용한다. 없는 이름, 없는 부모, 없는 keyform을
  지어내지 않는다.
* **단계 분리** — 파싱(바이너리) / IR(정규화) / 계층 복원 / keyform 복원 /
  CMO3 직렬화가 각각 별도 크레이트로 나뉜다.
* **증거 등급 관리** — 모든 필드에 `Exact` / `Derived` / `Heuristic` / `Unknown`
  신뢰도를 붙이고, writer가 요구되어 만든 기본값은 `WRITER_REQUIRED_DEFAULT`로
  따로 추적한다.
* **구조 검증과 에디터 검증 분리** — `Structural validation: PASS`와
  `Cubism validation: NOT TESTED`를 절대 뭉뚱그리지 않는다.

## 2. 개발 단계

| 단계 | 내용 | 상태 |
|---|---|---|
| AGENT.0 | 리서치/부트스트랩, 외부 참조 목록화 | PASS |
| AGENT.1 | 읽기 전용 .moc3 인스펙터 + 합성 fixture 생성기 | PASS |
| AGENT.2 | 정규화 Live2D IR (`live2d-ir/1`, EXPERIMENTAL) | PASS |
| AGENT.3 | 계층 복원 (`recovered-project/1`) | PASS |
| AGENT.3.5 | 외부 참조 감사 + 차분(differential) 검증 | PASS |
| AGENT.4 | Keyform/파라미터 바인딩 복원 (`recovered-keyforms/1`) | PASS |
| AGENT.5 | CMO3 writer 1차 (CAFF, Gate 5A) | PASS |
| AGENT.5.1 | 이미지 파이프라인 근거 확보 + 최소 main.xml (Gate 5B) | PASS |
| AGENT.5.2 | 전체 시맨틱 CMO3 통합 + E2E CLI (Gate 5D~5I) | PASS |
| AGENT.5.3 | Windows CLI v0.1 패키징 + Cubism 인수 준비 | PASS (Cubism 미검증) |
| AGENT.6 | Ground-Truth Fidelity Benchmark | **미시작** |

## 3. 아키텍처

```text
MOC3 / model3.json / Textures
        │
   moc3-ingest        읽기 전용 파서
        │
   live2d-ir          정규화 IR (live2d-ir/1)
        │
   hierarchy-recovery 계층 복원 (recovered-project/1)
        │
   keyform-recovery   바인딩/그리드/폼 복원 (recovered-keyforms/1)
        │
   cmo3-writer        CAFF + main.xml + 검증 → .cmo3
        │
   recovery-cli       recovery.exe (inspect / recover / inspect-cmo3 …)
```

| 크레이트 | 역할 |
|---|---|
| `crates/moc3-ingest` | .moc3 파서, 한계/오류, 검사 리포트 |
| `crates/live2d-ir` | IR 모델, 검증기, 정규 JSON |
| `crates/hierarchy-recovery` | RecoveryGraph, HR-001~010 규칙 |
| `crates/keyform-recovery` | ParameterAxis/BindingBand/Grid, KB·KF 규칙 |
| `crates/cmo3-writer` | CAFF 인코더/디코더, main.xml 직렬화, 검증기 |
| `crates/recovery-core` | 파일 IO, 파이프라인 연결 |
| `apps/recovery-cli` | `recovery.exe` |
| `tools/fixture-gen` | 합성 fixture 생성 (테스트 전용) |
| `tools/reference-harness` | 외부 구현 차분 검증 (개발 전용) |

## 4. 사용법

```text
Live2DRecovery.exe self-test
Live2DRecovery.exe recover model.moc3 --textures textures --output recovered.cmo3
Live2DRecovery.exe recover model3.json --output recovered.cmo3
Live2DRecovery.exe inspect model.moc3
Live2DRecovery.exe inspect-cmo3 recovered.cmo3 --json
```

* `recover`는 `recovered.cmo3`와 `recovered.report.json`을 만든다.
* 기존 출력 덮어쓰기는 `--force`가 필요하다.
* 해결 불가능한 필수 시맨틱이 있으면 strict 모드로 실패하고, 원인과
  `recover-keyforms --explain` 힌트를 출력한다.
* `--best-effort`는 명시적으로 요청한 경우에만 허용되며 결과에
  `BEST_EFFORT_OUTPUT`이 표시된다.

## 5. 검증 상태

| 항목 | 결과 |
|---|---|
| 워크스페이스 테스트 | 299 passed / 0 failed |
| clippy `-D warnings` | PASS |
| fmt | PASS |
| Structural validation | PASS (CAFF/XML, dangling 0, duplicate 0) |
| Determinism | PASS (동일 입력 → 바이트 동일) |
| Large model (1500 mesh) | PASS (약 0.5초, 28.8 MB) |
| Cubism Editor open | **NOT TESTED** |
| 실사용 모델 호환성 | **UNVALIDATED** |
| 실사용 계층 정확도 | **UNVALIDATED** |
| 실사용 keyform 정확도 | **UNVALIDATED** |

> Structural PASS는 "에디터에서 열린다"는 주장이 **아니다**.

## 6. Acceptance 파일 (0.1.0-alpha)

| 파일 | 구성 | Structural |
|---|---|---|
| `01-minimal.cmo3` | Part + ArtMesh + Texture | PASS |
| `02-deformer.cmo3` | Part → Warp → Rotation → ArtMesh | PASS |
| `03-keyform.cmo3` | 파라미터 1개, 키 3개(`[-30, 0, 30]`), keyed forms | PASS |

Cubism Editor에서의 Open/Save/Reopen 결과는 아직 없다
(`docs/reports/CUBISM_ACCEPTANCE_V0.1.md`의 체크리스트로 수동 검증 예정).

## 7. 한계

* Cubism Editor 인수 테스트 미실행 (에디터 환경 없음).
* 실제 소유 .moc3 모델로 검증되지 않음 (저장소에는 합성 fixture만 존재).
* PSD 복원, GUI, 설치 프로그램(MSI/NSIS) 없음 — portable zip만 제공.
* draw-order group, glue, offscreen surface 직렬화는 근거 부족으로
  unsupported note로만 기록된다.
* 레이어 이미지는 "합성 Layered Image"이며 원본 PSD 복원이 아니다.
* 다축 keyform 그리드 순서는 Unknown (writer 기본값은 추적됨).
* 대형 모델 메모리 피크는 출력의 약 9배 (문서화됨).

## 8. 권리 / 라이선스

* 본인 소유이거나 분석 권한이 있는 Live2D 에셋에만 사용해야 한다.
* 외부 연구 구현(moc2cmo, Stretchy Studio, py-moc3, Quadrism 등)은
  **포맷 연구 목적**으로만 참조했고, 코드를 복사하거나 런타임 의존성으로
  포함하지 않았다. 배포 패키지에는 프로젝트 자체 코드와 합성 에셋만 들어간다.
* Live2D 및 Cubism은 Live2D Inc.의 상표/저작물이며, 이 도구는 DRM이나
  라이선스 우회를 목적으로 하지 않는다.

## 9. 여담

* 저장소의 모든 `.moc3`/`.cmo3` 테스트 데이터는 프로젝트가 직접 만든 합성
  데이터이다. 타인의 모델은 포함되어 있지 않다.
* 차분 검증은 "정확도"가 아니라 **Cross-Implementation Agreement**로만
  부른다. 실제 정확도 측정은 AGENT.6(owned 모델 + Cubism 검증)에서 시작한다.
* private/owned 모델은 `owned-fixtures/`에 두면 git에 올라가지 않는다.

## 바깥 고리

* 저장소: https://github.com/loliRuriruri/LIVE2D-MOC3
* 인수인계(Codex): `docs/HANDOFF_CODEX.md`
* 상세 문서: `docs/` (`ARCHITECTURE.md`, `CMO3_WRITER.md`,
  `CMO3_VALIDATION.md`, `DIFFERENTIAL_FINDINGS.md`, `LIMITATIONS.md`,
  `docs/reports/AGENT*.md`)
