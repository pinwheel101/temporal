아키텍처 상세 설계 문서

>>>

좋습니다! 1단계부터 시작하겠습니다.

# 1단계: 아키텍처 상세 설계 문서

## 1.1 클러스터 전체 아키텍처

### 노드 구성 계획

**전체 노드: 191대 + α**

```
Control Plane 노드: 5대 (HA 구성)
├─ etcd 전용
├─ API Server, Controller Manager, Scheduler
└─ Taint 설정으로 일반 워크로드 배치 방지

Worker 노드: 186대 (191 - 5)
├─ 일반 워크로드 실행
└─ 팀별 리소스 할당

Infra 노드: 추가 5-10대 권장
├─ 모니터링 (Prometheus, Grafana)
├─ 로깅 (OpenSearch)
├─ Ingress Controller (Cilium Ingress)
├─ 레지스트리 (선택사항)
└─ 공통 시스템 (ArgoCD, Jenkins 등)
```

**권장사항:** 총 201대 구성 (Control Plane 5 + Worker 186 + Infra 10)
- 현재 191대만 있다면: Control Plane 3대 + Worker 183대 + Infra 5대로 조정 가능

### 노드 역할별 상세

#### Control Plane 노드 (5대)
```yaml
역할: Kubernetes Control Plane 컴포넌트 실행
리소스 할당:
  - CPU: 8-16 코어 (전체 48코어 중)
  - Memory: 32-64GB (전체 768GB 중)
  
컴포넌트:
  - etcd (클러스터 데이터 저장)
  - kube-apiserver
  - kube-controller-manager
  - kube-scheduler
  
디스크:
  - NVMe SSD #1: OS + etcd 데이터 (고성능 필수)
  - NVMe SSD #2: 백업 및 로그
  
네트워크:
  - 25GbE 사용 권장
  - LoadBalancer로 API 서버 고가용성 확보
```

#### Worker 노드 (186대)
```yaml
역할: 애플리케이션 워크로드 실행
리소스:
  - CPU: 전체 48코어 할당 가능
  - Memory: 전체 768GB 할당 가능
  
팀별 할당 (4개 팀):
  - 각 팀: 약 46-47대씩 할당 가능
  - 총 CPU: 약 2,208코어/팀 (46 x 48)
  - 총 Memory: 약 35TB/팀 (46 x 768GB)
  
디스크 활용:
  - NVMe SSD #1: OS + Container Runtime (100GB) + Local PV (나머지)
  - NVMe SSD #2: Local PV 또는 분산 스토리지
  
Label 전략:
  - team: team-a, team-b, team-c, team-d
  - node-role: worker
  - workload-type: compute, memory, gpu (향후)
```

#### Infra 노드 (5-10대)
```yaml
역할: 공통 인프라 서비스
워크로드:
  - Monitoring Stack (Prometheus, Grafana, AlertManager)
  - Logging Stack (OpenSearch, Fluentd/Fluent-bit)
  - Cilium Ingress Controller
  - ArgoCD
  - Jenkins
  - Internal Registry (선택)
  
특징:
  - 일반 워크로드와 격리
  - Taint/Toleration 사용
  - 높은 가용성 필요
```

---

## 1.2 네트워크 아키텍처

### IP 대역 설계

```yaml
# 191대 노드 + 대규모 Pod 고려
Host Network: 
  - 기존 네트워크 대역 사용 (확인 필요)
  - 예시: 10.0.0.0/16 또는 172.16.0.0/16

Pod CIDR:
  - 권장: 10.244.0.0/12 (1,048,576개 IP)
  - 노드당: /24 (254개 Pod/노드)
  - 계산: 191 노드 x 250 Pod = ~47,750 Pod 수용 가능
  
Service CIDR:
  - 권장: 10.96.0.0/12 (1,048,576개 Service IP)
  - Cluster IP 범위
  
참고: Cilium을 CNI로 사용하면 더 효율적인 IPAM 가능
```

### CNI 및 네트워크 플러그인

```yaml
CNI: Cilium
버전: 1.14+ 권장

선택 이유:
  - eBPF 기반 고성능
  - NetworkPolicy 강력한 지원 (팀 간 격리)
  - Ingress Controller 통합
  - Hubble로 네트워크 가시성
  - 25GbE 네트워크 활용에 최적

설정:
  - Native Routing Mode 사용 (성능 최적화)
  - Bandwidth Manager 활성화
  - Hubble 활성화 (네트워크 모니터링)
```

### 물리 네트워크 구성

```yaml
NIC 구성 (2-port 10/25GbE):
  
옵션 1: Active-Backup Bonding
  - bond0: 두 포트 묶어서 HA
  - 간단하지만 대역폭 활용 50%
  
옵션 2: 용도별 분리 (권장)
  - eth0 (25GbE): Pod 네트워크 전용
  - eth1 (25GbE): Node 관리 + Storage 트래픽
  - 최대 성능 활용
  
옵션 3: LACP Bonding
  - bond0: Active-Active로 50Gbps 대역폭
  - 스위치 LACP 지원 필요
```

### Ingress 전략

```yaml
Ingress Controller: Cilium Ingress
배치: Infra 노드에 DaemonSet 또는 Deployment

외부 접근:
  - External LoadBalancer 필요 (F5, HAProxy, MetalLB 등)
  - NodePort 범위: 30000-32767
  - 또는 LoadBalancer Service Type 사용
  
TLS:
  - cert-manager로 인증서 자동 관리
  - Let's Encrypt 또는 내부 CA

팀별 Ingress:
  - IngressClass로 분리 가능
  - 또는 팀별 Ingress 객체로 관리
```

### 외부 시스템 연결

```yaml
MinIO + Iceberg 클러스터:
  - S3 API 엔드포인트로 접근
  - NetworkPolicy로 필요한 Pod만 접근 허용
  - Service Entry (Istio) 또는 ExternalName Service
  
Keycloak:
  - OIDC Provider로 설정
  - kubectl 인증 통합
  - 애플리케이션 레벨 인증
```

---

## 1.3 스토리지 아키텍처

### 디스크 활용 전략

```yaml
각 노드: NVMe SSD 3.84TB x 2

옵션 1: OS + Local PV 분리 (권장)
  /dev/nvme0n1 (3.84TB):
    - 100GB: OS (/)
    - 100GB: Container Runtime (/var/lib/containerd)
    - 3.64TB: Local PV (고성능 워크로드용)
  
  /dev/nvme0n2 (3.84TB):
    - 3.84TB: Local PV 또는 분산 스토리지

옵션 2: 분산 스토리지 구성
  - Rook-Ceph 사용
  - 모든 NVMe를 Ceph OSD로 활용
  - 186대 x 2 = 372 OSDs
  - Replica 3 기준: 약 720TB 가용 용량
```

### StorageClass 전략

```yaml
1. local-nvme (고성능)
   - Local PV 사용
   - 노드 지역성 있음 (Pod가 특정 노드에 고정)
   - Spark, Trino 임시 데이터용
   - IOPS: ~500K+

2. ceph-block (선택사항)
   - Rook-Ceph RBD
   - 복제본 3
   - 일반 PVC용

3. nfs (선택사항)
   - 공유 데이터용
   - ReadWriteMany 지원

4. external-s3 (주요 데이터)
   - MinIO + Iceberg
   - CSI driver 또는 애플리케이션 직접 접근
```

### 데이터 경로

```yaml
Spark/Trino 실행 흐름:
  1. Input Data: MinIO (S3) + Iceberg
  2. Shuffle/Temp: Local NVMe SSD
  3. Output Data: MinIO (S3) + Iceberg
  
장점:
  - 고속 로컬 디스크로 성능 최적화
  - 영구 데이터는 외부 MinIO에 안전하게 보관
```

---

## 1.4 리소스 관리 및 멀티테넌시

### 팀별 Namespace 구조

```yaml
팀별 Namespace:
  team-a-ns:
    - workload pods
    - airflow
    - trino
    - cnpg (PostgreSQL)
  
  team-b-ns:
    - (동일 구조)
  
  team-c-ns:
    - (동일 구조)
  
  team-d-ns:
    - (동일 구조)

공통 Namespace:
  spark-operator-system:
    - Spark Operator
  
  argocd:
    - ArgoCD
  
  ci-cd:
    - Jenkins
  
  monitoring:
    - Prometheus, Grafana
  
  logging:
    - OpenSearch, Fluent-bit
  
  ingress-system:
    - Cilium Ingress
```

### ResourceQuota 설계

```yaml
각 팀당 할당 (186 Worker / 4팀 = 46-47노드):

team-a-quota:
  requests:
    cpu: "2000"        # 2000 코어 (약 42노드 분량)
    memory: "30Ti"     # 30TB
  limits:
    cpu: "2200"        # 버스트 허용
    memory: "33Ti"
  pods: "10000"
  persistentvolumeclaims: "500"
  
버퍼:
  - 10-20% 리소스를 버퍼로 유지
  - 버스트 워크로드 대응
```

### LimitRange 설정

```yaml
각 팀 Namespace에 적용:

limitrange:
  Pod:
    min:
      cpu: "100m"
      memory: "128Mi"
    max:
      cpu: "96"        # 노드 전체 CPU 2개
      memory: "700Gi"   # 노드 전체 메모리의 약 90%
    default:
      cpu: "1"
      memory: "2Gi"
    defaultRequest:
      cpu: "500m"
      memory: "1Gi"
  
  Container:
    min:
      cpu: "10m"
      memory: "64Mi"
    max:
      cpu: "48"
      memory: "384Gi"
```

### PodSecurityStandard

```yaml
팀별 Namespace: restricted
  - 대부분의 보안 기능 강제
  - 루트 실행 금지
  - 권한 상승 금지
  
공통 Namespace: privileged (필요시)
  - 시스템 컴포넌트용
  - 모니터링, 로깅 등
```

### NetworkPolicy

```yaml
기본 정책: Deny All

팀별 허용:
  team-a-ns:
    - 같은 namespace 내 통신 허용
    - 외부 MinIO 접근 허용
    - 외부 Keycloak 접근 허용
    - DNS 접근 허용
    - 타 팀 namespace 차단
  
공통 시스템:
  - 모든 namespace에서 접근 가능
  - Spark Operator, ArgoCD 등
```

---

## 1.5 스케줄링 전략

### 기본 스케줄러 vs YuniKorn

```yaml
현재 상황 분석:
  - 4개 팀, ResourceQuota로 엄격한 리소스 관리
  - Spark 같은 Gang Scheduling 필요 워크로드 존재
  - 초기 안정화가 중요
  
권장 접근:
  Phase 1 (초기):
    - 기본 Kubernetes 스케줄러 사용
    - ResourceQuota + LimitRange로 리소스 관리
    - 단순하고 안정적
  
  Phase 2 (필요시):
    - YuniKorn 도입
    - Gang Scheduling 활성화
    - Queue 기반 리소스 관리 (ResourceQuota와 병행)
```

### Node Affinity 및 Taints

```yaml
팀별 노드 할당 (선택사항):
  - Label: team=team-a
  - NodeSelector 또는 NodeAffinity로 팀별 노드 지정
  - 물리적 격리 효과
  
장단점:
  장점: 완전한 격리, 예측 가능한 성능
  단점: 리소스 활용률 저하, 유연성 감소
  
권장: 초기에는 Label 없이 시작, 필요시 단계적 도입
```

### Priority Classes

```yaml
system-critical:
  value: 2000000000
  description: 시스템 Pod (모니터링, 로깅)
  
high-priority:
  value: 1000000
  description: 프로덕션 워크로드
  
default-priority:
  value: 0
  description: 일반 워크로드
  
low-priority:
  value: -1000
  description: Batch 작업, 개발/테스트
```

---

## 1.6 보안 아키텍처

### 인증 (Authentication)

```yaml
Keycloak OIDC 통합:
  1. Kubernetes API Server:
     - OIDC Provider로 Keycloak 설정
     - kubectl 인증
  
  2. kubeconfig 생성:
     - kubectl oidc-login 플러그인 사용
     - 자동 토큰 갱신
  
  3. 애플리케이션 인증:
     - OAuth2 Proxy
     - 각 애플리케이션별 OIDC 클라이언트
```

### 인가 (Authorization)

```yaml
RBAC 구조:

ClusterRole (클러스터 전체):
  - cluster-admin: 전체 관리자
  - cluster-viewer: 읽기 전용
  
Role (Namespace 별):
  - team-admin: 팀 Namespace 전체 권한
  - team-developer: 배포, 읽기
  - team-viewer: 읽기 전용

RoleBinding:
  - Keycloak 그룹과 매핑
  - group: /team-a/admins -> team-admin Role
  - group: /team-a/developers -> team-developer Role
```

### Secret 관리

```yaml
옵션 1: External Secrets Operator (권장)
  - Keycloak이 있는 클러스터에 Vault 배포
  - ESO로 자동 sync
  
옵션 2: Sealed Secrets
  - GitOps 친화적
  - 암호화된 Secret을 Git에 저장
  
옵션 3: 기본 Secret + RBAC
  - 간단하지만 보안 약함
  - 최소한 etcd 암호화 활성화 필수
```

### Image 보안

```yaml
이미지 정책:
  1. Private Registry 사용 (Harbor 권장)
     - 이미지 스캔 (Trivy)
     - 취약점 자동 검사
  
  2. ImagePullPolicy: Always 또는 IfNotPresent
  
  3. Admission Controller:
     - 특정 레지스트리만 허용
     - 서명된 이미지만 허용 (Cosign)
```

---

## 1.7 모니터링 및 로깅 아키텍처

### 모니터링 스택

```yaml
Prometheus Stack:
  배치: Infra 노드
  구성:
    - Prometheus Operator
    - Prometheus (HA: 2 replicas)
    - Alertmanager (HA: 3 replicas)
    - Grafana
  
  메트릭 수집:
    - Node Exporter (모든 노드)
    - kube-state-metrics
    - cAdvisor (kubelet 내장)
    - 애플리케이션 메트릭 (ServiceMonitor)
  
  스토리지:
    - Retention: 30일
    - 용량: 약 500GB-1TB (예상)
    - PVC 사용 또는 Local PV
  
  대시보드:
    - 클러스터 전체
    - 노드별
    - 팀별 (Namespace)
    - 워크로드별 (Spark, Trino, Airflow)
```

### 로깅 스택

```yaml
OpenSearch:
  배치: Infra 노드
  구성:
    - OpenSearch Cluster (3 masters, 3+ data nodes)
    - OpenSearch Dashboards
  
로그 수집:
  - Fluent-bit (DaemonSet, 모든 노드)
  - 또는 Fluentd
  
로그 종류:
  - Container logs (stdout/stderr)
  - System logs (kubelet, containerd)
  - Application logs
  - Audit logs
  
인덱스 전략:
  - 팀별 인덱스 분리
  - 일별 인덱스 롤오버
  - Retention: 90일
  - ILM (Index Lifecycle Management)
```

### 분산 추적 (선택사항)

```yaml
Jaeger 또는 Tempo:
  - Cilium Hubble과 통합 가능
  - Spark, Trino 작업 추적
  - 나중 Phase에 도입 검토
```

---

## 1.8 백업 및 재해복구

### etcd 백업

```yaml
자동 백업:
  - 주기: 매일, 매주
  - 보관: 30일치
  - 저장소: 외부 스토리지 (NFS, S3)
  
방법:
  - etcdctl snapshot save
  - Cron Job으로 자동화
  
복구 절차:
  - 문서화 필수
  - 정기적 복구 테스트 (연 2회)
```

### 애플리케이션 백업

```yaml
Velero:
  - Namespace 단위 백업
  - PVC 백업 (Restic 사용)
  - 주기: 주 1회 또는 변경 시
  - 백업 저장소: MinIO 또는 S3
  
백업 대상:
  - 공통 시스템 (ArgoCD, Jenkins 설정)
  - 팀별 중요 워크로드
  - ConfigMap, Secret
```

### 재해복구 계획

```yaml
RTO (Recovery Time Objective): 4시간
RPO (Recovery Point Objective): 24시간

시나리오:
  1. etcd 장애: Snapshot에서 복구
  2. Control Plane 장애: HA로 자동 복구
  3. 전체 클러스터 장애: 새 클러스터 구축 + Velero 복구
  
문서화:
  - Runbook 작성
  - 복구 절차 상세 기록
```

---

## 1.9 CI/CD 전략

### ArgoCD 구조

```yaml
배치: 공통 Namespace (argocd)
구성:
  - HA 모드 (3 replicas)
  - SSO with Keycloak
  
사용 방법:
  - Git Repository per Team
  - ApplicationSet으로 팀별 애플리케이션 관리
  - Sync Policy: Manual 또는 Auto (팀 선택)
  
팀별 프로젝트:
  - team-a-project: team-a-ns 배포 권한
  - RBAC로 권한 제한
```

### Jenkins 구조

```yaml
배치: 공통 Namespace (ci-cd)
구성:
  - Jenkins Master: Persistent Storage 사용
  - Jenkins Agent: Kubernetes Plugin (동적 Pod)
  
팀별 사용:
  - 각 팀별 Folder/View
  - 팀별 Credential
  - Agent Pod는 팀 Namespace에 생성 가능
```

### GitOps 워크플로우

```
Developer -> Git Push -> Jenkins Build -> 
  Container Registry -> Git Commit (manifests) -> 
  ArgoCD Sync -> Kubernetes Deployment
```

---

## 1.10 업그레이드 전략

### Kubernetes 버전 관리

```yaml
초기 버전: 1.28 또는 1.29 (안정 버전)
업그레이드 주기: 6개월 (2 minor 버전 뒤)

업그레이드 절차:
  1. 테스트 클러스터에서 검증
  2. Control Plane 먼저 (5대 순차)
  3. Worker 노드 순차 (팀별 순환)
  4. 중요 애플리케이션 검증
  
롤백 계획:
  - etcd snapshot 사전 백업
  - 노드별 순차 업그레이드로 리스크 최소화
  - 실패 시 이전 버전 재설치
```

### Kubespray 업그레이드

```yaml
장점:
  - 멱등성 (여러 번 실행 가능)
  - 순차 업그레이드 (Drain -> Upgrade -> Uncordon)
  - 설정 파일로 버전 관리
  
주의사항:
  - 191대 실행 시간: 수 시간 소요
  - 노드별 실패 처리
  - 업그레이드 전 Validation 필수
```

---

이상으로 **1단계: 아키텍처 상세 설계 문서**를 완료했습니다.

### 확인이 필요한 사항:
1. OS 및 Kubernetes 버전 선택
2. NIC 2-port를 어떻게 활용할지 (Bonding vs 분리)
3. 노드 구성 (Control Plane 3대 vs 5대, Infra 노드 별도 확보 여부)
4. Local PV vs 분산 스토리지 (Ceph) 선택
5. 팀별 노드 물리적 할당 여부

위 사항들을 결정하신 후, **2단계: 설치 계획 및 체크리스트**로 진행하시겠습니까?

>>>

1. OS는 redhat 10 버전이야 쿠버네티스 버전은 추천해줘
2. NIC도 최적 성능을 위해 추천해줘
3. 191대는 고정이고 노드 숫자를 늘리는 것은 매우 어려워
4. 서비스에 맞춰 모두 운영해야 될것 같은데
5. 요구사항에 맞춰 팀별 노드를 가르는 것이 좋을지 아니면 같이 써도 좋을지 알려줘

>>>

좋습니다! 구체적으로 답변드리겠습니다.

## 결정사항 및 권장사항

### 1. OS 및 Kubernetes 버전

```yaml
OS: Red Hat Enterprise Linux 10
- 출시: 2024년 (매우 최신)
- 장기 지원 기대

Kubernetes 버전 권장:
  첫 번째 선택: v1.29.x (현재 시점)
    - 이유: Stable하고 RHEL 10과 검증된 호환성
    - 지원 기간: 약 1년
    - 프로덕션 안정성 높음
  
  두 번째 선택: v1.30.x
    - 더 최신 기능
    - RHEL 10에서 충분히 테스트 필요
  
  피해야 할 버전: v1.31+ (너무 최신, 프로덕션 리스크)

Container Runtime:
  - containerd v1.7.x (권장)
  - CRI-O도 RHEL에서 잘 지원되지만 containerd가 더 범용적

최종 권장: Kubernetes v1.29.8 + containerd v1.7.13
```

**중요:** Kubespray에서 RHEL 10 지원 여부를 반드시 확인해야 합니다.
- Kubespray가 RHEL 9는 지원하지만 10은 아직 공식 지원 안 할 수 있음
- 필요시 Rocky Linux 9 고려 (RHEL 호환)

---

### 2. NIC 구성 권장 (최적 성능)

```yaml
물리 구성: 10/25GbE 2-port Broadcom

권장: 용도별 분리 (최고 성능)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

eth0 (25GbE): Pod/Container 네트워크 전용
├─ Cilium CNI 트래픽
├─ Pod-to-Pod 통신
├─ Service 트래픽
├─ MinIO 데이터 전송 (S3 API)
└─ MTU: 9000 (Jumbo Frame 활성화)

eth1 (25GbE): 관리 및 스토리지 네트워크
├─ SSH 접근
├─ Kubernetes API 통신
├─ 모니터링 메트릭 수집
├─ 로그 전송
├─ 로컬 스토리지 복제 (Ceph 사용 시)
└─ MTU: 1500 (표준)

총 대역폭: 50Gbps (양방향 독립 사용)
```

**설정 예시:**
```bash
# /etc/sysconfig/network-scripts/ifcfg-eth0
DEVICE=eth0
BOOTPROTO=static
IPADDR=10.100.x.x      # Pod 네트워크용 IP 대역
MTU=9000
ONBOOT=yes

# /etc/sysconfig/network-scripts/ifcfg-eth1  
DEVICE=eth1
BOOTPROTO=static
IPADDR=10.0.x.x        # 관리 네트워크 IP 대역
MTU=1500
ONBOOT=yes
```

**Cilium 설정:**
```yaml
# Cilium에서 eth0 지정
devices: eth0
datapathMode: native
```

**성능 최적화:**
```bash
# IRQ 밸런싱
systemctl disable irqbalance

# NIC 큐 설정
ethtool -L eth0 combined 16
ethtool -L eth1 combined 8

# Ring buffer 증가
ethtool -G eth0 rx 4096 tx 4096
```

**장점:**
- 최대 대역폭 활용 (50Gbps)
- 트래픽 타입별 격리로 성능 예측 가능
- 관리 네트워크 장애가 데이터 트래픽에 영향 없음

**대안 (권장하지 않음):**
```yaml
Bonding mode=4 (LACP):
  - 이론상 50Gbps 대역폭
  - 스위치 LACP 설정 필요
  - 복잡도 증가
  - 단일 실패 지점
```

