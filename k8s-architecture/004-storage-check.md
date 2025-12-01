맞습니다! 죄송합니다. 관측성 스택의 스토리지 요구사항을 빠뜨렸네요. 추가해서 다시 정리해드리겠습니다.

-----

## 추가: 관측성 스택 스토리지 요구사항

### Prometheus

|구성 요소                |스토리지 유형                     |성능 요구    |용량        |비고                       |
|---------------------|----------------------------|---------|----------|-------------------------|
|TSDB (시계열 데이터)       |**Local (High Performance)**|**매우 높음**|수백 GB ~ TB|쓰기/읽기 모두 I/O 집약적, NVMe 권장|
|WAL (Write-Ahead Log)|**Local (High Performance)**|**매우 높음**|수십 GB     |TSDB와 동일 디스크 또는 분리 가능    |
|장기 보관 (선택)           |**Object (S3)**             |낮음       |TB 단위     |Thanos/Cortex 사용 시       |

**190대 규모 고려사항**

- 노드당 수백 개의 메트릭 × 190대 = 대량의 시계열 데이터
- 단일 Prometheus로는 한계 → Prometheus 샤딩 또는 Thanos/Victoria Metrics 고려 필요
- 보존 기간에 따라 용량 급증 (15일 보관 기준 수백 GB ~ 1TB 예상)

-----

### Grafana

|구성 요소                         |스토리지 유형                                |성능 요구|용량   |비고                      |
|------------------------------|---------------------------------------|-----|-----|------------------------|
|대시보드/설정 DB (SQLite/PostgreSQL)|**Block (RWO)** 또는 **Local (Standard)**|낮음   |수 GB |HA 구성 시 PostgreSQL 사용 권장|
|플러그인                          |**Local (Standard)**                   |낮음   |수백 MB|컨테이너 이미지에 포함 가능         |
|이미지 렌더링 캐시                    |**Local (Standard)**                   |낮음   |수 GB |알림 이미지 생성용              |

**참고**: Grafana 자체는 스토리지 부담이 적습니다. 데이터는 Prometheus/OpenSearch에서 조회합니다.

-----

### OpenSearch (로깅)

|구성 요소        |스토리지 유형                     |성능 요구    |용량      |비고                                 |
|-------------|----------------------------|---------|--------|-----------------------------------|
|데이터 노드 (Hot) |**Local (High Performance)**|**매우 높음**|TB 단위/노드|최근 로그, NVMe 필수                     |
|데이터 노드 (Warm)|**Local (Standard)**        |중간       |TB 단위/노드|오래된 로그, SAS SSD 가능                 |
|데이터 노드 (Cold)|**Object (S3)**             |낮음       |TB 단위   |아카이브용, Snapshot/Searchable Snapshot|
|마스터 노드       |**Local (Standard)**        |낮음       |수십 GB   |클러스터 메타데이터만                        |

**190대 규모 고려사항**

- 예상 로그량: 노드당 수십 MB/분 × 190대 = 수 GB/분
- 보존 기간과 인덱스 정책에 따라 수십 TB 필요 가능
- Hot-Warm-Cold 티어링 아키텍처 권장
- 전용 OpenSearch 노드 필요 (3~5대 이상)

-----

### 로그 수집기 (Fluent Bit / Fluentd / Vector)

|구성 요소|스토리지 유형             |성능 요구|용량     |비고                         |
|-----|--------------------|-----|-------|---------------------------|
|버퍼   |**Local (Standard)**|중간   |수 GB/노드|일시적 버퍼링, OpenSearch 장애 시 대비|

-----

## 수정된 요약: 스토리지 유형별 사용처

|스토리지 유형                     |사용 애플리케이션/용도                                                                                            |하드웨어 매핑           |
|----------------------------|--------------------------------------------------------------------------------------------------------|------------------|
|**Local (High Performance)**|Spark Shuffle, Flink State, Kafka 로그, Trino Spill, StarRocks BE, **Prometheus TSDB**, **OpenSearch Hot**|NVMe SSD 3.84T × 2|
|**Local (Standard)**        |OS, 컨테이너 런타임, Airflow Worker 임시, **Grafana**, **OpenSearch Warm**, **로그 수집기 버퍼**                        |SAS SSD 800G × 2  |
|**Block (RWO)**             |PostgreSQL (Airflow 메타DB), **Grafana DB (HA 시)**                                                        |결정 필요             |
|**File (RWX)**              |Airflow DAG (Git-sync 대안)                                                                               |Dell Isilon NAS   |
|**Object (S3)**             |모든 앱의 Checkpoint, 로그, 데이터 I/O, **Prometheus 장기보관**, **OpenSearch Cold/Snapshot**                        |외부 MinIO          |

