# Apache Flink 2.x vs 1.20 종합 기술 비교 문서

Apache Flink 2.0은 **2025년 3월 24일 출시**된 9년 만의 첫 메이저 릴리스로,  클라우드 네이티브 아키텍처로의 근본적 전환을 의미합니다. 165명의 기여자가 **25개의 FLIP**과 **369개의 이슈**를 완료했으며,  DataSet API와 Scala API 완전 제거,   새로운 분리형 상태 관리(ForSt), 비동기 실행 모델 도입이 핵심 변화입니다.  **1.x와 2.x 간 상태 호환성은 보장되지 않으므로**   마이그레이션 시 신규 Savepoint 계획이 필수입니다.

-----

## 1. 아키텍처 변경사항

### ForSt State Backend: 클라우드 네이티브 상태 관리의 핵심

**ForSt (For Streaming)**는 Flink 2.0의 가장 혁신적인 아키텍처 변경으로, 컴퓨트와 스토리지를 완전히 분리한 분산 상태 백엔드입니다.  기존 RocksDB가 로컬 디스크에 상태를 저장했다면, ForSt는 **S3, HDFS 등 원격 DFS를 주 저장소**로 사용합니다.  

|특성          |RocksDB (1.20)|ForSt (2.0)       |
|------------|--------------|------------------|
|**주 저장소**   |로컬 디스크        |원격 DFS (S3/HDFS)  |
|**상태 크기 제한**|로컬 디스크 용량     |**무제한**           |
|**체크포인트 방식**|전체 상태 업로드     |메타데이터만 (Zero-copy)|
|**복구 시간**   |상태 크기에 비례     |상태 크기와 **무관**     |
|**리소스 사용**  |주기적 스파이크      |안정적/지속적           |
|**리스케일링**   |느림 (상태 재분배)   |빠름 (파일 참조만 변경)    |

ForSt의 아키텍처 구조는 다음과 같습니다:

```
┌─────────────────────────────────────────┐
│           Task Manager                   │
│  ┌─────────────────────────────────────┐ │
│  │      ForSt Instance                 │ │
│  │  ┌─────────────┐  ┌──────────────┐  │ │
│  │  │ Memory Block│  │ Local Disk   │  │ │
│  │  │   Cache     │  │   Cache      │  │ │
│  │  └─────────────┘  └──────────────┘  │ │
│  └─────────────────────────────────────┘ │
└─────────────────────────────────────────┘
              │
              ▼
┌─────────────────────────────────────────┐
│    DFS (S3/HDFS/OSS/GFS)               │
│  ┌─────────────────────────────────────┐ │
│  │   Working Directory (SST Files)     │ │
│  │   Checkpoint Directory              │ │
│  └─────────────────────────────────────┘ │
└─────────────────────────────────────────┘
```

**Nexmark 벤치마크 결과**에서 I/O 집약적 쿼리의 경우 1GB 캐시만으로 로컬 스토어 대비 **75%~120% 처리량**을 달성했으며,  HDFS 비동기 모드는 동기 모드 대비 약 **2배 처리량 향상**을 보였습니다. 

### 비동기 실행 모델의 도입

Flink 2.0의 또 다른 핵심 변화는 **비동기 실행 모델(Asynchronous Execution Model)**입니다.   기존 동기식 모델에서는 상태 읽기 시 메인 태스크 스레드가 블로킹되어 HDFS 접근 지연(1.5ms)이 로컬 디스크(68μs)보다 20배 이상 높았고, DFS 사용 시 TPS가 95% 감소하는 문제가 있었습니다.

새로운 비동기 모델은 상태 접근과 계산을 분리하여 병렬 실행하며,  다음 세 가지 핵심 보장을 유지합니다:

- **동일 키 레코드 처리 순서 보장**
- **체크포인트 동기화 관리**
- **워터마크/타이머 시맨틱 유지** 

### Checkpoint/Savepoint 아키텍처 개선