---

### 3. 191대 고정 노드 구성

```yaml
총 노드: 191대 (증설 불가)

최적 구성:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Control Plane: 5대
├─ 홀수 개 필요 (etcd quorum)
├─ 5대가 3대보다 안정적 (장애 허용 2대)
├─ API 부하 분산
└─ 역할: etcd, api-server, controller, scheduler

Worker 노드: 183대
├─ 애플리케이션 워크로드 실행
├─ 4개 팀 사용
└─ 팀당 약 45-46대 활용 가능

Infra 노드: 3대 (Worker 중 Label 지정)
├─ Worker와 분리하지 않고 Label + Taint로 구분
├─ 모니터링: Prometheus, Grafana
├─ 로깅: OpenSearch
├─ Ingress: Cilium Ingress
├─ 공통 시스템: ArgoCD, Jenkins
└─ 고가용성 확보 (3대 배치)

실제 Worker: 180대 (183 - 3 Infra)
```

**노드 역할 할당:**
```yaml
# Control Plane 노드 (5대)
node001-005:
  role: control-plane
  taints:
    - node-role.kubernetes.io/control-plane:NoSchedule
  
# Infra 노드 (3대, Worker 중 선택)
node006-008:
  role: infra
  labels:
    - node-role.kubernetes.io/infra: "true"
  taints:
    - node-role.kubernetes.io/infra:NoSchedule
  toleration: infra 워크로드만 배치

# Worker 노드 (183대)
node009-191:
  role: worker
  labels:
    - node-role.kubernetes.io/worker: "true"
  # Taint 없음 - 일반 워크로드 배치
```

**리소스 계산:**
```
Worker 180대 × 48 코어 = 8,640 코어
Worker 180대 × 768GB = 138TB 메모리

팀당 할당 (4팀):
  - CPU: 2,160 코어/팀
  - Memory: 34.5TB/팀
  - 노드: 약 45대/팀 활용
```

---

### 4. 스토리지 전략 (모두 운영)

```yaml
하이브리드 스토리지 아키텍처
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

1. Local NVMe PV (고성능)
   용도:
     - Spark Shuffle 데이터
     - Trino 임시 캐시
     - 단기 캐시/스크래치 스페이스
   
   구성:
     - /dev/nvme0n1 일부 (3TB)
     - 각 Worker 노드에 설치
     - StorageClass: local-nvme
     - 노드 지역성 (Pod가 노드에 고정)
   
   성능: ~500K IOPS, ~6GB/s 처리량

2. Rook-Ceph (분산 스토리지)
   용도:
     - 애플리케이션 PVC (PostgreSQL, Airflow DB 등)
     - 공통 시스템 영구 데이터
     - 복제가 필요한 데이터
   
   구성:
     - /dev/nvme0n2 전체 (3.84TB) → Ceph OSD
     - 183 Worker 노드 활용
     - Replica: 3
     - 가용 용량: (183 × 3.84TB) / 3 = 약 234TB
   
   성능: ~100K IOPS (복제 오버헤드), 고가용성

3. External MinIO (주 데이터 레이크)
   용도:
     - Iceberg 테이블 데이터
     - Spark/Trino 입출력 데이터
     - 장기 보관 데이터
   
   구성:
     - 별도 클러스터 (50-100대)
     - S3 API로 접근
     - 애플리케이션 레벨 통합
```

**StorageClass 정의:**
```yaml
# 1. Local NVMe
apiVersion: storage.k8s.io/v1
kind: StorageClass
metadata:
  name: local-nvme
provisioner: kubernetes.io/no-provisioner
volumeBindingMode: WaitForFirstConsumer
---
# 2. Ceph RBD
apiVersion: storage.k8s.io/v1
kind: StorageClass
metadata:
  name: ceph-block
provisioner: rook-ceph.rbd.csi.ceph.com
parameters:
  pool: replicapool
  replication: "3"
reclaimPolicy: Delete
allowVolumeExpansion: true
---
# 3. Ceph FS (ReadWriteMany)
apiVersion: storage.k8s.io/v1
kind: StorageClass
metadata:
  name: ceph-filesystem
provisioner: rook-ceph.cephfs.csi.ceph.com
parameters:
  fsName: myfs
  pool: myfs-replicated
reclaimPolicy: Delete
allowVolumeExpansion: true
```

**디스크 레이아웃 (각 Worker 노드):**
```
/dev/nvme0n1 (3.84TB):
├─ 50GB:  / (Root FS)
├─ 100GB: /var/lib/containerd
├─ 50GB:  /var/log
├─ 3.64TB: Local PV (/mnt/local-storage)

/dev/nvme0n2 (3.84TB):
└─ 3.84TB: Ceph OSD (raw device)
```

**사용 시나리오:**
```yaml
Spark Job:
  Input: MinIO (S3) → 대용량 데이터
  Shuffle: Local NVMe → 초고속 임시 저장
  Output: MinIO (S3) → 영구 저장

Airflow:
  Database: Ceph Block PV → 고가용성
  Logs: Ceph FS PV → 공유 스토리지

PostgreSQL (CNPG):
  Data: Ceph Block PV → 복제 + 고가용성
  WAL: Local NVMe (성능) 또는 Ceph (안정성)

Trino:
  Metadata: MinIO (Iceberg)
  Cache: Local NVMe
  Spill: Local NVMe
```

---

### 5. 팀별 노드 분리 vs 공유

```yaml
결론: 노드 공유 방식 권장 (Soft Multi-tenancy)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

이유:
1. 리소스 효율성
   - 180대를 4팀이 유연하게 사용
   - 팀별 워크로드 패턴이 다를 수 있음
   - 한 팀이 유휴 시 다른 팀이 활용 가능
   - 전체 클러스터 활용률 향상

2. 비용 대비 효과
   - 노드 추가 불가 상황
   - 물리적 분리는 낭비 발생
   - 예: Team A 30대, Team B 60대 필요 시 유연한 할당

3. 운영 복잡도
   - 단일 클러스터로 관리
   - 업그레이드, 패치 한 번에 처리
   - 모니터링/로깅 통합

4. Kubernetes 멀티테넌시 기능 충분
   - ResourceQuota: 리소스 상한 보장
   - NetworkPolicy: 네트워크 격리
   - RBAC: 권한 분리
   - PodSecurityStandard: 보안 격리
```

**Soft Multi-tenancy 구현:**

```yaml
팀별 격리 메커니즘:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

1. Namespace 분리 ✓
   - team-a, team-b, team-c, team-d
   - 완전 독립적인 리소스 범위

2. ResourceQuota (엄격) ✓
   team-a-quota:
     requests.cpu: "2000"
     requests.memory: "32Ti"
     limits.cpu: "2200"    # 10% 버스트
     limits.memory: "35Ti"
   
   → 각 팀은 할당된 리소스 초과 불가

3. LimitRange (기본값 강제) ✓
   - request/limit 없는 Pod 차단
   - 최소/최대/기본값 설정

4. NetworkPolicy (네트워크 격리) ✓
   - 팀 간 Pod 통신 차단
   - 외부 MinIO, Keycloak만 허용
   - DNS 허용

5. RBAC (권한 격리) ✓
   - 팀별 관리자는 자기 팀 Namespace만 접근
   - 다른 팀 리소스 조회/수정 불가

6. PodSecurityStandard ✓
   - 보안 정책 강제
   - 권한 상승 방지

7. PriorityClass (선택) ✓
   - 팀별 우선순위 동일 설정
   - 또는 비즈니스 중요도에 따라 차등
```

**노드 레이블 전략 (물리 분리 없음):**
```yaml
노드에 팀별 Label 없음:
  node-role.kubernetes.io/worker: "true"
  topology.kubernetes.io/zone: zone-a  # 물리 위치만

Pod에 NodeSelector/Affinity 없음:
  - 스케줄러가 자유롭게 배치
  - 리소스 균등 분산
  - 최적 활용률
```

**예외 상황 대응:**

```yaml
만약 팀 간 성능 간섭이 발생한다면:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Phase 2에서 선택적 노드 할당:
  
  옵션 A: 일부 노드만 팀 전용
    - Team A가 고성능 보장 필요: 20대 전용 할당
    - 나머지 160대는 공유
  
  옵션 B: 워크로드 타입별 분리
    - Compute-intensive: 60대
    - Memory-intensive: 60대
    - General: 60대
    - 팀이 워크로드 타입에 따라 선택

  옵션 C: YuniKorn Queue
    - Gang Scheduling
    - 팀별 Queue 우선순위
    - 동적 리소스 재할당
```

**최종 권장 구성:**

```yaml
초기 (Phase 1): 노드 공유 방식
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
✓ 180대 Worker 노드 공유
✓ ResourceQuota로 팀당 리소스 보장
✓ NetworkPolicy로 격리
✓ 기본 스케줄러 사용
✓ 운영 단순, 효율 최대

3-6개월 후 (Phase 2): 필요시 조정
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
? 성능 간섭 발생 시 일부 노드 전용 할당
? YuniKorn 도입 (Gang Scheduling)
? Quota 조정
```

**모니터링 포인트:**
```yaml
팀별 리소스 사용률 추적:
  - CPU 사용률 (request vs actual)
  - 메모리 사용률
  - 네트워크 대역폭
  - 디스크 I/O
  - Pod Scheduling 지연 시간
  
간섭 징후:
  - 특정 팀의 Pending Pod 증가
  - 성능 저하 (Noisy Neighbor)
  - 네트워크 병목
  
→ 3개월 데이터 수집 후 재평가
```

---

## 수정된 아키텍처 요약

```yaml
노드 구성 (191대):
├─ Control Plane: 5대
├─ Worker: 183대
│   ├─ Infra (labeled): 3대
│   └─ General Worker: 180대 (4팀 공유)

OS & 버전:
├─ RHEL 10
├─ Kubernetes 1.29.8
└─ containerd 1.7.13

네트워크:
├─ eth0 (25GbE): Pod 네트워크 + 데이터 전송
└─ eth1 (25GbE): 관리 + 스토리지

스토리지:
├─ Local NVMe (nvme0n1): 고성능 임시 데이터
├─ Ceph (nvme0n2): 영구 데이터 + 고가용성
└─ External MinIO: 주 데이터 레이크

멀티테넌시:
├─ Soft Multi-tenancy (노드 공유)
├─ ResourceQuota + LimitRange + NetworkPolicy
└─ 필요시 Phase 2에서 조정
```

이제 **2단계: 설치 계획 및 체크리스트**로 진행하시겠습니까?

>>>

진행해줘

>>>

# 2단계: 설치 계획 및 체크리스트

## 2.1 전체 설치 일정 계획

```yaml
전체 기간: 약 4-6주 (검증 포함)

Week 1: 사전 준비 및 인프라 구성
├─ Day 1-2: 하드웨어 검증 및 OS 설치
├─ Day 3-4: 네트워크 구성 및 검증
└─ Day 5: 사전 요구사항 설치

Week 2: Kubernetes 클러스터 구축
├─ Day 1-2: Kubespray 준비 및 파일럿 (5-10대)
├─ Day 3-4: 전체 클러스터 설치 (191대)
└─ Day 5: 기본 검증 및 문제 해결

Week 3: 핵심 컴포넌트 설치
├─ Day 1: 스토리지 (Local PV, Rook-Ceph)
├─ Day 2: 네트워크 (Cilium Ingress, NetworkPolicy)
├─ Day 3: 모니터링 (Prometheus, Grafana)
├─ Day 4: 로깅 (OpenSearch, Fluent-bit)
└─ Day 5: 백업 (Velero, etcd backup)

Week 4: 공통 시스템 및 보안
├─ Day 1: 인증/인가 (Keycloak 연동, RBAC)
├─ Day 2: CI/CD (ArgoCD, Jenkins)
├─ Day 3: Spark Operator
├─ Day 4: 멀티테넌시 설정 (Quota, LimitRange, NetworkPolicy)
└─ Day 5: 보안 정책 (PSS, Admission Controller)

Week 5-6: 팀별 시스템 및 검증
├─ 팀별 시스템 설치 (Airflow, Trino, CNPG)
├─ 성능 테스트
└─ 최종 검증 및 문서화
```

---

## 2.2 Phase 0: 사전 준비 단계

### 2.2.1 프로젝트 킥오프 체크리스트

```yaml
□ 프로젝트 팀 구성
  □ 프로젝트 리더 지정
  □ Kubernetes 엔지니어 (2-3명)
  □ 네트워크 엔지니어
  □ 스토리지 엔지니어
  □ 보안 담당자
  □ 4개 팀 대표자

□ 의사소통 채널 설정
  □ Slack/Teams 채널 생성
  □ 주간 미팅 일정
  □ 에스컬레이션 프로세스

□ 문서 저장소 준비
  □ Git Repository (설정 파일, 문서)
  □ Wiki/Confluence (운영 문서)
  □ 이슈 트래킹 시스템

□ 외부 의존성 확인
  □ MinIO 클러스터 접근 가능 확인
  □ Keycloak 엔드포인트 확인
  □ 외부 네트워크 연결성 확인
  □ DNS 서버 준비
  □ NTP 서버 준비
  □ LoadBalancer 확보 (Control Plane용, Ingress용)
```

### 2.2.2 하드웨어 검증 체크리스트

```yaml
각 노드별 검증 (191대 전체):

□ CPU 검증
  □ lscpu로 코어 수 확인: 2 소켓 × 24 코어 = 48 코어
  □ CPU 모델: Intel Xeon Gold 6542Y
  □ Hyper-Threading 활성화 확인
  □ CPU 주파수 및 Governor 설정 확인
  
  명령어:
  lscpu
  cat /proc/cpuinfo | grep "model name" | head -1
  cat /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor

□ 메모리 검증
  □ 총 메모리: 768GB 확인
  □ 메모리 에러 확인 (EDAC logs)
  □ NUMA 노드 확인
  
  명령어:
  free -h
  dmidecode -t memory | grep Size
  numactl --hardware

□ 디스크 검증
  □ NVMe SSD 2개 인식 확인
  □ 각 디스크 용량: 3.84TB
  □ SMART 상태 확인
  □ 성능 벤치마크 (fio)
  
  명령어:
  lsblk
  nvme list
  smartctl -a /dev/nvme0n1
  smartctl -a /dev/nvme0n2
  
  # 성능 테스트
  fio --name=randwrite --ioengine=libaio --iodepth=32 \
      --rw=randwrite --bs=4k --direct=1 --size=10G \
      --numjobs=4 --runtime=60 --group_reporting \
      --filename=/dev/nvme0n1

□ 네트워크 검증
  □ NIC 2-port 인식 확인
  □ 각 포트 속도: 10/25Gbps
  □ 드라이버 버전 확인
  □ Firmware 버전 확인
  
  명령어:
  ip link show
  ethtool eth0
  ethtool eth1
  ethtool -i eth0  # 드라이버 정보
  lspci | grep -i ethernet

□ BIOS/Firmware 설정
  □ Virtualization 활성화 (VT-x, VT-d)
  □ IOMMU 활성화
  □ Power Management: Performance 모드
  □ Hyper-Threading: Enabled
  □ C-States: Disabled (성능 우선 시)
  □ Turbo Boost: Enabled

□ 하드웨어 일관성 검증
  □ 모든 191대가 동일 사양인지 확인
  □ 차이점 문서화 (있을 경우)
```

**자동화 스크립트 예시:**
```bash
#!/bin/bash
# hardware_check.sh - 모든 노드에서 실행

echo "=== Hardware Validation ==="
echo "Hostname: $(hostname)"
echo "CPU Cores: $(nproc)"
echo "Memory: $(free -h | grep Mem | awk '{print $2}')"
echo "NVMe Devices:"
lsblk | grep nvme
echo "Network Interfaces:"
ip link show | grep -E "eth|ens"
echo "============================"
```

---

## 2.3 Phase 1: OS 설치 및 설정

### 2.3.1 RHEL 10 설치 체크리스트

```yaml
□ OS 설치 준비
  □ RHEL 10 ISO 다운로드
  □ Kickstart 파일 준비 (자동 설치)
  □ PXE 부팅 서버 설정 (대규모 설치용)
  
□ 디스크 파티션 계획 (표준)
  /dev/nvme0n1:
    - /boot/efi: 512MB (EFI)
    - /boot: 1GB
    - /: 48GB (root)
    - /var/lib/containerd: 100GB
    - /var/log: 50GB
    - /mnt/local-storage: 나머지 (약 3.6TB) - Local PV용
  
  /dev/nvme0n2:
    - raw device (파티션 없음) - Ceph OSD용

□ OS 설치 설정
  □ Minimal Install (불필요한 패키지 최소화)
  □ SELinux: Enforcing (보안)
  □ Firewalld: 설치 후 설정
  □ 네트워크 설정: 고정 IP
  □ Hostname 설정: k8s-node001 ~ k8s-node191
  □ SSH 키 기반 인증 설정
  □ Root 비밀번호 설정

□ 네트워크 설정 (설치 시)
  eth0:
    - IP: 10.100.x.x (Pod 네트워크 대역)
    - Netmask: 255.255.0.0 (/16)
    - MTU: 9000
    - BOOTPROTO: static
  
  eth1:
    - IP: 10.0.x.x (관리 네트워크 대역)
    - Netmask: 255.255.0.0 (/16)
    - Gateway: 10.0.0.1
    - DNS: 10.0.0.10, 10.0.0.11
    - MTU: 1500
    - BOOTPROTO: static
```

**Kickstart 예시:**
```bash
# ks.cfg
install
url --url="http://mirror.example.com/rhel10"
text
lang en_US.UTF-8
keyboard us
timezone Asia/Seoul --utc

network --bootproto=static --device=eth0 --ip=10.100.0.X --netmask=255.255.0.0 --mtu=9000 --noipv6
network --bootproto=static --device=eth1 --ip=10.0.0.X --netmask=255.255.0.0 --gateway=10.0.0.1 --nameserver=10.0.0.10 --mtu=1500 --noipv6 --hostname=k8s-nodeXXX

rootpw --iscrypted $6$...
authselect select sssd --force
selinux --enforcing
firewall --enabled
firstboot --disable
skipx

ignoredisk --only-use=nvme0n1
clearpart --all --initlabel --drives=nvme0n1
part /boot/efi --fstype="efi" --ondisk=nvme0n1 --size=512
part /boot --fstype="xfs" --ondisk=nvme0n1 --size=1024
part pv.01 --fstype="lvmpv" --ondisk=nvme0n1 --size=50000 --grow
volgroup vg_system pv.01
logvol / --fstype="xfs" --size=49152 --name=lv_root --vgname=vg_system
logvol /var/lib/containerd --fstype="xfs" --size=102400 --name=lv_containerd --vgname=vg_system
logvol /var/log --fstype="xfs" --size=51200 --name=lv_log --vgname=vg_system
logvol /mnt/local-storage --fstype="xfs" --size=100 --grow --name=lv_local --vgname=vg_system

%packages
@^minimal-environment
vim
wget
curl
rsync
chrony
%end

%post
# SSH 키 추가
mkdir -p /root/.ssh
cat >> /root/.ssh/authorized_keys << 'EOF'
ssh-rsa AAAA... your-public-key
EOF
chmod 600 /root/.ssh/authorized_keys
chmod 700 /root/.ssh
%end

reboot
```

### 2.3.2 OS 설정 체크리스트

