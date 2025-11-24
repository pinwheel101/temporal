# **쿠버네티스 Trino 구축 검증 및 테스트 계획서**

작성일: 2025년 00월 00일  
대상 클러스터: 온프레미스 K8s (191 Node)  
테스트 목적: Trino Coordinator/Worker 정상 기동 확인, 외부 카탈로그(Iceberg/MinIO) 연동, 고성능 쿼리 처리(Spill to NVMe) 및 인증/인가(Keycloak) 검증

## **1\. 인프라 및 배포 상태 점검 (Infrastructure Check)**

Trino 클러스터가 K8s 상에 정상적으로 배포되었고, 계획된 하드웨어 자원(NVMe 등)을 점유하고 있는지 확인합니다.

| ID | 테스트 항목 | 점검 방법 (CLI/UI) | 예상 결과 (Expected Result) | 확인 |
| :---- | :---- | :---- | :---- | :---- |
| **INF-01** | **Pod 상태 점검** | kubectl get pods \-n \<trino-namespace\> | Coordinator 1개와 설정한 수(예: 180개)의 Worker Pod가 모두 Running 및 Ready 상태여야 함. | \[ \] |
| **INF-02** | **Service/Ingress 점검** | kubectl get svc, kubectl get ingress | Trino UI 접근용 Service가 생성되어 있고, Ingress 도메인으로 Web UI 접속이 가능해야 함. | \[ \] |
| **INF-03** | **ConfigMap 로딩** | kubectl exec \-it \<coordinator-pod\> \-- cat /etc/trino/catalog/iceberg.properties | Iceberg, MinIO(S3) 연결 정보가 담긴 설정 파일이 정상적으로 마운트되어 있어야 함. | \[ \] |
| **INF-04** | **Spill 볼륨 마운트** | kubectl describe pod \<worker-pod\> | 계획된 sc-local-nvme 스토리지 클래스를 사용하는 PVC가 /tmp/trino-spill 경로(설정값에 따름)에 마운트되어 있어야 함. | \[ \] |
| **INF-05** | **JVM 메모리 설정** | Trino Web UI \> Cluster Overview | Heap Memory 할당량이 노드 스펙(768GB)을 고려하여 적절하게(예: Node당 500GB 이상) 잡혀 있는지 확인. | \[ \] |

## **2\. 기본 기능 및 클러스터 통신 테스트 (Core Functionality)**

Trino CLI를 사용하여 기본적인 SQL 수행 능력과 노드 간 통신을 검증합니다.

| ID | 테스트 항목 | 테스트 쿼리 (SQL) | 예상 결과 | 확인 |
| :---- | :---- | :---- | :---- | :---- |
| **COR-01** | **CLI 접속 테스트** | kubectl exec \-it \<coordinator\> \-- trino | Trino 프롬프트(trino\>)가 에러 없이 떠야 함. | \[ \] |
| **COR-02** | **기본 연산 테스트** | SELECT 1; | 결과 1이 즉시 반환되어야 함. | \[ \] |
| **COR-03** | **워커 노드 인식** | SELECT \* FROM system.runtime.nodes; | Coordinator를 포함하여 배포된 모든 Worker 노드의 IP와 상태(active)가 출력되어야 함. | \[ \] |
| **COR-04** | **카탈로그 조회** | SHOW CATALOGS; | system, memory, tpch 외에 설정한 iceberg(또는 hive) 카탈로그가 보여야 함. | \[ \] |
| **COR-05** | **TPC-H 스모크 테스트** | SELECT count(\*) FROM tpch.tiny.lineitem; | 내장된 TPC-H 커넥터를 통해 데이터 생성이 되고 카운트 결과가 반환되어야 함. | \[ \] |

## **3\. 데이터 레이크 연동 테스트 (Iceberg & MinIO)**

**핵심 검증 단계입니다.** 타 클러스터에 있는 MinIO와 Iceberg 카탈로그에 정상적으로 접근하여 읽기/쓰기가 가능한지 확인합니다.