ForSt 백엔드에서는 Working Directory와 Checkpoint Directory가 동일한 DFS를 공유하여 **Zero-Copy 체크포인트**가 가능합니다:  

```
기존 (1.20):
1. 상태 → 로컬 디스크
2. 체크포인트 트리거 → 전체 상태 DFS 업로드
3. 업로드 완료 → 체크포인트 완료

Flink 2.0:
1. 상태 → DFS (지속적 스트리밍)
2. 체크포인트 트리거 → 메타데이터만 저장
3. 파일 참조 공유 → 거의 즉시 완료
```

Flink 2.0에서 **네이티브 savepoint 포맷이 기본값**으로 변경되며, **LEGACY 복원 모드는 제거**되었습니다. 

-----

## 2. Java API 레벨 변경사항

### 완전히 제거된 API 세트

|제거된 API                         |대체 API                              |영향도 |
|--------------------------------|------------------------------------|----|
|**DataSet API**                 |DataStream API 또는 Table API/SQL     |🔴 높음|
|**Scala DataStream/DataSet API**|Java DataStream API                 |🔴 높음|
|**SourceFunction, SinkFunction**|Source V2, Sink V2                  |🔴 높음|
|**TableSource, TableSink**      |DynamicTableSource, DynamicTableSink|🔴 높음|
|**flink-conf.yaml**             |config.yaml (표준 YAML)               |🟡 중간|
|**Java 8 지원**                   |Java 11+ (권장 Java 17)               |🔴 높음|

### Source API 마이그레이션 (FLIP-27)

```java
// ❌ Flink 1.20 - SourceFunction (제거됨)
env.addSource(new FlinkKafkaConsumer<>(...));

// ✅ Flink 2.x - FLIP-27 Source API
KafkaSource<String> source = KafkaSource.<String>builder()
    .setBootstrapServers("localhost:9092")
    .setTopics("input-topic")
    .setGroupId("my-group")
    .setStartingOffsets(OffsetsInitializer.earliest())
    .setValueOnlyDeserializer(new SimpleStringSchema())
    .build();

env.fromSource(source, WatermarkStrategy.noWatermarks(), "Kafka Source");
```

새로운 Source API의 핵심 컴포넌트는 **Source**(팩토리), **SplitEnumerator**(분할 발견/할당), **SourceReader**(실제 읽기), **SourceSplit**(작업 단위)입니다. 

### Sink API 마이그레이션 (FLIP-143/191)

```java
// ❌ Flink 1.20 - SinkFunction (제거됨)
stream.addSink(new FlinkKafkaProducer<>(...));

// ✅ Flink 2.x - Sink V2 API
KafkaSink<String> sink = KafkaSink.<String>builder()
    .setBootstrapServers("localhost:9092")
    .setRecordSerializer(KafkaRecordSerializationSchema.builder()
        .setTopic("output-topic")
        .setValueSerializationSchema(new SimpleStringSchema())
        .build())
    .setDeliveryGuarantee(DeliveryGuarantee.AT_LEAST_ONCE)
    .build();

stream.sinkTo(sink);
```

### RichFunction.open() 메서드 시그니처 변경

```java
// ❌ Flink 1.20
@Override
public void open(Configuration parameters) throws Exception {
    // 초기화 로직
}

// ✅ Flink 2.x
@Override
public void open(OpenContext openContext) throws Exception {
    // 초기화 로직
}
```

### 비동기 State API (State API V2)

Flink 2.0에서 비동기 상태 접근 API가 도입되어 논블로킹 상태 접근이 가능합니다:  

```java
// ❌ 기존 동기식 API (1.20)
Integer val = wordCounter.value();  // 블로킹
int updated = (val == null ? 1 : val + 1);
wordCounter.update(updated);

// ✅ 새로운 비동기 API (2.0)
wordCounter.asyncValue()
    .thenCompose(val -> {
        int updated = (val == null ? 1 : val + 1);
        return wordCounter.asyncUpdate(updated);
    })
    .thenAccept(empty -> {
        out.collect(Tuple2.of(value.f0, updated.get()));
    });
```

