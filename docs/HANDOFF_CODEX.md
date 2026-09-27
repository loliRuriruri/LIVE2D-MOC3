# Codex Handoff — Live2D Project Recovery Tool (0.1.0-alpha)

> GPT Codex가 이 저장소에서 작업을 이어받기 위한 인수인계 문서.
> 작업 시작 전에 이 문서 → `WIKI.md` → `docs/reports/AGENT5.3_REPORT.md` 순서로 읽을 것.

## 0. 한 줄 요약

.moc3(런타임) → IR → 계층 → keyform → **.cmo3(에디터 프로젝트)** 파이프라인이
구현·검증되었고, Windows portable CLI `0.1.0-alpha` 패키지까지 완료되었다.
현재는 **Cubism Editor 인수 테스트 대기(동결)** 상태이며, 코드 수정은 금지다.

```text
Repository:  D:\test\liver2d
Remote:      https://github.com/loliRuriruri/LIVE2D-MOC3 (public, branch master)
HEAD:        f624a97  (docs: add namu-wiki style project overview)
Tests:       299 passed / 0 failed
clippy:      PASS (-D warnings)   fmt: PASS
Cubism:      NOT TESTED           Real-world: UNVALIDATED
Ready for AGENT.6: NO
```

## 1. 현재 동결 상태와 판정 규칙

사용자가 로컬 Cubism Editor에서 아래 3개를 직접 테스트한 뒤 결과를 전달할 예정이다.

```text
dist/Live2DRecovery-0.1.0-alpha-win-x64/acceptance/01-minimal.cmo3
                                                     /02-deformer.cmo3
                                                     /03-keyform.cmo3
```

* 3종 모두 Open/Save/Reopen + 주요 기능 PASS → **AGENT.6** 진행
* 하나라도 Open 실패 / repair·recovery 경고 / texture·hierarchy·keyform 이상
  → **AGENT.5.4 Cubism Compatibility Repair** 진행

결과가 오기 전까지 **금지**: AGENT.6, GUI, PSD 복원, 새 recovery heuristic,
새 MOC3 역공학. packaging/CLI 버그 수정만 허용.

## 2. 반드시 지킬 규칙 (위반 시 즉시 중단)

1. **새 복원 추론 금지** — writer/후속 단계는 저장된 값만 사용한다. 없는 부모·
   이름·keyform·보간을 지어내지 않는다.
2. **미해결은 실패 또는 표시** — strict 실패 / `--best-effort`에서
   `BEST_EFFORT_OUTPUT` 표시. silent repair 절대 금지.
3. **writer 기본값 분리** — synthetic/default 값은 `WRITER_REQUIRED_DEFAULT`로
   추적하고 recovered로 승격하지 않는다 (`docs/CMO3_WRITER_DEFAULTS.md`).
4. **외부 구현 의존 금지** — moc2cmo, Stretchy Studio, py-moc3, Quadrism 등은
   포맷 연구 참조일 뿐이다. 코드 복사·링크·번들·런타임 의존 금지
   (특히 Quadrism은 LGPL-3.0, behavioral only).
5. **검증 문구 분리** — `Structural validation: PASS`를 Cubism compatible로
   표현 금지. Cubism은 별도 status(`NOT TESTED`).
6. **private 자산 위생** — 실제 소유 모델은 `owned-fixtures/`(gitignore)에만.
   git history/문서에 private 모델 데이터 금지. report에는 hash/카운트만.
7. **커밋 규칙** — 작은 논리 단위 커밋, conventional prefix(`feat:`/`fix:`/
   `test:`/`docs:`/`chore:`), history rewrite 금지.
8. **스키마** — `live2d-ir/1`, `recovered-project/1`, `recovered-keyforms/1`은
   EXPERIMENTAL. 변경 시 사유·마이그레이션·테스트 필수.

## 3. 검증 명령 (변경 후 필수)

```powershell
cargo test --workspace              # 기대: 299 passed / 0 failed
cargo clippy --workspace --all-targets --all-features   # exit 0
cargo fmt --all --check             # clean
cargo build --release -p recovery-cli
target\release\recovery.exe self-test        # SELF TEST PASS (exit 0)
target\release\recovery.exe --version        # Live2DRecovery 0.1.0-alpha (commit …)
```