```yaml
모든 노드 (191대)에서 실행:

□ 시스템 업데이트
  sudo dnf update -y
  sudo reboot

□ 필수 패키지 설치
  sudo dnf install -y \
    vim \
    wget \
    curl \
    git \
    htop \
    iotop \
    sysstat \
    net-tools \
    bind-utils \
    ipvsadm \
    ipset \
    socat \
    conntrack-tools \
    chrony \
    nfs-utils \
    iscsi-initiator-utils

□ 시간 동기화 (chrony)
  sudo systemctl enable chronyd
  sudo systemctl start chronyd
  sudo chronyc sources  # 확인

□ Hostname 설정
  # Control Plane
  sudo hostnamectl set-hostname k8s-master01 (~ k8s-master05)
  
  # Worker
  sudo hostnamectl set-hostname k8s-worker001 (~ k8s-worker183)

□ /etc/hosts 설정
  # 모든 노드의 IP와 hostname 추가
  # 또는 DNS 서버 사용 권장 (191대는 DNS 필수)

□ Firewalld 설정
  # Kubernetes 필요 포트 오픈
  # Control Plane 노드:
  sudo firewall-cmd --permanent --add-port=6443/tcp      # API Server
  sudo firewall-cmd --permanent --add-port=2379-2380/tcp # etcd
  sudo firewall-cmd --permanent --add-port=10250/tcp     # kubelet
  sudo firewall-cmd --permanent --add-port=10251/tcp     # kube-scheduler
  sudo firewall-cmd --permanent --add-port=10252/tcp     # kube-controller
  
  # Worker 노드:
  sudo firewall-cmd --permanent --add-port=10250/tcp     # kubelet
  sudo firewall-cmd --permanent --add-port=30000-32767/tcp # NodePort
  
  # 모든 노드:
  sudo firewall-cmd --permanent --add-port=179/tcp       # BGP (Cilium)
  sudo firewall-cmd --permanent --add-port=4789/udp      # VXLAN (Cilium)
  sudo firewall-cmd --permanent --add-port=8472/udp      # VXLAN
  sudo firewall-cmd --reload
  
  # 또는 방화벽 비활성화 (내부 네트워크만 있을 경우)
  sudo systemctl stop firewalld
  sudo systemctl disable firewalld

□ SELinux 설정
  # Kubernetes는 SELinux Enforcing 모드 지원
  # 그러나 초기 설치 시 문제 방지를 위해 Permissive 권장
  sudo setenforce 0
  sudo sed -i 's/^SELINUX=enforcing$/SELINUX=permissive/' /etc/selinux/config
  
  # 설치 후 다시 Enforcing으로 변경 검토

□ Swap 비활성화 (필수)
  sudo swapoff -a
  sudo sed -i '/swap/d' /etc/fstab
  
  # 확인
  free -h  # swap이 0이어야 함

□ 커널 모듈 로드
  cat <<EOF | sudo tee /etc/modules-load.d/k8s.conf
  overlay
  br_netfilter
  ip_vs
  ip_vs_rr
  ip_vs_wrr
  ip_vs_sh
  nf_conntrack
  EOF
  
  sudo modprobe overlay
  sudo modprobe br_netfilter
  sudo modprobe ip_vs
  sudo modprobe ip_vs_rr
  sudo modprobe ip_vs_wrr
  sudo modprobe ip_vs_sh
  sudo modprobe nf_conntrack

□ 커널 파라미터 설정
  cat <<EOF | sudo tee /etc/sysctl.d/k8s.conf
  # IP Forwarding
  net.ipv4.ip_forward = 1
  net.bridge.bridge-nf-call-iptables = 1
  net.bridge.bridge-nf-call-ip6tables = 1
  
  # 네트워크 성능 최적화
  net.ipv4.tcp_keepalive_time = 600
  net.ipv4.tcp_keepalive_intvl = 60
  net.ipv4.tcp_keepalive_probes = 3
  
  # 커넥션 추적 증가
  net.netfilter.nf_conntrack_max = 1000000
  net.nf_conntrack_max = 1000000
  
  # 파일 디스크립터 증가
  fs.file-max = 2097152
  fs.inotify.max_user_watches = 524288
  fs.inotify.max_user_instances = 8192
  
  # 메모리 설정
  vm.swappiness = 0
  vm.overcommit_memory = 1
  vm.panic_on_oom = 0
  
  # ARP 캐시
  net.ipv4.neigh.default.gc_thresh1 = 80000
  net.ipv4.neigh.default.gc_thresh2 = 90000
  net.ipv4.neigh.default.gc_thresh3 = 100000
  EOF
  
  sudo sysctl --system
  sudo sysctl -p /etc/sysctl.d/k8s.conf

□ 시스템 리소스 제한 설정
  cat <<EOF | sudo tee /etc/security/limits.d/k8s.conf
  * soft nofile 1048576
  * hard nofile 1048576
  * soft nproc 1048576
  * hard nproc 1048576
  * soft memlock unlimited
  * hard memlock unlimited
  EOF

□ NIC 최적화
  # eth0 설정 (Pod 네트워크)
  sudo ethtool -K eth0 tx-checksum-ip-generic on
  sudo ethtool -K eth0 gso on
  sudo ethtool -K eth0 tso on
  sudo ethtool -L eth0 combined 16
  sudo ethtool -G eth0 rx 4096 tx 4096
  
  # 영구 설정 (systemd service 생성)
  cat <<EOF | sudo tee /etc/systemd/system/nic-tuning.service
  [Unit]
  Description=NIC Performance Tuning
  After=network.target
  
  [Service]
  Type=oneshot
  ExecStart=/usr/sbin/ethtool -K eth0 tx-checksum-ip-generic on
  ExecStart=/usr/sbin/ethtool -K eth0 gso on
  ExecStart=/usr/sbin/ethtool -K eth0 tso on
  ExecStart=/usr/sbin/ethtool -L eth0 combined 16
  ExecStart=/usr/sbin/ethtool -G eth0 rx 4096 tx 4096
  
  [Install]
  WantedBy=multi-user.target
  EOF
  
  sudo systemctl daemon-reload
  sudo systemctl enable nic-tuning.service
  sudo systemctl start nic-tuning.service

□ IRQ 밸런싱 비활성화 (성능 최적화)
  sudo systemctl stop irqbalance
  sudo systemctl disable irqbalance

□ SSH 설정
  # 관리 서버에서 모든 노드로 passwordless SSH 설정
  ssh-keygen -t rsa -b 4096 -N "" -f ~/.ssh/id_rsa
  
  # 모든 노드에 공개키 배포
  for i in {1..5}; do
    ssh-copy-id root@k8s-master0$i
  done
  
  for i in {1..183}; do
    ssh-copy-id root@k8s-worker$(printf "%03d" $i)
  done
  
  # 연결 테스트
  ansible all -m ping
```

---

## 2.4 Phase 2: Kubespray 준비 및 설치

### 2.4.1 Kubespray 준비 체크리스트

```yaml
관리 서버 (Bastion Host)에서 실행:

□ Ansible 설치
  sudo dnf install -y python3 python3-pip
  pip3 install --upgrade pip
  pip3 install ansible==8.5.0
  
  # 확인
  ansible --version  # 2.15+ 권장

□ Kubespray 다운로드
  git clone https://github.com/kubernetes-sigs/kubespray.git
  cd kubespray
  git checkout release-2.24  # 최신 stable 브랜치
  
  # 또는 특정 태그
  git checkout v2.24.0

□ Python 의존성 설치
  pip3 install -r requirements.txt

□ Inventory 생성
  cp -rfp inventory/sample inventory/mycluster
  
  # inventory 편집기 사용
  CONFIG_FILE=inventory/mycluster/hosts.yaml python3 contrib/inventory_builder/inventory.py \
    10.0.0.1 10.0.0.2 10.0.0.3 10.0.0.4 10.0.0.5 \
    10.0.0.6 10.0.0.7 10.0.0.8 ... (191개 IP)
```

**Inventory 파일 생성 (hosts.yaml):**
```yaml
all:
  hosts:
    k8s-master01:
      ansible_host: 10.0.0.1
      ip: 10.0.0.1
      access_ip: 10.0.0.1
    k8s-master02:
      ansible_host: 10.0.0.2
      ip: 10.0.0.2
      access_ip: 10.0.0.2
    k8s-master03:
      ansible_host: 10.0.0.3
      ip: 10.0.0.3
      access_ip: 10.0.0.3
    k8s-master04:
      ansible_host: 10.0.0.4
      ip: 10.0.0.4
      access_ip: 10.0.0.4
    k8s-master05:
      ansible_host: 10.0.0.5
      ip: 10.0.0.5
      access_ip: 10.0.0.5
    k8s-worker001:
      ansible_host: 10.0.0.6
      ip: 10.0.0.6
      access_ip: 10.0.0.6
    # ... worker002 ~ worker183
  
  children:
    kube_control_plane:
      hosts:
        k8s-master01:
        k8s-master02:
        k8s-master03:
        k8s-master04:
        k8s-master05:
    
    kube_node:
      hosts:
        k8s-worker001:
        k8s-worker002:
        # ... worker003 ~ worker183
    
    etcd:
      hosts:
        k8s-master01:
        k8s-master02:
        k8s-master03:
        k8s-master04:
        k8s-master05:
    
    k8s_cluster:
      children:
        kube_control_plane:
        kube_node:
    
    calico_rr:
      hosts: {}
```

### 2.4.2 Kubespray 설정 체크리스트

```yaml
inventory/mycluster/group_vars/ 디렉토리에서 설정:

□ k8s_cluster/k8s-cluster.yml 편집
  # Kubernetes 버전
  kube_version: v1.29.8
  
  # 네트워크 플러그인
  kube_network_plugin: cilium
  
  # Service/Pod CIDR
  kube_service_addresses: 10.96.0.0/12
  kube_pods_subnet: 10.244.0.0/12
  
  # DNS
  dns_mode: coredns
  enable_nodelocaldns: true
  nodelocaldns_ip: 169.254.25.10
  
  # Proxy mode
  kube_proxy_mode: ipvs
  kube_proxy_strict_arp: true
  
  # Container Runtime
  container_manager: containerd
  
  # etcd 설정
  etcd_deployment_type: host
  etcd_data_dir: /var/lib/etcd
  etcd_events_cluster_enabled: false
  
  # API Server 설정
  kube_apiserver_enable_admission_plugins:
    - NodeRestriction
    - PodSecurityPolicy  # 또는 PodSecurity (PSA)
    - NamespaceLifecycle
    - ServiceAccount
    - DefaultStorageClass
    - ResourceQuota
    - DefaultTolerationSeconds
    - LimitRanger
  
  # Feature Gates
  kube_feature_gates:
    - "EphemeralContainers=true"
    - "TTLAfterFinished=true"
  
  # 인증서 만료일
  certificates_duration: 36500  # 100년 (또는 적절한 값)
  
  # kubectl 다운로드
  kubectl_localhost: true
  kubeconfig_localhost: true

□ k8s_cluster/addons.yml 편집
  # Helm 설치
  helm_enabled: true
  
  # Metrics Server
  metrics_server_enabled: true
  metrics_server_kubelet_insecure_tls: true
  metrics_server_kubelet_preferred_address_types: "InternalIP,ExternalIP,Hostname"
  
  # Ingress (Cilium으로 대체 예정이므로 비활성화)
  ingress_nginx_enabled: false
  
  # cert-manager
  cert_manager_enabled: true
  
  # Registry (선택사항)
  registry_enabled: false

□ all/all.yml 편집
  # 프록시 설정 (필요시)
  # http_proxy: "http://proxy.example.com:3128"
  # https_proxy: "http://proxy.example.com:3128"
  # no_proxy: "localhost,127.0.0.1,10.0.0.0/8,10.96.0.0/12,10.244.0.0/12"
  
  # 다운로드 설정
  download_run_once: true
  download_localhost: false
  
  # NTP 서버
  ntp_enabled: true
  ntp_timezone: "Asia/Seoul"

□ etcd.yml 편집
  # etcd 최적화 (대규모 클러스터용)
  etcd_heartbeat_interval: "250"
  etcd_election_timeout: "5000"
  etcd_snapshot_count: "10000"
  etcd_quota_backend_bytes: "8589934592"  # 8GB
  etcd_max_request_bytes: "10485760"  # 10MB
  etcd_grpc_keepalive_min_time: "5s"
  etcd_grpc_keepalive_interval: "2h"
  etcd_grpc_keepalive_timeout: "20s"
  
  # etcd 메트릭
  etcd_metrics: "extensive"
  etcd_listen_metrics_urls: "http://0.0.0.0:2381"
```

**Cilium 추가 설정 (필요시):**
```yaml
# inventory/mycluster/group_vars/k8s_cluster/k8s-net-cilium.yml

cilium_version: "v1.14.5"
cilium_enable_ipv4: true
cilium_enable_ipv6: false

# Native Routing (고성능)
cilium_tunnel_mode: disabled
cilium_routing_mode: native
cilium_auto_direct_node_routes: true

# IPAM
cilium_ipam_mode: kubernetes

# Bandwidth Manager
cilium_enable_bandwidth_manager: true

# Hubble (관측성)
cilium_enable_hubble: true
cilium_hubble_metrics_enabled:
  - dns
  - drop
  - tcp
  - flow
  - port-distribution
  - icmp
  - http

# Egress Gateway (선택)
cilium_enable_ipv4_egress_gateway: false
```

### 2.4.3 파일럿 설치 (5-10대)

```yaml
□ 파일럿 클러스터 구성
  # 작은 규모로 먼저 테스트
  # Control Plane 3대 + Worker 5대
  
  # inventory에서 일부 노드만 선택
  cp inventory/mycluster/hosts.yaml inventory/mycluster/hosts-pilot.yaml
  # hosts-pilot.yaml에서 8대만 남기고 제거

□ 파일럿 설치 실행
  ansible-playbook -i inventory/mycluster/hosts-pilot.yaml \
    --become --become-user=root \
    cluster.yml
  
  # 예상 소요 시간: 30-60분

□ 파일럿 검증
  # kubeconfig 복사
  mkdir -p ~/.kube
  cp inventory/mycluster/artifacts/admin.conf ~/.kube/config
  
  # 클러스터 상태 확인
  kubectl get nodes
  kubectl get pods -A
  kubectl cluster-info
  
  # 기본 워크로드 테스트
  kubectl create deployment nginx --image=nginx
  kubectl expose deployment nginx --port=80 --type=NodePort
  kubectl get svc nginx
  
  # 정리
  kubectl delete deployment nginx
  kubectl delete svc nginx

□ 문제 해결 및 조정
  # 발생한 문제 문서화
  # 설정 조정
  # 필요시 재설치

□ 파일럿 승인
  # 팀 리뷰
  # 다음 단계 진행 결정
```

### 2.4.4 전체 클러스터 설치

```yaml
□ 사전 점검
  # 모든 191대 노드 SSH 연결 확인
  ansible all -i inventory/mycluster/hosts.yaml -m ping
  
  # 디스크 공간 확인
  ansible all -i inventory/mycluster/hosts.yaml \
    -m shell -a "df -h /"
  
  # 시간 동기화 확인
  ansible all -i inventory/mycluster/hosts.yaml \
    -m shell -a "chronyc tracking"

□ 설치 실행
  # 전체 클러스터 설치
  ansible-playbook -i inventory/mycluster/hosts.yaml \
    --become --become-user=root \
    -v \
    cluster.yml
  
  # 예상 소요 시간: 4-8시간 (191대)
  # 병렬 처리: 기본 5개씩 (forks=5)
  # 더 빠르게 하려면: --forks=10 (리스크 증가)
  
  # 백그라운드 실행 (tmux/screen 사용 권장)
  tmux new -s k8s-install
  ansible-playbook ...
  # Ctrl+B, D로 detach

□ 설치 모니터링
  # 다른 터미널에서 로그 확인
  tail -f /var/log/ansible.log  # 또는 설정된 로그 위치
  
  # 특정 노드 로그 확인
  ssh k8s-master01
  journalctl -u kubelet -f

□ 설치 중 체크포인트
  # Control Plane 설치 완료 확인
  kubectl --kubeconfig inventory/mycluster/artifacts/admin.conf get nodes
  
  # etcd 클러스터 상태
  kubectl --kubeconfig inventory/mycluster/artifacts/admin.conf \
    -n kube-system exec -it etcd-k8s-master01 -- \
    etcdctl --endpoints=https://127.0.0.1:2379 \
    --cacert=/etc/ssl/etcd/ssl/ca.pem \
    --cert=/etc/ssl/etcd/ssl/node-k8s-master01.pem \
    --key=/etc/ssl/etcd/ssl/node-k8s-master01-key.pem \
    endpoint health
  
  # Worker 노드 Join 확인
  watch "kubectl get nodes | grep -c Ready"

□ 설치 완료 검증
  # kubeconfig 설정
  mkdir -p ~/.kube
  cp inventory/mycluster/artifacts/admin.conf ~/.kube/config
  
  # 모든 노드 Ready 상태 확인
  kubectl get nodes
  # 예상: 191개 노드 모두 Ready
  
  # 시스템 Pod 확인
  kubectl get pods -A
  # 모든 Pod가 Running 상태
  
  # Component 상태
  kubectl get cs  # deprecated, 대신 아래 사용
  kubectl get --raw='/readyz?verbose'
  
  # API 서버 응답 시간 측정
  time kubectl get nodes

□ 문제 해결
  # 실패한 노드 확인
  kubectl get nodes | grep NotReady
  
  # kubelet 로그 확인
  ssh <failed-node>
  journalctl -u kubelet -n 100
  
  # 재시도
  ansible-playbook -i inventory/mycluster/hosts.yaml \
    --limit <failed-node> \
    --become --become-user=root \
    cluster.yml
```

---

## 2.5 Phase 3: 노드 역할 및 Label 설정

```yaml
□ Control Plane Taint 확인
  # 이미 Kubespray가 설정했는지 확인
  kubectl get nodes -o json | jq '.items[] | select(.metadata.name | startswith("k8s-master")) | {name: .metadata.name, taints: .spec.taints}'
  
  # 없으면 추가
  for i in {1..5}; do
    kubectl taint nodes k8s-master0$i \
      node-role.kubernetes.io/control-plane:NoSchedule \
      --overwrite
  done

□ Infra 노드 설정
  # Worker 중 3대를 Infra 노드로 지정
  # 예: worker001, worker002, worker003
  
  kubectl label nodes k8s-worker001 k8s-worker002 k8s-worker003 \
    node-role.kubernetes.io/infra=true
  
  kubectl taint nodes k8s-worker001 k8s-worker002 k8s-worker003 \
    node-role.kubernetes.io/infra=true:NoSchedule

□ Worker 노드 Label
  # 나머지 Worker 노드에 Label
  for i in {4..183}; do
    kubectl label nodes k8s-worker$(printf "%03d" $i) \
      node-role.kubernetes.io/worker=true
  done

□ Zone/Rack Label (선택사항)
  # 물리적 위치에 따라 Label
  # 예: 랙별로 구분
  for i in {4..50}; do
    kubectl label nodes k8s-worker$(printf "%03d" $i) \
      topology.kubernetes.io/zone=rack-1
  done
  for i in {51..100}; do
    kubectl label nodes k8s-worker$(printf "%03d" $i) \
      topology.kubernetes.io/zone=rack-2
  done
  # ... 계속

□ Label 검증
  kubectl get nodes --show-labels
  kubectl get nodes -l node-role.kubernetes.io/infra=true
  kubectl get nodes -l node-role.kubernetes.io/worker=true
```

---

이상으로 2단계의 전반부를 완료했습니다. 다음은:
- 2.6 Phase 4: 스토리지 구성
- 2.7 Phase 5: 네트워크 구성
- 2.8 Phase 6: 보안 구성
등이 이어집니다.

계속 진행할까요?

>>>

계속 끝까지 진행해줘

>>>

계속 진행하겠습니다.

---

## 2.6 Phase 4: 스토리지 구성

### 2.6.1 Local PV 구성 체크리스트

```yaml
□ Local Storage 디렉토리 생성 (모든 Worker 노드)
  ansible -i inventory/mycluster/hosts.yaml kube_node \
    -m shell -a "mkdir -p /mnt/local-storage && chmod 755 /mnt/local-storage"
  
  # 마운트 확인
  ansible -i inventory/mycluster/hosts.yaml kube_node \
    -m shell -a "df -h /mnt/local-storage"

□ Local Storage Provisioner 설치
  # Local Path Provisioner 설치 (Rancher)
  kubectl apply -f https://raw.githubusercontent.com/rancher/local-path-provisioner/v0.0.26/deploy/local-path-storage.yaml
  
  # 또는 Sig-storage-local-static-provisioner
  git clone https://github.com/kubernetes-sigs/sig-storage-local-static-provisioner.git
  cd sig-storage-local-static-provisioner

□ Local PV StorageClass 생성
  cat <<EOF | kubectl apply -f -
  apiVersion: storage.k8s.io/v1
  kind: StorageClass
  metadata:
    name: local-nvme
  provisioner: kubernetes.io/no-provisioner
  volumeBindingMode: WaitForFirstConsumer
  EOF

□ Local PV 수동 생성 (각 Worker 노드)
  # 자동화 스크립트
  for i in {4..183}; do
    NODE="k8s-worker$(printf "%03d" $i)"
    cat <<EOF | kubectl apply -f -
  apiVersion: v1
  kind: PersistentVolume
  metadata:
    name: local-pv-${NODE}
  spec:
    capacity:
      storage: 3600Gi
    volumeMode: Filesystem
    accessModes:
    - ReadWriteOnce
    persistentVolumeReclaimPolicy: Retain
    storageClassName: local-nvme
    local:
      path: /mnt/local-storage
    nodeAffinity:
      required:
        nodeSelectorTerms:
        - matchExpressions:
          - key: kubernetes.io/hostname
            operator: In
            values:
            - ${NODE}
  EOF
  done

□ Local PV 검증
  kubectl get pv | grep local-nvme
  # 180개 PV가 Available 상태
  
  # 테스트 PVC 생성
  cat <<EOF | kubectl apply -f -
  apiVersion: v1
  kind: PersistentVolumeClaim
  metadata:
    name: test-local-pvc
  spec:
    accessModes:
    - ReadWriteOnce
    storageClassName: local-nvme
    resources:
      requests:
        storage: 100Gi
  EOF
  
  # 테스트 Pod
  cat <<EOF | kubectl apply -f -
  apiVersion: v1
  kind: Pod
  metadata:
    name: test-local-pod
  spec:
    containers:
    - name: test
      image: nginx
      volumeMounts:
      - name: storage
        mountPath: /data
    volumes:
    - name: storage
      persistentVolumeClaim:
        claimName: test-local-pvc
  EOF
  
  # Pod가 Running이고 PVC가 Bound 되는지 확인
  kubectl get pod test-local-pod
  kubectl get pvc test-local-pvc
  
  # I/O 테스트
  kubectl exec test-local-pod -- dd if=/dev/zero of=/data/testfile bs=1G count=1 oflag=direct
  
  # 정리
  kubectl delete pod test-local-pod
  kubectl delete pvc test-local-pvc
```

### 2.6.2 Rook-Ceph 구성 체크리스트