-----

## 수정된 매트릭스

|애플리케이션        |Local (NVMe)|Local (SAS)|Block (RWO)|File (RWX)|Object (S3)    |
|--------------|:----------:|:---------:|:---------:|:--------:|:-------------:|
|**Airflow**   |-           |✅ 임시       |✅ 메타DB     |✅ DAG     |✅ 로그           |
|**Spark**     |✅ Shuffle   |-          |-          |-         |✅ 데이터/CP       |
|**Trino**     |✅ Spill/캐시  |-          |-          |-         |✅ 데이터          |
|**Flink**     |✅ State     |-          |-          |-         |✅ CP/데이터       |
|**Kafka**     |✅ 로그        |-          |-          |-         |⚪ Tiered       |
|**StarRocks** |✅ BE 데이터    |✅ FE       |-          |-         |-              |
|**Prometheus**|✅ TSDB      |-          |-          |-         |⚪ 장기보관         |
|**Grafana**   |-           |✅ 기본       |⚪ HA시 DB   |-         |-              |
|**OpenSearch**|✅ Hot       |✅ Warm     |-          |-         |✅ Cold/Snapshot|
|**로그 수집기**    |-           |✅ 버퍼       |-          |-         |-              |

✅ 필수 / ⚪ 선택적

-----

## 추가 의사결정 사항

### 결정 4: Prometheus 아키텍처

190대 규모에서 단일 Prometheus는 한계가 있습니다.

|옵션                        |설명                                   |장점                     |단점                     |
|--------------------------|-------------------------------------|-----------------------|-----------------------|
|**A. Prometheus + Thanos**|Prometheus 여러 대 + Thanos로 통합 쿼리/장기 저장|수평 확장, S3 장기 보관, 글로벌 뷰 |구성 복잡도 증가              |
|**B. Prometheus + Cortex**|멀티테넌시 지원 분산 Prometheus               |팀별 격리 용이, 수평 확장        |운영 복잡도 높음              |
|**C. Victoria Metrics**   |Prometheus 호환 고성능 TSDB               |단일 바이너리, 리소스 효율적, 운영 단순|Prometheus 생태계 일부 기능 제한|
|**D. 단일 Prometheus (샤딩)** |팀별 또는 역할별 Prometheus 분리              |구성 단순                  |글로벌 뷰 어려움, 관리 포인트 증가   |

**권장**: 190대 규모 + 4개 팀 멀티테넌시를 고려하면 **옵션 A (Thanos)** 또는 **옵션 C (Victoria Metrics)**가 적합합니다.

-----

### 결정 5: OpenSearch 노드 전용 할당

OpenSearch는 리소스를 많이 사용하므로 전용 노드 할당을 고려해야 합니다.

|옵션              |설명                      |장점             |단점                        |
|----------------|------------------------|---------------|--------------------------|
|**A. 전용 노드**    |OpenSearch 전용 노드 3~5대 할당|워크로드 격리, 안정적 성능|노드 수 감소 (185~187대 워커)     |
|**B. 워커 노드와 혼합**|다른 워크로드와 함께 배치          |리소스 활용도 높음     |리소스 경합, 로깅 장애 시 다른 워크로드 영향|

**권장**: **옵션 A (전용 노드)**를 권장합니다. 로깅은 모든 워크로드의 기반이므로 격리하는 것이 안전합니다.

-----

## 정리: 전체 결정 요청 사항

|#|항목                 |옵션                                                 |
|-|-------------------|---------------------------------------------------|
|1|Block Storage 제공 방식|A(LocalPV) / B(Rook-Ceph) / C(외부 스토리지)             |
|2|Airflow DAG 저장 방식  |A(Git-sync) / B(NFS) / C(Object Storage)           |
|3|NVMe 파티셔닝 전략       |A(단일 용도) / B(용도별 분리) / C(노드 역할별)                   |
|4|Prometheus 아키텍처    |A(Thanos) / B(Cortex) / C(Victoria Metrics) / D(샤딩)|
|5|OpenSearch 노드 할당   |A(전용 노드) / B(혼합 배치)                                |

결정해주시면 스토리지 아키텍처 상세 설계로 진행하겠습니다!​​​​​​​​​​​​​​​​