### State TTL 변경사항

```java
// ❌ Flink 1.20 - Time 사용
import org.apache.flink.api.common.time.Time;
StateTtlConfig.newBuilder(Time.seconds(10))

// ✅ Flink 2.x - Duration 사용
import java.time.Duration;
StateTtlConfig ttlConfig = StateTtlConfig
    .newBuilder(Duration.ofMinutes(10))
    .setUpdateType(StateTtlConfig.UpdateType.OnCreateAndWrite)
    .setStateVisibility(StateTtlConfig.StateVisibility.NeverReturnExpired)
    .cleanupFullSnapshot()
    .cleanupInRocksdbCompactFilter(1000, Duration.ofDays(30))
    .build();
```

### keyBy 메서드 변경

```java
// ❌ Flink 1.20 (제거됨)
stream.keyBy(0)              // 필드 인덱스 사용
stream.keyBy("fieldName")    // 필드 이름 사용

// ✅ Flink 2.x (권장)
stream.keyBy(value -> value.f0)     // KeySelector 사용
stream.keyBy(MyClass::getKey)       // 메서드 참조 사용
```

### Table API / SQL API 변경

**새로운 SQL 기능:**

```sql
-- C-style Escape 문자열
SELECT E'Hello\nWorld';

-- QUALIFY 절 (윈도우 함수 필터링)
SELECT * FROM orders
QUALIFY ROW_NUMBER() OVER (PARTITION BY customer_id ORDER BY order_time DESC) = 1;

-- TABLE() 래퍼 없이 테이블 함수 호출
SELECT * FROM TUMBLE(orders, DESCRIPTOR(order_time), INTERVAL '1' HOUR);
```

-----

## 3. Breaking Changes 상세 목록

### 제거된 주요 클래스

**Core API:**

- `org.apache.flink.api.common.ExecutionMode`
- `org.apache.flink.api.common.time.Time` → `java.time.Duration`
- `org.apache.flink.api.common.restartstrategy.RestartStrategies` 전체

**Sink 관련:**

- `SinkFunction`, `RichSinkFunction`, `PrintSinkFunction`
- `TwoPhaseCommitSinkFunction`, `StreamingFileSink`

**Source 관련:**

- `SourceFunction`, `RichSourceFunction`, `ParallelSourceFunction`
- `FromElementsFunction`, `SocketTextStreamFunction`

**State Backend:**

- `FsStateBackend`, `MemoryStateBackend` (HashMapStateBackend + FileSystemCheckpointStorage로 대체)

**DataSet API 전체 패키지:**

- `org.apache.flink.api.java.DataSet`
- `org.apache.flink.api.java.ExecutionEnvironment`
- 모든 DataSet 연산자 및 I/O 클래스

### 제거된 설정 옵션 (주요 항목)

```yaml
# CheckpointingOptions
checkpointing.local-recovery → 제거됨
state.backend → 제거됨
state.backend.async → 제거됨

# JobManagerOptions
jobmanager.heap.size → 제거됨
jobmanager.scheduler: "Ng" → 제거됨
jobmanager.speculative.enabled → 제거됨

# NetworkOptions
taskmanager.network.blocking-shuffle.* → 제거됨
taskmanager.network.hybrid-shuffle.enable-new-mode → 제거됨

# TableOptions
table.exec.legacy-transformation-uids → 제거됨
table.exec.shuffle-mode → 제거됨
```

### 메서드 시그니처 변경

```java
// ExecutionConfig - 제거된 메서드들
void setRestartStrategy(RestartStrategyConfiguration)  // 제거됨
void setExecutionMode(ExecutionMode)                   // 제거됨

// TypeInformation
TypeSerializer<T> createSerializer(ExecutionConfig)    // 제거됨
// → TypeSerializer<T> createSerializer(SerializerConfig) 사용

// OutputFormat
void open(int taskNumber, int numTasks)                // 제거됨
// → void open(OutputFormat.InitializationContext context) 사용
```

