# Kafka 직렬화 포맷 선택 가이드: Strimzi 환경 최적화

**결론부터 말하면, 하루 1,000만 건 이상 처리하는 Strimzi 환경에서는 Protocol Buffers(Protobuf)가 성능 측면에서 최적이며, 스키마 진화 유연성이 중요하다면 Apache Avro를 권장한다.** Schema Registry는 Strimzi와의 네이티브 통합성과 Apache 2.0 라이선스를 고려할 때 Apicurio Registry가 가장 적합하다. 바이너리 포맷(Protobuf/Avro)은 JSON 대비 **5배 빠른 직렬화 속도** 와 **85% 작은 메시지 크기**를 제공하며,  대용량 처리 환경에서 필수적인 선택이다.

-----

## 직렬화 포맷별 핵심 특성 비교

각 직렬화 포맷은 근본적으로 다른 설계 철학을 따른다. **Apache Avro**는 Hadoop 생태계에서 탄생하여  스키마 진화와 동적 타이핑을 최우선으로 설계되었고,   스키마 없이도 런타임에 데이터를 처리할 수 있다.   **Protobuf**는 Google이 내부 RPC를 위해 만들어 성능과 효율성을 극대화했으며,  필드 번호 기반 식별로 빠른 파싱을 지원한다. **JSON Schema**는 JSON 문서 검증을 목적으로 만들어져  인간 가독성이 뛰어나지만 텍스트 기반의 한계가 있다. 

|특성                 |Avro       |Protobuf|JSON Schema|MessagePack|
|-------------------|-----------|--------|-----------|-----------|
|**데이터 형식**         |바이너리(+JSON)|바이너리    |텍스트(JSON)  |바이너리       |
|**스키마 필수**         |Yes        |Yes     |Yes        |No         |
|**코드 생성**          |선택적        |필수      |불필요        |불필요        |
|**Confluent SerDe**|✅ 네이티브     |✅ 네이티브  |✅ 네이티브     |❌ 커스텀 필요   |
|**Schema Registry**|✅ 완전 지원    |✅ 완전 지원 |✅ 완전 지원    |❌ 미지원      |

스키마 정의 방식도 상이하다. Avro는 JSON 기반 스키마(`{"type": "record", "name": "User", "fields": [...]}`)를 사용하고,  Protobuf는 `.proto` 파일과 `protoc` 컴파일러가 필요하며,   JSON Schema는 표준 JSON으로 정의한다. MessagePack은 스키마 없이 JSON과 유사한 데이터 모델을 바이너리로 인코딩한다. 

-----

## 성능 벤치마크: 수치로 본 실제 차이

### 직렬화/역직렬화 속도

2024년 Umeå University 학술 연구(Kafka 3 브로커, 104만 레코드 테스트)에서 측정된 총 처리 시간은 Protobuf의 압도적 우위를 보여준다: 

|레코드 크기    |Protobuf  |MessagePack|Avro  |JSON   |
|----------|----------|-----------|------|-------|
|**1,176B**|**6.27초** |8.09초      |17.2초 |34.16초 |
|**4,696B**|**28.15초**|40.32초     |76.4초 |142.5초 |
|**9,312B**|**58.18초**|92.21초     |159.8초|297.23초|

**Protobuf는 JSON 대비 약 5배 빠른 직렬화/역직렬화 속도**를 보이며,  Avro보다도 2.7배 빠르다. CPU 사용률 측면에서도 Protobuf는 전체 처리 시간의 **22%**만 직렬화에 사용하는 반면, JSON은 **50%** 이상을 소비한다. 

### 메시지 크기 및 압축률

동일 데이터의 직렬화된 크기 비교에서 바이너리 포맷의 효율성이 명확하게 드러난다: 

|원본 크기     |Protobuf       |Avro       |MessagePack|JSON   |
|----------|---------------|-----------|-----------|-------|
|**1,176B**|**181B** (85%↓)|239B (80%↓)|1,019B     |1,178B |
|**4,696B**|**825B**       |1,063B     |5,261B     |5,954B |
|**9,312B**|**1,630B**     |2,093B     |10,512B    |11,885B|

대용량 데이터셋(100만 레코드)에서는 Avro가 **64.5MB**로 Protobuf(68.5MB)보다 약 7% 더 컴팩트한데,  이는 Avro의 컨테이너 포맷이 반복 데이터에 더 효율적이기 때문이다.

### 처리량(Throughput) 비교

1,176바이트 레코드 기준 초당 처리량:

- **Protobuf**: 36,945 records/sec
- **MessagePack**: 34,254 records/sec
- **Avro**: 20,544 records/sec
- **JSON**: 14,243 records/sec 

대용량 레코드(9,312B)에서는 MessagePack이 **3,802 records/sec**로 가장 높은 처리량을 보이며,  메시지 크기에 따라 최적 포맷이 달라질 수 있음을 시사한다.

-----

## 스키마 진화: 안전한 스키마 변경 전략

### 호환성 모드 이해

