수정된 내용을 모두 반영하여 **CNPG 성능 테스트 가이드 및 결과 리포트** 템플릿을 다시 작성했습니다.

Direct 접속(RW/RO)과 PgBouncer(Pooler) 접속 시나리오가 명확히 분리되어 있으며, 명령어마다 `-h` 옵션을 명시하여 실수 없이 진행할 수 있도록 구성했습니다. 이 내용을 그대로 복사하여 사내 문서(Wiki)나 가이드 파일로 사용하시면 됩니다.

---

#📄 CNPG 성능 테스트 가이드 및 결과 리포트##1. 테스트 환경 (Test Environment)테스트 신뢰성을 위해 현재 환경 스펙을 기록합니다.

| 항목 | 상세 내용 (예시) | 비고 |
| --- | --- | --- |
| **K8s Cluster** | Production / Staging |  |
| **CNPG Version** | 1.18.0 |  |
| **PostgreSQL Ver** | 15.3 |  |
| **Resource (Pod)** | CPU: 2 Core / Mem: 4Gi | request/limit 동일 설정 권장 |
| **Storage (PVC)** | AWS gp3 / Azure Managed Disk | **IOPS/Throughput 스펙 기재** |
| **Node Spec** | m5.large (2 vCPU, 8GiB) | DB 파드가 배치된 노드 스펙 |
| **Service Names** | RW: `cluster-example-rw`<br>

<br>RO: `cluster-example-ro`<br>

<br>Pooler: `cluster-example-pooler-rw` | **실제 서비스명으로 수정 필요** |

---

##2. 테스트 준비 (Preparation)###2.1 벤치마크 클라이언트 생성DB 파드와 동일한 네트워크 상에서 테스트하기 위해 클라이언트 Pod를 생성합니다.

**`perf-test.yaml`**

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: cnpg-bench-client
spec:
  containers:
  - name: pgbench
    image: postgres:15
    command: ["sleep", "infinity"]
    env:
    - name: PGPASSWORD
      valueFrom:
        secretKeyRef:
          name: cluster-example-app  # CNPG Secret 이름 확인
          key: password
    - name: PGUSER
      value: app
    - name: PGDATABASE
      value: app
  restartPolicy: Never

```

* `kubectl apply -f perf-test.yaml` 로 생성

###2.2 데이터 초기화 (Initialize)테스트에 필요한 더미 데이터를 생성합니다. (기존 데이터 삭제됨 주의)

```bash
# Pod 접속
kubectl exec -it cnpg-bench-client -- bash

# 데이터 생성 (Scale Factor 100 = 약 1.5GB / 1,000만 행)
# 호스트(-h)는 RW 서비스로 지정
pgbench -h cluster-example-rw -i -s 100

```

---

##3. 테스트 시나리오 및 수행 방법각 시나리오의 목적에 맞는 **호스트(-h)**와 옵션을 사용하여 테스트를 수행합니다.

###🛑 공통 옵션 설명* **`-h`**: 접속할 서비스 주소 (Target Host)
* **`-c`**: 동시 접속자 수 (Clients)
* **`-j`**: 워커 스레드 수 (Jobs) - 보통 2~4 권장
* **`-T`**: 수행 시간 (Time, 초 단위) - 최소 60초 권장
* **`-P`**: 진행 상황 출력 (Progress, 초 단위) - 1초 권장

###🟢 시나리오 A: Direct RW (Primary) 부하 테스트* **목적:** DB 엔진 자체의 쓰기/읽기 통합 최대 성능 측정
* **대상 호스트:** `cluster-example-rw` (Primary Service)

```bash
# 1. 접속자 10명 (Baseline)
pgbench -h cluster-example-rw -c 10 -j 2 -T 60 -P 1

# 2. 접속자 50명
pgbench -h cluster-example-rw -c 50 -j 2 -T 60 -P 1

# 3. 접속자 100명 (병목 확인)
pgbench -h cluster-example-rw -c 100 -j 2 -T 60 -P 1

```

###🔵 시나리오 B: Direct RO (Replica) 조회 테스트* **목적:** Replica DB의 조회 전용 처리량 측정
* **대상 호스트:** `cluster-example-ro` (Replica Service)
* **필수 옵션:** `-S` (Select-only)

```bash
# 1. 접속자 50명 조회
pgbench -h cluster-example-ro -S -c 50 -j 2 -T 60 -P 1

# 2. 접속자 100명 조회
pgbench -h cluster-example-ro -S -c 100 -j 2 -T 60 -P 1

```

###🟣 시나리오 C: PgBouncer (Pooler) 대량 접속 테스트* **목적:** Connection Pooling 효율성 및 대량 트래픽 안정성 검증
* **대상 호스트:** `cluster-example-pooler-rw` (PgBouncer Service)
* **특이사항:** Direct 접속 시 오류가 날 법한 높은 접속자 수로 테스트

```bash
# 1. 접속자 100명 (Direct와 비교용)
pgbench -h cluster-example-pooler-rw -c 100 -j 2 -T 60 -P 1

# 2. 접속자 500명 (Pooler 효과 검증)
# 주의: 부하 생성을 위해 Job(-j) 수를 4로 상향
pgbench -h cluster-example-pooler-rw -c 500 -j 4 -T 60 -P 1

```

---

##4. 📊 성능 테스트 결과 리포트 (작성 양식)**테스트 일자:** 202X.XX.XX
**작성자:** OOO

###4.1 Direct RW (Primary) 테스트 결과* **Target:** `cluster-example-rw`

| Client 수 (-c) | TPS (excl. conn) | Avg Latency (ms) | CPU/Mem 사용률 | 비고 |
| --- | --- | --- | --- | --- |
| **10** |  |  |  | 정상 |
| **50** |  |  |  |  |
| **100** |  |  |  | 지연 발생 여부 확인 |

###4.2 Direct RO (Replica) 테스트 결과* **Target:** `cluster-example-ro` (Option: -S)

| Client 수 (-c) | TPS (excl. conn) | Avg Latency (ms) | 비고 |
| --- | --- | --- | --- |
| **50** |  |  |  |
| **100** |  |  |  |

###4.3 PgBouncer (Pooler) 테스트 결과* **Target:** `cluster-example-pooler-rw`

| Client 수 (-c) | TPS (excl. conn) | Avg Latency (ms) | 비고 |
| --- | --- | --- | --- |
| **100** |  |  | Direct 100명일 때와 비교 |
| **500** |  |  | **핵심 지표 (안정적으로 처리되는가?)** |

###4.4 💡 Direct vs Pooler 종합 비교 및 결론1. **Direct vs Pooler 성능 차이:**
* Client 100명 기준: Direct (TPS: `____`) vs Pooler (TPS: `____`)
* *분석:* (예: 접속자가 적을 때는 Direct가 빠르나, 많아지면 Pooler가 안정적임 등)


2. **최대 성능(Capacity) 분석:**
* 최대 TPS는 **OOOO** (Client **OO**명 일 때)
* 병목 원인: (CPU / Memory / Disk I/O 중 택1)


3. **개선 제안:**
* (예: Connection 수가 많으므로 Pooler 사용 필수)
* (예: PVC 디스크 IOPS 상향 필요)
