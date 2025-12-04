# Apache Iceberg REST Catalog 솔루션 종합 비교 분석

LakeKeeper, Apache Polaris, 그리고 HMS를 대체할 REST 기반 Iceberg Catalog 솔루션을 비교 분석한 결과, **LakeKeeper는 OPA 통합과 세밀한 접근 제어에서 우위**를 보이며, **Apache Polaris는 엔터프라이즈 성숙도와 Keycloak 통합**에서 강점을 가집니다. 두 솔루션 모두 현재 인프라(Kubernetes, MinIO, Trino 475, Spark 3.5/3.6)와 완전 호환되며, HMS 대비 **list 작업 성능 개선**, **credential vending**, **HTTP 기반 확장성**의 핵심 이점을 제공합니다.

---

## HMS의 한계점과 REST Catalog 전환 필요성

현재 사용 중인 Hive Metastore는 2013년 Hadoop 시대에 설계되어 현대 클라우드 네이티브 환경에서 구조적 한계를 보입니다. **파티션 10,000개 제한**이 Cloudera에서 공식 권장되며, 이를 초과하면 HMS 서버 메모리 압박이 심화됩니다. 파티션 열거 시 Thrift API가 복잡성과 지연을 추가하고, 각 쿼리마다 디렉토리 파일 목록 조회가 필요해 대규모 파티션 환경에서 성능이 급격히 저하됩니다.

REST Catalog는 이러한 한계를 근본적으로 해결합니다. HTTP 기반 API로 클라우드 인프라의 로드 밸런싱, 메트릭, 헬스 체크를 자연스럽게 활용하며, **StarRocks 사례에서 외부 스토리지 시스템 호출 90% 감소**가 보고되었습니다. 파일 단위 메타데이터 추적으로 디렉토리 스캔을 제거하고, credential vending으로 임시 스코프 자격증명을 제공해 보안도 강화됩니다.

---

## LakeKeeper의 기술 아키텍처와 특징

LakeKeeper는 **Rust로 작성된 단일 바이너리**로 JVM이나 Python 환경 없이 배포됩니다. `apache/iceberg-rust` 라이브러리 기반으로 현재 버전 **v0.9.5**이며, Iceberg 1.5~1.7을 지원합니다. 정규화된 관계형 데이터베이스 모델을 사용해 파일 시스템 접근 없이 빠른 메타데이터 조회가 가능하고, stateless 아키텍처로 수평 확장이 자유롭습니다.

인증/인가 측면에서 LakeKeeper의 가장 큰 차별점은 **OpenFGA 기반 Fine-Grained Authorization**과 **네이티브 OPA 브릿지**입니다. Server/Project/Warehouse/Namespace/Table 레벨의 세밀한 접근 제어를 지원하며, OPA 브릿지를 통해 Trino와 같은 멀티유저 쿼리 엔진에서 사용자별 권한을 강제할 수 있습니다. Keycloak은 OIDC provider로 완전 지원되며, Standard Flow, Device Authorization Grant, Token Exchange(RFC8693)까지 구성 가능합니다.

```yaml
# LakeKeeper Keycloak 연동 핵심 설정
LAKEKEEPER__OPENID_PROVIDER_URI: https://keycloak.example.com/realms/lakekeeper
LAKEKEEPER__AUTHZ_BACKEND: openfga
LAKEKEEPER__ENABLE_KUBERNETES_AUTHENTICATION: true
```

---

## Apache Polaris의 기술 아키텍처와 특징

Apache Polaris는 **Java 21 기반 Quarkus 프레임워크**를 사용하며, Snowflake에서 오픈소스화되어 현재 **Apache 인큐베이팅 v1.1.0** 상태입니다. Quarkus의 Kubernetes 네이티브 특성으로 빠른 시작 시간과 낮은 메모리 사용량을 제공하며, PostgreSQL을 권장 백엔드로 사용합니다.

인증은 내부(Built-in OAuth2 서버)와 외부(OIDC/Keycloak) 모드를 모두 지원합니다. **Keycloak 통합 예제가 공식 문서에 포함**되어 있으며, `realm-external` 또는 `realm-mixed` 모드로 구성 가능합니다. 그러나 **OPA 네이티브 통합은 없으며**, 내장 RBAC 모델에 의존합니다. Principal Roles와 Catalog Roles의 이중 구조로 Catalog/Namespace/Table/View 레벨 접근 제어를 제공합니다.

