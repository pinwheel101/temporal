# 190노드 온프레미스 Kubernetes 성능 테스트 및 카오스 엔지니어링 종합 계획

대규모 온프레미스 Kubernetes 클러스터(5 control plane + 180 worker nodes)에서 Apache Spark, Trino, Kafka, Airflow 빅데이터 워크로드의 성능 검증과 SPOF(Single Point of Failure) 발견을 위해 **Chaos Mesh + kube-burner 조합**을 핵심 도구로 권장한다. Cilium CNI의 eBPF 기반 네이트워크 모드는 기존 iptables 기반 카오스 도구와 호환성 문제가 있으므로, 네트워크 카오스 테스트 시 `bpf.hostLegacyRouting=true` 설정이 필요하다. Dev 환경(25노드, BGP 없음)에서 **80%의 테스트 시나리오를 검증**한 후 Production(BGP 활성화)으로 점진적 확대하는 전략이 핵심이다.

-----

## 성능 테스트 도구 선정 및 구성

### Control Plane 성능 측정 도구

대규모 클러스터의 컨트롤 플레인 성능 검증에는 **kube-burner**와 **ClusterLoader2**를 병행 사용한다. kube-burner는 Red Hat이 개발한 도구로 커스텀 워크로드 정의와 Prometheus 메트릭 수집이 용이하며, ClusterLoader2는 Kubernetes SIG-Scalability의 공식 도구로 **K8s SLI/SLO 표준 검증**에 필수적이다.

**kube-burner 클러스터 밀도 테스트 실행**:

```bash
kube-burner ocp cluster-density-v2 \
  --iterations=100 \
  --es-server=https://elasticsearch:9200 \
  --uuid=$(uuidgen) \
  --local-indexing \
  --report-dir=/tmp/benchmark-results
```

이 테스트는 반복당 3개 nginx deployment, 5개 service, 10개 secret/configmap, 3개 network policy를 생성하여 API server와 etcd에 실제 프로덕션과 유사한 부하를 가한다. **190노드 클러스터에서 iterations=100~500** 범위로 테스트하고, API latency가 **p99 < 1초** 기준을 만족하는지 확인한다. 

**etcd 전용 벤치마크**는 stacked etcd 구성에서 특히 중요하다:

```bash
benchmark --endpoints=192.168.2.1:2379 \
  --target-leader --conns=100 --clients=1000 \
  put --key-size=8 --sequential-keys --total=100000 --val-size=256 \
  --cacert /etc/kubernetes/pki/etcd/ca.crt \
  --cert /etc/kubernetes/pki/etcd/peer.crt \
  --key /etc/kubernetes/pki/etcd/peer.key
```

NVMe SSD 환경에서 **fsync latency p99 < 10ms**가 etcd 안정성의 핵심 지표다.  이 값이 25ms를 초과하면 리더 선출 지연과 클러스터 불안정이 발생한다.

### 네트워크 성능 측정 (Cilium Native Routing)

Cilium 환경에서 네트워크 성능 측정은 내장 도구인 `cilium connectivity perf`를 우선 사용한다: 

```bash
kubectl label nodes worker1 perf-test=server
kubectl label nodes worker2 perf-test=client
cilium connectivity perf \
  --node-selector-client perf-test=client \
  --node-selector-server perf-test=server \
  --duration 300s
```

이 도구는 **same-node vs cross-node 성능**, TCP/UDP throughput, latency percentiles(P50/P90/P99)를 측정한다.  Native routing 모드에서는 VXLAN 대비 **약 30% 높은 throughput**과 full MTU 사용이 가능해야 한다. 

**중요**: Cilium의 eBPF datapath는 고성능 모드에서 iptables를 우회하므로,  Chaos Mesh나 tc 기반 네트워크 카오스 도구가 정상 작동하지 않는다. 네트워크 지연/손실 테스트를 위해서는:

- Helm 설치 시 `bpf.hostLegacyRouting=true` 설정 (성능 저하 감수)
- 또는 물리 네트워크 장비 ACL을 통한 인프라 레벨 파티션 테스트

### 스토리지 성능 측정 (NVMe hostPath)

etcd WAL 및 빅데이터 워크로드의 I/O 성능은 fio로 검증한다:

```bash
fio --rw=write --ioengine=sync --fdatasync=1 \
    --directory=/var/lib/etcd --size=100m --bs=2300 \
    --name=etcd-wal-test

# NVMe 전체 성능 측정
fio --name=nvme-test --filename=/dev/nvme0n1 \
    --filesize=100G --time_based --runtime=5m \
    --ioengine=libaio --direct=1 \
    --bs=4K --iodepth=64 --rw=randwrite --numjobs=16
```

**목표 성능 지표**:

|메트릭                   |목표값      |용도                          |
|----------------------|---------|----------------------------|
|Random Write IOPS (4K)|> 50,000 |etcd WAL                    |
|Random Read IOPS (4K) |> 100,000|etcd snapshot, Spark shuffle|
|Sequential Write BW   |> 1 GB/s |Kafka broker, Spark output  |
|fsync latency (p99)   |< 10ms   |etcd 안정성 핵심                 |

### 애플리케이션별 벤치마크 도구

**Apache Spark**: TPC-DS 벤치마크를 표준으로 사용한다. AWS EKS Spark Benchmark 또는 Databricks TPC-DS Kit을 활용하며, K8s vs YARN 성능은 **약 5% 이내 차이**로 K8s가 68%의 쿼리에서 우위를 보인다.  Spark shuffle 성능을 위해 `spark.local.dir`을 NVMe hostPath로 설정하면 **최대 60% 성능 향상**이 가능하다.

**Trino**: 내장 TPC-H 커넥터(`connector.name=tpch`)를 활용하며, sf1~sf1000 스케일 팩터로 22개 복잡 SQL 쿼리를 실행한다. 메모리 집약적 워크로드이므로 `query.max-memory-per-node`를 컨테이너 limit 기준으로 설정한다.

**Kafka**: 내장 벤치마크 도구를 사용한다:

```bash
kafka-producer-perf-test.sh --topic perf-test \
  --num-records 10000000 --record-size 1024 \
  --throughput -1 --producer-props bootstrap.servers=kafka:9092 acks=all
```

NVMe 기반 브로커에서 **약 650 MB/s per broker** throughput이 기대값이다.

**Airflow**: KubernetesExecutor 환경에서  DAG 파싱 시간(1000 DAG 기준 < 30초), scheduler loop time(< 2초), task scheduling latency(< 5초)를 측정한다. `worker_pods_creation_batch_size=16`으로 설정하여 동시 worker pod 생성 성능을 개선한다. 

-----

## 카오스 엔지니어링 도구 및 장애 시나리오

### 도구 선정: Chaos Mesh + LitmusChaos 조합

온프레미스 환경에서는 **Chaos Mesh를 주 도구**로, **LitmusChaos를 보조 도구**로 권장한다.

|도구              |강점                                                              |약점                     |
|----------------|----------------------------------------------------------------|-----------------------|
|**Chaos Mesh**  |포괄적 fault 유형(Pod, Network, IO, DNS, Stress), Web UI, Workflow 지원|Cilium eBPF 모드와 호환성 이슈 |
|**LitmusChaos** |ChaosHub 170+ 실험, Argo 연동, Resilience Score                     |학습 곡선 높음               |
|**Kraken(Krkn)**|Cerberus 헬스체크, 베어메탈 지원                                          |OpenShift 최적화          |

Chaos Mesh 설치:

```bash
helm repo add chaos-mesh https://charts.chaos-mesh.org
helm install chaos-mesh chaos-mesh/chaos-mesh \
  --namespace chaos-mesh --create-namespace \
  --set dashboard.securityMode=true \
  --set controllerManager.allowedNamespaces='{chaos-testing,staging}'
```

### Control Plane 장애 시나리오

**etcd 쿼럼 테스트** (5노드 stacked etcd 기준):

쿼럼 공식: `(n/2)+1 = 3`. 따라서 3노드 이상 생존 시 정상, 2노드 이하 시 read-only 모드로 전환된다.

```yaml
apiVersion: chaos-mesh.org/v1alpha1
kind: PodChaos
metadata:
  name: etcd-quorum-test
  namespace: chaos-mesh
spec:
  action: pod-kill
  mode: fixed
  value: '2'  # 2노드 손실 시 쿼럼 유지 확인
  selector:
    namespaces: [kube-system]
    labelSelectors:
      component: etcd
  duration: '120s'
```