```yaml
□ Ceph 사전 준비 (모든 Worker 노드)
  # LVM2 설치 확인
  ansible -i inventory/mycluster/hosts.yaml kube_node \
    -m shell -a "dnf install -y lvm2"
  
  # /dev/nvme0n2 디스크 초기화 (주의!)
  ansible -i inventory/mycluster/hosts.yaml kube_node \
    -m shell -a "wipefs -a /dev/nvme0n2"
  
  # 디스크 확인
  ansible -i inventory/mycluster/hosts.yaml kube_node \
    -m shell -a "lsblk | grep nvme0n2"

□ Rook-Ceph Operator 설치
  # Rook 저장소 추가
  git clone --single-branch --branch v1.13.0 https://github.com/rook/rook.git
  cd rook/deploy/examples
  
  # CRD 생성
  kubectl create -f crds.yaml
  
  # Common 리소스
  kubectl create -f common.yaml
  
  # Operator 배포
  kubectl create -f operator.yaml
  
  # Operator Pod 확인
  kubectl -n rook-ceph get pod -l app=rook-ceph-operator
  # Running 상태 대기

□ Ceph Cluster 생성
  # cluster.yaml 수정
  cat <<EOF | kubectl apply -f -
  apiVersion: ceph.rook.io/v1
  kind: CephCluster
  metadata:
    name: rook-ceph
    namespace: rook-ceph
  spec:
    cephVersion:
      image: quay.io/ceph/ceph:v17.2.7
      allowUnsupported: false
    dataDirHostPath: /var/lib/rook
    mon:
      count: 5
      allowMultiplePerNode: false
    mgr:
      count: 2
      allowMultiplePerNode: false
    dashboard:
      enabled: true
      ssl: false
    crashCollector:
      disable: false
    storage:
      useAllNodes: false
      useAllDevices: false
      nodes:
      - name: "k8s-worker004"
        devices:
        - name: "nvme0n2"
      - name: "k8s-worker005"
        devices:
        - name: "nvme0n2"
      # ... worker006 ~ worker183까지 모두 추가
      # 총 180개 노드
    resources:
      mon:
        limits:
          cpu: "2"
          memory: "4Gi"
        requests:
          cpu: "1"
          memory: "2Gi"
      osd:
        limits:
          cpu: "4"
          memory: "8Gi"
        requests:
          cpu: "2"
          memory: "4Gi"
      mgr:
        limits:
          cpu: "2"
          memory: "4Gi"
        requests:
          cpu: "1"
          memory: "2Gi"
  EOF
  
  # 설치 진행 모니터링 (매우 오래 걸림: 1-2시간)
  watch kubectl -n rook-ceph get pod
  
  # Ceph 상태 확인
  kubectl -n rook-ceph get cephcluster
  # HEALTH: HEALTH_OK 대기

□ Ceph 도구 Pod 배포
  kubectl create -f toolbox.yaml
  
  # Ceph 상태 확인
  kubectl -n rook-ceph exec -it deploy/rook-ceph-tools -- ceph status
  kubectl -n rook-ceph exec -it deploy/rook-ceph-tools -- ceph osd status
  kubectl -n rook-ceph exec -it deploy/rook-ceph-tools -- ceph df
  
  # 예상: 180 OSDs, ~720TB raw capacity

□ Ceph Block Storage (RBD) 구성
  cat <<EOF | kubectl apply -f -
  apiVersion: ceph.rook.io/v1
  kind: CephBlockPool
  metadata:
    name: replicapool
    namespace: rook-ceph
  spec:
    replicated:
      size: 3
      requireSafeReplicaSize: true
    parameters:
      compression_mode: none
  EOF
  
  # StorageClass 생성
  cat <<EOF | kubectl apply -f -
  apiVersion: storage.k8s.io/v1
  kind: StorageClass
  metadata:
    name: ceph-block
  provisioner: rook-ceph.rbd.csi.ceph.com
  parameters:
    clusterID: rook-ceph
    pool: replicapool
    imageFormat: "2"
    imageFeatures: layering
    csi.storage.k8s.io/provisioner-secret-name: rook-csi-rbd-provisioner
    csi.storage.k8s.io/provisioner-secret-namespace: rook-ceph
    csi.storage.k8s.io/controller-expand-secret-name: rook-csi-rbd-provisioner
    csi.storage.k8s.io/controller-expand-secret-namespace: rook-ceph
    csi.storage.k8s.io/node-stage-secret-name: rook-csi-rbd-node
    csi.storage.k8s.io/node-stage-secret-namespace: rook-ceph
    csi.storage.k8s.io/fstype: ext4
  reclaimPolicy: Delete
  allowVolumeExpansion: true
  EOF

□ Ceph Filesystem (CephFS) 구성
  cat <<EOF | kubectl apply -f -
  apiVersion: ceph.rook.io/v1
  kind: CephFilesystem
  metadata:
    name: myfs
    namespace: rook-ceph
  spec:
    metadataPool:
      replicated:
        size: 3
    dataPools:
    - name: replicated
      replicated:
        size: 3
    preserveFilesystemOnDelete: true
    metadataServer:
      activeCount: 2
      activeStandby: true
  EOF
  
  # StorageClass 생성
  cat <<EOF | kubectl apply -f -
  apiVersion: storage.k8s.io/v1
  kind: StorageClass
  metadata:
    name: ceph-filesystem
  provisioner: rook-ceph.cephfs.csi.ceph.com
  parameters:
    clusterID: rook-ceph
    fsName: myfs
    pool: myfs-replicated
    csi.storage.k8s.io/provisioner-secret-name: rook-csi-cephfs-provisioner
    csi.storage.k8s.io/provisioner-secret-namespace: rook-ceph
    csi.storage.k8s.io/controller-expand-secret-name: rook-csi-cephfs-provisioner
    csi.storage.k8s.io/controller-expand-secret-namespace: rook-ceph
    csi.storage.k8s.io/node-stage-secret-name: rook-csi-cephfs-node
    csi.storage.k8s.io/node-stage-secret-namespace: rook-ceph
  reclaimPolicy: Delete
  allowVolumeExpansion: true
  EOF

□ Ceph Dashboard 접근
  # 패스워드 확인
  kubectl -n rook-ceph get secret rook-ceph-dashboard-password \
    -o jsonpath="{['data']['password']}" | base64 --decode && echo
  
  # Port Forward
  kubectl -n rook-ceph port-forward svc/rook-ceph-mgr-dashboard 8443:8443
  
  # 브라우저: https://localhost:8443
  # Username: admin
  # Password: (위에서 확인한 값)

□ 스토리지 검증
  # RBD 테스트
  cat <<EOF | kubectl apply -f -
  apiVersion: v1
  kind: PersistentVolumeClaim
  metadata:
    name: test-ceph-pvc
  spec:
    accessModes:
    - ReadWriteOnce
    storageClassName: ceph-block
    resources:
      requests:
        storage: 10Gi
  EOF
  
  kubectl get pvc test-ceph-pvc
  # Bound 상태 확인
  
  # CephFS 테스트
  cat <<EOF | kubectl apply -f -
  apiVersion: v1
  kind: PersistentVolumeClaim
  metadata:
    name: test-cephfs-pvc
  spec:
    accessModes:
    - ReadWriteMany
    storageClassName: ceph-filesystem
    resources:
      requests:
        storage: 10Gi
  EOF
  
  kubectl get pvc test-cephfs-pvc
  
  # 정리
  kubectl delete pvc test-ceph-pvc test-cephfs-pvc
```

---

## 2.7 Phase 5: 네트워크 구성

### 2.7.1 Cilium 검증 및 최적화

```yaml
□ Cilium 설치 확인
  # Kubespray가 이미 설치했는지 확인
  kubectl -n kube-system get pods -l k8s-app=cilium
  
  # Cilium CLI 설치
  CILIUM_CLI_VERSION=$(curl -s https://raw.githubusercontent.com/cilium/cilium-cli/master/stable.txt)
  curl -L --fail --remote-name-all https://github.com/cilium/cilium-cli/releases/download/${CILIUM_CLI_VERSION}/cilium-linux-amd64.tar.gz{,.sha256sum}
  sha256sum --check cilium-linux-amd64.tar.gz.sha256sum
  sudo tar xzvfC cilium-linux-amd64.tar.gz /usr/local/bin
  rm cilium-linux-amd64.tar.gz{,.sha256sum}
  
  # Cilium 상태 확인
  cilium status --wait

□ Cilium 설정 최적화
  # ConfigMap 확인
  kubectl -n kube-system get cm cilium-config -o yaml
  
  # 필요한 설정 추가/수정
  kubectl -n kube-system edit cm cilium-config
  
  # 주요 설정:
  # enable-ipv4: "true"
  # enable-ipv6: "false"
  # tunnel: "disabled"  # Native routing
  # routing-mode: "native"
  # auto-direct-node-routes: "true"
  # enable-bandwidth-manager: "true"
  # enable-hubble: "true"
  # kube-proxy-replacement: "partial"  # 또는 "strict"
  
  # DaemonSet 재시작
  kubectl -n kube-system rollout restart ds/cilium

□ Hubble 설치 (네트워크 관측성)
  # Hubble Relay
  cilium hubble enable --ui
  
  # Hubble UI 접근
  cilium hubble ui
  # 브라우저 자동 오픈
  
  # Hubble CLI 설치
  export HUBBLE_VERSION=$(curl -s https://raw.githubusercontent.com/cilium/hubble/master/stable.txt)
  curl -L --fail --remote-name-all https://github.com/cilium/hubble/releases/download/$HUBBLE_VERSION/hubble-linux-amd64.tar.gz{,.sha256sum}
  sha256sum --check hubble-linux-amd64.tar.gz.sha256sum
  sudo tar xzvfC hubble-linux-amd64.tar.gz /usr/local/bin
  rm hubble-linux-amd64.tar.gz{,.sha256sum}

□ Cilium 네트워크 정책 테스트
  # 테스트 Namespace
  kubectl create ns cilium-test
  
  # 연결성 테스트
  cilium connectivity test --test-namespace cilium-test
  
  # 정리
  kubectl delete ns cilium-test

□ Cilium 성능 검증
  # 대역폭 테스트
  kubectl create deployment iperf-server --image=networkstatic/iperf3 -- -s
  kubectl expose deployment iperf-server --port=5201
  
  kubectl run iperf-client --rm -it --image=networkstatic/iperf3 -- \
    -c iperf-server -t 30 -P 4
  
  # 예상: 20Gbps+ (25GbE 네트워크)
  
  # 정리
  kubectl delete deployment iperf-server
  kubectl delete svc iperf-server
```

### 2.7.2 Cilium Ingress Controller 설치

```yaml
□ Cilium Ingress 활성화
  # Ingress Controller 활성화
  cilium install --set ingressController.enabled=true
  
  # 또는 Helm으로 업그레이드
  helm upgrade cilium cilium/cilium --version 1.14.5 \
    --namespace kube-system \
    --reuse-values \
    --set ingressController.enabled=true \
    --set ingressController.loadbalancerMode=shared \
    --set ingressController.service.type=LoadBalancer

□ IngressClass 생성
  cat <<EOF | kubectl apply -f -
  apiVersion: networking.k8s.io/v1
  kind: IngressClass
  metadata:
    name: cilium
  spec:
    controller: cilium.io/ingress-controller
  EOF
  
  # 기본 IngressClass 설정 (선택)
  kubectl annotate ingressclass cilium \
    ingressclass.kubernetes.io/is-default-class="true"

□ LoadBalancer Service 확인
  # Cilium Ingress Service
  kubectl -n kube-system get svc cilium-ingress
  
  # External IP 필요 (MetalLB 또는 외부 LB)
  # 없으면 MetalLB 설치 또는 NodePort 사용

□ MetalLB 설치 (LoadBalancer 제공)
  # MetalLB 설치
  kubectl apply -f https://raw.githubusercontent.com/metallb/metallb/v0.13.12/config/manifests/metallb-native.yaml
  
  # IP Pool 생성
  cat <<EOF | kubectl apply -f -
  apiVersion: metallb.io/v1beta1
  kind: IPAddressPool
  metadata:
    name: default-pool
    namespace: metallb-system
  spec:
    addresses:
    - 10.0.100.100-10.0.100.200  # 사용 가능한 IP 범위
  ---
  apiVersion: metallb.io/v1beta1
  kind: L2Advertisement
  metadata:
    name: default
    namespace: metallb-system
  spec:
    ipAddressPools:
    - default-pool
  EOF

□ Ingress 테스트
  # 테스트 애플리케이션
  kubectl create deployment nginx-test --image=nginx
  kubectl expose deployment nginx-test --port=80
  
  # Ingress 생성
  cat <<EOF | kubectl apply -f -
  apiVersion: networking.k8s.io/v1
  kind: Ingress
  metadata:
    name: nginx-test
  spec:
    ingressClassName: cilium
    rules:
    - host: nginx-test.example.com
      http:
        paths:
        - path: /
          pathType: Prefix
          backend:
            service:
              name: nginx-test
              port:
                number: 80
  EOF
  
  # External IP 확인
  kubectl get ingress nginx-test
  
  # 테스트 (IP 또는 도메인)
  curl -H "Host: nginx-test.example.com" http://<EXTERNAL-IP>
  
  # 정리
  kubectl delete ingress nginx-test
  kubectl delete svc nginx-test
  kubectl delete deployment nginx-test
```

### 2.7.3 CoreDNS 최적화

```yaml
□ CoreDNS 설정 확인
  kubectl -n kube-system get cm coredns -o yaml
  
  # 대규모 클러스터용 최적화
  kubectl -n kube-system edit cm coredns
  
  # Corefile 수정:
  .:53 {
      errors
      health {
          lameduck 5s
      }
      ready
      kubernetes cluster.local in-addr.arpa ip6.arpa {
          pods insecure
          fallthrough in-addr.arpa ip6.arpa
          ttl 30
      }
      prometheus :9153
      forward . /etc/resolv.conf {
          max_concurrent 1000
      }
      cache 30
      loop
      reload
      loadbalance
  }

□ CoreDNS Replica 증가
  # 191대 클러스터: 최소 5-10 replicas
  kubectl -n kube-system scale deployment coredns --replicas=10
  
  # HPA 설정 (선택)
  cat <<EOF | kubectl apply -f -
  apiVersion: autoscaling/v2
  kind: HorizontalPodAutoscaler
  metadata:
    name: coredns
    namespace: kube-system
  spec:
    scaleTargetRef:
      apiVersion: apps/v1
      kind: Deployment
      name: coredns
    minReplicas: 10
    maxReplicas: 20
    metrics:
    - type: Resource
      resource:
        name: cpu
        target:
          type: Utilization
          averageUtilization: 70
  EOF

□ NodeLocalDNS 확인
  # Kubespray가 설치했는지 확인
  kubectl -n kube-system get ds nodelocaldns
  
  # 모든 노드에 Running 확인
  kubectl -n kube-system get pods -l k8s-app=node-local-dns
```

---

## 2.8 Phase 6: 보안 구성

### 2.8.1 RBAC 설정

```yaml
□ 기본 ClusterRole 확인
  kubectl get clusterrole
  # admin, edit, view 등 기본 Role 존재

□ 팀별 Namespace 생성
  kubectl create namespace team-a
  kubectl create namespace team-b
  kubectl create namespace team-c
  kubectl create namespace team-d
  
  # 공통 Namespace
  kubectl create namespace spark-operator-system
  kubectl create namespace argocd
  kubectl create namespace ci-cd
  kubectl create namespace monitoring
  kubectl create namespace logging
  kubectl create namespace ingress-system

□ 팀별 Role 생성
  # Team Admin Role (Namespace 내 모든 권한)
  cat <<EOF | kubectl apply -f -
  apiVersion: rbac.authorization.k8s.io/v1
  kind: Role
  metadata:
    name: team-admin
    namespace: team-a
  rules:
  - apiGroups: ["*"]
    resources: ["*"]
    verbs: ["*"]
  EOF
  
  # 나머지 팀도 동일하게 생성
  for ns in team-b team-c team-d; do
    kubectl create role team-admin --verb='*' --resource='*' -n $ns
  done
  
  # Team Developer Role
  cat <<EOF | kubectl apply -f -
  apiVersion: rbac.authorization.k8s.io/v1
  kind: Role
  metadata:
    name: team-developer
    namespace: team-a
  rules:
  - apiGroups: ["", "apps", "batch"]
    resources: ["pods", "services", "deployments", "jobs", "cronjobs", "configmaps", "secrets"]
    verbs: ["get", "list", "watch", "create", "update", "patch", "delete"]
  - apiGroups: [""]
    resources: ["pods/log", "pods/exec"]
    verbs: ["get", "list"]
  EOF
  
  # 나머지 팀도 동일

□ Keycloak 그룹과 RoleBinding
  # Keycloak에서 그룹 생성:
  # /team-a/admins
  # /team-a/developers
  # /team-a/viewers
  # ... (team-b, c, d 동일)
  
  # RoleBinding (예: team-a admins)
  cat <<EOF | kubectl apply -f -
  apiVersion: rbac.authorization.k8s.io/v1
  kind: RoleBinding
  metadata:
    name: team-a-admins
    namespace: team-a
  subjects:
  - kind: Group
    name: /team-a/admins
    apiGroup: rbac.authorization.k8s.io
  roleRef:
    kind: Role
    name: team-admin
    apiGroup: rbac.authorization.k8s.io
  EOF
  
  # 모든 팀/역할 조합 생성

□ ClusterRole (읽기 전용)
  cat <<EOF | kubectl apply -f -
  apiVersion: rbac.authorization.k8s.io/v1
  kind: ClusterRole
  metadata:
    name: cluster-viewer
  rules:
  - apiGroups: [""]
    resources: ["nodes", "namespaces", "persistentvolumes"]
    verbs: ["get", "list", "watch"]
  EOF
  
  # ClusterRoleBinding
  cat <<EOF | kubectl apply -f -
  apiVersion: rbac.authorization.k8s.io/v1
  kind: ClusterRoleBinding
  metadata:
    name: all-users-viewer
  subjects:
  - kind: Group
    name: /all-users
    apiGroup: rbac.authorization.k8s.io
  roleRef:
    kind: ClusterRole
    name: cluster-viewer
    apiGroup: rbac.authorization.k8s.io
  EOF
```

### 2.8.2 Keycloak 통합

```yaml
□ Keycloak 정보 확인
  # 외부 클러스터의 Keycloak 엔드포인트
  # 예: https://keycloak.example.com
  # Realm: kubernetes
  # Client ID: kubernetes-cluster

□ OIDC 설정 (kube-apiserver)
  # Control Plane 노드에서 설정
  # /etc/kubernetes/manifests/kube-apiserver.yaml 수정
  
  ssh k8s-master01
  sudo vi /etc/kubernetes/manifests/kube-apiserver.yaml
  
  # spec.containers[0].command에 추가:
  - --oidc-issuer-url=https://keycloak.example.com/realms/kubernetes
  - --oidc-client-id=kubernetes-cluster
  - --oidc-username-claim=email
  - --oidc-groups-claim=groups
  - --oidc-ca-file=/etc/kubernetes/ssl/ca.crt  # Keycloak CA
  
  # API Server 자동 재시작 대기
  
  # 나머지 Control Plane 노드도 동일하게 설정

□ kubectl OIDC 플러그인 설치
  # kubelogin 설치
  wget https://github.com/int128/kubelogin/releases/download/v1.28.0/kubelogin_linux_amd64.zip
  unzip kubelogin_linux_amd64.zip
  sudo mv kubelogin /usr/local/bin/kubectl-oidc_login
  chmod +x /usr/local/bin/kubectl-oidc_login

□ kubeconfig 설정 (사용자용)
  cat <<EOF > ~/.kube/config-oidc
  apiVersion: v1
  kind: Config
  clusters:
  - name: kubernetes
    cluster:
      server: https://k8s-api.example.com:6443
      certificate-authority-data: <CA_DATA>
  users:
  - name: oidc
    user:
      exec:
        apiVersion: client.authentication.k8s.io/v1beta1
        command: kubectl
        args:
        - oidc-login
        - get-token
        - --oidc-issuer-url=https://keycloak.example.com/realms/kubernetes
        - --oidc-client-id=kubernetes-cluster
        - --oidc-client-secret=<CLIENT_SECRET>
  contexts:
  - name: kubernetes
    context:
      cluster: kubernetes
      user: oidc
  current-context: kubernetes
  EOF

□ 인증 테스트
  export KUBECONFIG=~/.kube/config-oidc
  kubectl get nodes
  # 브라우저에서 Keycloak 로그인
  # 토큰 받아서 인증 성공
```

### 2.8.3 Pod Security Standards

```yaml
□ Pod Security Admission 활성화
  # Namespace Label로 정책 적용
  
  # 팀별 Namespace: restricted
  for ns in team-a team-b team-c team-d; do
    kubectl label namespace $ns \
      pod-security.kubernetes.io/enforce=restricted \
      pod-security.kubernetes.io/audit=restricted \
      pod-security.kubernetes.io/warn=restricted
  done
  
  # 공통 시스템 Namespace: privileged
  for ns in kube-system monitoring logging rook-ceph; do
    kubectl label namespace $ns \
      pod-security.kubernetes.io/enforce=privileged \
      pod-security.kubernetes.io/audit=privileged \
      pod-security.kubernetes.io/warn=privileged
  done
  
  # CI/CD, ArgoCD: baseline
  for ns in argocd ci-cd spark-operator-system; do
    kubectl label namespace $ns \
      pod-security.kubernetes.io/enforce=baseline \
      pod-security.kubernetes.io/audit=baseline \
      pod-security.kubernetes.io/warn=baseline
  done

□ 정책 테스트
  # Privileged Pod (실패해야 함)
  cat <<EOF | kubectl apply -n team-a -f -
  apiVersion: v1
  kind: Pod
  metadata:
    name: test-privileged
  spec:
    containers:
    - name: test
      image: nginx
      securityContext:
        privileged: true
  EOF
  # 에러 발생 예상: restricted 정책 위반
  
  # 정상 Pod
  cat <<EOF | kubectl apply -n team-a -f -
  apiVersion: v1
  kind: Pod
  metadata:
    name: test-normal
  spec:
    securityContext:
      runAsNonRoot: true
      runAsUser: 1000
      seccompProfile:
        type: RuntimeDefault
    containers:
    - name: test
      image: nginx
      securityContext:
        allowPrivilegeEscalation: false
        capabilities:
          drop:
          - ALL
  EOF
  # 성공
  
  kubectl delete pod test-normal -n team-a
```

### 2.8.4 Network Policy

```yaml
□ 기본 NetworkPolicy (Deny All)
  # 각 팀 Namespace에 적용
  for ns in team-a team-b team-c team-d; do
    cat <<EOF | kubectl apply -f -
  apiVersion: networking.k8s.io/v1
  kind: NetworkPolicy
  metadata:
    name: default-deny-all
    namespace: $ns
  spec:
    podSelector: {}
    policyTypes:
    - Ingress
    - Egress
  EOF
  done

□ 팀별 NetworkPolicy (같은 Namespace 내 통신 허용)
  for ns in team-a team-b team-c team-d; do
    cat <<EOF | kubectl apply -f -
  apiVersion: networking.k8s.io/v1
  kind: NetworkPolicy
  metadata:
    name: allow-same-namespace
    namespace: $ns
  spec:
    podSelector: {}
    policyTypes:
    - Ingress
    - Egress
    ingress:
    - from:
      - podSelector: {}
    egress:
    - to:
      - podSelector: {}
  EOF
  done

□ DNS 허용
  for ns in team-a team-b team-c team-d; do
    cat <<EOF | kubectl apply -f -
  apiVersion: networking.k8s.io/v1
  kind: NetworkPolicy
  metadata:
    name: allow-dns
    namespace: $ns
  spec:
    podSelector: {}
    policyTypes:
    - Egress
    egress:
    - to:
      - namespaceSelector:
          matchLabels:
            kubernetes.io/metadata.name: kube-system
      ports:
      - protocol: UDP
        port: 53
  EOF
  done

□ 외부 MinIO 접근 허용
  for ns in team-a team-b team-c team-d; do
    cat <<EOF | kubectl apply -f -
  apiVersion: networking.k8s.io/v1
  kind: NetworkPolicy
  metadata:
    name: allow-minio
    namespace: $ns
  spec:
    podSelector: {}
    policyTypes:
    - Egress
    egress:
    - to:
      - ipBlock:
          cidr: 10.200.0.0/16  # MinIO 클러스터 IP 범위
      ports:
      - protocol: TCP
        port: 9000  # MinIO S3 API
  EOF
  done

□ 외부 Keycloak 접근 허용
  for ns in team-a team-b team-c team-d; do
    cat <<EOF | kubectl apply -f -
  apiVersion: networking.k8s.io/v1
  kind: NetworkPolicy
  metadata:
    name: allow-keycloak
    namespace: $ns
  spec:
    podSelector: {}
    policyTypes:
    - Egress
    egress:
    - to:
      - ipBlock:
          cidr: 10.200.100.0/24  # Keycloak IP 범위
      ports:
      - protocol: TCP
        port: 443
  EOF
  done

□ 공통 시스템 접근 허용 (Spark Operator 등)
  for ns in team-a team-b team-c team-d; do
    cat <<EOF | kubectl apply -f -
  apiVersion: networking.k8s.io/v1
  kind: NetworkPolicy
  metadata:
    name: allow-common-systems
    namespace: $ns
  spec:
    podSelector: {}
    policyTypes:
    - Egress
    egress:
    - to:
      - namespaceSelector:
          matchLabels:
            name: spark-operator-system
    - to:
      - namespaceSelector:
          matchLabels:
            name: argocd
  EOF
  done

□ NetworkPolicy 검증
  # Cilium으로 검증
  cilium connectivity test --test-namespace team-a
```