### 커넥터 호환성 현황

|커넥터              |Flink 2.0 호환 버전              |상태    |
|-----------------|-----------------------------|------|
|**Kafka**        |`flink-connector-kafka:4.0.1`|✅ 출시됨 |
|**JDBC**         |`flink-connector-jdbc:4.0.0` |✅ 출시됨 |
|**Elasticsearch**|Flink 2.0 호환 버전              |✅ 출시됨 |
|**Paimon**       |Flink 2.0 호환 버전              |✅ 출시됨 |
|**기타**           |Flink 2.3까지 순차 지원            |🔄 진행 중|

### Java 버전 변경

|버전     |Flink 1.20|Flink 2.x  |
|-------|----------|-----------|
|Java 8 |✅ 지원      |❌ **제거**   |
|Java 11|✅ 지원      |✅ 최소 버전    |
|Java 17|✅ 지원      |✅ **기본/권장**|
|Java 21|❌ 미지원     |✅ 공식 지원    |

-----

## 4. Flink 2.x 신규 기능

### Materialized Table

**Materialized Table**은 배치와 스트림 데이터 파이프라인을 통합하는 새로운 테이블 유형입니다. 

```sql
-- Materialized Table 생성
CREATE MATERIALIZED TABLE my_materialized_table
PARTITIONED BY (ds)
WITH (
    'format' = 'json',
    'partition.fields.ds.date-formatter' = 'yyyy-MM-dd'
)
FRESHNESS = INTERVAL '1' HOUR
AS SELECT 
    user_id,
    COUNT(*) as order_count,
    SUM(amount) as total_amount,
    ds
FROM orders
GROUP BY user_id, ds;

-- 테이블 관리
ALTER MATERIALIZED TABLE my_table REFRESH;                    -- 수동 새로고침
ALTER MATERIALIZED TABLE my_table SUSPEND;                    -- 일시 중지
ALTER MATERIALIZED TABLE my_table RESUME;                     -- 재개
SHOW MATERIALIZED TABLES;                                      -- 목록 조회 (2.2+)
```

**새로고침 모드:**

- `CONTINUOUS`: 스트리밍 작업이 지속적으로 데이터 갱신
- `FULL`: 스케줄러가 주기적으로 배치 작업 트리거 

### 향상된 Watermark 정렬

Split 레벨까지 확장된 Watermark 정렬로 불균형 소스 문제를 해결합니다:  

```java
DataStream<Long> eventStream = env.fromSource(
    new NumberSequenceSource(0, Long.MAX_VALUE),
    WatermarkStrategy.<Long>forMonotonousTimestamps()
        .withTimestampAssigner(new LongTimestampAssigner())
        .withWatermarkAlignment(
            "alignment-group-1",      // 정렬 그룹 레이블
            Duration.ofSeconds(30),   // 최대 허용 드리프트
            Duration.ofSeconds(1)     // 업데이트 간격
        ),
    "NumberSequenceSource"
);
```

### Adaptive Batch Execution 개선

**10TB TPC-DS 벤치마크** 결과:

- ANALYZE TABLE 통계 정보 사용 시: Flink 1.20 대비 **8% 성능 향상**
- 추가 통계 정보 없이: Flink 1.20 대비 **16% 성능 향상** 

주요 개선 사항:

- **Adaptive Broadcast Join**: 런타임에 입력 크기 기반 자동 전환 
- **Automatic Join Skew Optimization**: 스큐된 데이터 파티션 동적 분할  

### AI 통합 기능 (Flink 2.1/2.2)

```sql
-- ML_PREDICT 함수 (2.1+)
SELECT ML_PREDICT('openai-model', text_column) FROM logs;

-- VECTOR_SEARCH 함수 (2.2)
SELECT VECTOR_SEARCH(embedding_column, 'similarity_index') FROM data;
```

### 직렬화 개선

