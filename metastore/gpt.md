아래는 **전체 보고서를 한국어로 완전 번역한 버전**입니다.
문맥·기술 용어·구조를 유지하면서 한국어 기술 보고서 형식으로 자연스럽게 재작성했습니다.

---

# 📘 **Hive Metastore(HMS), LakeKeeper, Apache Polaris 및 Iceberg 메타스토어 비교 분석 보고서**

## 1. 아키텍처 및 메타데이터 처리 방식 비교

### **1.1 Hive Metastore (HMS)**

HMS는 Hadoop/Hive 시대에 설계된 레거시 메타스토어로, 테이블·파티션 정보를 RDBMS(MySQL/Postgres)에 저장하고 Thrift API로 노출한다.
Iceberg를 HMS 기반으로 사용할 경우, HMS는 **Iceberg 메타데이터 파일 위치(pointer)**만 저장하며 실제 스냅샷·매니페스트 데이터는 객체스토리지(S3/HDFS)에 저장된다.

**제한점**

* Iceberg REST Catalog API *미지원*
* 다중 테이블 트랜잭션 미지원
* 버전 관리(Branch/Tag) 부재
* 동시성 처리 및 대규모 메타데이터 조회에 취약
* Kerberos 또는 외부 Ranger 기반의 구식 보안 모델

---

### **1.2 Apache Polaris**

Snowflake가 기여한 Apache Incubating 프로젝트로, **Iceberg REST Catalog API의 완전한 구현체**이다.

**특징**

* **Stateless REST 서비스 + PostgreSQL** 구조
* 메타데이터 포인터 및 테이블 정보는 DB에 저장하고 Iceberg commit은 REST API로 처리
* **Iceberg REST Commit 프로토콜 기반의 원자적 커밋 지원**
* **내부 카탈로그(Polaris 관리), 외부 카탈로그(HMS/Glue 등 Read-only)** 동시 지원 → *점진적 마이그레이션 가능*
* 엔진 간 공유 메타데이터 중앙집중화

**메타데이터 처리 방식**

* 테이블의 현재 상태를 Polaris DB에 저장
* 커밋 충돌 감지 및 동시성 제어는 Polaris 서버에서 처리
* 서버 측에서 다중 테이블 커밋 가능

---

### **1.3 LakeKeeper**

Rust로 구현된 경량 Iceberg REST Catalog 서비스.

**특징**

* 단일 바이너리 실행 방식 → 매우 간단한 배포
* **Iceberg REST Catalog 100% 구현**
* Postgres 기반의 정규화된 메타데이터 스키마 저장
* 테이블/스냅샷 메타데이터 일부를 DB에 캐시해 빠른 조회
* 커밋은 DB 트랜잭션으로 처리 → **멀티 테이블 원자적 커밋 가능**
* Multi-tenant 지원: 하나의 LakeKeeper instance로 여러 Catalog/Project 운영 가능
* Stateless → Kubernetes에서 수평 확장 용이

---

### **1.4 Iceberg REST API 구현 관점**

| 솔루션        | REST Catalog API | 커밋 처리 방식              | 멀티 테이블 트랜잭션 |
| ---------- | ---------------- | --------------------- | ----------- |
| HMS        | ❌                | Thrift, 커밋 충돌 엔진에서 처리 | ❌           |
| Polaris    | ✔️               | 서버 측 원자 커밋            | ✔️          |
| LakeKeeper | ✔️               | DB 트랜잭션 기반 커밋         | ✔️          |
| Nessie     | ✔️               | Git-like Commit       | ✔️          |

---

## 2. 성능 비교 (Latency, 병목, 동시성)

### **2.1 Hive Metastore**

* 대규모 테이블·파티션 조회 시 RDBMS 의존 → 병목
* 높은 동시성 부하에서 장애/타임아웃 발생 가능
* 커밋은 테이블 단위로만 가능 → 대규모 ETL에 비효율
* 메타데이터 변경 작업이 느리고 스케일링 난해

---

### **2.2 Polaris 성능**

벤치마크(공식 Gatling 기반):

* **40 동시 클라이언트 환경에서 약 135 ops/sec 처리**
* **Median latency ≈ 89ms, P99 ≈ 154ms**
* 수평 확장으로 처리량 증가 가능
* 병목은 Postgres 성능에 비례

**장점**

* REST 기반 비차단 방식
* 효율적인 DB 트랜잭션 처리
* 멀티 테이블 커밋으로 ETL 처리 효율 상승