### 2.8.5 Secret 관리

```yaml
□ etcd 암호화 설정
  # Control Plane 노드에서
  # 암호화 키 생성
  head -c 32 /dev/urandom | base64
  
  # EncryptionConfiguration 생성
  cat <<EOF > /etc/kubernetes/encryption-config.yaml
  apiVersion: apiserver.config.k8s.io/v1
  kind: EncryptionConfiguration
  resources:
  - resources:
    - secrets
    providers:
    - aescbc:
        keys:
        - name: key1
          secret: <BASE64_ENCODED_SECRET>
    - identity: {}
  EOF
  
  # kube-apiserver.yaml 수정
  # --encryption-provider-config=/etc/kubernetes/encryption-config.yaml
  # volumeMounts와 volumes 추가
  
  # 모든 Control Plane 노드에 적용

□ Sealed Secrets 설치 (선택)
  # Controller 설치
  kubectl apply -f https://github.com/bitnami-labs/sealed-secrets/releases/download/v0.24.5/controller.yaml
  
  # kubeseal CLI 설치
  wget https://github.com/bitnami-labs/sealed-secrets/releases/download/v0.24.5/kubeseal-0.24.5-linux-amd64.tar.gz
  tar xfz kubeseal-0.24.5-linux-amd64.tar.gz
  sudo mv kubeseal /usr/local/bin/
  
  # 사용 예시
  echo -n mypassword | kubectl create secret generic mysecret \
    --dry-run=client --from-file=password=/dev/stdin -o yaml | \
    kubeseal -o yaml > mysealedsecret.yaml
  
  kubectl apply -f mysealedsecret.yaml
```

---

## 2.9 Phase 7: 모니터링 및 로깅

### 2.9.1 Prometheus Stack 설치

```yaml
□ kube-prometheus-stack Helm Chart 설치
  # Helm repo 추가
  helm repo add prometheus-community https://prometheus-community.github.io/helm-charts
  helm repo update
  
  # Values 파일 생성
  cat <<EOF > prometheus-values.yaml
  prometheus:
    prometheusSpec:
      replicas: 2
      retention: 30d
      retentionSize: "450GB"
      storageSpec:
        volumeClaimTemplate:
          spec:
            storageClassName: ceph-block
            accessModes: ["ReadWriteOnce"]
            resources:
              requests:
                storage: 500Gi
      resources:
        requests:
          cpu: 2
          memory: 8Gi
        limits:
          cpu: 4
          memory: 16Gi
      nodeSelector:
        node-role.kubernetes.io/infra: "true"
      tolerations:
      - key: node-role.kubernetes.io/infra
        operator: Equal
        value: "true"
        effect: NoSchedule
  
  alertmanager:
    alertmanagerSpec:
      replicas: 3
      storage:
        volumeClaimTemplate:
          spec:
            storageClassName: ceph-block
            accessModes: ["ReadWriteOnce"]
            resources:
              requests:
                storage: 10Gi
      nodeSelector:
        node-role.kubernetes.io/infra: "true"
      tolerations:
      - key: node-role.kubernetes.io/infra
        operator: Equal
        value: "true"
        effect: NoSchedule
  
  grafana:
    replicas: 2
    persistence:
      enabled: true
      storageClassName: ceph-block
      size: 10Gi
    adminPassword: <ADMIN_PASSWORD>
    nodeSelector:
      node-role.kubernetes.io/infra: "true"
    tolerations:
    - key: node-role.kubernetes.io/infra
      operator: Equal
      value: "true"
      effect: NoSchedule
    ingress:
      enabled: true
      ingressClassName: cilium
      hosts:
      - grafana.example.com
  
  prometheus-node-exporter:
    hostNetwork: true
    hostPID: true
  
  kube-state-metrics:
    replicas: 2
  EOF
  
  # 설치
  helm install prometheus prometheus-community/kube-prometheus-stack \
    -n monitoring \
    -f prometheus-values.yaml \
    --create-namespace

□ Prometheus 확인
  kubectl -n monitoring get pods
  # prometheus, alertmanager, grafana 모두 Running
  
  kubectl -n monitoring get svc
  # prometheus, alertmanager, grafana 서비스 확인

□ Grafana 접근
  # Ingress 사용
  # https://grafana.example.com
  # Username: admin
  # Password: prometheus-values.yaml에 설정한 값
  
  # 또는 Port Forward
  kubectl -n monitoring port-forward svc/prometheus-grafana 3000:80

□ 기본 대시보드 확인
  # Grafana에서 Dashboards 확인
  # - Kubernetes / Compute Resources / Cluster
  # - Kubernetes / Compute Resources / Namespace (Pods)
  # - Node Exporter / Nodes
  # - etcd

□ 팀별 Namespace 모니터링 대시보드 생성
  # Grafana에서 Dashboard 생성
  # 변수: namespace (team-a, team-b, team-c, team-d)
  # 패널: CPU, Memory, Network, Pod 수 등

□ ServiceMonitor 확인
  kubectl -n monitoring get servicemonitor
  # kube-state-metrics, node-exporter 등
  
  # Custom ServiceMonitor 예시 (애플리케이션 메트릭)
  cat <<EOF | kubectl apply -f -
  apiVersion: monitoring.coreos.com/v1
  kind: ServiceMonitor
  metadata:
    name: spark-metrics
    namespace: spark-operator-system
  spec:
    selector:
      matchLabels:
        app: spark
    endpoints:
    - port: metrics
      interval: 30s
  EOF
```

### 2.9.2 OpenSearch 설치

```yaml
□ OpenSearch Operator 설치
  kubectl apply -f https://github.com/opensearch-project/opensearch-k8s-operator/releases/download/v2.5.0/opensearch-operator.yaml

□ OpenSearch Cluster 생성
  cat <<EOF | kubectl apply -f -
  apiVersion: opensearch.opster.io/v1
  kind: OpenSearchCluster
  metadata:
    name: opensearch-cluster
    namespace: logging
  spec:
    general:
      serviceName: opensearch-cluster
      version: 2.11.0
      httpPort: 9200
      vendor: opensearch
      pluginsList: []
    dashboards:
      enable: true
      version: 2.11.0
      replicas: 2
      resources:
        requests:
          cpu: 1
          memory: 2Gi
        limits:
          cpu: 2
          memory: 4Gi
    nodePools:
    - component: masters
      replicas: 3
      diskSize: "50Gi"
      storageClass: ceph-block
      nodeSelector:
        node-role.kubernetes.io/infra: "true"
      tolerations:
      - key: node-role.kubernetes.io/infra
        operator: Equal
        value: "true"
        effect: NoSchedule
      resources:
        requests:
          cpu: 2
          memory: 4Gi
        limits:
          cpu: 4
          memory: 8Gi
      roles:
      - "cluster_manager"
      - "data"
    - component: data
      replicas: 5
      diskSize: "500Gi"
      storageClass: ceph-block
      nodeSelector:
        node-role.kubernetes.io/infra: "true"
      tolerations:
      - key: node-role.kubernetes.io/infra
        operator: Equal
        value: "true"
        effect: NoSchedule
      resources:
        requests:
          cpu: 4
          memory: 16Gi
        limits:
          cpu: 8
          memory: 32Gi
      roles:
      - "data"
  EOF

□ OpenSearch 확인
  kubectl -n logging get pods
  kubectl -n logging get svc
  
  # admin 비밀번호 확인
  kubectl -n logging get secret opensearch-cluster-admin-password \
    -o jsonpath='{.data.password}' | base64 -d

□ Fluent-bit 설치
  # Helm repo 추가
  helm repo add fluent https://fluent.github.io/helm-charts
  helm repo update
  
  # Values 파일
  cat <<EOF > fluent-bit-values.yaml
  image:
    repository: fluent/fluent-bit
    tag: 2.2.0
  
  config:
    service: |
      [SERVICE]
          Daemon Off
          Flush 1
          Log_Level info
          Parsers_File parsers.conf
          HTTP_Server On
          HTTP_Listen 0.0.0.0
          HTTP_Port 2020
          Health_Check On
    
    inputs: |
      [INPUT]
          Name tail
          Path /var/log/containers/*.log
          multiline.parser docker, cri
          Tag kube.*
          Mem_Buf_Limit 5MB
          Skip_Long_Lines On
    
    filters: |
      [FILTER]
          Name kubernetes
          Match kube.*
          Kube_URL https://kubernetes.default.svc:443
          Kube_CA_File /var/run/secrets/kubernetes.io/serviceaccount/ca.crt
          Kube_Token_File /var/run/secrets/kubernetes.io/serviceaccount/token
          Kube_Tag_Prefix kube.var.log.containers.
          Merge_Log On
          Keep_Log Off
          K8S-Logging.Parser On
          K8S-Logging.Exclude On
      
      [FILTER]
          Name nest
          Match kube.*
          Operation lift
          Nested_under kubernetes
          Add_prefix k8s_
    
    outputs: |
      [OUTPUT]
          Name opensearch
          Match kube.*
          Host opensearch-cluster.logging.svc.cluster.local
          Port 9200
          HTTP_User admin
          HTTP_Passwd <ADMIN_PASSWORD>
          Logstash_Format On
          Logstash_Prefix kubernetes
          Retry_Limit False
          tls On
          tls.verify Off
          Suppress_Type_Name On
  
  tolerations:
  - operator: Exists
  EOF
  
  # 설치
  helm install fluent-bit fluent/fluent-bit \
    -n logging \
    -f fluent-bit-values.yaml

□ OpenSearch Dashboards 접근
  # Port Forward
  kubectl -n logging port-forward svc/opensearch-cluster-dashboards 5601:5601
  
  # 브라우저: http://localhost:5601
  # Username: admin
  # Password: (위에서 확인한 값)
  
  # Index Pattern 생성
  # Management > Index Patterns > Create
  # Pattern: kubernetes-*

□ 팀별 Index 분리 (선택사항)
  # Fluent-bit 설정에 추가
  [FILTER]
      Name rewrite_tag
      Match kube.*
      Rule $k8s_namespace_name ^team-a$ team-a.logs false
      Rule $k8s_namespace_name ^team-b$ team-b.logs false
      Rule $k8s_namespace_name ^team-c$ team-c.logs false
      Rule $k8s_namespace_name ^team-d$ team-d.logs false
  
  [OUTPUT]
      Name opensearch
      Match team-a.logs
      Host opensearch-cluster.logging.svc.cluster.local
      Port 9200
      Logstash_Prefix team-a
      # ... 기타 설정
```

---

## 2.10 Phase 8: 백업 구성

### 2.10.1 etcd 백업

```yaml
□ etcd 백업 스크립트 생성
  cat <<'EOF' > /usr/local/bin/backup-etcd.sh
  #!/bin/bash
  set -euo pipefail
  
  BACKUP_DIR="/var/backups/etcd"
  RETENTION_DAYS=30
  DATE=$(date +%Y%m%d-%H%M%S)
  BACKUP_FILE="$BACKUP_DIR/etcd-snapshot-$DATE.db"
  
  mkdir -p $BACKUP_DIR
  
  # etcd 백업
  ETCDCTL_API=3 etcdctl snapshot save $BACKUP_FILE \
    --endpoints=https://127.0.0.1:2379 \
    --cacert=/etc/ssl/etcd/ssl/ca.pem \
    --cert=/etc/ssl/etcd/ssl/node-$(hostname).pem \
    --key=/etc/ssl/etcd/ssl/node-$(hostname)-key.pem
  
  # 백업 검증
  ETCDCTL_API=3 etcdctl snapshot status $BACKUP_FILE
  
  # 압축
  gzip $BACKUP_FILE
  
  # 오래된 백업 삭제
  find $BACKUP_DIR -name "etcd-snapshot-*.db.gz" -mtime +$RETENTION_DAYS -delete
  
  # 원격 저장소로 복사 (선택)
  # rsync -avz $BACKUP_FILE.gz backup-server:/backups/etcd/
  
  echo "Backup completed: $BACKUP_FILE.gz"
  EOF
  
  chmod +x /usr/local/bin/backup-etcd.sh

□ Cron Job 설정
  # Control Plane 노드에서
  crontab -e
  
  # 매일 3AM 백업
  0 3 * * * /usr/local/bin/backup-etcd.sh >> /var/log/etcd-backup.log 2>&1
  
  # 또는 Kubernetes CronJob
  cat <<EOF | kubectl apply -f -
  apiVersion: batch/v1
  kind: CronJob
  metadata:
    name: etcd-backup
    namespace: kube-system
  spec:
    schedule: "0 3 * * *"
    jobTemplate:
      spec:
        template:
          spec:
            hostNetwork: true
            nodeName: k8s-master01
            containers:
            - name: backup
              image: registry.k8s.io/etcd:3.5.10-0
              command:
              - /bin/sh
              - -c
              - |
                ETCDCTL_API=3 etcdctl snapshot save /backup/etcd-snapshot-\$(date +%Y%m%d-%H%M%S).db \
                  --endpoints=https://127.0.0.1:2379 \
                  --cacert=/etc/kubernetes/pki/etcd/ca.crt \
                  --cert=/etc/kubernetes/pki/etcd/server.crt \
                  --key=/etc/kubernetes/pki/etcd/server.key
                find /backup -name "etcd-snapshot-*.db" -mtime +30 -delete
              volumeMounts:
              - name: etcd-certs
                mountPath: /etc/kubernetes/pki/etcd
                readOnly: true
              - name: backup
                mountPath: /backup
            restartPolicy: OnFailure
            volumes:
            - name: etcd-certs
              hostPath:
                path: /etc/kubernetes/pki/etcd
            - name: backup
              hostPath:
                path: /var/backups/etcd
  EOF

□ 복구 절차 문서화
  # 복구 스크립트 예시
  cat <<'EOF' > /usr/local/bin/restore-etcd.sh
  #!/bin/bash
  # etcd 복구 스크립트
  # 주의: 클러스터 전체 중단 필요
  
  BACKUP_FILE=$1
  
  if [ -z "$BACKUP_FILE" ]; then
    echo "Usage: $0 <backup-file>"
    exit 1
  fi
  
  # etcd 중지
  systemctl stop etcd
  
  # 기존 데이터 백업
  mv /var/lib/etcd /var/lib/etcd.backup-$(date +%Y%m%d)
  
  # 복구
  ETCDCTL_API=3 etcdctl snapshot restore $BACKUP_FILE \
    --data-dir=/var/lib/etcd \
    --name=$(hostname) \
    --initial-cluster=k8s-master01=https://10.0.0.1:2380,k8s-master02=https://10.0.0.2:2380,k8s-master03=https://10.0.0.3:2380,k8s-master04=https://10.0.0.4:2380,k8s-master05=https://10.0.0.5:2380 \
    --initial-advertise-peer-urls=https://$(hostname -i):2380
  
  # 권한 설정
  chown -R etcd:etcd /var/lib/etcd
  
  # etcd 시작
  systemctl start etcd
  
  echo "Restore completed. Check etcd status."
  EOF
  
  chmod +x /usr/local/bin/restore-etcd.sh
```

### 2.10.2 Velero 설치

```yaml
□ MinIO 버킷 생성 (백업 저장소)
  # 외부 MinIO에 버킷 생성
  # 버킷명: velero-backups

□ Velero CLI 설치
  wget https://github.com/vmware-tanzu/velero/releases/download/v1.12.3/velero-v1.12.3-linux-amd64.tar.gz
  tar -xvf velero-v1.12.3-linux-amd64.tar.gz
  sudo mv velero-v1.12.3-linux-amd64/velero /usr/local/bin/
  
□ Credentials 파일 생성
  cat <<EOF > credentials-velero
  [default]
  aws_access_key_id = <MINIO_ACCESS_KEY>
  aws_secret_access_key = <MINIO_SECRET_KEY>
  EOF

□ Velero 설치
  velero install \
    --provider aws \
    --plugins velero/velero-plugin-for-aws:v1.8.2 \
    --bucket velero-backups \
    --secret-file ./credentials-velero \
    --use-volume-snapshots=false \
    --backup-location-config region=minio,s3ForcePathStyle="true",s3Url=http://minio.example.com:9000 \
    --namespace velero

□ Velero 확인
  kubectl -n velero get pods
  velero version
  velero backup-location get

□ 백업 스케줄 생성
  # 전체 클러스터 백업 (주 1회)
  velero schedule create weekly-full-backup \
    --schedule="0 2 * * 0" \
    --ttl 720h
  
  # 팀별 Namespace 백업 (일 1회)
  velero schedule create daily-team-a-backup \
    --schedule="0 1 * * *" \
    --include-namespaces team-a \
    --ttl 168h
  
  # 동일하게 team-b, c, d 생성
  
  # 공통 시스템 백업 (일 1회)
  velero schedule create daily-common-backup \
    --schedule="0 1 * * *" \
    --include-namespaces argocd,ci-cd,spark-operator-system \
    --ttl 168h

□ 수동 백업 테스트
  # 특정 Namespace 백업
  velero backup create test-backup --include-namespaces team-a
  
  # 백업 상태 확인
  velero backup describe test-backup
  velero backup logs test-backup
  
  # 백업 목록
  velero backup get

□ 복구 테스트
  # 테스트 Namespace 생성 후 삭제
  kubectl create namespace velero-test
  kubectl create deployment nginx --image=nginx -n velero-test
  kubectl delete namespace velero-test
  
  # 복구
  velero restore create --from-backup test-backup
  
  # 복구 확인
  kubectl get ns velero-test
  kubectl get deployment -n velero-test
  
  # 정리
  kubectl delete namespace velero-test
```

---

## 2.11 Phase 9: CI/CD 구성

### 2.11.1 ArgoCD 설치

```yaml
□ ArgoCD 설치
  kubectl create namespace argocd
  kubectl apply -n argocd -f https://raw.githubusercontent.com/argoproj/argo-cd/v2.9.5/manifests/install.yaml

□ ArgoCD CLI 설치
  curl -sSL -o argocd-linux-amd64 https://github.com/argoproj/argo-cd/releases/download/v2.9.5/argocd-linux-amd64
  sudo install -m 555 argocd-linux-amd64 /usr/local/bin/argocd
  rm argocd-linux-amd64

□ ArgoCD 접근
  # Admin 비밀번호 확인
  kubectl -n argocd get secret argocd-initial-admin-secret \
    -o jsonpath="{.data.password}" | base64 -d; echo
  
  # Port Forward
  kubectl port-forward svc/argocd-server -n argocd 8080:443
  
  # 브라우저: https://localhost:8080
  # Username: admin
  # Password: (위에서 확인한 값)

□ Ingress 설정
  cat <<EOF | kubectl apply -f -
  apiVersion: networking.k8s.io/v1
  kind: Ingress
  metadata:
    name: argocd-server
    namespace: argocd
    annotations:
      nginx.ingress.kubernetes.io/ssl-passthrough: "true"
      nginx.ingress.kubernetes.io/backend-protocol: "HTTPS"
  spec:
    ingressClassName: cilium
    rules:
    - host: argocd.example.com
      http:
        paths:
        - path: /
          pathType: Prefix
          backend:
            service:
              name: argocd-server
              port:
                number: 443
    tls:
    - hosts:
      - argocd.example.com
      secretName: argocd-server-tls
  EOF

□ Keycloak SSO 연동
  # ArgoCD ConfigMap 수정
  kubectl -n argocd edit cm argocd-cm
  
  # data 섹션에 추가:
  data:
    url: https://argocd.example.com
    oidc.config: |
      name: Keycloak
      issuer: https://keycloak.example.com/realms/kubernetes
      clientID: argocd
      clientSecret: <CLIENT_SECRET>
      requestedScopes: ["openid", "profile", "email", "groups"]

□ RBAC 설정
  # ArgoCD RBAC ConfigMap
  kubectl -n argocd edit cm argocd-rbac-cm
  
  # data 섹션:
  data:
    policy.csv: |
      g, /team-a/admins, role:admin
      p, role:team-a-admin, applications, *, team-a/*, allow
      g, /team-a/developers, role:team-a-developer
      # ... 각 팀별 설정

□ 팀별 AppProject 생성
  cat <<EOF | kubectl apply -f -
  apiVersion: argoproj.io/v1alpha1
  kind: AppProject
  metadata:
    name: team-a
    namespace: argocd
  spec:
    description: Team A Project
    sourceRepos:
    - '*'
    destinations:
    - namespace: team-a
      server: https://kubernetes.default.svc
    clusterResourceWhitelist:
    - group: ''
      kind: Namespace
    namespaceResourceWhitelist:
    - group: '*'
      kind: '*'
  EOF
  
  # team-b, c, d도 동일하게 생성
```

### 2.11.2 Jenkins 설치