**예상 결과 및 복구 기준**:

|노드 손실|남은 노드|클러스터 상태             |RTO     |
|-----|-----|--------------------|--------|
|1    |4    |정상, 약간의 latency 증가  |즉시      |
|2    |3    |쿼럼 유지, 리더 선출 발생 가능  |< 60초   |
|3    |2    |**Read-only**, 쓰기 불가|수동 복구 필요|

**Stacked etcd 특이사항**: etcd와 API server가 동일 노드에서 실행되므로, etcd pod kill보다 **노드 전체 장애 테스트**가 더 현실적이다. etcd fsync latency가 API server I/O와 경합하는 상황을 IOChaos로 시뮬레이션한다.

### Worker Node 및 네트워크 장애 시나리오

**특정 팀 노드 전체 장애 시뮬레이션**:

```yaml
apiVersion: chaos-mesh.org/v1alpha1
kind: PodChaos
metadata:
  name: team-bmc-node-failure
spec:
  action: pod-failure
  mode: all
  selector:
    namespaces: [bmc]
    labelSelectors:
      'app.kubernetes.io/team': 'bmc'
  duration: '300s'
```

**네트워크 파티션 (Pod 간 통신 단절)**:

```yaml
apiVersion: chaos-mesh.org/v1alpha1
kind: NetworkChaos
metadata:
  name: kafka-spark-partition
spec:
  action: partition
  mode: all
  selector:
    namespaces: [kafka]
    labelSelectors:
      app: kafka-broker
  direction: both
  target:
    selector:
      namespaces: [spark]
      labelSelectors:
        spark-role: executor
  duration: '180s'
```

**DNS 장애 주입**:

```yaml
apiVersion: chaos-mesh.org/v1alpha1
kind: DNSChaos
metadata:
  name: coredns-failure
spec:
  action: error
  mode: all
  selector:
    namespaces: [default, bmc, fdc]
  patterns:
    - '*.svc.cluster.local'
  duration: '60s'
```

### 애플리케이션별 장애 시나리오

**Kafka Broker 손실**:

```yaml
apiVersion: litmuschaos.io/v1alpha1
kind: ChaosEngine
metadata:
  name: kafka-broker-failure
  namespace: kafka
spec:
  appinfo:
    appns: kafka
    applabel: 'app=kafka'
    appkind: statefulset
  experiments:
    - name: kafka-broker-pod-failure
      spec:
        components:
          env:
            - name: TOTAL_CHAOS_DURATION
              value: '60'
            - name: KAFKA_REPLICATION_FACTOR
              value: '3'
        probe:
          - name: kafka-isr-check
            type: cmdProbe
            cmdProbe/inputs:
              command: 'kafka-topics.sh --describe --under-replicated-partitions'
              comparator:
                type: string
                criteria: equals
                value: ''
```

**예상 복구 시간**: 리더 브로커 손실 시 새 리더 선출 **약 10초**, ISR(In-Sync Replica) 복구는 복제 데이터 양에 따라 수분 소요.

**Spark Executor 장애**:

```yaml
apiVersion: chaos-mesh.org/v1alpha1
kind: PodChaos
metadata:
  name: spark-executor-kill
spec:
  action: pod-kill
  mode: fixed-percent
  value: '30'  # 30% executor 손실
  selector:
    labelSelectors:
      'spark-role': 'executor'
  duration: '30s'
```

Spark의 task 재실행 메커니즘이 정상 작동하는지 확인하고, `spark.dynamicAllocation.enabled=true` 설정 시 executor 자동 복구를 검증한다.

**Airflow Scheduler 장애**:

```yaml
apiVersion: chaos-mesh.org/v1alpha1
kind: PodChaos
metadata:
  name: airflow-scheduler-failure
spec:
  action: pod-kill
  mode: one
  selector:
    namespaces: [airflow]
    labelSelectors:
      component: scheduler
  duration: '120s'
```

Airflow 2.x의 Active/Active scheduler 구성에서 **< 30초 내 failover** 완료가 기대값이다.

### Steady State 정의 및 프로브 설정

**정상 상태 판단 기준**:

|컴포넌트                             |정상     |경고       |위험     |
|---------------------------------|-------|---------|-------|
|API Server Latency (p99)         |< 200ms|200-500ms|> 500ms|
|etcd fsync (p99)                 |< 10ms |10-50ms  |> 50ms |
|Pod Ready Ratio                  |> 99%  |95-99%   |< 95%  |
|Kafka Under-replicated Partitions|0      |1-5      |> 5    |
|Spark Job Failure Rate           |< 1%   |1-5%     |> 5%   |

**LitmusChaos Probe 설정 예시**:

```yaml
probes:
  - name: check-api-server-health
    type: httpProbe
    mode: Continuous
    httpProbe/inputs:
      url: 'https://kubernetes.default.svc/healthz'
      insecureSkipVerify: true
      method:
        get:
          criteria: '=='
          responseCode: '200'
    runProperties:
      probeTimeout: 30
      interval: 5
      stopOnFailure: true  # 실패 시 실험 자동 중단

  - name: check-error-rate-slo
    type: promProbe
    mode: Edge
    promProbe/inputs:
      endpoint: 'http://prometheus:9090'
      query: 'sum(rate(http_requests_total{status=~"5.."}[5m])) / sum(rate(http_requests_total[5m])) * 100'
      comparator:
        type: float
        criteria: '<='
        value: '1'  # Error rate <= 1%
```

-----

## 단계별 실행 계획 및 우선순위

### Dev 환경 (25노드, BGP 없음) 테스트 범위

**Phase 1: Critical Path (2-3주)**

|우선순위|테스트 항목                      |도구                      |Dev에서 가능|
|----|----------------------------|------------------------|--------|
|P1  |etcd fsync latency 벤치마크     |fio, etcd benchmark     |✅       |
|P1  |API Server response time    |kube-burner             |✅       |
|P1  |Pod 네트워크 기본 성능              |cilium connectivity test|✅       |
|P1  |단일 Pod/Container 장애 복구      |Chaos Mesh PodChaos     |✅       |
|P1  |Kafka broker 장애 & 리더 선출     |LitmusChaos             |✅       |
|P1  |Spark executor 장애 & task 재실행|Chaos Mesh              |✅       |

**Phase 2: Important (2-3주)**

|우선순위|테스트 항목                    |도구          |Dev에서 가능      |
|----|--------------------------|------------|--------------|
|P2  |etcd 1-2노드 손실 쿼럼 테스트      |Chaos Mesh  |✅ (시뮬레이션)     |
|P2  |네트워크 latency/loss 주입      |NetworkChaos|⚠️ Legacy 모드 필요|
|P2  |Storage I/O latency 주입    |IOChaos     |✅             |
|P2  |DNS 장애                    |DNSChaos    |✅             |
|P2  |Airflow scheduler failover|Chaos Mesh  |✅             |
|P2  |Trino worker 장애 mid-query |Chaos Mesh  |✅             |

**Phase 3: Nice-to-Have (Production 전 검증)**

|우선순위|테스트 항목                     |도구                    |Dev에서 가능|
|----|---------------------------|----------------------|--------|
|P3  |Node drain 시 workload 재스케줄링|kubectl drain + 모니터링  |✅       |
|P3  |다중 팀 동시 부하                 |kube-burner 병렬 실행     |✅       |
|P3  |Cross-namespace 의존성 장애     |NetworkChaos partition|⚠️       |
|P3  |eBPF map 고갈 시나리오           |수동 부하                 |✅       |

### Production (190노드, BGP 활성화) 전용 테스트

Dev에서 검증 불가능하며 Production에서만 테스트해야 하는 항목:

- **BGP 피어링 안정성**: `cilium bgp peers` 상태 모니터링, 세션 failover 시간 측정 
- **ECMP 경로 분산**: 업스트림 라우터에서 다중 next-hop 트래픽 분배 확인 
- **BGP Route 수렴 시간**: 노드 장애 시 route withdrawal → convergence 소요 시간 
- **Graceful Restart**: Cilium 업그레이드 중 BGP 세션 유지 검증 
- **대규모 동시 Pod 스케줄링**: 180 worker에서 수천 Pod 동시 생성

**Production 테스트 전 체크리스트**:

- [ ] Dev에서 동일 시나리오 3회 이상 성공
- [ ] Abort 조건 정상 작동 확인
- [ ] Runbook 문서화 완료
- [ ] 변경 승인 프로세스 완료
- [ ] Off-peak 시간대 스케줄링 (예: 새벽 2-5시)
- [ ] 롤백 절차 테스트 완료