- **컬렉션 타입 직렬화기**: Map/List/Set에 대한 효율적인 내장 직렬화기 (기본 활성화) 
- **Kryo 5.6 업그레이드**: 더 빠르고 메모리 효율적, 최신 Java 버전 지원 개선  

-----

## 5. 마이그레이션 가이드

### 권장 마이그레이션 순서

1. **Java 버전 업그레이드**: Java 8 → Java 11/17/21 (권장: Java 17)
1. **설정 파일 마이그레이션**: `flink-conf.yaml` → `config.yaml`
1. **Deprecated API 제거 확인 및 코드 수정**
1. **커넥터 버전 업그레이드**
1. **새 환경에서 테스트**
1. **새 Savepoint 생성 후 배포**

### 설정 파일 마이그레이션

```bash
# 마이그레이션 도구 사용
bin/flink migrate-config --source flink-conf.yaml --target config.yaml
```

**새로운 config.yaml 형식:**

```yaml
jobmanager:
  rpc:
    address: localhost
    port: 6123
  memory:
    process:
      size: 1600m

taskmanager:
  memory:
    process:
      size: 1728m
  numberOfTaskSlots: 1

parallelism:
  default: 1
```

### 코드 마이그레이션 패턴

**ExecutionConfig API 변경:**

```java
// ❌ Flink 1.20
env.getConfig().setRestartStrategy(
    RestartStrategies.fixedDelayRestart(3, Time.seconds(10)));

// ✅ Flink 2.x
Configuration config = new Configuration();
config.set(RestartStrategyOptions.RESTART_STRATEGY, "fixed-delay");
config.set(RestartStrategyOptions.RESTART_STRATEGY_FIXED_DELAY_ATTEMPTS, 3);
config.set(RestartStrategyOptions.RESTART_STRATEGY_FIXED_DELAY_DELAY, 
    Duration.ofSeconds(10));
env.configure(config);
```

### 상태 호환성 주의사항

**핵심 주의:** 1.x → 2.x 간 상태 호환성이 **보장되지 않습니다**.  

권장 마이그레이션 전략:

1. 1.20에서 최종 Savepoint 생성
1. 2.x 환경에서 새 Job 배포 (상태 없이 시작)
1. 데이터 재처리 또는 외부 상태 복구 메커니즘 활용

-----

## 6. Flink 2.x 개발 시작 가이드

### Maven 프로젝트 설정