```yaml
□ Jenkins 설치 (Helm)
  helm repo add jenkins https://charts.jenkins.io
  helm repo update
  
  # Values 파일
  cat <<EOF > jenkins-values.yaml
  controller:
    adminPassword: <ADMIN_PASSWORD>
    resources:
      requests:
        cpu: 2
        memory: 4Gi
      limits:
        cpu: 4
        memory: 8Gi
    nodeSelector:
      node-role.kubernetes.io/infra: "true"
    tolerations:
    - key: node-role.kubernetes.io/infra
      operator: Equal
      value: "true"
      effect: NoSchedule
    installPlugins:
    - kubernetes:4029.v5712230ccb_f8
    - workflow-aggregator:596.v8c21c963d92d
    - git:5.1.0
    - configuration-as-code:1670.v564dc8b_982d0
    - credentials-binding:631.v861fa_8386a_f8
    - kubernetes-cli:1.12.0
    ingress:
      enabled: true
      ingressClassName: cilium
      hostName: jenkins.example.com
    serviceAccount:
      create: true
      name: jenkins
    persistence:
      enabled: true
      storageClass: ceph-block
      size: 100Gi
  
  agent:
    enabled: true
    podName: jenkins-agent
    customJenkinsLabels: []
    podRetention: "Never"
    resources:
      requests:
        cpu: 500m
        memory: 1Gi
      limits:
        cpu: 2
        memory: 4Gi
  EOF
  
  # 설치
  helm install jenkins jenkins/jenkins \
    -n ci-cd \
    --create-namespace \
    -f jenkins-values.yaml

□ Jenkins 접근
  # Admin 비밀번호 확인 (values에서 설정)
  # 또는
  kubectl -n ci-cd get secret jenkins \
    -o jsonpath="{.data.jenkins-admin-password}" | base64 -d; echo
  
  # Ingress: https://jenkins.example.com
  # Username: admin
  # Password: (위 비밀번호)

□ Kubernetes Plugin 설정
  # Jenkins > Manage Jenkins > Manage Nodes and Clouds > Configure Clouds
  # Kubernetes 클라우드 추가
  # Kubernetes URL: https://kubernetes.default.svc
  # Kubernetes Namespace: ci-cd
  # Jenkins URL: http://jenkins.ci-cd.svc.cluster.local:8080

□ 팀별 Credential 추가
  # Jenkins > Manage Jenkins > Manage Credentials
  # 각 팀별 Git, Registry Credential 추가

□ Pipeline 예시
  cat <<'EOF' > Jenkinsfile
  pipeline {
    agent {
      kubernetes {
        yaml '''
          apiVersion: v1
          kind: Pod
          spec:
            containers:
            - name: maven
              image: maven:3.8.6-openjdk-11
              command:
              - cat
              tty: true
            - name: docker
              image: docker:24.0.7
              command:
              - cat
              tty: true
              volumeMounts:
              - name: docker-sock
                mountPath: /var/run/docker.sock
            volumes:
            - name: docker-sock
              hostPath:
                path: /var/run/docker.sock
        '''
      }
    }
    stages {
      stage('Build') {
        steps {
          container('maven') {
            sh 'mvn clean package'
          }
        }
      }
      stage('Docker Build') {
        steps {
          container('docker') {
            sh 'docker build -t myapp:${BUILD_NUMBER} .'
          }
        }
      }
      stage('Deploy') {
        steps {
          sh 'kubectl apply -f k8s/deployment.yaml'
        }
      }
    }
  }
  EOF
```

---

## 2.12 Phase 10: 공통 시스템 설치

### 2.12.1 Spark Operator 설치

```yaml
□ Spark Operator 설치
  helm repo add spark-operator https://googlecloudplatform.github.io/spark-on-k8s-operator
  helm repo update
  
  helm install spark-operator spark-operator/spark-operator \
    --namespace spark-operator-system \
    --create-namespace \
    --set webhook.enable=true \
    --set sparkJobNamespace=team-a,team-b,team-c,team-d

□ Spark Operator 확인
  kubectl -n spark-operator-system get pods
  kubectl -n spark-operator-system get crd
  # sparkapplications.sparkoperator.k8s.io

□ 팀별 ServiceAccount 생성
  for ns in team-a team-b team-c team-d; do
    kubectl create serviceaccount spark -n $ns
    
    kubectl create role spark-role --verb=get,list,watch,create,update,patch,delete \
      --resource=pods,services,configmaps -n $ns
    
    kubectl create rolebinding spark-rolebinding \
      --role=spark-role --serviceaccount=$ns:spark -n $ns
  done

□ Spark 애플리케이션 테스트
  cat <<EOF | kubectl apply -f -
  apiVersion: sparkoperator.k8s.io/v1beta2
  kind: SparkApplication
  metadata:
    name: spark-pi
    namespace: team-a
  spec:
    type: Scala
    mode: cluster
    image: "gcr.io/spark-operator/spark:v3.5.0"
    imagePullPolicy: Always
    mainClass: org.apache.spark.examples.SparkPi
    mainApplicationFile: "local:///opt/spark/examples/jars/spark-examples_2.12-3.5.0.jar"
    sparkVersion: "3.5.0"
    restartPolicy:
      type: Never
    driver:
      cores: 1
      coreLimit: "1200m"
      memory: "512m"
      labels:
        version: 3.5.0
      serviceAccount: spark
    executor:
      cores: 1
      instances: 2
      memory: "512m"
      labels:
        version: 3.5.0
  EOF
  
  # 상태 확인
  kubectl get sparkapplication spark-pi -n team-a
  kubectl logs <spark-driver-pod> -n team-a
  
  # 정리
  kubectl delete sparkapplication spark-pi -n team-a
```

---

## 2.13 Phase 11: 멀티테넌시 최종 구성

### 2.13.1 ResourceQuota 설정

```yaml
□ 팀별 ResourceQuota 생성
  # Team A (180노드의 1/4 = 45노드)
  cat <<EOF | kubectl apply -f -
  apiVersion: v1
  kind: ResourceQuota
  metadata:
    name: team-a-quota
    namespace: team-a
  spec:
    hard:
      requests.cpu: "2000"
      requests.memory: "32Ti"
      limits.cpu: "2200"
      limits.memory: "35Ti"
      persistentvolumeclaims: "500"
      pods: "10000"
      services: "100"
      services.loadbalancers: "10"
      services.nodeports: "20"
  EOF
  
  # Team B, C, D도 동일하게 생성

□ LimitRange 설정
  for ns in team-a team-b team-c team-d; do
    cat <<EOF | kubectl apply -f -
  apiVersion: v1
  kind: LimitRange
  metadata:
    name: resource-limits
    namespace: $ns
  spec:
    limits:
    - max:
        cpu: "48"
        memory: "700Gi"
      min:
        cpu: "100m"
        memory: "128Mi"
      default:
        cpu: "1"
        memory: "2Gi"
      defaultRequest:
        cpu: "500m"
        memory: "1Gi"
      type: Container
    - max:
        cpu: "96"
        memory: "700Gi"
      min:
        cpu: "100m"
        memory: "128Mi"
      type: Pod
  EOF
  done

□ ResourceQuota 검증
  # 제한 초과 Pod 생성 시도 (실패해야 함)
  kubectl run test-quota --image=nginx --requests=cpu=3000 -n team-a
  # Error: exceeded quota

□ Quota 사용률 모니터링
  kubectl describe resourcequota -n team-a
  kubectl get resourcequota -A
```

### 2.13.2 Admission Controller 설정

```yaml
□ LimitRange Enforcer
  # 이미 설정됨 (kube-apiserver admission plugins)

□ PodSecurity Admission
  # 이미 설정됨 (Namespace Label)

□ Custom Admission Webhook (선택사항)
  # request/limit 없는 Pod 차단
  # 예: OPA Gatekeeper 또는 Kyverno
  
  # Kyverno 설치
  helm repo add kyverno https://kyverno.github.io/kyverno/
  helm repo update
  helm install kyverno kyverno/kyverno -n kyverno --create-namespace
  
  # Policy: require resources
  cat <<EOF | kubectl apply -f -
  apiVersion: kyverno.io/v1
  kind: ClusterPolicy
  metadata:
    name: require-requests-limits
  spec:
    validationFailureAction: enforce
    background: true
    rules:
    - name: validate-resources
      match:
        any:
        - resources:
            kinds:
            - Pod
            namespaces:
            - team-a
            - team-b
            - team-c
            - team-d
      validate:
        message: "CPU and memory resource requests and limits are required"
        pattern:
          spec:
            containers:
            - resources:
                requests:
                  memory: "?*"
                  cpu: "?*"
                limits:
                  memory: "?*"
                  cpu: "?*"
  EOF
  
  # 테스트
  kubectl run test-no-resources --image=nginx -n team-a
  # Error: CPU and memory resource requests and limits are required
```

---

## 2.14 Phase 12: 최종 검증

### 2.14.1 클러스터 상태 체크리스트

```yaml
□ 노드 상태
  kubectl get nodes
  # 191개 노드 모두 Ready
  
  kubectl top nodes
  # 리소스 사용률 확인

□ 시스템 Pod 상태
  kubectl get pods -A | grep -v Running
  # Running 아닌 Pod가 없어야 함
  
  kubectl get pods -A -o wide | grep -E "Error|CrashLoopBackOff|Pending"
  # 에러 상태 Pod 없어야 함

□ Control Plane 컴포넌트
  kubectl get componentstatuses  # deprecated
  kubectl get --raw='/readyz?verbose'
  kubectl get --raw='/livez?verbose'

□ etcd 상태
  kubectl -n kube-system exec -it etcd-k8s-master01 -- \
    etcdctl --endpoints=https://127.0.0.1:2379 \
    --cacert=/etc/ssl/etcd/ssl/ca.pem \
    --cert=/etc/ssl/etcd/ssl/node-k8s-master01.pem \
    --key=/etc/ssl/etcd/ssl/node-k8s-master01-key.pem \
    endpoint health --cluster
  # 5개 etcd 멤버 모두 healthy

□ 스토리지
  kubectl get pv
  kubectl get storageclass
  
  # Ceph 상태
  kubectl -n rook-ceph exec -it deploy/rook-ceph-tools -- ceph status
  # HEALTH_OK

□ 네트워크
  cilium status
  cilium connectivity test

□ 모니터링
  kubectl -n monitoring get pods
  # Prometheus, Grafana 모두 Running
  
  # Grafana 대시보드 접근 확인

□ 로깅
  kubectl -n logging get pods
  # OpenSearch, Fluent-bit 모두 Running
  
  # OpenSearch Dashboards 로그 확인

□ 백업
  velero backup get
  # 정기 백업 스케줄 확인
  
  ls -lh /var/backups/etcd/
  # etcd 백업 파일 존재 확인

□ CI/CD
  kubectl -n argocd get pods
  kubectl -n ci-cd get pods
  # ArgoCD, Jenkins 모두 Running
```

### 2.14.2 기능 검증 체크리스트

```yaml
□ 인증/인가
  # Keycloak OIDC 로그인 테스트
  kubectl --kubeconfig config-oidc get nodes
  
  # RBAC 테스트
  # Team A 사용자로 Team B Namespace 접근 시도 (실패해야 함)

□ 멀티테넌시
  # ResourceQuota 테스트
  kubectl describe quota -n team-a
  
  # NetworkPolicy 테스트
  # Team A Pod에서 Team B Pod 접근 시도 (실패해야 함)

□ 스토리지
  # Local PV 테스트
  # Ceph Block PV 테스트
  # CephFS PV 테스트 (ReadWriteMany)

□ 네트워크
  # Pod-to-Pod 통신
  # Service 통신
  # Ingress 접근
  # 외부 MinIO 접근
  # 외부 Keycloak 접근

□ Spark
  # Spark Application 실행
  # Executor Pod가 올바른 Namespace에서 실행되는지 확인

□ 모니터링
  # 메트릭 수집 확인
  # Alert 발생/해제 테스트
  # 대시보드 데이터 확인

□ 로깅
  # 로그 수집 확인
  # 팀별 로그 검색
  # 로그 Retention 확인

□ 백업/복구
  # Velero 백업 테스트
  # Velero 복구 테스트
  # etcd 백업 확인
```

---

## 2.15 문서화 체크리스트

```yaml
□ 아키텍처 문서
  □ 전체 클러스터 구성도
  □ 네트워크 토폴로지
  □ 스토리지 아키텍처
  □ 보안 구성

□ 운영 매뉴얼
  □ 일일 점검 사항
  □ 주간/월간 점검 사항
  □ Troubleshooting 가이드
  □ 에스컬레이션 절차

□ 절차서 (Runbook)
  □ 클러스터 업그레이드
  □ 노드 추가/제거
  □ 백업/복구
  □ 장애 대응
  □ 보안 인시던트 대응

□ 사용자 가이드
  □ kubectl 설정
  □ Namespace 사용법
  □ 리소스 제한
  □ 스토리지 사용
  □ CI/CD 파이프라인
  □ 모니터링/로깅 접근

□ API 문서
  □ 외부 시스템 연동 (MinIO, Keycloak)
  □ Custom Resource 정의 (Spark, Airflow 등)

□ 변경 이력
  □ Change Log
  □ 버전 정보
  □ Known Issues
```

---

## 2.16 인수인계 체크리스트

```yaml
□ 시스템 인수인계
  □ 관리자 계정 정보
  □ 접근 권한 이관
  □ 문서 이관
  □ 도구/스크립트 이관

□ 운영 팀 교육
  □ 클러스터 구조 설명
  □ 일상 운영 업무
  □ 모니터링 대시보드 사용
  □ 로그 검색
  □ 백업/복구 절차
  □ 장애 대응

□ 개발 팀 교육
  □ kubectl 사용법
  □ Namespace 리소스 사용
  □ CI/CD 파이프라인
  □ Spark Job 실행
  □ 모니터링 접근

□ 지원 체계
  □ 문의 채널
  □ 긴급 연락처
  □ 외부 지원 (벤더, 커뮤니티)
```

---

## 최종 요약

**2단계: 설치 계획 및 체크리스트 완료**

주요 내용:
1. ✅ Phase 0: 사전 준비
2. ✅ Phase 1: OS 설치 및 설정
3. ✅ Phase 2: Kubespray로 Kubernetes 설치
4. ✅ Phase 3: 노드 역할 설정
5. ✅ Phase 4: 스토리지 구성 (Local PV + Rook-Ceph)
6. ✅ Phase 5: 네트워크 구성 (Cilium)
7. ✅ Phase 6: 보안 구성 (RBAC, Keycloak, PSS, NetworkPolicy)
8. ✅ Phase 7: 모니터링 (Prometheus) 및 로깅 (OpenSearch)
9. ✅ Phase 8: 백업 (etcd, Velero)
10. ✅ Phase 9: CI/CD (ArgoCD, Jenkins)
11. ✅ Phase 10: 공통 시스템 (Spark Operator)
12. ✅ Phase 11: 멀티테넌시 (Quota, LimitRange)
13. ✅ Phase 12: 최종 검증
14. ✅ 문서화 및 인수인계

이제 **3단계: 성능 테스트 계획**으로 진행하시겠습니까?

>>>

진행

>>>

# 3단계: 성능 테스트 계획

## 3.1 성능 테스트 전체 개요

### 3.1.1 성능 테스트 목표

```yaml
주요 목표:
1. 클러스터 안정성 검증
   - 191대 노드에서 안정적으로 운영 가능한지 확인
   - 장시간 운영 시 메모리 누수, 리소스 고갈 없음 확인

2. 성능 기준선 수립
   - 각 컴포넌트의 성능 기준 설정
   - 향후 성능 저하 감지를 위한 베이스라인

3. 병목 지점 파악
   - 네트워크, 스토리지, CPU, 메모리 등 병목 지점 식별
   - 최적화 포인트 도출

4. 용량 계획
   - 팀별 실제 사용 가능한 리소스 확인
   - 확장 계획 수립을 위한 데이터

5. SLA 검증
   - 응답 시간, 처리량, 가용성 목표 달성 확인
```

### 3.1.2 성능 테스트 일정

```yaml
전체 기간: 2-3주

Week 1: 인프라 및 기본 성능 테스트
├─ Day 1-2: 하드웨어 성능 (CPU, 메모리, 디스크, 네트워크)
├─ Day 3: Control Plane 성능
├─ Day 4: 네트워크 성능 (Pod-to-Pod, Service)
└─ Day 5: 스토리지 성능 (Local PV, Ceph)

Week 2: 애플리케이션 및 워크로드 테스트
├─ Day 1-2: Spark 성능 테스트
├─ Day 3: 데이터베이스 성능 (PostgreSQL/CNPG)
├─ Day 4: 동시 사용자 부하 테스트
└─ Day 5: 확장성 테스트 (대량 Pod 생성)

Week 3: 통합 및 장애 테스트
├─ Day 1-2: 종합 부하 테스트 (4개 팀 동시 운영)
├─ Day 3: 장애 복구 테스트
├─ Day 4: 장시간 안정성 테스트 (Soak Test)
└─ Day 5: 결과 분석 및 보고서 작성
```

### 3.1.3 테스트 환경 준비

```yaml
□ 테스트 Namespace 생성
  kubectl create namespace perf-test
  kubectl create namespace perf-test-team-a
  kubectl create namespace perf-test-team-b

□ 테스트 도구 설치
  # 필요한 도구들은 각 테스트 섹션에서 설치

□ 모니터링 강화
  # 테스트 기간 동안 메트릭 수집 간격 단축
  # Prometheus scrape interval: 15s → 10s
  
□ 베이스라인 수집
  # 테스트 전 클러스터 상태 기록
  kubectl top nodes > baseline-nodes.txt
  kubectl get pods -A --field-selector=status.phase=Running | wc -l > baseline-pods.txt
```

---

## 3.2 하드웨어 성능 테스트

### 3.2.1 CPU 성능 테스트

```yaml
목적:
- CPU 처리 능력 검증
- CPU 멀티코어 활용도 확인
- CPU Throttling 없는지 확인

테스트 도구: sysbench, stress-ng

테스트 방법:
□ 단일 노드 CPU 벤치마크
  # 테스트 노드 선택 (예: k8s-worker004)
  ssh k8s-worker004
  
  # sysbench 설치
  sudo dnf install -y sysbench
  
  # CPU 벤치마크 (단일 스레드)
  sysbench cpu --cpu-max-prime=20000 --threads=1 run
  
  # CPU 벤치마크 (전체 코어: 48 threads)
  sysbench cpu --cpu-max-prime=20000 --threads=48 run
  
  # 결과 기록
  # - events per second
  # - latency (avg, 95th percentile)

□ Kubernetes Pod에서 CPU 테스트
  cat <<EOF | kubectl apply -f -
  apiVersion: v1
  kind: Pod
  metadata:
    name: cpu-test
    namespace: perf-test
  spec:
    containers:
    - name: stress
      image: polinux/stress
      resources:
        requests:
          cpu: "32"
          memory: "1Gi"
        limits:
          cpu: "32"
          memory: "1Gi"
      args:
      - stress
      - --cpu
      - "32"
      - --timeout
      - "300s"
      - --verbose
    restartPolicy: Never
  EOF
  
  # 모니터링
  kubectl top pod cpu-test -n perf-test --containers
  
  # Prometheus에서 확인
  # container_cpu_usage_seconds_total

□ 멀티 노드 동시 CPU 부하
  # 10개 노드에 동시 부하
  for i in {1..10}; do
    cat <<EOF | kubectl apply -f -
  apiVersion: v1
  kind: Pod
  metadata:
    name: cpu-test-$i
    namespace: perf-test
  spec:
    nodeName: k8s-worker$(printf "%03d" $((i+3)))
    containers:
    - name: stress
      image: polinux/stress
      resources:
        requests:
          cpu: "32"
        limits:
          cpu: "32"
      args:
      - stress
      - --cpu
      - "32"
      - --timeout
      - "600s"
    restartPolicy: Never
  EOF
  done
  
  # 전체 클러스터 CPU 사용률 모니터링
  watch kubectl top nodes

성공 기준:
✓ 단일 노드 CPU 벤치마크: 예상 성능의 90% 이상
✓ CPU Throttling 없음 (container_cpu_cfs_throttled_seconds_total 증가 없음)
✓ 멀티코어 활용: 48코어 모두 활용됨
✓ 노드 간 성능 편차: 5% 이내
```

### 3.2.2 메모리 성능 테스트

```yaml
목적:
- 메모리 대역폭 확인
- 메모리 누수 없는지 확인
- NUMA 최적화 확인

테스트 도구: sysbench, stress-ng, mbw

테스트 방법:
□ 메모리 대역폭 테스트
  ssh k8s-worker004
  
  # mbw (Memory Bandwidth) 설치
  sudo dnf install -y mbw
  
  # 메모리 대역폭 측정
  mbw 1024
  # 예상: ~200GB/s (NUMA 고려)
  
  # sysbench 메모리 테스트
  sysbench memory --memory-block-size=1M \
    --memory-total-size=100G \
    --threads=48 run

□ Kubernetes Pod 메모리 테스트
  cat <<EOF | kubectl apply -f -
  apiVersion: v1
  kind: Pod
  metadata:
    name: memory-test
    namespace: perf-test
  spec:
    containers:
    - name: stress
      image: polinux/stress
      resources:
        requests:
          cpu: "4"
          memory: "500Gi"
        limits:
          cpu: "4"
          memory: "500Gi"
      args:
      - stress
      - --vm
      - "8"
      - --vm-bytes
      - "60G"
      - --timeout
      - "300s"
    restartPolicy: Never
  EOF
  
  # 메모리 사용률 모니터링
  kubectl top pod memory-test -n perf-test
  
  # OOMKill 발생하지 않는지 확인
  kubectl get events -n perf-test | grep OOM

□ 메모리 누수 테스트
  # 장시간 실행 (6시간)
  cat <<EOF | kubectl apply -f -
  apiVersion: v1
  kind: Pod
  metadata:
    name: memory-leak-test
    namespace: perf-test
  spec:
    containers:
    - name: app
      image: nginx
      resources:
        requests:
          cpu: "1"
          memory: "1Gi"
        limits:
          cpu: "1"
          memory: "1Gi"
    restartPolicy: Never
  EOF
  
  # Prometheus에서 메모리 사용량 추이 확인
  # container_memory_usage_bytes{pod="memory-leak-test"}
  # 6시간 동안 메모리 사용량이 일정해야 함

성공 기준:
✓ 메모리 대역폭: 노드당 150GB/s 이상
✓ 메모리 할당 지연: 1ms 이하
✓ OOMKill 없음 (request/limit 내에서)
✓ 메모리 누수 없음 (6시간 동안 증가 추세 없음)
```

### 3.2.3 디스크 I/O 성능 테스트