### 자동화 방안

**Argo Workflows + LitmusChaos 연동**:

```yaml
apiVersion: argoproj.io/v1alpha1
kind: Workflow
metadata:
  name: weekly-chaos-suite
spec:
  entrypoint: chaos-pipeline
  schedules:
    - cron: "0 2 * * 0"  # 매주 일요일 새벽 2시
  templates:
    - name: chaos-pipeline
      steps:
        - - name: baseline-check
            template: prometheus-health-check
        - - name: kafka-chaos
            template: kafka-broker-failure
          - name: spark-chaos
            template: spark-executor-kill
        - - name: recovery-validation
            template: prometheus-health-check
        - - name: report-generation
            template: generate-report
```

**CI/CD 게이트로서의 카오스 테스트**:

- 새로운 애플리케이션 배포 시 기본 resilience 테스트 자동 실행
- P1 카오스 테스트 실패 시 배포 차단
- 결과를 Elasticsearch에 인덱싱하여 추적

-----

## 안전장치 및 롤백 계획

### Blast Radius 제어

**Namespace 화이트리스트 설정**:

```yaml
# Chaos Mesh Helm values
controllerManager:
  allowedNamespaces:
    - chaos-testing
    - bmc-staging
    - fdc-staging
  # kube-system, monitoring은 자동 제외
```

**Opt-in 라벨 기반 타게팅**:

```yaml
spec:
  selector:
    labelSelectors:
      'chaos.team.io/enabled': 'true'  # 명시적 라벨 필요
```

### 자동 중단 조건

**Prometheus 기반 중단 트리거**:

```yaml
probes:
  - name: abort-on-high-error-rate
    type: promProbe
    promProbe/inputs:
      query: 'sum(rate(http_requests_total{status=~"5.."}[1m]))'
      comparator:
        type: float
        criteria: '<='
        value: '100'  # 분당 100 에러 초과 시 중단
    runProperties:
      stopOnFailure: true
```

### 긴급 롤백 절차

```bash
#!/bin/bash
# chaos-emergency-stop.sh
NAMESPACES=$(kubectl get ns -o jsonpath='{.items[*].metadata.name}')
for ns in $NAMESPACES; do
  kubectl delete podchaos,networkchaos,iochaos,dnschaos,stresschaos \
    -n $ns --all --ignore-not-found=true
done
echo "모든 카오스 실험 종료됨"

# LitmusChaos 엔진 중단
kubectl patch chaosengine --all --type merge \
  -p '{"spec":{"engineState":"stop"}}'
```

-----

## 결과 분석 및 문서화 프레임워크

### 성능 Baseline 문서 템플릿

```markdown
# Performance Baseline: [서비스명]

## 측정 일시: YYYY-MM-DD
## 환경: Dev (25노드) / Production (190노드)

### Control Plane 메트릭
| 메트릭 | 측정값 | 목표값 | 상태 |
|--------|--------|--------|------|
| API latency (p99) | X ms | < 200ms | ✅/⚠️/❌ |
| etcd fsync (p99) | X ms | < 10ms | |
| Scheduler throughput | X pods/s | > 100 | |

### 네트워크 메트릭 (Cilium)
| 메트릭 | Same-node | Cross-node | 목표 |
|--------|-----------|------------|------|
| TCP throughput | X Gbps | X Gbps | > 20 Gbps |
| Latency (p99) | X μs | X μs | < 500μs |

### 애플리케이션별 메트릭
- Kafka throughput: X MB/s (목표: 650 MB/s/broker)
- Spark TPC-DS Q1 완료 시간: X초
- Trino TPC-H sf100 완료 시간: X분
- Airflow scheduler loop: X초 (목표: < 2초)
```

### SPOF 발견 문서 템플릿