패키지 재생성(선택, python 필요):

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools\package\package_release.ps1
# 산출: dist/Live2DRecovery-0.1.0-alpha-win-x64/ + .zip (dist/는 gitignore)
```

E2E 시나리오는 `crates/recovery-core/tests/e2e_cmo3.rs`(10 케이스)와
`apps/recovery-cli/tests/cli_recover.rs`(8 케이스)에 있다.

## 4. 구조 요약

```text
crates/moc3-ingest         읽기 전용 .moc3 파서 (+fixture 전용 tools/fixture-gen)
crates/live2d-ir           정규화 IR/검증기/JSON
crates/hierarchy-recovery  계층 복원 (HR-001~010)
crates/keyform-recovery    바인딩/그리드/폼 (KB-001~007, KF-001~010)
crates/cmo3-writer         CAFF + main.xml + 검증 + self-test용 API
crates/recovery-core       파이프라인 연결, 파일 IO
apps/recovery-cli          recovery.exe (Live2DRecovery.exe로 배포)
tools/reference-harness    차분 검증 (개발 전용)
fixtures/synthetic         44 합성 fixture, expected-*/ golden
docs/                      설계·근거·게이트·리포트
```

핵심 문서: `docs/ARCHITECTURE.md`, `docs/CMO3_WRITER.md`,
`docs/CMO3_VALIDATION.md`(게이트 원장), `docs/CMO3_IMAGE_PIPELINE_EVIDENCE.md`,
`docs/CMO3_WRITER_DEFAULTS.md`, `docs/DIFFERENTIAL_FINDINGS.md`(DF-001~005),
`docs/LIMITATIONS.md`, `docs/reports/AGENT*.md`.

## 5. 다음 단계 플레이북

### AGENT.5.4 (Cubism 실패 시)

1. 사용자가 전달한 editor 버전/경고 원문/스크린샷을 `docs/reports/CUBISM_ACCEPTANCE_V0.1.md`에 기록.
2. 경고 문자열 → 해당 CMO3 요소/속성 추적 (`serialize/builders.rs`, `serialize/mod.rs`).
3. 최소 수정 + **최소 재현 + regression test** 필수.
4. 재패키징 후 acceptance 3종 재생성, 사용자 재테스트 요청.

### AGENT.6 (Cubism PASS 시)

1. owned real 모델 확보 절차: `docs/GROUND_TRUTH_BENCHMARK.md`.
2. 벤치마크: parse → IR → hierarchy → keyforms → cmo3 → Cubism open/save/reopen.
3. 측정 정의 준수: "accuracy" 표현은 owned+Cubism 검증 이후에만.
4. `docs/RUNTIME_ORACLE_PLAN.md`의 oracle 준비 상태 확인.

## 6. 알려진 한계 (그대로 유지할 것)

* Cubism Editor open/save/reopen: NOT TESTED (에디터 환경 없음).
* 실사용 호환성/계층 정확도/keyform 정확도: UNVALIDATED.
* draw-order group / glue / offscreen: unsupported note로만 기록(직렬화 안 함).
* 다축 grid ordering: Unknown + writer 기본값(fastest-first) 추적.
* `--force` 덮어쓰기 시 Windows 특성상 짧은 crash window 존재(문서화됨).
* 대형 모델 메모리 피크 ≈ 출력의 9배 (1500 mesh: 28.8 MB 출력, ~266 MB).
* blend mode는 Normal/Add/Multiply만 매핑, 나머지는 strict 실패.

## 7. Codex 시작 프롬프트 (복붙용)

```text
저장소 D:\test\liver2d (GitHub: loliRuriruri/LIVE2D-MOC3, branch master)에서
이어서 작업한다. 먼저 docs/HANDOFF_CODEX.md, WIKI.md,
docs/reports/AGENT5.3_REPORT.md, docs/CMO3_VALIDATION.md를 읽어라.

현재 상태: HEAD f624a97, 299 tests PASS, clippy/fmt PASS,
CLI 0.1.0-alpha 패키지 READY, Cubism acceptance는 아직 결과 대기 중이다.

[지금 할 일을 여기에 적는다: 예) "Cubism 결과가 왔고 02-deformer에서
경고 X가 나왔다. AGENT.5.4로 최소 수정 + regression test + 재패키징."]

규칙: 새 복원 추론 금지, 미해결 silent repair 금지, writer 기본값은
WRITER_REQUIRED_DEFAULT로 추적, 외부 구현 코드 사용 금지,
Structural PASS를 Cubism compatible로 표현 금지.
변경 후 cargo test --workspace / clippy -D warnings / fmt / self-test 필수.
```