```xml
<?xml version="1.0" encoding="UTF-8"?>
<project xmlns="http://maven.apache.org/POM/4.0.0"
         xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
         xsi:schemaLocation="http://maven.apache.org/POM/4.0.0 
         http://maven.apache.org/xsd/maven-4.0.0.xsd">
    <modelVersion>4.0.0</modelVersion>
    
    <groupId>com.example</groupId>
    <artifactId>flink-job</artifactId>
    <version>1.0-SNAPSHOT</version>
    
    <properties>
        <flink.version>2.0.0</flink.version>
        <java.version>17</java.version>
        <maven.compiler.source>${java.version}</maven.compiler.source>
        <maven.compiler.target>${java.version}</maven.compiler.target>
    </properties>
    
    <dependencies>
        <!-- Core Streaming API -->
        <dependency>
            <groupId>org.apache.flink</groupId>
            <artifactId>flink-streaming-java</artifactId>
            <version>${flink.version}</version>
            <scope>provided</scope>
        </dependency>
        
        <!-- Clients -->
        <dependency>
            <groupId>org.apache.flink</groupId>
            <artifactId>flink-clients</artifactId>
            <version>${flink.version}</version>
            <scope>provided</scope>
        </dependency> [![](claude-citation:/icon.png?validation=C9FA770F-2139-48CD-8E05-774CDC5D1D07&citation=eyJlbmRJbmRleCI6MTI5MjEsIm1ldGFkYXRhIjp7Imljb25VcmwiOiJodHRwczpcL1wvd3d3Lmdvb2dsZS5jb21cL3MyXC9mYXZpY29ucz9zej02NCZkb21haW49YXBhY2hlLm9yZyIsInByZXZpZXdUaXRsZSI6IkFwYWNoZSBGbGluayAyLjAuMSBSZWxlYXNlIEFubm91bmNlbWVudCB8IEFwYWNoZSBGbGluayIsInNvdXJjZSI6IkFwYWNoZSBGbGluayIsInR5cGUiOiJnZW5lcmljX21ldGFkYXRhIn0sInNvdXJjZXMiOlt7Imljb25VcmwiOiJodHRwczpcL1wvd3d3Lmdvb2dsZS5jb21cL3MyXC9mYXZpY29ucz9zej02NCZkb21haW49YXBhY2hlLm9yZyIsInNvdXJjZSI6IkFwYWNoZSBGbGluayIsInRpdGxlIjoiQXBhY2hlIEZsaW5rIDIuMC4xIFJlbGVhc2UgQW5ub3VuY2VtZW50IHwgQXBhY2hlIEZsaW5rIiwidXJsIjoiaHR0cHM6XC9cL2ZsaW5rLmFwYWNoZS5vcmdcLzIwMjVcLzExXC8xMFwvYXBhY2hlLWZsaW5rLTIuMC4xLXJlbGVhc2UtYW5ub3VuY2VtZW50XC8ifV0sInN0YXJ0SW5kZXgiOjExNjY5LCJ0aXRsZSI6IkFwYWNoZSBGbGluayIsInVybCI6Imh0dHBzOlwvXC9mbGluay5hcGFjaGUub3JnXC8yMDI1XC8xMVwvMTBcL2FwYWNoZS1mbGluay0yLjAuMS1yZWxlYXNlLWFubm91bmNlbWVudFwvIiwidXVpZCI6ImJlYjE3ODlmLTVhYTktNGI4NC1hMTgzLTAzYzg1M2Y0MTAwMSJ9 "Apache Flink")](https://flink.apache.org/2025/11/10/apache-flink-2.0.1-release-announcement/)
        
        <!-- Kafka Connector -->
        <dependency>
            <groupId>org.apache.flink</groupId>
            <artifactId>flink-connector-kafka</artifactId>
            <version>4.0.1</version>
        </dependency>
        
        <!-- Test -->
        <dependency>
            <groupId>org.apache.flink</groupId>
            <artifactId>flink-test-utils</artifactId>
            <version>${flink.version}</version>
            <scope>test</scope>
        </dependency>
    </dependencies>
</project>
```

**Artifact ID 변경 주의:** Scala suffix가 제거되었습니다.

- `flink-streaming-java_2.12` → `flink-streaming-java`
- `flink-clients_2.12` → `flink-clients`

### 기본 DataStream 애플리케이션 구조

```java
import org.apache.flink.api.common.functions.FilterFunction;
import org.apache.flink.api.common.functions.MapFunction;
import org.apache.flink.streaming.api.datastream.DataStream;
import org.apache.flink.streaming.api.environment.StreamExecutionEnvironment;

public class DataStreamJob {
    public static void main(String[] args) throws Exception {
        // 1. 실행 환경 생성
        final StreamExecutionEnvironment env = 
            StreamExecutionEnvironment.getExecutionEnvironment();
        
        // 2. 데이터 소스 정의 (FLIP-27 Source API)
        KafkaSource<String> source = KafkaSource.<String>builder()
            .setBootstrapServers("localhost:9092")
            .setTopics("input-topic")
            .setGroupId("flink-group")
            .setStartingOffsets(OffsetsInitializer.earliest())
            .setValueOnlyDeserializer(new SimpleStringSchema())
            .build();
        
        DataStream<String> stream = env.fromSource(
            source, WatermarkStrategy.noWatermarks(), "Kafka Source");
        
        // 3. 변환 적용
        DataStream<String> processed = stream
            .filter((FilterFunction<String>) value -> value.length() > 4)
            .map((MapFunction<String, String>) String::toUpperCase);
        
        // 4. 싱크 정의 (Sink V2 API)
        KafkaSink<String> sink = KafkaSink.<String>builder()
            .setBootstrapServers("localhost:9092")
            .setRecordSerializer(KafkaRecordSerializationSchema.builder()
                .setTopic("output-topic")
                .setValueSerializationSchema(new SimpleStringSchema())
                .build())
            .setDeliveryGuarantee(DeliveryGuarantee.AT_LEAST_ONCE)
            .build();
        
        processed.sinkTo(sink);
        
        // 5. 실행
        env.execute("Flink 2.x DataStream Job");
    }
}
```

