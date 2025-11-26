쿠버네티스 환경(특히 191대 규모의 온프레미스)에서 **Strimzi Kafka Operator**를 통해 Kafka 클러스터를 구축한 후, 설치가 성공적이고 운영 가능한 상태인지 검증하기 위한 상세 테스트 계획서입니다.

단순히 "파드가 떴는지" 확인하는 것을 넘어, **메시지 송수신, 토픽 관리, 스토리지 지속성(Persistence), 그리고 장애 복구 능력**까지 검증해야 합니다.

-----

# Strimzi Kafka 구축 검증 및 테스트 계획서

**작성일:** 2025년 00월 00일
**대상:** Strimzi Kafka Operator & Kafka Cluster
**목적:** 오퍼레이터의 관리 기능 정상 동작 확인 및 Kafka 클러스터의 데이터 처리/보존 능력 검증

-----

## 1\. Operator 및 CRD 상태 점검 (Operator Health)

가장 먼저 오퍼레이터가 정상적으로 설치되었고, 쿠버네티스 API와 통신하여 Custom Resource(CR)를 인식할 수 있는지 확인합니다.

| ID | 테스트 항목 | 점검 방법 (CLI) | 예상 결과 (Expected Result) | 확인 |
| :--- | :--- | :--- | :--- | :--- |
| **OP-01** | **Operator Pod 상태** | `kubectl get pods -n <strimzi-namespace> -l name=strimzi-cluster-operator` | `strimzi-cluster-operator` 파드가 `Running` 상태이며 재시작 횟수(Restarts)가 0이어야 함. | [ ] |
| **OP-02** | **CRD 등록 확인** | `kubectl get crd | grep strimzi` | `kafkas.kafka.strimzi.io`, `kafkatopics...`, `kafkausers...` 등 CRD 목록이 출력되어야 함. | [ ] |
| **OP-03** | **Operator 로그 점검** | `kubectl logs <operator-pod> -n <strimzi-namespace>` | 로그에 `ERROR`나 `Exception` 없이 "Reconciliation" 관련 로그가 주기적으로 보여야 함. | [ ] |

-----

## 2\. Kafka 클러스터 인프라 점검 (Cluster Infrastructure)

`Kafka` 리소스(YAML) 배포 후, 실제 브로커와 주키퍼(또는 KRaft)가 계획대로 구성되었는지 확인합니다.

| ID | 테스트 항목 | 점검 방법 (CLI) | 예상 결과 (Expected Result) | 확인 |
| :--- | :--- | :--- | :--- | :--- |
| **INF-01** | **Zookeeper/KRaft 상태** | `kubectl get pods -l app.kubernetes.io/name=zookeeper` (또는 KRaft) | 설정한 수(예: 3대)만큼 파드가 `Running` 상태여야 함. | [ ] |
| **INF-02** | **Kafka Broker 상태** | `kubectl get pods -l app.kubernetes.io/name=kafka` | 설정한 브로커 수(예: 3대 이상)만큼 파드가 `Running` 및 `Ready` 상태여야 함. | [ ] |
| **INF-03** | **Entity Operator 상태** | `kubectl get pods -l app.kubernetes.io/name=entity-operator` | TopicOperator와 UserOperator 역할을 하는 파드가 정상 구동되어야 함. | [ ] |
| **INF-04** | **Service 연결성** | `kubectl get svc` | `<cluster-name>-kafka-bootstrap`, `<cluster-name>-kafka-brokers` 서비스가 생성되어야 함. | [ ] |
| **INF-05** | **PVC 바인딩 (중요)** | `kubectl get pvc -l strimzi.io/cluster=<cluster-name>` | 모든 Kafka 브로커와 Zookeeper가 `sc-local-nvme` (또는 지정한 SC)와 `Bound` 상태여야 함. | [ ] |
| **INF-06** | **Config 적용 확인** | 브로커 파드 내부 접속 -\> `cat /tmp/strimzi.properties` | 설정한 Kafka 옵션(예: `log.retention.hours`, `message.max.bytes`)이 파일에 반영되어 있어야 함. | [ ] |

-----

## 3\. 기능 및 메시지 처리 테스트 (Functional Testing)

실제 토픽을 생성하고 메시지를 주고받으며 클러스터의 기능을 검증합니다.

### FUNC-01: 토픽 생성 (Topic Operator 검증)

  * **방법:** `KafkaTopic` CR(Custom Resource) 배포
    ```yaml
    apiVersion: kafka.strimzi.io/v1beta2
    kind: KafkaTopic
    metadata:
      name: test-topic
      labels:
        strimzi.io/cluster: <my-cluster>
    spec:
      partitions: 3
      replicas: 3
    ```
  * **확인:** `kubectl get kafkatopics`에서 `Ready` 상태 확인. 실제 브로커 내부에서 토픽이 생성되었는지 확인.

### FUNC-02: 내부 클라이언트 통신 (Producer/Consumer)