```yaml
목적:
- NVMe SSD 성능 검증
- IOPS, 처리량 확인
- 순차/랜덤 읽기/쓰기 성능

테스트 도구: fio

테스트 방법:
□ 원시 디스크 성능 (호스트)
  ssh k8s-worker004
  
  # fio 설치
  sudo dnf install -y fio
  
  # 랜덤 읽기 (4K)
  sudo fio --name=randread --ioengine=libaio --iodepth=32 \
    --rw=randread --bs=4k --direct=1 --size=10G \
    --numjobs=4 --runtime=60 --group_reporting \
    --filename=/mnt/local-storage/testfile
  
  # 랜덤 쓰기 (4K)
  sudo fio --name=randwrite --ioengine=libaio --iodepth=32 \
    --rw=randwrite --bs=4k --direct=1 --size=10G \
    --numjobs=4 --runtime=60 --group_reporting \
    --filename=/mnt/local-storage/testfile
  
  # 순차 읽기 (1M)
  sudo fio --name=seqread --ioengine=libaio --iodepth=32 \
    --rw=read --bs=1m --direct=1 --size=10G \
    --numjobs=4 --runtime=60 --group_reporting \
    --filename=/mnt/local-storage/testfile
  
  # 순차 쓰기 (1M)
  sudo fio --name=seqwrite --ioengine=libaio --iodepth=32 \
    --rw=write --bs=1m --direct=1 --size=10G \
    --numjobs=4 --runtime=60 --group_reporting \
    --filename=/mnt/local-storage/testfile
  
  # 결과 기록
  # IOPS, 처리량(MB/s), 지연시간

□ Local PV 성능 (Kubernetes)
  # PVC 생성
  cat <<EOF | kubectl apply -f -
  apiVersion: v1
  kind: PersistentVolumeClaim
  metadata:
    name: local-pv-test
    namespace: perf-test
  spec:
    accessModes:
    - ReadWriteOnce
    storageClassName: local-nvme
    resources:
      requests:
        storage: 100Gi
  EOF
  
  # fio Pod
  cat <<EOF | kubectl apply -f -
  apiVersion: v1
  kind: Pod
  metadata:
    name: fio-local-pv
    namespace: perf-test
  spec:
    containers:
    - name: fio
      image: ljishen/fio
      command:
      - sh
      - -c
      - |
        fio --name=randread --ioengine=libaio --iodepth=32 \
          --rw=randread --bs=4k --direct=1 --size=10G \
          --numjobs=4 --runtime=60 --group_reporting \
          --directory=/data
        
        fio --name=randwrite --ioengine=libaio --iodepth=32 \
          --rw=randwrite --bs=4k --direct=1 --size=10G \
          --numjobs=4 --runtime=60 --group_reporting \
          --directory=/data
      volumeMounts:
      - name: data
        mountPath: /data
      resources:
        requests:
          cpu: "4"
          memory: "8Gi"
    volumes:
    - name: data
      persistentVolumeClaim:
        claimName: local-pv-test
    restartPolicy: Never
  EOF
  
  # 로그 확인
  kubectl logs fio-local-pv -n perf-test

□ Ceph RBD 성능
  # PVC 생성
  cat <<EOF | kubectl apply -f -
  apiVersion: v1
  kind: PersistentVolumeClaim
  metadata:
    name: ceph-rbd-test
    namespace: perf-test
  spec:
    accessModes:
    - ReadWriteOnce
    storageClassName: ceph-block
    resources:
      requests:
        storage: 100Gi
  EOF
  
  # fio Pod (동일한 테스트, PVC만 변경)
  # ... (위와 동일, claimName만 ceph-rbd-test로 변경)
  
  # Ceph 성능 모니터링
  kubectl -n rook-ceph exec -it deploy/rook-ceph-tools -- ceph osd perf

성공 기준:
✓ Local NVMe SSD:
  - 랜덤 읽기 IOPS: 400K+ (4K)
  - 랜덤 쓰기 IOPS: 300K+ (4K)
  - 순차 읽기: 6GB/s+ (1M)
  - 순차 쓰기: 5GB/s+ (1M)
  - 지연시간: p99 < 1ms

✓ Ceph RBD (복제본 3):
  - 랜덤 읽기 IOPS: 50K+ (4K)
  - 랜덤 쓰기 IOPS: 30K+ (4K)
  - 순차 읽기: 1GB/s+ (1M)
  - 순차 쓰기: 500MB/s+ (1M)
  - 지연시간: p99 < 10ms
```

### 3.2.4 네트워크 성능 테스트 (물리)

```yaml
목적:
- 25GbE NIC 성능 검증
- 네트워크 대역폭 및 지연시간 확인
- Jumbo Frame 효과 확인

테스트 도구: iperf3, netperf

테스트 방법:
□ 노드 간 네트워크 대역폭 (호스트)
  # 서버 노드
  ssh k8s-worker004
  iperf3 -s -p 5201
  
  # 클라이언트 노드 (다른 터미널)
  ssh k8s-worker005
  
  # TCP 처리량 테스트 (단일 스트림)
  iperf3 -c k8s-worker004 -t 60 -i 10
  # 예상: ~23 Gbits/sec (25GbE의 ~92%)
  
  # TCP 처리량 (병렬 스트림)
  iperf3 -c k8s-worker004 -t 60 -P 4
  # 예상: ~24 Gbits/sec
  
  # UDP 처리량
  iperf3 -c k8s-worker004 -t 60 -u -b 25G
  
  # 지연시간 측정
  ping -c 100 -i 0.2 k8s-worker004
  # 예상: RTT < 0.5ms

□ Jumbo Frame 효과
  # MTU 9000 (eth0)
  iperf3 -c k8s-worker004 -t 60 -M 8960
  # MTU 1500과 비교

□ 여러 노드 쌍 동시 테스트
  # 10개 노드 쌍에서 동시 iperf3
  # 전체 네트워크 스위치 대역폭 확인

성공 기준:
✓ 단일 스트림 TCP: 20Gbps 이상
✓ 병렬 스트림 TCP: 23Gbps 이상
✓ RTT 지연시간: 0.5ms 이하
✓ Packet loss: 0.01% 이하
✓ Jumbo Frame 사용 시 CPU 사용률 10% 감소
```

---

## 3.3 Kubernetes Control Plane 성능 테스트

### 3.3.1 API Server 성능 테스트

```yaml
목적:
- API Server 응답 시간 확인
- 대량 요청 처리 능력
- 동시 사용자 확장성

테스트 도구: kubectl, k6, kube-burner

테스트 방법:
□ 기본 API 응답 시간
  # 단일 요청
  time kubectl get nodes
  time kubectl get pods -A
  time kubectl get all -A
  
  # 여러 번 반복 측정
  for i in {1..100}; do
    time kubectl get nodes &>/dev/null
  done

□ API Server 부하 테스트
  # kube-burner 설치
  wget https://github.com/kube-burner/kube-burner/releases/download/v1.7.10/kube-burner-1.7.10-Linux-x86_64.tar.gz
  tar xzf kube-burner-1.7.10-Linux-x86_64.tar.gz
  sudo mv kube-burner /usr/local/bin/
  
  # 설정 파일
  cat <<EOF > api-load-test.yaml
  global:
    writeToFile: true
    metricsDirectory: metrics
    measurements:
    - name: podLatency
  
  jobs:
  - name: api-load
    jobType: create
    jobIterations: 1000
    qps: 50
    burst: 50
    namespacedIterations: true
    namespace: perf-test
    objects:
    - objectTemplate: pod.yaml
      replicas: 1
  EOF
  
  # Pod 템플릿
  cat <<EOF > pod.yaml
  apiVersion: v1
  kind: Pod
  metadata:
    name: test-pod-{{.Iteration}}
    labels:
      test: api-load
  spec:
    containers:
    - name: pause
      image: registry.k8s.io/pause:3.9
      resources:
        requests:
          cpu: 10m
          memory: 10Mi
  EOF
  
  # 실행
  kube-burner init -c api-load-test.yaml
  
  # 결과 분석
  # - Pod 생성 지연시간
  # - API Server CPU/메모리 사용률
  # - etcd 쓰기 지연시간

□ 동시 사용자 시뮬레이션
  # 100명의 사용자가 동시에 kubectl 명령 실행
  cat <<'EOF' > concurrent-users.sh
  #!/bin/bash
  for i in {1..100}; do
    (
      kubectl get pods -A &>/dev/null
      kubectl get nodes &>/dev/null
      kubectl get services -A &>/dev/null
    ) &
  done
  wait
  EOF
  
  chmod +x concurrent-users.sh
  time ./concurrent-users.sh
  
  # API Server 메트릭 확인
  # apiserver_request_duration_seconds
  # apiserver_request_total

□ Watch 연결 부하 테스트
  # 1000개의 watch 연결 생성
  for i in {1..1000}; do
    kubectl get pods -A --watch &
  done
  
  # API Server 메모리 사용률 확인
  # 몇 분 후 프로세스 종료
  killall kubectl

성공 기준:
✓ kubectl get nodes: 100ms 이하
✓ kubectl get pods -A: 500ms 이하
✓ 1000 Pod 생성: p95 < 10초
✓ API Server CPU: 50% 이하 (평상시)
✓ 동시 100명 사용자: 응답시간 2배 이내
✓ 1000개 watch 연결: 메모리 증가 < 2GB
```

### 3.3.2 etcd 성능 테스트

```yaml
목적:
- etcd 쓰기/읽기 성능
- 클러스터 상태 저장 성능
- 대규모 데이터 처리

테스트 도구: etcd benchmark

테스트 방법:
□ etcd 벤치마크 (직접)
  kubectl -n kube-system exec -it etcd-k8s-master01 -- sh
  
  # 쓰기 성능
  benchmark --endpoints=https://127.0.0.1:2379 \
    --cacert=/etc/ssl/etcd/ssl/ca.pem \
    --cert=/etc/ssl/etcd/ssl/node-k8s-master01.pem \
    --key=/etc/ssl/etcd/ssl/node-k8s-master01-key.pem \
    --conns=100 --clients=1000 \
    put --key-size=8 --sequential-keys --total=100000 --val-size=256
  
  # 읽기 성능
  benchmark --endpoints=https://127.0.0.1:2379 \
    --cacert=/etc/ssl/etcd/ssl/ca.pem \
    --cert=/etc/ssl/etcd/ssl/node-k8s-master01.pem \
    --key=/etc/ssl/etcd/ssl/node-k8s-master01-key.pem \
    --conns=100 --clients=1000 \
    range key --total=100000
  
  # 결과 기록
  # - Requests/sec
  # - Average latency
  # - p99 latency

□ etcd 크기 및 성능
  # etcd 데이터베이스 크기 확인
  kubectl -n kube-system exec -it etcd-k8s-master01 -- \
    etcdctl --endpoints=https://127.0.0.1:2379 \
    --cacert=/etc/ssl/etcd/ssl/ca.pem \
    --cert=/etc/ssl/etcd/ssl/node-k8s-master01.pem \
    --key=/etc/ssl/etcd/ssl/node-k8s-master01-key.pem \
    endpoint status --write-out=table
  
  # DB 크기, 리비전 수 확인

□ 대량 오브젝트 생성 시 etcd 성능
  # 10000개 ConfigMap 생성
  for i in {1..10000}; do
    kubectl create configmap test-cm-$i \
      --from-literal=key1=value1 \
      -n perf-test &
    
    if (( $i % 100 == 0 )); then
      wait
    fi
  done
  wait
  
  # etcd 메트릭 확인
  # etcd_disk_wal_fsync_duration_seconds
  # etcd_disk_backend_commit_duration_seconds

성공 기준:
✓ etcd 쓰기: 5000 requests/sec 이상
✓ etcd 읽기: 10000 requests/sec 이상
✓ 쓰기 지연시간: p99 < 50ms
✓ 읽기 지연시간: p99 < 10ms
✓ DB 크기: 8GB 이하 (100일 운영 후)
✓ 대량 생성 시 API Server 응답: 정상
```

### 3.3.3 스케줄러 성능 테스트

```yaml
목적:
- 대량 Pod 스케줄링 성능
- 스케줄링 지연시간
- 리소스 제약 하에서 스케줄링

테스트 도구: kube-burner

테스트 방법:
□ 대량 Pod 스케줄링
  # 10000개 Pod를 빠르게 생성
  cat <<EOF > scheduler-test.yaml
  global:
    writeToFile: true
    metricsDirectory: metrics-scheduler
  
  jobs:
  - name: scheduler-test
    jobType: create
    jobIterations: 10000
    qps: 100
    burst: 100
    namespacedIterations: false
    namespace: perf-test
    objects:
    - objectTemplate: pod-small.yaml
      replicas: 1
  EOF
  
  cat <<EOF > pod-small.yaml
  apiVersion: v1
  kind: Pod
  metadata:
    name: sched-test-{{.Iteration}}
    labels:
      test: scheduler
  spec:
    containers:
    - name: pause
      image: registry.k8s.io/pause:3.9
      resources:
        requests:
          cpu: 100m
          memory: 128Mi
        limits:
          cpu: 100m
          memory: 128Mi
  EOF
  
  # 실행
  kube-burner init -c scheduler-test.yaml
  
  # 스케줄링 지연시간 확인
  # scheduler_scheduling_attempt_duration_seconds
  # scheduler_pending_pods

□ 리소스 제약 스케줄링
  # ResourceQuota가 있는 상황에서 스케줄링
  # 일부 Pod는 Pending 상태로 남아야 함
  
  # 10000개 Pod (2000 CPU 요청)
  # Quota: 2000 CPU
  # 예상: ~2000개 Running, 나머지 Pending

□ Affinity/Anti-Affinity 테스트
  # Node Affinity가 있는 Pod 1000개
  cat <<EOF > pod-affinity.yaml
  apiVersion: v1
  kind: Pod
  metadata:
    name: affinity-test-{{.Iteration}}
  spec:
    affinity:
      nodeAffinity:
        requiredDuringSchedulingIgnoredDuringExecution:
          nodeSelectorTerms:
          - matchExpressions:
            - key: node-role.kubernetes.io/worker
              operator: In
              values:
              - "true"
    containers:
    - name: pause
      image: registry.k8s.io/pause:3.9
      resources:
        requests:
          cpu: 10m
          memory: 10Mi
  EOF
  
  # 스케줄링 시간 측정

성공 기준:
✓ 10000 Pod 스케줄링: 10분 이내 완료
✓ 평균 스케줄링 지연: 500ms 이하
✓ p99 스케줄링 지연: 5초 이하
✓ Scheduler CPU 사용률: 80% 이하
✓ Affinity 조건 스케줄링: 정상 동작
```

---

## 3.4 네트워크 성능 테스트 (Kubernetes)

### 3.4.1 Pod-to-Pod 네트워크 성능

```yaml
목적:
- Cilium CNI 성능 검증
- Pod 간 통신 대역폭 및 지연시간
- 네트워크 정책 오버헤드

테스트 도구: iperf3, netperf

테스트 방법:
□ 같은 노드 내 Pod 간 통신
  # Server Pod
  cat <<EOF | kubectl apply -f -
  apiVersion: v1
  kind: Pod
  metadata:
    name: iperf-server-same-node
    namespace: perf-test
  spec:
    nodeName: k8s-worker004
    containers:
    - name: iperf3
      image: networkstatic/iperf3
      args: ["-s"]
      ports:
      - containerPort: 5201
  EOF
  
  # Client Pod (같은 노드)
  cat <<EOF | kubectl apply -f -
  apiVersion: v1
  kind: Pod
  metadata:
    name: iperf-client-same-node
    namespace: perf-test
  spec:
    nodeName: k8s-worker004
    containers:
    - name: iperf3
      image: networkstatic/iperf3
      command: ["sleep", "3600"]
  EOF
  
  # 테스트 실행
  SERVER_IP=$(kubectl get pod iperf-server-same-node -n perf-test \
    -o jsonpath='{.status.podIP}')
  
  kubectl exec -it iperf-client-same-node -n perf-test -- \
    iperf3 -c $SERVER_IP -t 60 -P 4
  
  # 예상: ~40 Gbits/sec (veth pair 오버헤드)

□ 다른 노드 간 Pod 통신
  # Server Pod
  cat <<EOF | kubectl apply -f -
  apiVersion: v1
  kind: Pod
  metadata:
    name: iperf-server-diff-node
    namespace: perf-test
  spec:
    nodeName: k8s-worker004
    containers:
    - name: iperf3
      image: networkstatic/iperf3
      args: ["-s"]
      ports:
      - containerPort: 5201
  EOF
  
  # Client Pod (다른 노드)
  cat <<EOF | kubectl apply -f -
  apiVersion: v1
  kind: Pod
  metadata:
    name: iperf-client-diff-node
    namespace: perf-test
  spec:
    nodeName: k8s-worker005
    containers:
    - name: iperf3
      image: networkstatic/iperf3
      command: ["sleep", "3600"]
  EOF
  
  # 테스트
  SERVER_IP=$(kubectl get pod iperf-server-diff-node -n perf-test \
    -o jsonpath='{.status.podIP}')
  
  kubectl exec -it iperf-client-diff-node -n perf-test -- \
    iperf3 -c $SERVER_IP -t 60 -P 4
  
  # 예상: ~23 Gbits/sec (25GbE의 92%)
  
  # 지연시간
  kubectl exec -it iperf-client-diff-node -n perf-test -- \
    ping -c 100 $SERVER_IP
  # 예상: RTT ~1ms

□ NetworkPolicy 오버헤드 측정
  # NetworkPolicy 없는 상태 측정 (위 테스트)
  
  # NetworkPolicy 적용
  cat <<EOF | kubectl apply -f -
  apiVersion: networking.k8s.io/v1
  kind: NetworkPolicy
  metadata:
    name: test-netpol
    namespace: perf-test
  spec:
    podSelector:
      matchLabels:
        app: iperf-server
    policyTypes:
    - Ingress
    ingress:
    - from:
      - podSelector:
          matchLabels:
            app: iperf-client
      ports:
      - protocol: TCP
        port: 5201
  EOF
  
  # Pod에 Label 추가
  kubectl label pod iperf-server-diff-node app=iperf-server -n perf-test
  kubectl label pod iperf-client-diff-node app=iperf-client -n perf-test
  
  # 재측정
  # 성능 차이 확인 (보통 5% 이내)

□ 대규모 Pod 간 메시 통신
  # 100개 Pod가 서로 통신
  # 네트워크 혼잡 상황 시뮬레이션

성공 기준:
✓ 같은 노드 내: 30Gbps 이상
✓ 다른 노드 간: 20Gbps 이상
✓ RTT 지연시간: 1ms 이하
✓ NetworkPolicy 오버헤드: 5% 이내
✓ Packet loss: 0.01% 이하
```

### 3.4.2 Service 성능 테스트

```yaml
목적:
- ClusterIP Service 성능
- IPVS 부하 분산 성능
- Service 엔드포인트 확장성

테스트 도구: wrk, ab (Apache Bench)

테스트 방법:
□ ClusterIP Service 처리량
  # Backend Pods (10개)
  kubectl create deployment nginx-backend \
    --image=nginx --replicas=10 -n perf-test
  
  kubectl expose deployment nginx-backend \
    --port=80 --target-port=80 -n perf-test
  
  # Load Generator Pod
  kubectl run load-generator --image=williamyeh/wrk \
    --rm -it -n perf-test -- /bin/sh
  
  # 내부에서 실행
  wrk -t 12 -c 400 -d 60s http://nginx-backend.perf-test.svc.cluster.local/
  
  # 결과 기록
  # - Requests/sec
  # - Latency (avg, p99)
  # - Transfer/sec

□ Endpoint 수 증가 테스트
  # Backend 수를 증가시키면서 테스트
  for replicas in 10 50 100 200; do
    kubectl scale deployment nginx-backend --replicas=$replicas -n perf-test
    kubectl wait --for=condition=ready pod -l app=nginx-backend -n perf-test --timeout=300s
    
    # wrk 테스트
    # 결과 비교
  done

□ NodePort Service 성능
  kubectl expose deployment nginx-backend \
    --type=NodePort --port=80 -n perf-test --name=nginx-nodeport
  
  NODEPORT=$(kubectl get svc nginx-nodeport -n perf-test \
    -o jsonpath='{.spec.ports[0].nodePort}')
  
  # 외부에서 테스트
  wrk -t 12 -c 400 -d 60s http://k8s-worker004:$NODEPORT/

성공 기준:
✓ ClusterIP Service: 50K requests/sec 이상
✓ Latency p99: 50ms 이하
✓ 200 endpoints: 성능 저하 20% 이내
✓ NodePort: ClusterIP의 80% 성능
✓ IPVS CPU 오버헤드: 5% 이내
```

### 3.4.3 Ingress 성능 테스트

```yaml
목적:
- Cilium Ingress Controller 성능
- TLS 처리 성능
- 동시 연결 처리

테스트 도구: wrk, h2load

테스트 방법:
□ HTTP Ingress 처리량
  # Ingress 생성
  cat <<EOF | kubectl apply -f -
  apiVersion: networking.k8s.io/v1
  kind: Ingress
  metadata:
    name: nginx-ingress-test
    namespace: perf-test
  spec:
    ingressClassName: cilium
    rules:
    - host: perf-test.example.com
      http:
        paths:
        - path: /
          pathType: Prefix
          backend:
            service:
              name: nginx-backend
              port:
                number: 80
  EOF
  
  # External IP 확인
  INGRESS_IP=$(kubectl get ingress nginx-ingress-test -n perf-test \
    -o jsonpath='{.status.loadBalancer.ingress[0].ip}')
  
  # HTTP 테스트
  wrk -t 12 -c 400 -d 60s \
    -H "Host: perf-test.example.com" \
    http://$INGRESS_IP/

□ HTTPS Ingress 처리량
  # TLS Secret 생성
  openssl req -x509 -nodes -days 365 -newkey rsa:2048 \
    -keyout tls.key -out tls.crt \
    -subj "/CN=perf-test.example.com"
  
  kubectl create secret tls nginx-tls \
    --key tls.key --cert tls.crt -n perf-test
  
  # Ingress에 TLS 추가
  cat <<EOF | kubectl apply -f -
  apiVersion: networking.k8s.io/v1
  kind: Ingress
  metadata:
    name: nginx-ingress-tls
    namespace: perf-test
  spec:
    ingressClassName: cilium
    tls:
    - hosts:
      - perf-test.example.com
      secretName: nginx-tls
    rules:
    - host: perf-test.example.com
      http:
        paths:
        - path: /
          pathType: Prefix
          backend:
            service:
              name: nginx-backend
              port:
                number: 80
  EOF
  
  # HTTPS 테스트
  wrk -t 12 -c 400 -d 60s \
    -H "Host: perf-test.example.com" \
    https://$INGRESS_IP/ --latency

□ HTTP/2 성능
  # h2load 설치
  sudo dnf install -y nghttp2
  
  # HTTP/2 테스트
  h2load -n 100000 -c 100 -m 10 \
    https://$INGRESS_IP/ \
    --h1  # HTTP/1.1 비교
  
  h2load -n 100000 -c 100 -m 10 \
    https://$INGRESS_IP/
    # HTTP/2

□ 동시 연결 수 테스트
  # 10000 동시 연결
  wrk -t 12 -c 10000 -d 60s \
    -H "Host: perf-test.example.com" \
    http://$INGRESS_IP/
  
  # Ingress Pod 메트릭 확인
  # CPU, 메모리, 연결 수

성공 기준:
✓ HTTP Ingress: 40K requests/sec 이상
✓ HTTPS Ingress: 20K requests/sec 이상
✓ TLS 오버헤드: 50% 이내
✓ HTTP/2: HTTP/1.1 대비 20% 성능 향상
✓ 10000 동시 연결: 안정적 처리
✓ Ingress Pod CPU: 80% 이하
```

---

## 3.5 스토리지 성능 테스트 (Kubernetes)

### 3.5.1 Local PV 워크로드 테스트