```markdown
# SPOF 분석: [컴포넌트명]

## 발견 일시: YYYY-MM-DD
## 발견 실험: [Chaos Mesh/LitmusChaos 실험명]

### 장애 시나리오
- 실험 유형: [pod-kill/network-partition/io-delay]
- 대상: [namespace/label selector]
- 지속 시간: X초

### 영향 분석
| 영향 영역 | 심각도 | 영향받은 서비스 |
|-----------|--------|-----------------|
| 가용성 | High/Med/Low | [서비스 목록] |
| 데이터 무결성 | High/Med/Low | |
| 비즈니스 영향 | High/Med/Low | |

### Blast Radius
- 영향받은 팀: bmc, fdc, pluto, smartbig 중 [X]
- 복구 소요 시간: X분

### 근본 원인
[기술적 설명]

### 개선 권고사항
- 우선순위: Critical/High/Medium/Low
- 예상 작업량: S/M/L/XL
- 개선 방안: [구체적 조치]
```

### 개선 권고 우선순위 매트릭스

|심각도 \ 발생빈도  |낮음     |중간     |높음     |
|------------|-------|-------|-------|
|**Critical**|P1 (1주)|P1 (즉시)|P1 (즉시)|
|**High**    |P2 (2주)|P1 (1주)|P1 (즉시)|
|**Medium**  |P3 (분기)|P2 (월간)|P2 (2주)|
|**Low**     |Backlog|P3 (분기)|P3 (월간)|

-----

## 멀티테넌트 환경 테스트 전략

### 4개 팀 Namespace 격리 테스트

각 팀(bmc, fdc, pluto, smartbig)별로 **독립적인 ServiceAccount**와 **RBAC 정책**을 생성하여 카오스 실험 범위를 제한한다:

```yaml
apiVersion: v1
kind: ServiceAccount
metadata:
  name: chaos-bmc-sa
  namespace: bmc
---
apiVersion: rbac.authorization.k8s.io/v1
kind: Role
metadata:
  name: chaos-bmc-role
  namespace: bmc
rules:
  - apiGroups: ["chaos-mesh.org"]
    resources: ["*"]
    verbs: ["*"]
  - apiGroups: [""]
    resources: ["pods"]
    verbs: ["get", "list", "delete"]  # 자기 namespace만
```

### 팀 간 의존성 장애 테스트

팀 간 서비스 의존성(예: pluto의 Spark가 fdc의 Kafka 사용)이 있는 경우:

1. 의존성 맵핑 문서화
1. Upstream 팀 서비스 장애 시 Downstream 팀의 graceful degradation 검증
1. Timeout 및 retry 설정 적정성 확인
1. Circuit breaker 패턴 적용 여부 점검

### 리소스 경합 테스트

빅데이터 워크로드의 burst 특성을 고려한 테스트:

```yaml
apiVersion: v1
kind: ResourceQuota
metadata:
  name: spark-quota
  namespace: spark
spec:
  hard:
    requests.cpu: "500"     # 180 worker × 48 core 중 할당량
    requests.memory: 2Ti
    limits.cpu: "750"
    limits.memory: 3Ti
```

**PriorityClass 기반 우선순위 설정**:

```yaml
apiVersion: scheduling.k8s.io/v1
kind: PriorityClass
metadata:
  name: kafka-critical
value: 1000000
---
apiVersion: scheduling.k8s.io/v1
kind: PriorityClass
metadata:
  name: spark-batch
value: 100000  # Kafka보다 낮은 우선순위
```

-----

## 핵심 권장사항 요약

1. **도구 조합**: Chaos Mesh(주) + LitmusChaos(보조) + kube-burner(성능)
1. **Cilium 주의**: eBPF 고성능 모드에서 네트워크 카오스 테스트 시 `bpf.hostLegacyRouting=true` 필요
1. **Dev 우선**: 80% 시나리오를 Dev에서 검증 후 Production 확대
1. **BGP 테스트**: Production에서만 가능한 BGP failover, ECMP 분산은 별도 윈도우 확보
1. **자동 중단**: Prometheus 프로브로 error rate > 5% 시 즉시 실험 중단
1. **팀 격리**: Namespace별 RBAC, opt-in 라벨로 blast radius 제한
1. **문서화**: 모든 SPOF 발견 시 즉시 문서화 및 우선순위 매트릭스로 분류
1. **점진적 확대**: Phase 1(Critical) → Phase 2(Important) → Phase 3(Nice-to-have) 순서 준수

이 계획을 기반으로 **4-6주 내 Dev 환경 검증 완료**, **8-12주 내 Production 기본 카오스 테스트 체계 구축**이 가능하다.