| 구분 | LakeKeeper | Apache Polaris |
|------|------------|----------------|
| **OPA 통합** | ✅ 네이티브 브릿지 | ❌ 미지원 (RBAC 내장) |
| **Keycloak/OIDC** | ✅ 완전 지원 | ✅ 완전 지원 |
| **접근 제어 레벨** | Server/Project/Warehouse/Namespace/Table | Catalog/Namespace/Table/View |
| **인가 엔진** | OpenFGA (CNCF) | 내장 RBAC |

---

## 쿼리 엔진 호환성 상세 분석

**Trino 475 호환성**: 두 솔루션 모두 표준 Iceberg REST Catalog 커넥터를 통해 완전 호환됩니다. Trino 474/476 버전과의 통합 사례가 문서화되어 있으며, OAuth2 보안과 credential vending을 지원합니다. LakeKeeper는 추가로 중첩 네임스페이스 지원(`iceberg.rest-catalog.nested-namespace-enabled = true`)이 필요합니다.

```properties
# Trino Iceberg REST Catalog 설정 예시
connector.name=iceberg
iceberg.catalog.type=rest
iceberg.rest-catalog.uri=https://catalog.example.com
iceberg.rest-catalog.security=OAUTH2
iceberg.rest-catalog.vended-credentials-enabled=true
```

**Spark 호환성**: LakeKeeper와 Polaris 모두 Spark 3.5+를 지원합니다. Polaris 문서에는 `iceberg-spark-runtime-3.5_2.12:1.9.0` 예제가 포함되어 있으며, LakeKeeper는 PySpark 동적 버전 감지를 지원합니다. Spark 4.x는 Iceberg 최신 버전(1.8+)과의 호환성에 따라 달라지며, 두 솔루션 모두 REST API 표준을 따르므로 업그레이드 시 호환성 문제는 최소화됩니다.

---

## 스토리지 호환성과 Credential Vending

**MinIO 지원**: 두 솔루션 모두 MinIO를 완전 지원합니다. LakeKeeper는 통합 테스트에서 MinIO 호환성을 검증하며, Polaris는 공식 getting-started 가이드에 MinIO Docker Compose 예제를 제공합니다.

**Credential Vending 비교**:

| 기능 | LakeKeeper | Apache Polaris |
|------|------------|----------------|
| **S3 Remote Signing** | ✅ | ✅ |
| **S3 Vended Credentials (STS)** | ✅ | ✅ |
| **Azure SAS Tokens** | ✅ | ✅ |
| **GCS Service Account** | ✅ | ✅ |
| **Cloudflare R2** | ✅ (v0.8.2+) | 문서화 미확인 |

LakeKeeper의 특징적 기능으로 **System Identity** 지원이 있어 AWS/Azure/GCP의 환경 기반 인증(Managed Identity, Service Account)을 활용할 수 있습니다. 이는 쿠버네티스 환경에서 IRSA(IAM Roles for Service Accounts)나 Workload Identity와 통합할 때 유용합니다.

---

## 쿠버네티스 배포와 운영 비교

두 솔루션 모두 **공식 Helm 차트**를 제공하며, 고가용성 구성이 가능합니다.

**LakeKeeper 배포**:
- Helm 저장소: `https://lakekeeper.github.io/lakekeeper-charts/`
- Kubernetes Operator 개발 중 (`lakekeeper/lakekeeper-operator`)
- PostgreSQL 15+ 필수, Read Replica 분리 구성 가능
- Autoscaling 내장 지원, `/health` 엔드포인트 제공
- CloudEvents를 NATS/Kafka로 전송하여 변경 추적 가능

**Apache Polaris 배포**:
- Helm 차트: v1.0.0부터 공식 지원 (`helm/polaris`)
- Pod Disruption Budgets(PDB) 지원 (v1.1.0+)
- Quarkus 기반으로 Micrometer/Prometheus 메트릭, OpenTelemetry 트레이싱 내장
- SmallRye Health 엔드포인트 제공

**모니터링 비교**: Polaris는 Quarkus 생태계 덕분에 Prometheus, OpenTelemetry 통합이 더 성숙하며, LakeKeeper는 v0.9.3부터 트레이싱 레이어가 추가되고 내부 통계를 데이터베이스에 저장합니다.

---

## 부가 기능과 테이블 유지보수