---

### **2.3 LakeKeeper 성능**

정량적 벤치마크는 제한적이지만 구조적으로 높은 성능이 기대됨:

* Rust 기반의 낮은 메모리/CPU 오버헤드
* DB에 메타데이터를 정규화해 저장 → **빠른 조회**
* 멀티 테이블 커밋 지원
* Stateless scaling → Kubernetes 환경에서 고성능 처리 가능

**핵심 장점**

* 높은 동시성 처리 능력
* 매우 낮은 시스템 오버헤드
* Postgres 튜닝 시 극대화 가능

---

## 3. 보안 모델 비교 (Keycloak / OIDC / OPA)

### **3.1 Hive Metastore**

* OIDC / OAuth2 지원 없음
* Kerberos 의존
* Ranger/Sentry 기반 ACL → Iceberg-specific 정책 미흡
* ABAC 불가

---

### **3.2 Polaris**

완전한 현대적 인증/인가 모델 제공.

**인증(AuthN)**

* OIDC/OAuth2 지원
* **Keycloak 공식 지원**
* Polaris는 자체 Token 생성 안 하고 IdP(JWT) 기반 신뢰 모델 사용

**인가(AuthZ)**

* **카탈로그/네임스페이스/테이블 단위 RBAC**
* OPA 연동 가능 → **ABAC 정책 적용 가능**
* Storage Credential Vending → 최소 권한 접근 제어

---

### **3.3 LakeKeeper**

보안 분야에서 Polaris 못지 않음.

**인증(AuthN)**

* Keycloak(OIDC) 완전 지원
* 외부 IdP에서 JWT 받아 검증

**인가(AuthZ)**

* **OpenFGA 기반 세밀한 접근 제어(관계 기반 모델)**
* OPA Check Endpoint 제공 → Trino 등과 통합
* 테이블 단위 권한 관리
* Storage Signed URL / Temp Credentials 지원

---

### **비교 요약**

| 기능             | HMS              | Polaris               | LakeKeeper           |
| -------------- | ---------------- | --------------------- | -------------------- |
| Keycloak(OIDC) | ❌                | ✔️ 정식 지원              | ✔️ 정식 지원             |
| RBAC           | 약함(외부 Ranger 필요) | ✔️ 강력                 | ✔️ 강력(OpenFGA)       |
| ABAC(OPA)      | ❌                | ✔️                    | ✔️                   |
| 데이터 접근 제어      | Hadoop config 의존 | ✔️ Credential Vending | ✔️ Signed URL / Role |

---

## 4. Kubernetes 배포 및 인프라 호환성(PostgreSQL/MinIO)

### **4.1 HMS**

* 공식 Helm chart 없음
* StatefulSet + External DB로 직접 구축 필요
* HA 구성 어려움
* S3/MinIO 연동은 Hadoop 설정에 의존

---

### **4.2 Polaris**

* **공식 Helm Chart 지원**
* Stateless 서비스 → 수평 확장 쉬움
* Backend DB: **PostgreSQL 공식 지원 (CPNG PostgreSQL 문제 없음)**
* MinIO(S3 호환) 공식 문서로 검증
* Kubernetes 네이티브(health check, metrics 등 Quarkus 지원)

---

### **4.3 LakeKeeper**

* 단일 바이너리 또는 Docker → 가장 간단한 배포
* Helm chart 제공, Operator도 개발 중
* Backend DB: **PostgreSQL 15+ 공식 지원**
* MinIO 포함 모든 S3 호환 스토리지 지원
* Web UI 기본 포함 → 운영 편의성 높음

---

### **비교 요약**

| 항목                  | HMS   | Polaris | LakeKeeper |
| ------------------- | ----- | ------- | ---------- |
| 설치 난이도              | 높음    | 중간      | 매우 쉬움      |
| Kubernetes 적합성      | 낮음    | 높음      | 매우 높음      |
| PostgreSQL(CPNG) 호환 | ✔️    | ✔️      | ✔️         |
| MinIO 호환            | 엔진 의존 | ✔️      | ✔️         |

---

## 5. 기타 고려할 Iceberg 메타스토어

### **5.1 Project Nessie**

* Git 같은 **Branch/Tag 제공 → 버전 관리 최강**
* Iceberg REST 지원
* Multi-table commit 지원
* 안정성·성숙도 높음
* 대규모 커뮤니티

