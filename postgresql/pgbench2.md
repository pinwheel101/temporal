# pgbench 실전 사용 가이드

## 1. pgbench 기본 구조

```
pgbench 워크플로우:
1. 초기화 (pgbench -i)  → 테스트용 테이블 생성
2. 벤치마크 (pgbench)    → 부하 발생 및 측정
3. 결과 분석             → TPS, 레이턴시 해석
```

## 2. 핵심 옵션 완전 정리

### 초기화 옵션 (-i 모드)

```bash
pgbench -i [options] dbname

# 가장 중요한 옵션
-s, --scale=NUM          # 스케일 팩터 (기본 1)
                         # scale 1 = 약 15MB
                         # scale 100 = 약 1.5GB
                         # scale 1000 = 약 15GB
                         # scale 10000 = 약 150GB

-F, --fillfactor=NUM     # 테이블 fillfactor (기본 100)
                         # UPDATE 많으면 70-80 권장

--foreign-keys           # 외래키 생성 (리얼리즘↑, 성능↓)

--partitions=NUM         # pgbench_accounts를 파티션으로
--partition-method=NAME  # range/hash

# 예시: 100GB DB, 파티션, 외래키
pgbench -i -s 6700 --partitions=10 --partition-method=hash \
  --foreign-keys -h cnpg-rw -U postgres perftest
```

**스케일 팩터 결정 기준:**
- shared_buffers보다 작으면 → 캐시 hit 테스트
- shared_buffers보다 크면 → 실제 디스크 I/O 테스트
- 프로덕션 DB 크기와 유사하게 설정 권장

### 벤치마크 핵심 옵션

```bash
pgbench [options] dbname

# === 부하 제어 ===
-c, --client=NUM         # 동시 클라이언트 수 (중요!)
-j, --jobs=NUM           # pgbench 워커 스레드 수
                         # 보통 -j = 코어 수, -c는 실험 변수
-T, --time=NUM           # 실행 시간 (초)
-t, --transactions=NUM   # 클라이언트당 트랜잭션 수
                         # -T와 -t 중 하나만 사용

# === 워크로드 타입 ===
-S, --select-only        # SELECT만 (read-only)
-N, --skip-some-updates  # UPDATE 제외 (light write)
-b, --builtin=NAME       # 내장 스크립트
  # tpcb-like (기본): SELECT, UPDATE, INSERT
  # simple-update: UPDATE만
  # select-only: SELECT만

-f, --file=FILENAME      # 커스텀 SQL 스크립트
-F, --protocol=NAME      # simple/extended/prepared

# === 성능 측정 ===
-r, --report-per-command # 명령어별 레이턴시 (필수!)
-P, --progress=NUM       # NUM초마다 진행상황 출력
-l, --log                # 상세 로그 기록
--log-prefix=PREFIX      # 로그 파일 프리픽스

# === 결과 정확도 ===
-C, --connect            # 매 트랜잭션마다 재연결
                         # (커넥션 풀 테스트 시)
-M, --protocol=NAME      # simple/extended/prepared
-R, --rate=NUM           # 목표 TPS 제한 (throttling)
--sampling-rate=NUM      # 로깅 샘플링 비율
```

## 3. 실전 시나리오별 커맨드

### Scenario 1: 기본 성능 베이스라인

```bash
# 목적: "이 PostgreSQL이 얼마나 빠른가?"

# Step 1: 적당한 크기로 초기화
pgbench -i -s 1000 -h cnpg-bmc-rw -U postgres perftest

# Step 2: Read-Write Mixed (기본)
pgbench -c 50 -j 10 -T 300 -r -P 10 \
  -h cnpg-bmc-rw -U postgres perftest

# 해석:
# - TPS: 전체 처리량
# - latency average: 평균 응답시간
# - latency stddev: 편차 (클수록 불안정)
```

### Scenario 2: 동시성 스케일링 테스트

```bash
# 목적: "클라이언트 수 증가 시 TPS는?"

for clients in 10 50 100 200 400; do
  echo "=== Testing with $clients clients ==="
  pgbench -c $clients -j 10 -T 120 -r -P 10 \
    --log --log-prefix=scale_${clients}_ \
    -h cnpg-bmc-rw -U postgres perftest
  sleep 30  # 안정화 대기
done

# 결과 분석:
# - 선형 증가 → 병목 없음
# - TPS 정체/하락 → CPU/Lock 경합
```

### Scenario 3: Read-Only 성능 (캐시 효율)

```bash
# 목적: "캐시가 얼마나 효과적인가?"

# Small dataset (캐시에 모두 들어감)
pgbench -i -s 100 -h cnpg-bmc-rw -U postgres cache_test
pgbench -c 100 -j 10 -T 300 -S -r -P 10 \
  -h cnpg-bmc-rw -U postgres cache_test

# Large dataset (캐시 미스 발생)
pgbench -i -s 5000 -h cnpg-bmc-rw -U postgres cache_test
pgbench -c 100 -j 10 -T 300 -S -r -P 10 \
  -h cnpg-bmc-rw -U postgres cache_test

# 비교:
# - Small: TPS 높음, latency 낮음
# - Large: TPS 낮음, latency 높음 (I/O 대기)
```

