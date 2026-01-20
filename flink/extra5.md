좋습니다! 상세한 정보 감사합니다. 하나씩 정리하겠습니다.
1. Step Start/End 시그널 방식 비교

┌─────────────────────────────────────────────────────────────────────────┐
│           방식 A: 별도 토픽으로 Step Start/End 제공                      │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│   TraceFlat 토픽 ─────────────────┐                                     │
│                                   │                                     │
│   StepEvent 토픽 ─────┐           │                                     │
│   (Start/End 이벤트)  │           │                                     │
│                       ▼           ▼                                     │
│                 ┌─────────────────────────┐                             │
│                 │   Flink: Interval Join  │                             │
│                 │   또는 CoProcess        │                             │
│                 └─────────────────────────┘                             │
│                                                                         │
│   장점:                                                                 │
│   ✅ 명확한 경계 신호                                                   │
│   ✅ 토픽 분리로 관심사 분리                                            │
│                                                                         │
│   단점:                                                                 │
│   ❌ 두 스트림 Join 필요 (Stateful, 복잡)                               │
│   ❌ 시간 동기화 문제 (Start 도착 전에 데이터 도착 가능)                │
│   ❌ End 누락 시 Window 닫히지 않음                                     │
│   ❌ 다른 팀 의존도 증가 (또 다른 토픽 품질 의존)                       │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│           방식 B: TraceRaw/Flat 내에 Start/End 메시지 포함               │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│   TraceFlat 토픽                                                        │
│   ┌─────────────────────────────────────────────────────────────┐       │
│   │  { type: "STEP_START", stepId: "6", ... }                   │       │
│   │  { type: "TRACE_DATA", paramValue: "0.5", ... }             │       │
│   │  { type: "TRACE_DATA", paramValue: "0.7", ... }             │       │
│   │  { type: "TRACE_DATA", paramValue: "0.6", ... }             │       │
│   │  { type: "STEP_END", stepId: "6", ... }                     │       │
│   └─────────────────────────────────────────────────────────────┘       │
│                       │                                                 │
│                       ▼                                                 │
│                 ┌─────────────────────────┐                             │
│                 │   Flink: 단일 스트림    │                             │
│                 │   State Machine 처리    │                             │
│                 └─────────────────────────┘                             │
│                                                                         │
│   장점:                                                                 │
│   ✅ 단일 토픽 → Join 불필요                                            │
│   ✅ 순서 보장 (같은 파티션 내)                                         │
│   ✅ Start/End와 데이터 원자적 전달                                     │
│                                                                         │
│   단점:                                                                 │
│   ❌ 메시지 타입 분기 로직 필요                                         │
│   ❌ 스키마 복잡 (여러 타입 공존)                                       │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘


Flink 관점 권장: 방식 B

┌─────────────────────────────────────────────────────────────────────────┐
│                    권장 이유                                             │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│   1. 순서 보장이 핵심                                                   │
│      - Integral 계산에 시간순 정렬 필요                                 │
│      - 같은 파티션 내 순서 보장 활용 가능                               │
│      - 별도 토픽 시 순서 보장 불가                                      │
│                                                                         │
│   2. Session Window 대체 가능                                           │
│      - STEP_START → State 초기화                                        │
│      - TRACE_DATA → State 누적                                          │
│      - STEP_END → 집계 트리거 + State Flush                             │
│                                                                         │
│   3. 누락 처리 용이                                                     │
│      - STEP_END 없이 새 STEP_START 도착 → 이전 Step 강제 종료           │
│      - Timeout 기반 강제 종료 로직 추가 가능                            │
│                                                                         │
│   권장 파티션 키: fab + eqpId + fdcChambId (장비 단위)                   │
│   → 같은 장비의 Step 이벤트가 순서대로 도착                             │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘


2. Trino 쿼리: 카디널리티 체크
2-1. GroupBy 키 조합의 카디널리티

-- fdc_trace_flat 테이블 기준
-- 전체 유니크 키 조합 수 확인

SELECT COUNT(*) AS total_unique_keys
FROM (
    SELECT DISTINCT
        fab,
        eqpId,
        fdcChambId,
        lotCd,
        lotId,
        slotNo,
        prodId,
        operId,
        operDesc,
        recipeId,
        mesRecipeId,
        batchId,
        batchTyp,
        name,        -- paramList.name
        alias,       -- paramList.alias
        svid         -- paramList.svid
        -- fdcLotCd는 현재 없으므로 제외
    FROM fdc_trace_flat
    WHERE eventTime >= TIMESTAMP '2026-01-01 00:00:00'  -- 분석 기간 지정
      AND eventTime < TIMESTAMP '2026-01-14 00:00:00'
) subq;


2-2. 각 키 컬럼별 카디널리티 분해

-- 어떤 컬럼이 카디널리티를 높이는지 확인

SELECT 
    'fab' AS column_name, COUNT(DISTINCT fab) AS cardinality FROM fdc_trace_flat WHERE eventTime >= TIMESTAMP '2026-01-01'