### **5.2 Apache Gravitino**

* 연합(Federation) 메타데이터 시스템
* 다중 region 메타스토어 동기화
* Iceberg/Hudi/Delta 모두 지원 예정
* 대규모 엔터프라이즈 지향

### **5.3 Unity Catalog OSS**

* Databricks가 오픈소스화
* HMS Thrift API + Iceberg REST API 동시 지원
* Delta/Iceberg/Parquet 모두 지원
* Fine-grained Governance 강점
* 다소 무거움

---

## 6. 고급 기능 비교

| 기능                      | HMS | Polaris | LakeKeeper | Nessie | Gravitino | Unity Catalog OSS |
| ----------------------- | --- | ------- | ---------- | ------ | --------- | ----------------- |
| Branch/Tag              | ❌   | ❌       | ❌          | ✔️     | 일부(계획)    | ❌                 |
| Multi-table Transaction | ❌   | ✔️      | ✔️         | ✔️     | ✔️        | ✔️ 예상             |
| Iceberg View            | ❌   | ✔️      | ✔️         | 제한적    | ✔️        | ✔️                |
| Spark 지원                | ✔️  | ✔️      | ✔️         | ✔️     | ✔️        | ✔️                |
| Trino 지원                | ✔️  | ✔️      | ✔️         | ✔️     | ✔️        | ✔️                |

---

## 7. 성능 / 보안 / 마이그레이션 / 설치 편의성 종합 비교

### **7.1 성능 (가장 중요한 기준)**

* **HMS → 매우 느리고 동시성 한계 존재**
* **Polaris → 공식 벤치마크로 고성능 입증**
* **LakeKeeper → Rust 기반으로 최적화된 성능, Postgres 캐시 설계로 빠름**

**결론: Polaris ≈ LakeKeeper ≫ HMS**

---

### **7.2 권한 설정(보안)**

* HMS: Kerberos + Ranger → 복잡, Iceberg 권한 부족
* Polaris: OIDC + RBAC + OPA 지원
* LakeKeeper: OIDC + OpenFGA + OPA → 매우 세밀한 권한 모델

**결론: Polaris = LakeKeeper ≫ HMS**

---

### **7.3 HMS 사용자에게 매끄러운 전환성**

* Polaris: **Hive → Polaris Read-only Federation** 제공 → *가장 부드러운 전환*
* LakeKeeper: 수동 마이그레이션이 필요하지만 복잡하지 않음
* Nessie: 엔진 설정 변경이 필요하지만 Iceberg 기반이라 쉬운 편

**결론: Polaris > LakeKeeper > HMS**

---

### **7.4 설치 및 운영 편의성**

* HMS: 설치 복잡, K8s에서 비효율적
* Polaris: Helm chart, 문서 우수
* LakeKeeper: 단일 바이너리 → **가장 설치가 쉽고 가벼움**

**결론: LakeKeeper > Polaris ≫ HMS**

---

# 📌 **최종 결론**

### **추천 1 – Polaris (엔터프라이즈형, 장기 운영 중심)**

* 안정적, 기능 완성도 높음
* Keycloak/OPA/RBAC 준비되어 있음
* HMS에서 점진적 마이그레이션 가능
* 대규모 클러스터(200 노드)에도 안정적

### **추천 2 – LakeKeeper (경량 고성능, DevOps 친화)**

* Rust 기반 고성능
* 설치·운영 난이도 최저
* 권한 모델(OpenFGA) 강력
* K8s 환경에서는 Polaris보다 더 간단

### **추천 3 – Nessie (데이터 버전 관리 필요 시 필수)**

* Branch/Tag 필요하면 유일한 선택지

### **총괄 요약**

| 우선순위               | 최적 솔루션               |
| ------------------ | -------------------- |
| **1. 성능**          | Polaris = LakeKeeper |
| **2. 권한/보안**       | LakeKeeper = Polaris |
| **3. HMS 사용자 전환성** | Polaris              |
| **4. 설치 편의성**      | LakeKeeper           |

---

필요하시면 다음도 추가해 드립니다:

✅ 선택 기준별 의사결정 트리
✅ Polaris vs LakeKeeper 실제 도입 시 아키텍처 다이어그램
✅ HMS → Polaris/LakeKeeper 마이그레이션 구체 단계
✅ 벤치마크 필요 시 테스트 스크립트/도구 구성

원하시는 내용을 알려주세요!