### Scenario 4: Write-Heavy 워크로드

```bash
# 목적: "WAL write 성능 측정"

pgbench -i -s 1000 -h cnpg-bmc-rw -U postgres perftest

# Simple update (WAL만 집중)
pgbench -c 50 -j 10 -T 300 -N -r -P 10 \
  -h cnpg-bmc-rw -U postgres perftest

# 동시 모니터링 (다른 터미널)
watch -n 5 "psql -h cnpg-bmc-rw -U postgres -c \"
  SELECT 
    (pg_wal_lsn_diff(pg_current_wal_lsn(), '0/0')/1024/1024)::int AS wal_mb,
    pg_current_wal_lsn() AS current_lsn;
\""
```

### Scenario 5: 커넥션 풀 vs Direct 연결

```bash
# Direct 연결 (커넥션 오버헤드 포함)
pgbench -c 100 -j 10 -T 300 -C -r \
  -h cnpg-bmc-rw -U postgres perftest

# Persistent 연결 (기본)
pgbench -c 100 -j 10 -T 300 -r \
  -h cnpg-bmc-rw -U postgres perftest

# 비교:
# - Direct: 낮은 TPS (연결 오버헤드)
# - Persistent: 높은 TPS
# → PgBouncer 효과 측정 가능
```

### Scenario 6: 커스텀 워크로드 (실제 쿼리)

```bash
# custom_workload.sql 작성
cat > custom_workload.sql << 'EOF'
\set aid random(1, 100000)
\set bid random(1, 1000)
\set delta random(-5000, 5000)

BEGIN;
SELECT abalance FROM pgbench_accounts WHERE aid = :aid;
UPDATE pgbench_accounts SET abalance = abalance + :delta WHERE aid = :aid;
UPDATE pgbench_branches SET bbalance = bbalance + :delta WHERE bid = :bid;
INSERT INTO pgbench_history (tid, bid, aid, delta, mtime) 
  VALUES (:bid, :bid, :aid, :delta, CURRENT_TIMESTAMP);
END;
EOF

# 실행
pgbench -c 50 -j 10 -T 300 -f custom_workload.sql -r -P 10 \
  -h cnpg-bmc-rw -U postgres perftest
```

### Scenario 7: Rate Limiting (안정성 테스트)

```bash
# 목적: "목표 TPS 유지하며 장시간 안정성"

# 목표 5000 TPS로 1시간
pgbench -c 100 -j 10 -T 3600 -R 5000 -r -P 60 \
  --log --log-prefix=stability_ \
  -h cnpg-bmc-rw -U postgres perftest

# 분석:
# - 목표 달성 여부
# - 시간에 따른 레이턴시 증가 (메모리 누수?)
# - 체크포인트 시점의 스파이크
```

## 4. 결과 해석 가이드

### 기본 출력 이해

```
transaction type: <builtin: TPC-B (sort of)>
scaling factor: 1000
query mode: simple
number of clients: 50
number of threads: 10
maximum number of tries: 1
duration: 300 s
number of transactions actually processed: 1523450
number of failed transactions: 0 (0.000%)
latency average = 9.845 ms          ← 평균 응답시간
latency stddev = 8.234 ms           ← 표준편차 (중요!)
initial connection time = 45.123 ms
tps = 5078.166667 (without initial connection time)  ← TPS
```

**주요 지표 해석:**

```yaml
TPS (Transactions Per Second):
  > 10000: 매우 우수
  5000-10000: 우수
  1000-5000: 보통
  < 1000: 문제 있음 (조사 필요)

Latency Average:
  < 5ms: 매우 빠름
  5-10ms: 빠름
  10-50ms: 보통
  > 50ms: 느림

Latency Stddev:
  중요! 평균보다 편차가 중요
  stddev < average: 안정적
  stddev > average: 불안정 (조사 필요)
```

### -r 옵션 상세 출력

```
statement latencies in milliseconds and failures:
         0.002           0  \set aid random(1, 100000 * :scale)
         0.001           0  \set bid random(1, 1 * :scale)
         0.001           0  \set tid random(1, 10 * :scale)
         0.001           0  \set delta random(-5000, 5000)
         0.037           0  BEGIN;
         0.105           0  UPDATE pgbench_accounts SET abalance = ...
         1.234           0  SELECT abalance FROM pgbench_accounts WHERE aid = :aid;
         0.089           0  UPDATE pgbench_tellers SET tbalance = ...
         0.076           0  UPDATE pgbench_branches SET bbalance = ...
         0.067           0  INSERT INTO pgbench_history ...
         8.543           0  END;
```

**병목 찾기:**
- 어떤 쿼리가 가장 느린가?
- UPDATE vs SELECT 비율
- COMMIT(END) 시간 → WAL 성능