| ID | 테스트 항목 | 테스트 시나리오 / SQL | 예상 결과 | 확인 |
| :---- | :---- | :---- | :---- | :---- |
| **DAT-01** | **스키마 생성** | CREATE SCHEMA iceberg.test\_schema WITH (location \= 's3a://test-bucket/'); | MinIO 버킷 내에 폴더가 생성되고 스키마가 등록되어야 함. (S3 접근 권한 검증) | \[ \] |
| **DAT-02** | **테이블 생성 (CTAS)** | CREATE TABLE iceberg.test\_schema.sample AS SELECT \* FROM tpch.tiny.customer; | 데이터가 MinIO에 Parquet/Iceberg 포맷으로 저장되고 테이블이 생성되어야 함. | \[ \] |
| **DAT-03** | **데이터 조회 (Read)** | SELECT \* FROM iceberg.test\_schema.sample LIMIT 10; | 저장된 데이터를 정상적으로 읽어와야 함. (네트워크 대역폭/Latency 체크) | \[ \] |
| **DAT-04** | **데이터 변경 (Update/Delete)** | DELETE FROM iceberg.test\_schema.sample WHERE custkey \= 1; | Iceberg의 ACID 트랜잭션이 동작하여 레코드가 삭제되어야 함. | \[ \] |
| **DAT-05** | **스냅샷 조회** | SELECT \* FROM iceberg.test\_schema.sample.snapshots; | Iceberg 메타데이터 조회를 통해 방금 수행한 작업의 스냅샷 이력이 보여야 함. | \[ \] |

## **4\. 성능 및 리소스 관리 테스트 (Performance & Spill)**

NVMe를 활용한 **Spill-to-Disk** 기능과 리소스 제한 정책이 동작하는지 극한의 상황을 가정해 테스트합니다.

| ID | 테스트 항목 | 테스트 시나리오 | 예상 결과 | 확인 |
| :---- | :---- | :---- | :---- | :---- |
| **PFM-01** | **메모리 초과 쿼리 (Spill 유도)** | 대량의 JOIN과 GROUP BY를 포함한 무거운 쿼리 실행 (메모리 제한을 낮게 설정 후 테스트 권장). | 쿼리가 OOM으로 죽지 않고, Worker 로그에 **"Spilling to disk"** 메시지가 찍히며 성공해야 함. | \[ \] |
| **PFM-02** | **Spill I/O 확인** | Spill 발생 중 Worker 노드에서 iostat 또는 모니터링 확인. | NVMe 디스크(/tmp/trino-spill)에 **Write I/O**가 급증하는 것이 관측되어야 함. | \[ \] |
| **PFM-03** | **쿼리 큐잉(Queuing)** | 동시 실행 쿼리 수를 제한(max-concurrent-queries)하고 다수 쿼리 제출. | 허용된 수 이상의 쿼리는 Queued 상태로 대기하다가 앞선 쿼리가 끝나면 실행되어야 함. | \[ \] |
| **PFM-04** | **리소스 그룹(Resource Group)** | Team A로 로그인하여 쿼리 실행 vs Team B로 로그인하여 쿼리 실행. | 설정된 리소스 그룹 정책(CPU/Memory Quota)에 따라 자원이 격리/제한되는지 확인. | \[ \] |

## **5\. 보안 및 인증 테스트 (Security & Keycloak)**

Keycloak 연동을 통한 사용자 인증 및 데이터 접근 제어를 검증합니다.

| ID | 테스트 항목 | 테스트 시나리오 | 예상 결과 | 확인 |
| :---- | :---- | :---- | :---- | :---- |
| **SEC-01** | **Web UI 로그인** | 브라우저 접속 시 Keycloak 로그인 화면 리다이렉트. | 사내 계정으로 로그인 성공 후 Trino UI 메인 화면으로 진입해야 함. | \[ \] |
| **SEC-02** | **CLI 인증 (Token)** | trino \--server ... \--user \<user\> \--password \<token\> 접속 시도. | 인증 토큰이 유효할 경우 접속 성공, 무효할 경우 401 Unauthorized 반환. | \[ \] |
| **SEC-03** | **권한 제어 (Authorization)** | Team A 사용자가 Team B의 스키마/테이블 조회 시도 (SELECT \* FROM iceberg.team\_b.secret). | **Access Denied** 에러가 발생하며 쿼리가 거부되어야 함. (RBAC 검증) | \[ \] |
| **SEC-04** | **Impersonation (옵션)** | Trino가 Hive/HDFS 접근 시 사용자 계정으로 위장(Impersonation) 설정 확인. | MinIO/HDFS 로그에 Trino 시스템 계정이 아닌 실제 요청 사용자 ID로 접근 기록이 남는지 확인. | \[ \] |

## **6\. 테스트 결과 요약 (Summary)**

* **테스트 수행자:** \_\_\_\_\_\_\_\_\_\_\_\_\_\_\_\_\_\_\_  
* **수행 일시:** \_\_\_\_\_\_\_\_\_\_\_\_\_\_\_\_\_\_\_  
* **성공 항목:** \_\_\_\_ / \_\_\_\_  
* **실패 항목:** \_\_\_\_\_\_\_\_\_\_\_\_\_\_\_\_\_\_\_ (Jira 티켓 번호: \_\_\_\_\_)  
* **특이 사항:** (예: MinIO 네트워크 Latency가 간헐적으로 10ms 이상 튐 등)