**멀티 카탈로그**: LakeKeeper는 Project/Warehouse 계층으로 멀티테넌트를 지원하고, Polaris는 Internal/External 카탈로그 모드로 Snowflake, Glue, Dremio Arctic과 연동 가능합니다.

**스키마 진화와 타임 트래블**: 두 솔루션 모두 Iceberg REST API를 통해 완전 지원합니다.

**테이블 유지보수**:
- **LakeKeeper**: Snapshot Expiration 자동화 (v0.10+), 커밋 후 이벤트 기반 스케줄링, Compaction은 "Coming Soon"으로 현재 외부 Spark/Flink 작업 필요
- **Polaris**: Policy Store (v1.0.0+)로 Data Compaction, Snapshot Expiry 정책 관리, REST CRUD 엔드포인트로 정책 설정

---

## 대안 솔루션 비교 분석

### Project Nessie
**Git-like 데이터 버전 관리**가 핵심 차별점입니다. 브랜치, 태그, 커밋, 머지를 카탈로그 레벨에서 지원하여 데이터 실험 워크플로우에 최적화되어 있습니다. OIDC 인증을 지원하지만 OPA 네이티브 통합은 없습니다. Dremio가 지원하며 성숙한 Helm 차트(`helm repo add nessie https://charts.projectnessie.org`)를 제공합니다. 다만 **Iceberg만 지원**하고 인가 모델이 경쟁 솔루션 대비 덜 세분화되어 있습니다.

### Unity Catalog (Databricks OSS)
**멀티 포맷(Delta, Iceberg, Hudi)** 지원이 강점이며, AI/ML 자산(모델, 함수) 관리까지 포함합니다. Iceberg REST Catalog API를 구현하고 credential vending을 내장합니다. 그러나 OSS 버전은 상용 버전 대비 기능 격차가 있고, Databricks 중심 설계로 완전한 중립성이 부족합니다. v0.3에서 Helm 차트가 추가되었습니다.

### Apache Gravitino (Incubating)
**연합 메타데이터 레이크** 개념으로 다양한 소스(Hive, MySQL, PostgreSQL, Iceberg, Paimon, Kafka)를 단일 인터페이스로 관리합니다. **Apache Ranger 통합**을 통한 인가 푸시다운을 지원하며, 이는 OPA와 유사한 외부 정책 엔진 연동 관점에서 주목할 만합니다. 그러나 RBAC가 아직 알파 단계이고, 쿠버네티스 배포 문서화가 부족합니다.

| 솔루션 | 핵심 강점 | 주요 약점 | 추천 시나리오 |
|--------|----------|----------|--------------|
| **Nessie** | Git-like 버전 관리 | Iceberg 전용, OPA 미지원 | 데이터 실험 워크플로우 |
| **Unity Catalog** | 멀티포맷, AI/ML 자산 | OSS 기능 제한 | Databricks 에코시스템 |
| **Gravitino** | 연합 카탈로그, Ranger 통합 | RBAC 미성숙 | 이기종 메타데이터 통합 |

---

## 종합 권장사항

현재 요구사항(OPA 통합, Keycloak, MinIO, Trino 475, Spark 3.5/3.6, 쿠버네티스)을 기준으로 **LakeKeeper를 1순위로 권장**합니다.

**LakeKeeper 선택 이유**:
1. **OPA 네이티브 브릿지**로 Trino 멀티유저 환경에서 세밀한 접근 제어 가능
2. OpenFGA 기반 **Table 레벨까지 Fine-Grained Authorization** 지원
3. Rust 기반 단일 바이너리로 **운영 복잡도 최소화**
4. 모든 요구 스토리지와 쿼리 엔진 완전 호환

**Apache Polaris 고려 상황**:
- OPA가 필수가 아니고 **내장 RBAC로 충분한 경우**
- **Snowflake Open Catalog** 매니지드 서비스 활용 계획이 있는 경우
- Java/Quarkus 기반으로 기존 모니터링 스택(Prometheus, OTel)과 더 성숙한 통합이 필요한 경우

**마이그레이션 전략**: HMS에서 REST Catalog로의 전환 시 Iceberg의 `migrate` 프로시저(in-place)나 `snapshot` 접근법을 활용할 수 있으며, 데이터 파일 이동 없이 메타데이터만 전환하는 것이 가능합니다. 두 솔루션 모두 HMS와 병행 운영이 가능하므로 점진적 마이그레이션을 권장합니다.