### 상태 관리 모범 사례

```java
public class StatefulFunction extends KeyedProcessFunction<String, Event, Result> {
    
    private ValueState<Long> countState;
    private MapState<String, Long> mapState;
    
    @Override
    public void open(OpenContext openContext) throws Exception {
        // TTL 설정
        StateTtlConfig ttlConfig = StateTtlConfig
            .newBuilder(Duration.ofHours(1))
            .setUpdateType(StateTtlConfig.UpdateType.OnCreateAndWrite)
            .setStateVisibility(StateTtlConfig.StateVisibility.NeverReturnExpired)
            .cleanupFullSnapshot()
            .build();
        
        ValueStateDescriptor<Long> countDescriptor = 
            new ValueStateDescriptor<>("count", Long.class);
        countDescriptor.enableTimeToLive(ttlConfig);
        countState = getRuntimeContext().getState(countDescriptor);
        
        MapStateDescriptor<String, Long> mapDescriptor = 
            new MapStateDescriptor<>("map-state", String.class, Long.class);
        mapState = getRuntimeContext().getMapState(mapDescriptor);
    }
    
    @Override
    public void processElement(Event event, Context ctx, Collector<Result> out) 
            throws Exception {
        Long currentCount = countState.value();
        if (currentCount == null) {
            currentCount = 0L;
        }
        currentCount++;
        countState.update(currentCount);
        
        out.collect(new Result(event.getKey(), currentCount));
    }
}
```

### ForSt State Backend 설정

```yaml
# config.yaml
state.backend.type: forst
table.exec.async-state.enabled: true
execution.checkpointing.incremental: true
execution.checkpointing.dir: s3://your-bucket/flink-checkpoints

# 아직 지원하지 않는 기 [![](claude-citation:/icon.png?validation=C9FA770F-2139-48CD-8E05-774CDC5D1D07&citation=eyJlbmRJbmRleCI6MTcxNTQsIm1ldGFkYXRhIjp7Imljb25VcmwiOiJodHRwczpcL1wvd3d3Lmdvb2dsZS5jb21cL3MyXC9mYXZpY29ucz9zej02NCZkb21haW49YXBhY2hlLm9yZyIsInByZXZpZXdUaXRsZSI6IkRpc2FnZ3JlZ2F0ZWQgU3RhdGUgTWFuYWdlbWVudCB8IEFwYWNoZSBGbGluayIsInNvdXJjZSI6IkFwYWNoZSIsInR5cGUiOiJnZW5lcmljX21ldGFkYXRhIn0sInNvdXJjZXMiOlt7Imljb25VcmwiOiJodHRwczpcL1wvd3d3Lmdvb2dsZS5jb21cL3MyXC9mYXZpY29ucz9zej02NCZkb21haW49YXBhY2hlLm9yZyIsInNvdXJjZSI6IkFwYWNoZSIsInRpdGxlIjoiRGlzYWdncmVnYXRlZCBTdGF0ZSBNYW5hZ2VtZW50IHwgQXBhY2hlIEZsaW5rIiwidXJsIjoiaHR0cHM6XC9cL25pZ2h0bGllcy5hcGFjaGUub3JnXC9mbGlua1wvZmxpbmstZG9jcy1tYXN0ZXJcL2RvY3NcL29wc1wvc3RhdGVcL2Rpc2FnZ3JlZ2F0ZWRfc3RhdGVcLyJ9XSwic3RhcnRJbmRleCI6MTY5NDgsInRpdGxlIjoiQXBhY2hlIiwidXJsIjoiaHR0cHM6XC9cL25pZ2h0bGllcy5hcGFjaGUub3JnXC9mbGlua1wvZmxpbmstZG9jcy1tYXN0ZXJcL2RvY3NcL29wc1wvc3RhdGVcL2Rpc2FnZ3JlZ2F0ZWRfc3RhdGVcLyIsInV1aWQiOiJhOWE3NGU0Ni1hYzgyLTQzZmUtOTg3Yi0zYTczMTZkNTJiYWEifQ%3D%3D "Apache")](https://nightlies.apache.org/flink/flink-docs-master/docs/ops/state/disaggregated_state/)능 비활성화 (ForSt 실험적 단계)
table.exec.mini-batch.enabled: false
table.optimizer.agg-phase-strategy: ONE_PHASE
```