### 로그 파일 분석 (--log)

```bash
# 생성된 로그: pgbench_log.12345.1
# 형식: client_id transaction_no time script_no time_epoch time_us [schedule_lag]

# 레이턴시 분포 확인
cat pgbench_log.* | awk '{print $3}' | sort -n | \
  awk '{
    sum += $1; 
    values[NR] = $1
  } 
  END {
    print "Min:", values[1]
    print "P50:", values[int(NR*0.5)]
    print "P95:", values[int(NR*0.95)]
    print "P99:", values[int(NR*0.99)]
    print "Max:", values[NR]
    print "Avg:", sum/NR
  }'

# 시간대별 TPS 추이
cat pgbench_log.* | awk '{print int($5)}' | uniq -c
```

## 5. 실전 팁 및 함정

### 반드시 피해야 할 실수

```bash
# ❌ 잘못된 예: -j가 너무 큼
pgbench -c 10 -j 100  # 워커가 클라이언트보다 많음

# ✅ 올바른 예: -j는 CPU 코어 수 정도
pgbench -c 100 -j 16  # 클라이언트 >> 워커

# ❌ 잘못된 예: 너무 짧은 테스트
pgbench -c 100 -T 10  # 10초는 워밍업도 안됨

# ✅ 올바른 예: 최소 5분, 권장 10분+
pgbench -c 100 -T 600

# ❌ 잘못된 예: scale이 너무 작음
pgbench -i -s 1  # 15MB, 모두 메모리에
pgbench -c 100 -T 300

# ✅ 올바른 예: 실전과 유사한 크기
pgbench -i -s 5000  # 75GB
```

### 테스트 전 체크리스트

```sql
-- 1. 현재 설정 확인
SHOW shared_buffers;
SHOW work_mem;
SHOW effective_cache_size;
SHOW checkpoint_timeout;
SHOW max_wal_size;

-- 2. 캐시 워밍업 (선택적)
SELECT pg_prewarm('pgbench_accounts');

-- 3. 통계 초기화 (재현성)
SELECT pg_stat_reset();
SELECT pg_stat_reset_shared('bgwriter');
```

### 모니터링과 함께 실행

```bash
# Terminal 1: pgbench 실행
pgbench -c 100 -j 10 -T 600 -r -P 10 \
  -h cnpg-bmc-rw -U postgres perftest

# Terminal 2: PostgreSQL 메트릭
watch -n 5 "psql -h cnpg-bmc-rw -U postgres << 'EOF'
SELECT 
  numbackends,
  xact_commit,
  xact_rollback,
  blks_read,
  blks_hit,
  tup_returned,
  tup_fetched,
  tup_inserted,
  tup_updated
FROM pg_stat_database WHERE datname = 'perftest';
EOF"

# Terminal 3: 시스템 리소스
kubectl top pod -n bmc-prod -l cnpg.io/cluster=bmc-cluster
```

## 6. CNPG 특화 테스트 시나리오

### Primary vs Replica 성능 비교

```bash
# Primary에 Write 부하
pgbench -c 50 -j 10 -T 300 -N -r \
  -h cnpg-bmc-cluster-rw -U postgres perftest &

# Replica에 Read 부하 (동시)
pgbench -c 50 -j 10 -T 300 -S -r \
  -h cnpg-bmc-cluster-ro -U postgres perftest &

wait

# 확인 사항:
# - Replica lag 발생 여부
# - Primary TPS 저하 없는지
# - Replica의 cache hit ratio
```

### Failover 중 성능 영향

```bash
# 백그라운드로 지속적인 부하
pgbench -c 50 -j 10 -T 1800 -r -P 5 \
  --log --log-prefix=failover_ \
  -h cnpg-bmc-cluster-rw -U postgres perftest &

PGBENCH_PID=$!

# 5분 후 Primary Pod 삭제
sleep 300
kubectl delete pod -n bmc-prod cnpg-bmc-cluster-1

# pgbench 결과에서 spike 확인
wait $PGBENCH_PID

# 로그 분석으로 정확한 failover 시간 측정
```

## 7. 결과 비교 스크립트

```bash
#!/bin/bash
# compare_pgbench_results.sh

echo "Test,Clients,TPS,Latency_Avg,Latency_Stddev,Latency_P95"

for log in pgbench_*.log; do
  test_name=$(basename $log .log)
  clients=$(grep "number of clients" $log | awk '{print $5}')
  tps=$(grep "^tps" $log | awk '{print $3}')
  lat_avg=$(grep "latency average" $log | awk '{print $4}')
  lat_std=$(grep "latency stddev" $log | awk '{print $4}')
  
  echo "$test_name,$clients,$tps,$lat_avg,$lat_std"
done | column -t -s,
```

이제 pgbench로 CNPG의 실제 성능을 정확하게 측정할 수 있습니다. 특정 시나리오나 옵션에 대해 더 궁금한 점이 있으면 말씀해주세요!