UNION ALL
SELECT 'eqpId', COUNT(DISTINCT eqpId) FROM fdc_trace_flat WHERE eventTime >= TIMESTAMP '2026-01-01'
UNION ALL
SELECT 'fdcChambId', COUNT(DISTINCT fdcChambId) FROM fdc_trace_flat WHERE eventTime >= TIMESTAMP '2026-01-01'
UNION ALL
SELECT 'lotCd', COUNT(DISTINCT lotCd) FROM fdc_trace_flat WHERE eventTime >= TIMESTAMP '2026-01-01'
UNION ALL
SELECT 'lotId', COUNT(DISTINCT lotId) FROM fdc_trace_flat WHERE eventTime >= TIMESTAMP '2026-01-01'
UNION ALL
SELECT 'slotNo', COUNT(DISTINCT slotNo) FROM fdc_trace_flat WHERE eventTime >= TIMESTAMP '2026-01-01'
UNION ALL
SELECT 'prodId', COUNT(DISTINCT prodId) FROM fdc_trace_flat WHERE eventTime >= TIMESTAMP '2026-01-01'
UNION ALL
SELECT 'operId', COUNT(DISTINCT operId) FROM fdc_trace_flat WHERE eventTime >= TIMESTAMP '2026-01-01'
UNION ALL
SELECT 'recipeId', COUNT(DISTINCT recipeId) FROM fdc_trace_flat WHERE eventTime >= TIMESTAMP '2026-01-01'
UNION ALL
SELECT 'mesRecipeId', COUNT(DISTINCT mesRecipeId) FROM fdc_trace_flat WHERE eventTime >= TIMESTAMP '2026-01-01'
UNION ALL
SELECT 'batchId', COUNT(DISTINCT batchId) FROM fdc_trace_flat WHERE eventTime >= TIMESTAMP '2026-01-01'
UNION ALL
SELECT 'batchTyp', COUNT(DISTINCT batchTyp) FROM fdc_trace_flat WHERE eventTime >= TIMESTAMP '2026-01-01'
UNION ALL
SELECT 'name (param)', COUNT(DISTINCT name) FROM fdc_trace_flat WHERE eventTime >= TIMESTAMP '2026-01-01'
UNION ALL
SELECT 'alias (param)', COUNT(DISTINCT alias) FROM fdc_trace_flat WHERE eventTime >= TIMESTAMP '2026-01-01'
UNION ALL
SELECT 'svid (param)', COUNT(DISTINCT svid) FROM fdc_trace_flat WHERE eventTime >= TIMESTAMP '2026-01-01'
ORDER BY cardinality DESC;


2-3. 시간대별 동시 활성 키 수 (State 크기 추정)

-- 특정 시간 윈도우 내 동시에 존재하는 유니크 키 수
-- 이 값이 Flink State에 동시에 유지되는 그룹 수

SELECT 
    date_trunc('hour', eventTime) AS hour_bucket,
    COUNT(DISTINCT CONCAT(
        fab, '|', eqpId, '|', fdcChambId, '|', 
        lotCd, '|', lotId, '|', CAST(slotNo AS VARCHAR), '|',
        prodId, '|', operId, '|', recipeId, '|',
        mesRecipeId, '|', batchId, '|', batchTyp, '|',
        name, '|', alias, '|', svid
    )) AS concurrent_keys
FROM fdc_trace_flat
WHERE eventTime >= TIMESTAMP '2026-01-13 00:00:00'
  AND eventTime < TIMESTAMP '2026-01-14 00:00:00'
GROUP BY date_trunc('hour', eventTime)
ORDER BY hour_bucket;


3. Trino 쿼리: Step당 데이터 포인트 수
3-1. Step 정의 (stepId + 추가 컨텍스트)

-- Step 단위 = lotId + slotNo + stepId + recipeId + eqpId 조합 (가정)
-- 실제 Step 경계 정의에 따라 조정 필요

SELECT 
    fab,
    eqpId,
    fdcChambId,
    lotId,
    slotNo,
    stepId,
    stepNm,
    recipeId,
    name AS param_name,
    COUNT(*) AS data_points_per_step_per_param,
    MIN(eventTime) AS step_start,
    MAX(eventTime) AS step_end,
    date_diff('millisecond', MIN(eventTime), MAX(eventTime)) AS duration_ms
FROM fdc_trace_flat
WHERE eventTime >= TIMESTAMP '2026-01-13 00:00:00'
  AND eventTime < TIMESTAMP '2026-01-14 00:00:00'
GROUP BY 
    fab, eqpId, fdcChambId, lotId, slotNo, 
    stepId, stepNm, recipeId, name
ORDER BY data_points_per_step_per_param DESC
LIMIT 100;


3-2. Step당 평균/최대 데이터 포인트 통계

-- Percentile/Median 계산을 위해 Step당 몇 개의 값을 메모리에 보관해야 하는지