### Kubernetes 배포 설정

```yaml
apiVersion: flink.apache.org/v1beta1
kind: FlinkDeployment
metadata:
  name: flink-job
  namespace: flink
spec:
  image: flink:2.0.0
  flinkVersion: v2_0
  flinkConfiguration:
    taskmanager.numberOfTaskSlots: "2"
    state.backend.type: rocksdb
    state.checkpoints.dir: s3://bucket/checkpoints
    state.savepoints.dir: s3://bucket/savepoints
    execution.checkpointing.interval: "60000"
    execution.checkpointing.mode: EXACTLY_ONCE
    restart-strategy.type: exponential-delay
    restart-strategy.exponential-delay.initial-backoff: 10s
    restart-strategy.exponential-delay.max-backoff: 5min
  serviceAccount: flink
  jobManager:
    resource:
      memory: "2048m"
      cpu: 1
  taskManager:
    resource:
      memory: "4096m"
      cpu: 2
    replicas: 3
  job:
    jarURI: local:///opt/flink/usrlib/my-job.jar
    parallelism: 6
    upgradeMode: savepoint
    state: running
```

### 테스트 전략

```java
import org.apache.flink.runtime.testutils.MiniClusterResourceConfiguration;
import org.apache.flink.test.util.MiniClusterWithClientResource;
import org.junit.ClassRule;
import org.junit.Test;

public class StreamingJobIntegrationTest {
    
    @ClassRule
    public static MiniClusterWithClientResource flinkCluster =
        new MiniClusterWithClientResource(
            new MiniClusterResourceConfiguration.Builder()
                .setNumberSlotsPerTaskManager(2)
                .setNumberTaskManagers(1)
                .build());
    
    @Test
    public void testStreamingJob() throws Exception {
        StreamExecutionEnvironment env = 
            StreamExecutionEnvironment.getExecutionEnvironment();
        env.setParallelism(2);
        
        // 테스트 로직
        env.fromData(1L, 2L, 3L)
            .map(new IncrementMapFunction())
            .print();
        
        env.execute();
    }
}
```

-----

## 결론

Apache Flink 2.0은 클라우드 네이티브 환경을 위한 **근본적인 아키텍처 전환**을 제공합니다. **ForSt State Backend**와 **비동기 실행 모델**은 대규모 상태 처리의 병목을 해결하고, **Materialized Table**은 스트림-배치 통합을 단순화합니다.

마이그레이션 시 가장 중요한 점은 **상태 호환성이 보장되지 않는다**는 것입니다. 따라서 신중한 마이그레이션 계획과 충분한 테스트가 필수입니다. Flink 1.20은 **2년간 LTS로 지원**되므로, 급하지 않다면 커넥터 생태계가 안정화되는 Flink 2.3 이후 마이그레이션을 고려할 수 있습니다.

새로운 프로젝트라면 **Flink 2.x**로 시작하여 최신 API와 아키텍처의 이점을 활용하는 것이 권장됩니다. 특히 Kubernetes 환경에서 대규모 상태를 다루는 애플리케이션이라면 ForSt State Backend의 이점이 클 것입니다.