Strimzi 이미지가 제공하는 내부 스크립트를 사용하여 송수신 테스트를 수행합니다.

  * **Producer 실행:**
    ```bash
    kubectl run kafka-producer -ti --image=quay.io/strimzi/kafka:latest-kafka-3.6.0 --rm=true --restart=Never -- bin/kafka-console-producer.sh --bootstrap-server <my-cluster>-kafka-bootstrap:9092 --topic test-topic
    > Hello Strimzi
    > Test Message
    ```
  * **Consumer 실행 (다른 터미널):**
    ```bash
    kubectl run kafka-consumer -ti --image=quay.io/strimzi/kafka:latest-kafka-3.6.0 --rm=true --restart=Never -- bin/kafka-console-consumer.sh --bootstrap-server <my-cluster>-kafka-bootstrap:9092 --topic test-topic --from-beginning
    ```
  * **성공 기준:** Producer에서 입력한 메시지가 Consumer 창에 실시간으로 출력되어야 함.

-----

## 4\. 고급 기능 및 보안 테스트 (Advanced & Security)

운영 환경에 필수적인 외부 접속 및 인증, 모니터링을 검증합니다.

| ID | 테스트 항목 | 테스트 시나리오 | 예상 결과 | 확인 |
| :--- | :--- | :--- | :--- | :--- |
| **ADV-01** | **외부 접속 (NodePort/LB)** | 외부 리스너(External Listener) 설정 후 로컬 PC(또는 타 서버)에서 `kcat` 또는 `kafka-console-...`로 접속 시도. | 외부 IP/도메인을 통해 클러스터 접속 및 메시지 송수신이 가능해야 함. | [ ] |
| **ADV-02** | **사용자 인증 (User Operator)** | `KafkaUser` CR 생성 (SCRAM-SHA-512 또는 TLS). | 인증 정보 없이 접속 시도 시 거부되어야 하며, 생성된 Secret의 인증서/비번으로 접속 시 성공해야 함. | [ ] |
| **ADV-03** | **Kafka Exporter** | `kubectl get pods -l app.kubernetes.io/name=kafka-exporter` | Exporter 파드가 구동 중이어야 하며, Prometheus에서 `kafka_server_...` 메트릭이 수집되어야 함. | [ ] |
| **ADV-04** | **Rack Awareness** | (설정 시) 브로커 파드가 서로 다른 노드(또는 Zone)에 배치되었는지 확인. | `kubectl get pod -o wide` 확인 시 브로커들이 물리적으로 분산되어 있어야 함. | [ ] |

-----

## 5\. 장애 및 복구 테스트 (Resilience & Chaos)

데이터 유실 방지와 서비스 연속성을 검증하는 **가장 중요한 단계**입니다.

| ID | 테스트 항목 | 테스트 시나리오 | 예상 결과 | 확인 |
| :--- | :--- | :--- | :--- | :--- |
| **RES-01** | **브로커 강제 종료** | `kubectl delete pod <kafka-broker-0>` (Active Controller가 아닌 브로커 추천) | 1. StatefulSet에 의해 파드가 즉시 재시작(`Pending` -\> `Running`)되어야 함.<br>2. 재시작 중에도 클러스터 전체 서비스는 중단되지 않아야 함. | [ ] |
| **RES-02** | **데이터 지속성 (Persistence)** | 1. `test-persistence` 토픽에 메시지 저장.<br>2. 해당 파티션 리더 브로커 파드 삭제(`delete pod`).<br>3. 파드 복구 후 Consumer로 메시지 조회. | 파드가 재시작되어도 **메시지가 유실되지 않고** 그대로 조회되어야 함 (PVC 데이터 보존 확인). | [ ] |
| **RES-03** | **오퍼레이터 장애** | `kubectl delete pod <strimzi-cluster-operator>` | 오퍼레이터가 죽어도 **실행 중인 Kafka 클러스터는 아무 영향 없이** 정상 동작해야 함. (Control Plane과 Data Plane의 분리 검증). | [ ] |
| **RES-04** | **롤링 업데이트** | Kafka 설정 변경 (예: 리소스 Limit 변경) 후 `kubectl apply`. | Strimzi가 브로커를 **한 번에 한 대씩** 순차적으로 재시작하며, 이 과정에서 서비스 다운타임이 없어야 함. | [ ] |

-----

## 6\. 테스트 결과 요약 (Summary)

  * **테스트 수행자:** \_\_\_\_\_\_\_\_\_\_\_\_\_\_\_\_\_\_\_
  * **수행 일시:** \_\_\_\_\_\_\_\_\_\_\_\_\_\_\_\_\_\_\_
  * **성공 항목:** \_\_\_\_ / \_\_\_\_
  * **실패 항목:** \_\_\_\_\_\_\_\_\_\_\_\_\_\_\_\_\_\_\_ (Jira 티켓 번호: \_\_\_\_\_)
  * **최종 판정:** [ ] 운영 이관 가능 / [ ] 보완 필요

<!-- end list -->

```
```