Schema Registry는 6가지 호환성 모드를 지원하며,  배포 순서에 직접적인 영향을 미친다:

|모드               |설명                 |업그레이드 순서   |
|-----------------|-------------------|-----------|
|**BACKWARD** (기본)|새 스키마가 이전 데이터 읽기 가능|Consumer 먼저|
|**FORWARD**      |이전 스키마가 새 데이터 읽기 가능|Producer 먼저|
|**FULL**         |양방향 호환             |순서 무관      |
|**TRANSITIVE 변형**|모든 이전 버전과 호환 검사    |동일         |

### 필드 변경 시 동작 비교

|작업           |Avro         |Protobuf      |JSON Schema|
|-------------|-------------|--------------|-----------|
|**선택적 필드 추가**|✅ default 필수 |✅ 새 번호로       |콘텐츠 모델에 따름 |
|**필수 필드 추가** |⚠️ Forward만   |❌ 불가          |⚠️ Open 모델만 |
|**필드 삭제**    |✅ default 있으면|✅ reserved 선언 |콘텐츠 모델에 따름 |
|**필드 이름 변경** |✅ aliases 사용 |✅ 이름은 wire에 없음|❌ Breaking |
|**타입 변경**    |⚠️ 제한적 승격만    |⚠️ 호환 타입만      |❌ Breaking |

**Avro**는 필드명 기반 매칭과 `aliases` 지원으로 가장 유연한 스키마 진화를 제공한다.  **Protobuf**는 필드 번호 기반이라 이름 변경은 자유롭지만,  한번 사용한 필드 번호는 절대 재사용하면 안 된다.  `reserved` 키워드로 삭제된 번호를 명시적으로 예약해야 한다: 

```protobuf
message Foo {
  reserved 2, 15, 9 to 11;  // 삭제된 필드 번호
  reserved "foo", "bar";     // 삭제된 필드 이름
}
```

### 실전 배포 시나리오

**Consumer 먼저 업그레이드 시** (BACKWARD 호환): 새 Consumer가 이전 데이터를 default 값으로 처리하므로 가장 안전하다. **Producer 먼저 업그레이드 시** (FORWARD 호환): Avro/Protobuf 모두 이전 Consumer가 알 수 없는 필드를 무시하므로 상대적으로 안전하지만,  Schema Registry에 새 스키마를 사전 등록해야 한다.

-----

## Schema Registry 솔루션 비교

### 아키텍처 및 핵심 기능

|특성       |Confluent Schema Registry  |Apicurio Registry            |AWS Glue Schema Registry   |
|---------|---------------------------|-----------------------------|---------------------------|
|**스토리지** |Kafka 토픽 (`_schemas`)      |Kafka/PostgreSQL/In-Memory   |서버리스 관리형                   |
|**지원 포맷**|Avro, Protobuf, JSON Schema|+OpenAPI, AsyncAPI, GraphQL 등|Avro, Protobuf, JSON Schema|
|**라이선스** |Community (제한적)            |**Apache 2.0 (완전 오픈)**       |무료                         |
|**고가용성** |Primary-Secondary          |Kafka HA 상속                  |자동 Multi-AZ                |

**Apicurio Registry**는 가장 넓은 포맷 지원과 Apache 2.0 라이선스로, 벤더 종속 없이 자유롭게 사용할 수 있다.  **Confluent Schema Registry**는 가장 성숙한 생태계를 제공하지만  Community License의 경쟁 제품 개발 제한이 있다.

### Strimzi 환경 통합

**Apicurio Registry가 Strimzi와 가장 우수한 네이티브 통합**을 제공한다. 공식 Kubernetes Operator를 통해 ApicurioRegistry CR을 배포하고, KafkaSQL 모드로 Strimzi 관리 Kafka 클러스터를 직접 사용할 수 있다:

```yaml
apiVersion: registry.apicur.io/v1
kind: ApicurioRegistry
spec:
  configuration:
    persistence: "kafkasql"
    kafkasql:
      bootstrapServers: "my-cluster-kafka-bootstrap:9093"
      security:
        tls:
          keystoreSecretName: my-user
          truststoreSecretName: my-cluster-cluster-ca-cert
```

Confluent Schema Registry는 별도의 CFK(Confluent for Kubernetes) Operator 설치가 필요하며, Strimzi와 직접 통합되지 않아 Bootstrap 서버를 수동으로 지정해야 한다.

### 성능 오버헤드

모든 Registry는 **클라이언트 측 캐싱**을 지원하여 런타임 오버헤드를 최소화한다. Confluent 테스트에서 캐싱 적용 시 SpecificAvroSerde는 **~1.87M ops/sec**, 캐싱 미적용 시 **~261K ops/sec**로 약 **7배 성능 차이**가 발생했다. 첫 스키마 조회 시에만 네트워크 호출이 발생하고, 이후는 로컬 캐시(기본 24시간)를 사용한다.

-----

## Kafka 생태계 통합 가이드

### Kafka Connect 호환성

