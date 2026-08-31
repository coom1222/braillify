# 점자세상 (braillekorea.org) 정답률 벤치마크

- 측정일: 2026-08-31
- 비교 기준: PDF 규정 (2024 개정 한국 점자 규정)
  - PDF 정답 = test_cases JSON 의 `unicode` 필드
  - 점자세상 결과 = test_cases JSON 의 `world` 필드 (fetch-world.ts 가 braillekorea.org API 에서 수집)
- 비교 방식: 유니코드 문자열 동치. 입력 경계가 영문자인 경우에만 API의 외곽 영문 시작(⠴)/끝(⠲) 표지를 비교에서 제외한다.
- Skip 정책: LaTeX 변형, 빈 input, world 미수집, unicode 미정의 항목 제외

## 전체 요약

| 항목 | 값 |
|---|---:|
| 전체 testcase | 5141 |
| 측정 대상 | 1939 |
| 제외 (LaTeX) | 351 |
| 제외 (빈 input) | 0 |
| 제외 (world 미수집) | 2851 |
| 제외 (unicode 없음) | 0 |
| **점자세상 PDF 정답 일치** | **671 (34.61%)** |
| **점자세상 PDF 정답 불일치** | **1268 (65.39%)** |

> 참고 — braillify 의 PDF 정답 일치: **2419/2419 = 100.00%** (cargo test test_by_testcase).
> 단, braillify 측정에는 `KNOWN_FAILURES` 라우팅이 포함되어 있어 raw encode 정답률은 별도 측정 필요.

## 카테고리별

| 카테고리 | 전체 | 측정 | 일치 | 불일치 | 일치율 |
|---|---:|---:|---:|---:|---:|
| english/ | 2722 | 0 | 0 | 0 | 0.0% |
| korean/ | 1527 | 1465 | 602 | 863 | 41.09% |
| math/ | 892 | 474 | 69 | 405 | 14.56% |

## 파일별 (상위 30개, 일치율 낮은 순)

| 파일 | 측정 | 일치 | 불일치 | 일치율 |
|---|---:|---:|---:|---:|
| korean/rule_10.json | 4 | 0 | 4 | 0.00% |
| korean/rule_12_b1.json | 2 | 0 | 2 | 0.00% |
| korean/rule_14_b1.json | 3 | 0 | 3 | 0.00% |
| korean/rule_19.json | 8 | 0 | 8 | 0.00% |
| korean/rule_20.json | 2 | 0 | 2 | 0.00% |
| korean/rule_21.json | 2 | 0 | 2 | 0.00% |
| korean/rule_22.json | 7 | 0 | 7 | 0.00% |
| korean/rule_23.json | 8 | 0 | 8 | 0.00% |
| korean/rule_24.json | 11 | 0 | 11 | 0.00% |
| korean/rule_25.json | 7 | 0 | 7 | 0.00% |
| korean/rule_26.json | 2 | 0 | 2 | 0.00% |
| korean/rule_27.json | 7 | 0 | 7 | 0.00% |
| korean/rule_29.json | 3 | 0 | 3 | 0.00% |
| korean/rule_30.json | 51 | 0 | 51 | 0.00% |
| korean/rule_31.json | 2 | 0 | 2 | 0.00% |
| korean/rule_32.json | 3 | 0 | 3 | 0.00% |
| korean/rule_33.json | 4 | 0 | 4 | 0.00% |
| korean/rule_33_b1.json | 7 | 0 | 7 | 0.00% |
| korean/rule_34.json | 3 | 0 | 3 | 0.00% |
| korean/rule_35.json | 11 | 0 | 11 | 0.00% |
| korean/rule_36.json | 19 | 0 | 19 | 0.00% |
| korean/rule_37.json | 32 | 0 | 32 | 0.00% |
| korean/rule_38.json | 4 | 0 | 4 | 0.00% |
| korean/rule_39.json | 3 | 0 | 3 | 0.00% |
| korean/rule_42.json | 2 | 0 | 2 | 0.00% |
| korean/rule_44_b1.json | 8 | 0 | 8 | 0.00% |
| korean/rule_46.json | 5 | 0 | 5 | 0.00% |
| korean/rule_47.json | 4 | 0 | 4 | 0.00% |
| korean/rule_48.json | 1 | 0 | 1 | 0.00% |
| korean/rule_51.json | 1 | 0 | 1 | 0.00% |

## 해석

이 측정은 점자세상의 PDF 규정 준수도에 대한 객관적 지표이다.
일치하지 않는 testcase는 점자세상 결과가 2024 개정 한국 점자 규정과 다르다는 의미이며,
braillify 의 정답성과는 무관하다 (braillify 알고리즘은 점자세상 결과를 참조하지 않는다 — AGENTS.md RED LINE).

상세 미스매치 목록은 [`WORLD_MISMATCHES.md`](./WORLD_MISMATCHES.md) 참고.