```yaml
목적:
- 실제 워크로드에서 Local PV 성능
- 여러 Pod 동시 I/O 성능
- Spark/Trino 시나리오 검증

테스트 방법:
□ 데이터베이스 워크로드 시뮬레이션
  # pgbench (PostgreSQL 벤치마크)
  cat <<EOF | kubectl apply -f -
  apiVersion: v1
  kind: Pod
  metadata:
    name: postgres-bench
    namespace: perf-test
  spec:
    containers:
    - name: postgres
      image: postgres:15
      env:
      - name: POSTGRES_PASSWORD
        value: testpassword
      - name: PGDATA
        value: /var/lib/postgresql/data/pgdata
      volumeMounts:
      - name: data
        mountPath: /var/lib/postgresql/data
      resources:
        requests:
          cpu: "4"
          memory: "16Gi"
        limits:
          cpu: "4"
          memory: "16Gi"
    volumes:
    - name: data
      persistentVolumeClaim:
        claimName: local-pv-test
  EOF
  
  # Pod가 Running이 되면
  kubectl exec -it postgres-bench -n perf-test -- /bin/bash
  
  # pgbench 초기화
  pgbench -i -s 100 postgres
  
  # 읽기/쓰기 혼합 테스트
  pgbench -c 50 -j 4 -T 300 postgres
  
  # 결과 기록: TPS, 지연시간

□ 대량 파일 I/O 테스트
  # Spark shuffle 시나리오
  cat <<EOF | kubectl apply -f -
  apiVersion: v1
  kind: Pod
  metadata:
    name: file-io-test
    namespace: perf-test
  spec:
    containers:
    - name: fio
      image: ljishen/fio
      command:
      - sh
      - -c
      - |
        # 많은 작은 파일 쓰기 (shuffle 시나리오)
        fio --name=many-files --ioengine=libaio --iodepth=16 \
          --rw=randwrite --bs=64k --direct=1 \
          --numjobs=32 --nrfiles=1000 --filesize=10m \
          --runtime=300 --group_reporting \
          --directory=/data
      volumeMounts:
      - name: data
        mountPath: /data
      resources:
        requests:
          cpu: "8"
          memory: "16Gi"
    volumes:
    - name: data
      persistentVolumeClaim:
        claimName: local-pv-test
    restartPolicy: Never
  EOF

□ 여러 Pod 동시 I/O
  # 10개 Pod가 동시에 Local PV 사용
  # 각각 다른 PV
  for i in {1..10}; do
    # PVC 생성
    # Pod 생성 (fio 실행)
  done
  
  # 노드별 I/O 대역폭 모니터링
  # node_disk_io_time_seconds_total

성공 기준:
✓ PostgreSQL TPS: 10000+ (읽기/쓰기 혼합)
✓ 대량 파일 쓰기: 2GB/s+ 처리량
✓ 10개 Pod 동시 I/O: 성능 저하 20% 이내
✓ I/O Wait: 10% 이하
```

### 3.5.2 Ceph 성능 테스트 (실제 워크로드)

```yaml
목적:
- Ceph 실제 워크로드 성능
- 복제 오버헤드 확인
- 동시 접근 성능

테스트 방법:
□ 데이터베이스 워크로드 (Ceph RBD)
  # PostgreSQL + Ceph
  # Local PV와 동일한 테스트
  # PVC만 ceph-block으로 변경
  
  # 성능 비교
  # Local PV vs Ceph RBD

□ 공유 파일시스템 워크로드 (CephFS)
  # ReadWriteMany PVC
  cat <<EOF | kubectl apply -f -
  apiVersion: v1
  kind: PersistentVolumeClaim
  metadata:
    name: cephfs-shared
    namespace: perf-test
  spec:
    accessModes:
    - ReadWriteMany
    storageClassName: ceph-filesystem
    resources:
      requests:
        storage: 100Gi
  EOF
  
  # 여러 Pod에서 동시 접근
  for i in {1..5}; do
    cat <<EOF | kubectl apply -f -
  apiVersion: v1
  kind: Pod
  metadata:
    name: cephfs-test-$i
    namespace: perf-test
  spec:
    containers:
    - name: test
      image: busybox
      command:
      - sh
      - -c
      - |
        while true; do
          dd if=/dev/zero of=/data/testfile-$i bs=1M count=1000
          rm /data/testfile-$i
          sleep 5
        done
      volumeMounts:
      - name: data
        mountPath: /data
    volumes:
    - name: data
      persistentVolumeClaim:
        claimName: cephfs-shared
  EOF
  done
  
  # 처리량 모니터링

□ Ceph 복구 중 성능
  # OSD 하나를 의도적으로 중지
  kubectl -n rook-ceph scale deployment rook-ceph-osd-0 --replicas=0
  
  # 복구 중 워크로드 성능 측정
  # 성능 저하 확인
  
  # OSD 복구
  kubectl -n rook-ceph scale deployment rook-ceph-osd-0 --replicas=1

성공 기준:
✓ Ceph RBD TPS: Local PV의 40% 이상
✓ CephFS 동시 쓰기: 500MB/s 이상
✓ 복구 중 성능 저하: 30% 이내
✓ 복구 완료 시간: 1시간 이내 (10GB)
```

---

## 3.6 Spark 워크로드 성능 테스트

### 3.6.1 Spark 기본 벤치마크

```yaml
목적:
- Spark on Kubernetes 성능
- Executor 확장성
- Local PV 활용 효과

테스트 도구: TPC-DS, spark-bench

테스트 방법:
□ Spark Pi (기본 검증)
  cat <<EOF | kubectl apply -f -
  apiVersion: sparkoperator.k8s.io/v1beta2
  kind: SparkApplication
  metadata:
    name: spark-pi-perf
    namespace: team-a
  spec:
    type: Scala
    mode: cluster
    image: "gcr.io/spark-operator/spark:v3.5.0"
    imagePullPolicy: Always
    mainClass: org.apache.spark.examples.SparkPi
    mainApplicationFile: "local:///opt/spark/examples/jars/spark-examples_2.12-3.5.0.jar"
    arguments:
    - "100000"
    sparkVersion: "3.5.0"
    restartPolicy:
      type: Never
    driver:
      cores: 4
      coreLimit: "4000m"
      memory: "8g"
      serviceAccount: spark
      labels:
        version: 3.5.0
    executor:
      cores: 4
      instances: 50
      memory: "16g"
      labels:
        version: 3.5.0
      volumeMounts:
      - name: spark-local
        mountPath: /tmp
    volumes:
    - name: spark-local
      emptyDir: {}
  EOF
  
  # 실행 시간 측정
  kubectl get sparkapplication spark-pi-perf -n team-a -w
  
  # 로그 확인
  kubectl logs <driver-pod> -n team-a | grep "Pi is roughly"

□ Spark SQL 벤치마크 (TPC-DS)
  # TPC-DS 데이터 생성 (1TB)
  # MinIO에 업로드
  
  # TPC-DS 쿼리 실행
  cat <<EOF > tpcds-queries.scala
  val spark = SparkSession.builder()
    .appName("TPC-DS Benchmark")
    .config("spark.sql.warehouse.dir", "s3a://tpcds/warehouse")
    .config("spark.hadoop.fs.s3a.endpoint", "http://minio.example.com:9000")
    .config("spark.hadoop.fs.s3a.access.key", "<ACCESS_KEY>")
    .config("spark.hadoop.fs.s3a.secret.key", "<SECRET_KEY>")
    .config("spark.hadoop.fs.s3a.path.style.access", "true")
    .getOrCreate()
  
  // Query 1
  val startTime = System.currentTimeMillis()
  spark.sql("""
    SELECT s_store_name, SUM(ss_net_profit)
    FROM store_sales, date_dim, store
    WHERE ss_sold_date_sk = d_date_sk
      AND ss_store_sk = s_store_sk
      AND d_year = 2000
    GROUP BY s_store_name
    ORDER BY 2 DESC
    LIMIT 10
  """).show()
  val endTime = System.currentTimeMillis()
  println(s"Query 1 time: ${endTime - startTime} ms")
  
  // 여러 쿼리 실행...
  EOF
  
  # SparkApplication으로 실행
  # 각 쿼리 실행 시간 기록

□ Executor 확장성 테스트
  # Executor 수를 증가시키면서 성능 측정
  for executors in 10 20 50 100 200; do
    # SparkApplication 생성 (executor.instances: $executors)
    # 동일한 작업 실행
    # 실행 시간 비교
  done
  
  # Scalability 분석
  # Ideal: 선형 확장
  # Actual: ?

□ Local PV vs emptyDir 비교
  # Shuffle 데이터를 Local PV에 저장
  volumes:
  - name: spark-local
    persistentVolumeClaim:
      claimName: spark-shuffle-pv
  
  # 성능 비교
  # emptyDir (메모리) vs Local PV (NVMe)

성공 기준:
✓ Spark Pi (50 executors): 10분 이내
✓ TPC-DS Query (1TB): Q1 < 60초
✓ Executor 확장성: 100 executors까지 선형
✓ Local PV shuffle: emptyDir 대비 성능 유사
✓ Job 실패율: 1% 이하
```

### 3.6.2 대규모 Spark 작업

```yaml
목적:
- 클러스터 최대 용량 확인
- 여러 Spark Job 동시 실행
- 리소스 경합 상황

테스트 방법:
□ 단일 대규모 Job
  # 500 Executor Spark Job
  # 팀별 Quota 내 최대 규모
  
  # SparkApplication (executor.instances: 500)
  # 실행 시간, 안정성 확인

□ 여러 Job 동시 실행
  # 4개 팀이 동시에 Spark Job 실행
  for team in team-a team-b team-c team-d; do
    cat <<EOF | kubectl apply -f -
  apiVersion: sparkoperator.k8s.io/v1beta2
  kind: SparkApplication
  metadata:
    name: concurrent-spark-${team}
    namespace: ${team}
  spec:
    # ... (50 executors 각각)
  EOF
  done
  
  # 총 200 Executors 동시 실행
  # 클러스터 리소스 사용률 모니터링
  # Job 완료 시간 비교

□ Spark Job + 다른 워크로드
  # Spark Job + Database + Airflow 동시 실행
  # 실제 운영 환경 시뮬레이션

성공 기준:
✓ 500 Executor Job: 안정적 실행
✓ 4개 팀 동시 Job: 모두 성공
✓ 리소스 Quota 준수: 100%
✓ Job 실행 시간: 단독 실행 대비 50% 이내 증가
✓ 클러스터 전체 CPU 사용률: 80% 이상
```

---

## 3.7 종합 부하 테스트

### 3.7.1 실제 운영 시나리오 테스트

```yaml
목적:
- 4개 팀이 동시에 다양한 워크로드 실행
- 실제 운영 환경 완전 시뮬레이션
- 장시간 안정성 확인

시나리오:
Team A:
  - Airflow DAG 실행 (10개)
  - Spark Job (50 executors)
  - Trino 쿼리 (10 동시 세션)
  - PostgreSQL 워크로드

Team B:
  - Spark Job (100 executors)
  - CNPG Database
  - API 서버 (100 requests/sec)

Team C:
  - Spark Streaming Job
  - Kafka (시뮬레이션)
  - 실시간 대시보드

Team D:
  - 배치 처리 (Spark)
  - ML 모델 훈련 (TensorFlow)
  - Jupyter Notebook

테스트 방법:
□ 시나리오 준비
  # 각 팀별 워크로드 스크립트 작성
  # Helm Chart, Manifest 준비

□ 동시 실행
  # 모든 팀이 동시에 워크로드 시작
  ./start-team-a.sh &
  ./start-team-b.sh &
  ./start-team-c.sh &
  ./start-team-d.sh &
  
  # 6시간 실행

□ 모니터링
  # 실시간 대시보드 감시
  # 메트릭 수집
  # 로그 확인
  # Alert 발생 여부

□ 결과 수집
  # 각 워크로드 완료 시간
  # 리소스 사용률
  # 에러/실패율
  # 네트워크/스토리지 대역폭

성공 기준:
✓ 모든 워크로드 성공적 완료
✓ ResourceQuota 위반 없음
✓ NetworkPolicy 위반 없음
✓ Pod OOMKill 없음
✓ 노드 장애 없음
✓ Control Plane 안정적
✓ 응답 시간: 평상시 대비 2배 이내
```

### 3.7.2 장시간 안정성 테스트 (Soak Test)

```yaml
목적:
- 메모리 누수 감지
- 리소스 고갈 방지 확인
- 장기 운영 안정성

테스트 방법:
□ 72시간 연속 운영
  # 중간 규모 워크로드 지속 실행
  # 각 팀: 30% 리소스 사용
  
  # 워크로드:
  - Spark Job (연속 실행)
  - Database (지속적 읽기/쓰기)
  - API 서버 (지속적 트래픽)
  - 배경 작업 (Cron Job)

□ 모니터링 포인트
  # 메모리 사용량 추이
  # - Control Plane 노드
  # - Worker 노드
  # - 각 시스템 Pod
  
  # CPU 사용량 추이
  # 네트워크 연결 수
  # 파일 디스크립터 수
  # etcd 크기
  # 로그 볼륨

□ 자동 Alert
  # 메모리 사용률 > 90%
  # CPU 사용률 > 90%
  # 디스크 사용률 > 85%
  # etcd 응답 시간 > 100ms
  # API Server 응답 시간 > 1s

□ 정기 체크 (매 6시간)
  # 클러스터 상태
  kubectl get nodes
  kubectl get pods -A | grep -v Running
  
  # etcd 상태
  # Ceph 상태
  # 시스템 리소스

성공 기준:
✓ 72시간 무중단 운영
✓ 메모리 누수 없음 (선형 증가 없음)
✓ 리소스 고갈 없음
✓ 성능 저하 없음 (초기 대비 10% 이내)
✓ 에러율: 0.1% 이하
✓ Control Plane 응답: 일정
```

---

## 3.8 장애 및 복구 테스트

### 3.8.1 노드 장애 테스트

```yaml
목적:
- 노드 장애 시 복구 능력
- Pod 재스케줄링 시간
- 데이터 손실 방지

테스트 방법:
□ Worker 노드 1대 장애
  # 실행 중인 워크로드
  kubectl create deployment test-app --image=nginx --replicas=100
  
  # 노드 장애 시뮬레이션 (전원 끄기)
  NODE=k8s-worker050
  ssh $NODE
  sudo shutdown -h now
  
  # 모니터링
  watch kubectl get nodes
  watch kubectl get pods -o wide
  
  # Pod 재스케줄링 시간 측정
  # NotReady → Pod Terminating → 새 노드에 재생성
  
  # 노드 복구
  # (전원 켜기)
  
  # 노드가 다시 Ready 상태로 복구되는지 확인

□ Control Plane 노드 1대 장애
  # 5대 중 1대 장애
  NODE=k8s-master03
  ssh $NODE
  sudo shutdown -h now
  
  # API Server 접근 가능한지 확인
  kubectl get nodes
  
  # etcd 쿼럼 유지되는지 확인
  kubectl -n kube-system exec -it etcd-k8s-master01 -- \
    etcdctl --endpoints=https://127.0.0.1:2379 \
    --cacert=/etc/ssl/etcd/ssl/ca.pem \
    --cert=/etc/ssl/etcd/ssl/node-k8s-master01.pem \
    --key=/etc/ssl/etcd/ssl/node-k8s-master01-key.pem \
    endpoint health --cluster
  
  # 4/5 healthy 확인
  
  # 노드 복구
  # 자동으로 클러스터에 재참여하는지 확인

□ 여러 노드 동시 장애
  # 10대 Worker 노드 동시 장애
  # 클러스터 안정성 확인
  # 대량 Pod 재스케줄링

성공 기준:
✓ Worker 노드 장애: Pod 재스케줄링 < 5분
✓ Control Plane 1대 장애: 서비스 무중단
✓ 데이터 손실 없음 (PV 사용 Pod)
✓ 장애 복구 시 자동 재참여
✓ 10대 동시 장애: 클러스터 안정
```

### 3.8.2 네트워크 장애 테스트

```yaml
목적:
- 네트워크 파티션 복구
- DNS 장애 복구
- Ingress 장애 복구

테스트 방법:
□ Pod 네트워크 장애
  # 특정 노드의 Cilium Pod 재시작
  kubectl -n kube-system delete pod -l k8s-app=cilium \
    --field-selector spec.nodeName=k8s-worker050
  
  # 해당 노드의 Pod 네트워크 복구 확인

□ CoreDNS 장애
  # CoreDNS Pod 모두 삭제
  kubectl -n kube-system delete pods -l k8s-app=kube-dns
  
  # 재생성 확인
  # DNS 쿼리 복구 확인
  kubectl run test-dns --image=busybox --rm -it -- nslookup kubernetes.default

□ Ingress Controller 장애
  # Cilium Ingress Pod 삭제
  kubectl -n kube-system delete pods -l io.cilium/app=operator
  
  # 외부 접근 복구 확인

성공 기준:
✓ Cilium Pod 재시작: 네트워크 30초 내 복구
✓ CoreDNS 장애: 1분 내 복구
✓ Ingress 장애: 2분 내 복구
✓ 진행 중인 연결: 최소 영향
```

### 3.8.3 스토리지 장애 테스트

```yaml
목적:
- Ceph OSD 장애 복구
- 데이터 무결성 확인

테스트 방법:
□ Ceph OSD 1대 장애
  # OSD Pod 삭제
  kubectl -n rook-ceph delete pod rook-ceph-osd-50-xxx
  
  # Ceph 상태 확인
  kubectl -n rook-ceph exec -it deploy/rook-ceph-tools -- ceph status
  # HEALTH_WARN 예상
  
  # 자동 복구 확인
  # HEALTH_OK로 복구

□ OSD 여러 대 장애
  # 3대 동시 장애 (복제본 3이므로 위험)
  # 데이터 손실 없는지 확인

□ 데이터 무결성 검증
  # 장애 전 데이터 작성
  # 장애 중
  # 장애 후 데이터 읽기
  # 일치 여부 확인

성공 기준:
✓ OSD 1대 장애: 자동 복구
✓ 데이터 무결성: 100%
✓ 복구 시간: 30분 이내 (100GB)
✓ 복구 중 I/O: 정상 (성능 저하 가능)
```

---

## 3.9 성능 테스트 결과 정리

### 3.9.1 결과 수집 체크리스트

```yaml
□ 하드웨어 성능
  □ CPU 벤치마크 결과
  □ 메모리 대역폭
  □ 디스크 IOPS/처리량
  □ 네트워크 대역폭

□ Kubernetes 성능
  □ API Server 응답 시간
  □ etcd 성능
  □ 스케줄러 지연시간
  □ Pod 생성 시간

□ 네트워크 성능
  □ Pod-to-Pod 처리량
  □ Service 처리량
  □ Ingress 처리량
  □ 지연시간

□ 스토리지 성능
  □ Local PV 성능
  □ Ceph 성능
  □ 워크로드별 성능

□ 애플리케이션 성능
  □ Spark Job 실행 시간
  □ 데이터베이스 TPS
  □ 동시 사용자 처리

□ 확장성
  □ Pod 확장성
  □ Executor 확장성
  □ 리소스 사용률

□ 안정성
  □ 장시간 운영 (72시간)
  □ 메모리 누수 여부
  □ 에러율

□ 복구
  □ 노드 장애 복구 시간
  □ 네트워크 장애 복구
  □ 스토리지 장애 복구
```

### 3.9.2 성능 보고서 템플릿

```yaml
성능 테스트 보고서
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

1. Executive Summary
   - 테스트 목적
   - 전체 결과 요약
   - 주요 발견사항
   - 권장사항

2. 테스트 환경
   - 클러스터 구성 (191대)
   - 소프트웨어 버전
   - 테스트 기간
   - 테스트 도구

3. 하드웨어 성능
   - CPU: [결과]
   - 메모리: [결과]
   - 디스크: [결과]
   - 네트워크: [결과]
   - 베이스라인 대비: [%]

4. Kubernetes 성능
   - Control Plane: [결과]
   - API Server: [결과]
   - etcd: [결과]
   - 스케줄러: [결과]

5. 네트워크 성능
   - Pod-to-Pod: [Gbps]
   - Service: [requests/sec]
   - Ingress: [requests/sec]
   - 지연시간: [ms]

6. 스토리지 성능
   - Local PV: [IOPS, MB/s]
   - Ceph: [IOPS, MB/s]
   - 비교 분석

7. 애플리케이션 성능
   - Spark: [실행 시간]
   - 데이터베이스: [TPS]
   - 동시 사용자: [처리량]

8. 확장성 분석
   - Pod 확장성: [그래프]
   - 리소스 확장성: [그래프]
   - 병목 지점: [분석]

9. 안정성 테스트
   - 72시간 테스트: [결과]
   - 에러율: [%]
   - 메모리 누수: [없음/있음]

10. 장애 복구
    - 노드 장애: [복구 시간]
    - 네트워크 장애: [복구 시간]
    - 스토리지 장애: [복구 시간]

11. 병목 및 최적화
    - 발견된 병목 지점
    - 최적화 권장사항
    - 예상 개선 효과

12. 결론
    - 목표 달성 여부
    - 운영 준비 상태
    - 향후 계획

부록:
- 상세 테스트 데이터
- 그래프 및 차트
- 로그 샘플
- 설정 파일
```

### 3.9.3 최적화 권장사항 도출

```yaml
성능 테스트 결과를 바탕으로:

□ 병목 지점 식별
  - CPU, 메모리, 네트워크, 스토리지 중 제약 요소
  - 특정 컴포넌트의 성능 한계
  - 확장 시 고려사항

□ 튜닝 포인트
  - 커널 파라미터 조정
  - Kubernetes 설정 최적화
  - 애플리케이션 설정 최적화
  - 네트워크 설정 개선
  - 스토리지 설정 개선

□ 용량 계획
  - 현재 용량 평가
  - 향후 증설 계획
  - 팀별 리소스 재조정
  - 버퍼 확보 전략

□ 운영 개선
  - 모니터링 임계값 설정
  - Alert 규칙 조정
  - 자동화 범위 확대
  - SOP 업데이트
```

---

## 최종 요약

**3단계: 성능 테스트 계획 완료**

주요 내용:
1. ✅ 하드웨어 성능 테스트 (CPU, 메모리, 디스크, 네트워크)
2. ✅ Kubernetes Control Plane 성능 테스트
3. ✅ 네트워크 성능 테스트 (Pod, Service, Ingress)
4. ✅ 스토리지 성능 테스트 (Local PV, Ceph)
5. ✅ Spark 워크로드 성능 테스트
6. ✅ 종합 부하 테스트 (실제 운영 시나리오)
7. ✅ 장애 및 복구 테스트
8. ✅ 결과 정리 및 보고서 작성

이제 **4단계: 운영 정책 문서**로 진행하시겠습니까?

>>>

진행

>>>