WITH step_param_counts AS (
    SELECT 
        fab,
        eqpId,
        lotId,
        stepId,
        recipeId,
        name AS param_name,
        COUNT(*) AS data_points
    FROM fdc_trace_flat
    WHERE eventTime >= TIMESTAMP '2026-01-13 00:00:00'
      AND eventTime < TIMESTAMP '2026-01-14 00:00:00'
    GROUP BY fab, eqpId, lotId, stepId, recipeId, name
)
SELECT 
    MIN(data_points) AS min_points,
    MAX(data_points) AS max_points,
    AVG(data_points) AS avg_points,
    APPROX_PERCENTILE(data_points, 0.5) AS median_points,
    APPROX_PERCENTILE(data_points, 0.95) AS p95_points,
    APPROX_PERCENTILE(data_points, 0.99) AS p99_points,
    COUNT(*) AS total_step_param_combinations
FROM step_param_counts;


3-3. Step당 파라미터 수

-- 하나의 Step에서 몇 개의 파라미터를 추적하는지

WITH step_params AS (
    SELECT 
        fab,
        eqpId,
        lotId,
        stepId,
        recipeId,
        COUNT(DISTINCT name) AS param_count
    FROM fdc_trace_flat
    WHERE eventTime >= TIMESTAMP '2026-01-13 00:00:00'
      AND eventTime < TIMESTAMP '2026-01-14 00:00:00'
    GROUP BY fab, eqpId, lotId, stepId, recipeId
)
SELECT 
    MIN(param_count) AS min_params,
    MAX(param_count) AS max_params,
    AVG(param_count) AS avg_params,
    APPROX_PERCENTILE(param_count, 0.5) AS median_params
FROM step_params;


3-4. State 크기 추정용 종합 쿼리

-- Flink State 메모리 요구량 추정을 위한 종합 정보

WITH step_stats AS (
    SELECT 
        fab,
        eqpId,
        fdcChambId,
        lotId,
        slotNo,
        stepId,
        recipeId,
        name AS param_name,
        COUNT(*) AS data_points,
        MIN(eventTime) AS step_start,
        MAX(eventTime) AS step_end
    FROM fdc_trace_flat
    WHERE eventTime >= TIMESTAMP '2026-01-13 00:00:00'
      AND eventTime < TIMESTAMP '2026-01-14 00:00:00'
    GROUP BY fab, eqpId, fdcChambId, lotId, slotNo, stepId, recipeId, name
)
SELECT 
    '=== Step당 데이터 포인트 ===' AS metric_group,
    NULL AS value
UNION ALL
SELECT 'avg_data_points_per_step_param', CAST(AVG(data_points) AS VARCHAR) FROM step_stats
UNION ALL
SELECT 'max_data_points_per_step_param', CAST(MAX(data_points) AS VARCHAR) FROM step_stats
UNION ALL
SELECT 'p99_data_points', CAST(APPROX_PERCENTILE(data_points, 0.99) AS VARCHAR) FROM step_stats
UNION ALL
SELECT '=== 동시 활성 Step 수 ===' AS metric_group, NULL
UNION ALL
SELECT 'total_step_param_combinations', CAST(COUNT(*) AS VARCHAR) FROM step_stats
UNION ALL
SELECT '=== 메모리 추정 (8 bytes per double) ===', NULL
UNION ALL
SELECT 'estimated_max_memory_per_key_MB', 
       CAST(MAX(data_points) * 8 / 1024.0 / 1024.0 AS VARCHAR) 
FROM step_stats;


4. 쿼리 결과 해석 가이드

┌─────────────────────────────────────────────────────────────────────────┐
│                    결과 해석 및 설계 반영                                │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│   쿼리 결과 예시:                                                       │
│   ┌─────────────────────────────────────────────────────────────┐       │
│   │  avg_data_points_per_step_param: 500                        │       │
│   │  max_data_points_per_step_param: 5,000                      │       │
│   │  p99_data_points: 2,000                                     │       │
│   │  concurrent_active_keys (hourly): 50,000                    │       │
│   └─────────────────────────────────────────────────────────────┘       │
│                                                                         │
│   State 메모리 계산:                                                    │
│   ┌─────────────────────────────────────────────────────────────┐       │
│   │  median/percentile 계산 → 모든 값 보관 필요                 │       │
│   │                                                             │       │
│   │  키당 메모리 = p99_data_points × 8 bytes (double)           │       │
│   │             = 2,000 × 8 = 16 KB                             │       │
│   │                                                             │       │
│   │  총 State = concurrent_keys × 키당 메모리                   │       │
│   │          = 50,000 × 16 KB = 800 MB (데이터만)               │       │
│   │                                                             │       │
│   │  RocksDB 오버헤드 포함 = 800 MB × 2~3 = 1.6 ~ 2.4 GB        │       │
│   └─────────────────────────────────────────────────────────────┘       │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘


쿼리 결과가 나오면 공유해주세요. 그 숫자를 바탕으로:
	1.	State 크기 정확히 산정
	2.	Parallelism 결정
	3.	Checkpoint 전략 수립
을 진행할 수 있습니다.​​​​​​​​​​​​​​​​