|포맷         |Converter            |Schema Registry 필수|복잡도|
|-----------|---------------------|------------------|---|
|Avro       |`AvroConverter`      |Yes               |중간 |
|Protobuf   |`ProtobufConverter`  |Yes               |중간 |
|JSON Schema|`JsonSchemaConverter`|Yes               |중간 |
|JSON       |`JsonConverter`      |No                |낮음 |
|MessagePack|커스텀 필요               |No                |높음 |

Avro, Protobuf, JSON Schema는 모든 Confluent 및 커뮤니티 커넥터(JDBC, S3, Elasticsearch 등)와 완전 호환된다. 주요 주의점으로 **Protobuf의 map 타입**은 기본적으로 Array of Struct로 변환되며, `protoMapConversionType=map` 설정이 필요할 수 있다.

### Kafka Streams 처리

```java
// Avro Serde 설정 예시
Properties settings = new Properties();
settings.put(StreamsConfig.DEFAULT_VALUE_SERDE_CLASS_CONFIG, SpecificAvroSerde.class);
settings.put("schema.registry.url", "http://localhost:8081");
```

**상태 저장소(State Store)**는 바이너리 포맷(Avro/Protobuf)이 RocksDB 스토리지 효율성을 높인다. **Changelog 토픽**은 상태 저장소와 동일한 Serde를 사용하므로, 스키마 진화가 상태 복원에 영향을 미친다. Kafka Streams는 **BACKWARD, BACKWARD_TRANSITIVE, FULL, FULL_TRANSITIVE**만 지원하며, Streams 애플리케이션을 업스트림 Producer보다 먼저 업그레이드해야 한다.

### Producer/Consumer 구현 복잡도

|포맷             |Producer 복잡도      |Consumer 복잡도|의존성                         |
|---------------|------------------|------------|----------------------------|
|**Avro**       |중간                |중간          |kafka-avro-serializer       |
|**Protobuf**   |중간 (.proto 컴파일 필요)|중간          |kafka-protobuf-serializer   |
|**JSON Schema**|낮음                |낮음          |kafka-json-schema-serializer|
|**JSON**       |매우 낮음             |매우 낮음       |기본 제공                       |

다국어 지원은 **Java, Python, Go, .NET** 모두 Confluent 공식 클라이언트에서 Avro, Protobuf, JSON Schema를 네이티브로 지원한다.  

-----

## 대용량 환경(10M+/일) 권장사항

### 규모별 포맷 선택 가이드

|일일 처리량       |권장 포맷               |근거           |
|-------------|--------------------|-------------|
|< 100만 건     |JSON Schema         |디버깅 용이, 개발 속도|
|100만~1,000만 건|Avro 또는 MessagePack |균형 잡힌 성능     |
|1,000만~1억 건  |**Protobuf 또는 Avro**|필수적인 바이너리 효율 |
|> 1억 건       |**Protobuf**        |최저 지연시간      |

### Schema Registry 도입 의사결정 프레임워크

**도입해야 하는 경우:**

- 다수의 Producer/Consumer가 동일 토픽 사용
- 스키마가 시간에 따라 변경될 가능성이 높음
- 데이터 품질과 계약(Contract) 관리가 중요
- Kafka Connect 또는 Kafka Streams 활용

**도입하지 않아도 되는 경우:**

- 단일 서비스 내부 통신
- 스키마가 거의 변경되지 않는 고정 포맷
- 빠른 프로토타이핑 단계

### Strimzi 환경 최종 권장 구성

1. **직렬화 포맷**: 성능 최우선 시 **Protobuf**, 스키마 유연성 필요 시 **Avro**
1. **Schema Registry**: **Apicurio Registry** (KafkaSQL 모드, Operator 배포)
1. **호환성 모드**: **BACKWARD_TRANSITIVE** (Protobuf) 또는 **FULL** (Avro)
1. **캐싱 전략**: 클라이언트 캐시 활성화, TTL 24시간 이상 권장
1. **모니터링**: Schema Registry JMX 메트릭 + Prometheus/Grafana 연동

-----

## 결론: 실전 선택 기준

**성능이 최우선**이라면 Protobuf를 선택하라.  JSON 대비 5배 빠른 속도와 85% 작은 메시지 크기로 대용량 처리에서 압도적 우위를 보인다.  **스키마 변경이 잦은 환경**에서는 Avro가 더 나은 선택이다— aliases를 통한 필드 이름 변경, 유연한 default 값 처리, Kafka 생태계와의 오랜 통합 경험이 운영 안정성을 높인다.

Strimzi 환경에서는 **Apicurio Registry**가 네이티브 Kubernetes Operator 지원, Apache 2.0 라이선스, KafkaSQL 스토리지 모드로 가장 적합하다.  Confluent Schema Registry 대비 라이선스 제약이 없으면서 동일한 호환성 API를 제공하므로 마이그레이션도 용이하다. 

핵심 원칙을 기억하라: **바이너리 포맷(Protobuf/Avro)은 대용량 환경에서 선택이 아닌 필수**이며,  Schema Registry는 스키마 진화가 예상되는 모든 프로덕션 환경에서 도입해야 한다. 