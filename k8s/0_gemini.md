쿠버네티스 클러스터를 온프레미스에서 구축 하려고 해. 이를 진행 하기 위한 계획과 체크리스트를 만들고 마지막으로는 이를 바탕으로 보고서를 작성 해야해. 하나씩 단계 별로 진행하고 싶어.

클러스터를 구성할 노드 하드웨어 사양은 아래와 같아
- CPU: 2소켓 intel xeon gold 6542y 이고 24 core cpu 2개를 사용
- 메모리: 768G
- 디스크: nvme ssd 3.84t * 2개 
- NIC: 10/25Gbe 2-port broadcom

위와 같은 노드 191대를 사용해서 클러스터를 구성할 예정이야

쿠버네티스 클러스터 구성은 kubespray를 사용할 예정이지만, 올바른 선택인지 의문인 상태야. 회사에서 다른 팀에서 쿠버네티스 클러스터 구성을 kubespary로 진행했고 우리팀에 맞는 다는 보장은 없지만 위에서는 사용하길 원하는 눈치야. 쿠버네티스 클러스터 업데이트와 업데이트 실패시 롤백 시나리오에 다른 설치법에 비해 Kubespary가 강점이 있는지 모르겠어.

클러스터 사용 계획은 4개의 팀이 namespace를 나누어서 namespace별로 각자의 클러스터 처럼 사용할 예정임

4개의 팀이 클러스터에 공통으로 설치해서 사용할 시스템은  kubeflow/spark-operator, ArgoCD, Jenkins등을 공통으로 사용 할꺼야.

각 팀별로 각기 따로 설치해서 사용할 시스템은 CNPG, Airflow, Trino등이고  팀별로 서로 다르긴하지만 데이터 파이프라인 운영이나 분석에 필요한 시스템이 주를 이루게 될꺼야. Airflowk에서 kubeflow/spark-operator를 사용해서 spark을 실행하면 각자의 팀의 namespace에서 실행이 되어야 해. 

Trino, Spark에서 사용할 데이터는 다른 쿠버네티스 클러스터에서 50-100대로 MinIO + Iceberg 등이 사용되어 구축되어 있어. 더불어 해당 클러스터에 Keycloak도 있어서 인증 인가는 해당 클러스터의 KeyCloak을 사용할 예정이야.

추가로 Apache/Yunikorn 사용은 검토 중인데 4개의 팀에 리소스 쿼타를 설정해서 자기 팀별 리소스를 지키는 방안을 상당히 선호하는 상태라서 Yunikorn 사용이 어렵지 않을까 싶고 갱스케쥴링을 위한 전용 도구가 될것 같아.

클러스터 모니터링은 프로메테우스 그라파나 조합이고 로그 검색을 위해 opensearch를 사용할 예정이야

클러스터 Ingress는 Cillium을 사용할 예정이야.

클러스터 구축 후 체크리스트를 작성하고 셋업이 잘되었는지, 설치한  시스템들 e.g. airflow, trino, spark-operator등이 올바로 셋업이 되었는지, 기능적으로 올바로 동작하는지 등등 체크리스트도 반드시 작성이 되어야 해.

더불어서 모든 측면에서 가능한한 모든 퍼포먼스 테스트도 이루어져야해. 이를 위한 계획과 체크리스트 반드시 작성 되어야 해.

로그 정책, naming-rule등등 운영에 수반되는 정책도 수립되어야해. 그리고 클러스터에 프로비저닝시에 request나 limit 없이 프로비저닝 하지 못하도록 설정이 필요해

추가로 더 필요한 정보나 더 고려해야 할 사항이 있을까?

---

대규모 온프레미스 쿠버네티스 클러스터(191노드) 구축이라는 매우 중요하고 도전적인 프로젝트를 맡으셨군요. 하드웨어 스펙이 상당히 고사양(High-End)이며, 데이터/ML 워크로드가 중심인 만큼 **성능 최적화**와 **안정적인 운영**이 핵심 성공 요인이 될 것입니다.

요청하신 대로 **단계별(Step-by-Step)**로 진행하겠습니다. 먼저 본격적인 계획 수립에 앞서, **현재 계획의 타당성 검토(Kubespray 등)** 및 **추가로 고려해야 할 결정적인 사항(Missing Links)**을 짚어보고자 합니다.

이 내용을 바탕으로 답변을 주시면, 그 다음 단계인 [상세 구축 계획 및 체크리스트]를 작성하겠습니다.

---

### 1. 핵심 이슈 검토: Kubespray가 과연 최선인가?

가장 먼저 고민 중이신 **"Kubespray 사용 여부"**에 대한 의견입니다.

* **Kubespray의 장점:** Ansible 기반이라 익숙하고, 커스터마이징이 매우 강력합니다. 온프레미스(Baremetal) 환경에서 가장 널리 쓰이는 표준 도구 중 하나입니다.
* **Kubespray의 단점 (191대 규모 기준):**
    * **속도:** 191대 노드에 대해 Ansible Playbook을 돌리면, 병렬 처리를 하더라도 시간이 꽤 오래 걸립니다.
    * **업데이트/롤백:** "Rolling Update" 방식을 취하는데, 중간에 실패할 경우 디버깅이 까다롭고, 롤백이 '버튼 클릭'처럼 자동화되어 있지 않습니다 (백업된 etcd 복구 및 수동 개입 필요).
* **대안 (Cluster API - CAPI):** 최근 엔터프라이즈 환경에서는 **Cluster API (with Metal3 or Tinkerbell)**를 사용하여 쿠버네티스를 '선언적'으로 관리하는 추세입니다. 이는 업데이트 실패 시 롤백이 더 용이하고, 노드 프로비저닝부터 생명주기 관리가 통합됩니다.
* **결론 및 제언:**
    * 회사 내 표준이 Kubespray라면 **정치적/기술적 통일성을 위해 따르는 것이 안전**합니다.
    * 단, **업데이트 전략**을 보완해야 합니다. Kubespray로 진행하되, **etcd 백업/복구 자동화 스크립트**를 완벽하게 준비하고, 업그레이드 시에는 **배치 사이즈(Batch size) 조절**을 통해 다운타임을 최소화하는 전략을 수립하면 충분히 커버 가능합니다.

---

### 2. 추가로 고려해야 할 사항 & 질문 (Critical Checkpoints)

성공적인 보고서와 구축을 위해 아래 내용들이 먼저 확정되거나 고려되어야 합니다.

#### A. 스토리지 (Storage) 아키텍처의 부재 [가장 중요]
제공해주신 스펙에 `NVMe 3.84T * 2개`가 있습니다. 이를 어떻게 사용할지 정책이 비어 있습니다. 데이터 파이프라인(Spark/Trino)과 DB(CNPG)는 **Disk I/O**가 생명입니다.

* **OS 영역:** RAID 1로 묶어서 안정성을 확보할 것인가? 아니면 1개는 OS, 1개는 데이터용으로 쓸 것인가?
* **CSI (Container Storage Interface):** Pod가 사용할 PVC를 어떻게 제공할 것인가?
    * **옵션 1 (Local PV):** 속도는 가장 빠르나, Pod가 특정 노드에 고정됨 (데이터 노드 장애 시 복구 복잡). Spark Shuffle 데이터나 임시 데이터에는 적합.
    * **옵션 2 (분산 스토리지 - Rook/Ceph):** NVMe 디스크의 일부 파티션을 모아 Ceph 클러스터를 구성. 관리 복잡도는 높지만, 고가용성(HA)과 유연한 PVC 제공 가능.
    * **옵션 3 (External NAS/SAN):** 별도 스토리지가 없다면 논외.
* **질문:** NVMe 디스크 구성 계획과 사용할 스토리지 솔루션(예: Rook-Ceph, OpenEBS, Local Path Provisioner 등)이 결정되었나요?

#### B. 네트워크 토폴로지 및 대역폭
191대 노드가 통신하고, 외부 MinIO(50~100대)에서 데이터를 끌어와야 합니다.

* **Top-of-Rack (ToR) 스위치 구성:** 25GbE 2-port bonding(LACP)을 사용하시나요?
* **Spine-Leaf 구조:** Spark/Trino의 Shuffle 트래픽은 **East-West 트래픽**이 폭발적입니다. 스위치 간 병목(Oversubscription ratio)은 검토되었나요?
* **Cilium 모드:** VXLAN(터널링) 모드를 쓸지, 성능이 더 좋은 **Native Routing (BGP)** 모드를 쓸지 결정해야 합니다. (BGP 모드 권장)

#### C. Etcd 설계 (규모 고려)
191 노드는 기본 설정으로는 위험할 수 있습니다.

* **분리 여부:** Control Plane 노드(Master)에 etcd를 같이 둘 것인지, 별도 노드로 뺄 것인지? (이 규모라면 고성능 NVMe를 장착한 별도 3~5대 노드로 분리하거나, Control Plane 노드를 매우 강력하게 격리해야 합니다.)

#### D. YuniKorn vs ResourceQuota 오해 풀기
* **우려하시는 점:** "YuniKorn을 쓰면 팀별 리소스 쿼터 설정이 어렵지 않을까?"
* **사실:** 정반대입니다. **Apache YuniKorn은 K8s 기본 Quota보다 훨씬 강력한 계층형 큐(Hierarchical Queues) 기반의 쿼터 관리**를 제공합니다.
    * `Root > Team A > Spark Job` 형태로 쿼터를 쪼갤 수 있고, A팀이 안 쓸 때 B팀이 잠시 빌려 쓰는(Preemption/Borrowing) 설정도 가능합니다.
    * **제언:** Spark/Batch 작업이 주라면 **YuniKorn 도입은 필수**에 가깝습니다. 갱스케줄링 뿐만 아니라 "팀별 쿼터 관리"를 위해서라도 적극 권장합니다.

#### E. 외부 MinIO와의 연동 (Latency)
* 데이터 레이크(MinIO)가 다른 클러스터에 있습니다.
* **망 구성:** 두 클러스터 사이의 네트워크 대역폭과 지연 시간(Latency)은 충분한가요? (전용선 혹은 내부망 100G 백본 등). Spark 작업 시 이 구간이 병목이 될 확률이 매우 높습니다.

---

### 3. 향후 진행 로드맵 (제안)

위 내용들이 정리되면 아래 순서로 결과물을 만들어 드리고자 합니다.

1.  **Phase 1: 아키텍처 상세 설계 (Architecture Design)**
    * 하드웨어(Disk/Net) 배치도, 네트워크(Cilium BGP) 설계, 스토리지(CSI) 전략 수립.
    * Kubespray 설정 값(튜닝 포인트) 정의.
2.  **Phase 2: 설치 및 구성 계획 (Installation Plan)**
    * OS 세팅부터 K8s 설치, 애드온(ArgoCD, Prometheus 등) 설치 순서.
    * Team별 Namespace 및 YuniKorn Queue 설계.
3.  **Phase 3: 검증 및 테스트 계획 (Test & Verification)**
    * 기능 테스트 (Smoke Test).
    * 성능 테스트 (FIO, Iperf, Spark Benchmark 등).
    * 장애 복구 테스트 (Etcd 복구, 노드 장애 등).
4.  **Phase 4: 운영 정책 및 보고서 작성 (Operations & Reporting)**
    * Log/Metric 정책, Naming Rule, OPA(Gatekeeper/Kyverno) 정책.
    * 최종 구축 보고서 초안.

---

### [Next Step]
가장 시급한 **'스토리지 구성(NVMe 2장을 어떻게 쓸 것인가)'**과 **'네트워크 구성(BGP 사용 여부)'**에 대한 현재 생각이나 상황을 알려주시면, 바로 **Phase 1: 아키텍처 상세 설계 및 Kubespray 튜닝 가이드** 작성을 시작하겠습니다.

혹시 이 부분에 대해 아직 결정된 바가 없다면, **"추천 구성안을 먼저 제시해달라"**고 요청해 주셔도 좋습니다. 어떻게 진행할까요?


---

네트워크부터 이야기 해보자. 일단 네트워크 지식이 깊지 않은 상태라서 너무 헷갈려. 쿠버네티스 클러스터를 191대를 묶으려면 물리적 L4 스위치가 필요하잖아. 물리적 L4 스위치는 구입 예정이야. 물리적 L4 스위치에 BGP 기능이 있으면 되는거야? 스위치에 BGP기능이 필요한거야 라우터에 BGP 기능이 필요한거야?

---

네트워크 부분이 가장 어렵고, 191대 규모라면 물리 네트워크 설계가 전체 성능을 좌우합니다. 아주 쉽게 핵심만 짚어서 정리해 드릴게요.

결론부터 말씀드리면, **"191대를 연결하기 위한 스위치"는 L3 기능(라우팅)이 포함된 스위치**여야 하고, **BGP 기능이 반드시 있어야 합니다.**

이해하기 쉽게 **물리적 구성**부터 **BGP가 왜 필요한지** 순서대로 설명해 드릴게요.

---

### 1. 191대 서버를 어떻게 연결할까? (Spine-Leaf 구조)

191대의 서버는 물리적으로 하나의 랙(Rack)에 다 들어가지 않습니다. 보통 랙 하나에 서버가 20~40대 정도 들어갑니다. 즉, 약 **5~8개의 랙**이 필요합니다.

이때 가장 추천하는 구성은 **Spine-Leaf(스파인-리프)** 구조입니다.

1.  **Leaf 스위치 (Top of Rack 스위치):**
    * 각 랙의 맨 위에 달리는 스위치입니다.
    * 그 랙에 있는 서버들(약 20~40대)이 이 스위치에 케이블로 연결됩니다.
    * **여기서 BGP가 필요합니다.**
2.  **Spine 스위치 (백본 역할):**
    * 서버와 직접 연결되지 않고, **Leaf 스위치들끼리만 연결**해주는 고성능 스위치입니다.
    * 모든 Leaf 스위치는 모든 Spine 스위치와 연결됩니다.

> **[핵심]** "물리적 L4 스위치"를 구매한다고 하셨는데, 정확한 용어로는 **L3 스위치 (Data Center Switch)**를 구매하셔야 합니다. (업계에서는 보통 Cisco Nexus, Arista, Dell, Juniper 등의 데이터센터용 스위치를 말합니다.)

---

### 2. BGP는 어디에 필요한가? (스위치 vs 라우터)

질문하신 내용에 답을 드리자면:

* **"스위치에 BGP 기능이 필요한가?"**
    * **네, 필수입니다.** 서버들이 연결되는 Leaf 스위치(L3 스위치)와 Spine 스위치 모두 BGP를 지원해야 합니다.
* **"라우터에 필요한가?"**
    * 데이터센터 네트워크에서는 **L3 스위치가 곧 라우터 역할**을 합니다. 별도의 "라우터 장비"를 따로 사는 게 아니라, 스위치 장비가 라우팅(길 찾기) 기능을 수행합니다.

---

### 3. 왜 하필 BGP이고, 왜 Cilium인가?

Cilium CNI를 BGP 모드로 쓴다는 것은 **"쿠버네티스 네트워크 정보를 물리 스위치에 직접 알려주겠다"**는 뜻입니다.

* **BGP가 없을 때 (VXLAN/Overlay):**
    * 택배(데이터)를 보낼 때, 포장지를 한번 더 싸서(터널링) 보냅니다. 받는 쪽에서 포장을 뜯어야 내용물을 알 수 있습니다.
    * **단점:** 포장하고 뜯는 과정에서 CPU를 쓰고 속도가 느려집니다. 트러블슈팅이 어렵습니다.
* **BGP를 쓸 때 (Native Routing):**
    * 서버(Cilium)가 스위치에게 직접 말합니다. "내 안에 10.1.1.5번 파드(Pod)가 살고 있어. 이쪽으로 바로 보내!"
    * 스위치는 이 정보를 옆 스위치들에게 공유합니다 (이게 BGP 프로토콜입니다).
    * **장점:** 포장을 안 싸도 됩니다. 속도가 가장 빠르고(Wire speed), 지연시간(Latency)이 가장 적습니다. **데이터 분석(Spark, Trino) 환경에서는 필수**입니다.

---

### 4. 구매 및 구성 체크리스트 (네트워크 팀 전달용)

네트워크 장비 구매 전, 아래 내용을 체크리스트로 만들어 네트워크 담당자나 벤더(납품업체)에게 전달하시면 됩니다.

#### A. 장비 사양 (Leaf/Spine 스위치 공통)
1.  **L3 라우팅 지원:** 반드시 **L3 License**가 포함되어야 합니다.
2.  **BGP 지원:** BGP (eBGP, iBGP) 및 **ECMP**(Equal-Cost Multi-Path, 길이 여러 개일 때 분산해서 보내는 기술) 기능 필수.
3.  **대역폭:**
    * Leaf 스위치 하향 링크(서버 연결): 10/25GbE (서버 NIC 속도에 맞춤)
    * Leaf 스위치 상향 링크(Spine 연결): 100GbE 권장 (서버가 25G이므로 위쪽 길은 더 넓어야 병목이 안 생깁니다.)
4.  **버퍼(Buffer):** Spark/Trino 같은 데이터 작업은 순간적으로 트래픽이 폭증(Burst)합니다. **"Deep Buffer"** 모델인지 확인하세요. (버퍼가 작으면 패킷 드랍이 발생하여 속도가 급격히 느려집니다.)

#### B. 토폴로지 구성안
* **서버 연결:** 각 서버의 2개 포트(NIC)를 **LACP(Bonding)**로 묶어서 Leaf 스위치 2대(MLAG/vPC 구성)에 각각 연결하여 이중화할 것인가? (고가용성을 위해 권장)
* **BGP 구성:** Cilium BGP Control Plane 기능을 켜서, 각 쿠버네티스 노드가 Tor(Leaf) 스위치와 BGP Peering을 맺도록 설정.

---

### [요약] 사용자님을 위한 한 줄 정리

> "191대 서버를 위해 **데이터센터용 L3 스위치(Leaf/Spine 구조)**를 구매해야 하며, 이 스위치들은 **BGP**와 **ECMP** 기능을 지원해야 합니다. 그래야 Cilium을 통해 쿠버네티스 통신 속도를 최대로 뽑아낼 수 있습니다."

**다음 단계 제안:**
이제 네트워크 장비(하드웨어)의 방향성은 잡혔습니다.
혹시 **191대 노드의 디스크(NVMe 3.84T * 2)를 어떻게 구성할지(스토리지 아키텍처)** 결정하셨나요? 이 부분이 결정되어야 OS 설치 및 쿠버네티스 세팅 계획으로 넘어갈 수 있습니다. 스토리지 구성안을 제안해 드릴까요?

---

그전에 물리적 스위치와 쿠버네티스의 LoadBalancer(MetalLB 등)과의 관계가 어떻게 돼? 물리적 스위치가 있으면 MetalLB 구성을 안해도 돼는거야?

---

아주 좋은 질문입니다. 이 부분이 온프레미스 쿠버네티스 구축 시 가장 혼동하기 쉬운 "미싱 링크(Missing Link)"입니다.

결론부터 말씀드리면:
1.  **물리적 스위치만으로는 부족합니다.**
2.  **MetalLB(또는 Cilium의 LB 기능) 같은 소프트웨어가 반드시 필요합니다.**
3.  하지만 님이 **Cilium**을 쓰기로 했으니, **MetalLB는 설치할 필요가 없습니다.** (Cilium이 그 역할을 대신합니다.)

이 관계를 명확하게 이해하기 위해 **[누가 무엇을 담당하는지]** 정리해 드릴게요.

---

### 1. 왜 물리 스위치만으로는 안 될까?

쿠버네티스에서 `type: LoadBalancer` 서비스(Service)를 만들면, 외부에서 접속 가능한 **가상의 IP(VIP)**가 필요합니다.

* **클라우드(AWS/GCP)의 경우:**
    * 쿠버네티스가 클라우드 API를 호출 -> 클라우드 사업자가 알아서 IP를 주고 라우팅을 연결해 줌.
* **온프레미스의 경우:**
    * 쿠버네티스는 "IP를 줘야 하는데 누가 주지?" 하고 멍하니 기다립니다 (Status: `Pending` 무한 대기).
    * **물리 스위치(L3)**는 멍청합니다. 191대 노드에 대한 길은 알지만, 갑자기 생성된 **서비스의 가상 IP**가 어느 노드에 있는지 전혀 모릅니다.

**즉, 누군가가 물리 스위치에게 "야, 이 서비스 IP(10.x.x.x)는 지금 저 노드들에 있어! 이리로 트래픽 보내!"라고 알려줘야(Advertise) 합니다.**
이 "알려주는 역할(Speaker)"을 하는 것이 바로 **MetalLB**나 **Cilium**입니다.

---

### 2. BGP를 쓴다면, 흐름은 이렇게 됩니다



1.  **사용자:** "Airflow 웹 화면을 띄우기 위해 `LoadBalancer` 서비스 생성해줘. IP는 `10.0.10.100`으로!"
2.  **Cilium (또는 MetalLB):** "오케이, 내가 `10.0.10.100` IP를 할당할게."
3.  **Cilium (Speaker):** 물리 스위치에게 **BGP 프로토콜**로 외칩니다.
    * *"여기보세요! `10.0.10.100`으로 가는 트래픽은 우리 노드(Node 1, Node 2...)로 보내시면 됩니다!"*
4.  **물리 스위치:** BGP 정보를 받고 라우팅 테이블을 업데이트합니다.
    * *`10.0.10.100` -> `Node 1(192.168.1.1)` or `Node 2(192.168.1.2)`*
5.  **외부 사용자:** `10.0.10.100`으로 접속 시도 -> 물리 스위치가 BGP 정보를 보고 해당 노드로 패킷 전달.

---

### 3. "물리적 L4 스위치" vs "L3 스위치 + 소프트웨어 LB"

여기서 용어 정리가 필요합니다. 보통 "물리적 L4 스위치"라고 하면 두 가지 의미로 쓰입니다.

#### 경우 A: F5 Big-IP, Citrix ADC 같은 "전용 로드밸런서 장비" (비쌈)
* 이 장비가 있으면 MetalLB/Cilium LB 기능 대신 이 장비를 쓸 수도 있습니다 (CIS 등의 전용 컨트롤러 필요).
* 하지만 191대 규모의 내부 통신(East-West)이나 개발/분석용 클러스터에는 **비효율적이고 관리가 복잡**하여 잘 안 씁니다. 보통 대고객 서비스(Ingress 앞단)에만 둡니다.

#### 경우 B: 그냥 라우팅 기능이 있는 "L3 데이터센터 스위치" (Cisco Nexus, Arista 등)
* **님이 구매하실 예정인 것은 아마 이 장비일 것입니다.**
* 이 장비는 "로드밸런싱(부하분산)"을 전문적으로 하는 게 아니라 "라우팅(길찾기)"을 합니다.
* 따라서 **Cilium(소프트웨어 LB)**이 BGP를 통해 스위치에게 길을 알려주고, 스위치는 ECMP(Equal Cost Multi Path)를 통해 트래픽을 여러 노드로 뿌려주는 역할을 합니다.

---

### 4. 결론 및 추천 구성

현재 계획하신 **Cilium + BGP** 조합은 **MetalLB를 대체**하는 가장 현대적이고 성능이 좋은 구성입니다.

1.  **MetalLB 설치:** **불필요함.** (Cilium에 `LoadBalancer IPAM` 및 `BGP Control Plane` 기능이 내장되어 있습니다.)
2.  **물리 스위치 역할:**
    * 각 노드와 BGP Peering(친구 맺기) 설정.
    * Cilium이 던져주는 라우팅 정보를 받아서 트래픽을 전달하는 역할.
3.  **Cilium 설정:**
    * `CiliumBGPPeeringPolicy`를 통해 물리 스위치 IP와 AS(Autonomous System) 번호를 설정하면 끝입니다.

---

### [Check Point]
네트워크 부분은 이제 명확해진 것 같습니다.
**"Cilium의 내장 BGP 기능을 사용하여 물리 스위치와 연동한다(MetalLB 불필요)."**로 확정하면 됩니다.

이제 앞서 말씀드렸던 **스토리지(디스크) 구성**으로 넘어가도 될까요?
**"NVMe 3.84T * 2개"**를 어떻게 묶어야 데이터 파이프라인(Spark/Trino) 성능이 최대로 나올지 제안해 드리겠습니다.

>>>

스토리지 추천 구성 방안 제시해줘.

>>>

191대 노드에 NVMe 3.84TB 2장이라는 사양은 **엄청난 I/O 잠재력**을 가지고 있습니다. 이 자원을 단순히 OS용 RAID 1으로 묶어서 낭비하거나, 느린 네트워크 스토리지로만 구성하면 하드웨어 스펙이 너무 아깝습니다.

Spark, Trino, Opensearch와 같은 **데이터 중심 워크로드**에 최적화된 **"Hybrid Partitioning (성능 + 안정성)"** 구성안을 제안합니다.

---

### 1. 물리적 디스크 파티션 구성 (OS 설치 시점)

NVMe 2장을 **소프트웨어 RAID (MDADM)와 LVM**을 조합하여 구성하는 것이 베스트입니다. 하드웨어 RAID 카드가 있더라도, 최근 트렌드(특히 Ceph, MinIO 등)는 OS 레벨에서 디스크를 직접 제어하는 것을 선호합니다.

**추천 파티션 전략 (Node 당)**

| 디스크 구분 | 파티션 | 용량 | 용도 | 파일시스템 | 비고 |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Disk 1 & 2** | `p1` (RAID 1) | 300 GB | **OS & K8s System** | XFS | `/` (루트), `/var/lib/docker`, `/var/lib/kubelet` |
| **Disk 1** | `p2` (Raw) | 나머지 (~3.5T) | **Data 1 (Local PV)** | - | LVM Volume Group 또는 Raw Partition |
| **Disk 2** | `p2` (Raw) | 나머지 (~3.5T) | **Data 2 (Local PV)** | - | LVM Volume Group 또는 Raw Partition |

* **핵심:** OS 영역은 RAID 1로 묶어 **디스크 하나가 죽어도 노드가 죽지 않게(안정성)** 하고, 나머지 7TB에 달하는 영역은 RAID를 풀고 **각각의 고속 데이터 저장소(Local PV)**로 사용합니다.

---

### 2. 쿠버네티스 스토리지 클래스(StorageClass) 구성 전략

위의 파티션을 쿠버네티스에서 어떻게 소비할지에 대한 전략입니다. 191대의 규모와 워크로드 특성을 고려할 때 **두 가지 스토리지 클래스**를 혼용해야 합니다.

#### A. 주력 스토리지: Local PV (OpenEBS LVM LocalPV 또는 TopoLVM)
데이터 분석 및 DB 워크로드의 90%는 이것을 사용합니다.

* **특징:** 네트워크를 타지 않고 해당 노드의 NVMe에 직접 씁니다. (속도가 가장 빠름)
* **사용 대상:**
    * **Spark/Trino:** Shuffle 데이터, Temp spill (속도가 생명, 데이터 날아가도 재수행하면 그만).
    * **Opensearch:** 자체적으로 Sharding/Replica를 관리하므로 스토리지는 빠른 게 최고.
    * **CNPG (PostgreSQL):** 자체 복제본(Primary-Standby)이 있으므로 Local PV가 권장됨.
    * **Kafka/MinIO:** (만약 설치한다면) 무조건 Local PV.
* **구축 도구:** **OpenEBS LVM LocalPV** (가장 널리 쓰임) 또는 **TopoLVM**.

#### B. 보조 스토리지: Distributed Block Storage (Rook-Ceph)
데이터 보존이 중요하고, 파드가 노드를 옮겨 다녀야 하는 경우 사용합니다.

* **특징:** 3-Replica 복제로 데이터 안전성 보장. 노드가 죽어도 다른 노드에서 데이터에 접근 가능.
* **사용 대상:**
    * **ArgoCD, Jenkins:** 설정 데이터 보존.
    * **Airflow:** DAG 저장소 (RWX - ReadWriteMany 모드가 필요한 경우).
    * **Prometheus/Grafana:** 설정 및 대시보드 데이터 (단, 메트릭 데이터 자체는 Local PV 권장).
* **구축 팁:** 191대 모든 노드에 Ceph를 깔면 오버헤드가 큽니다. **별도의 스토리지 전용 노드군(5~10대)을 지정**하거나, 각 랙 별로 3대씩만 선정하여 구성하는 것을 추천합니다.

---

### 3. 워크로드별 매핑 가이드 (체크리스트용)

구축 후 각 팀에게 가이드할 내용입니다.

| 시스템 | 권장 스토리지 클래스 | 이유 |
| :--- | :--- | :--- |
| **Spark Operator** | `local-nvme-fast` | Shuffle 발생 시 엄청난 Random Write 발생. 네트워크 스토리지 사용 시 전체 클러스터 네트워크 마비됨. |
| **Trino Worker** | `local-nvme-fast` | Spill-to-disk 발생 시 NVMe 속도 필수. |
| **CNPG (PostgreSQL)** | `local-nvme-fast` | DB 트랜잭션 성능 확보. HA는 CNPG 오퍼레이터가 담당. |
| **Opensearch** | `local-nvme-fast` | 인덱싱 속도 확보. HA는 Opensearch가 담당. |
| **Airflow (Logs)** | `ceph-filesystem` (RWX) | 여러 워커에서 로그를 한곳에 모으거나 읽어야 함. |
| **Jenkins Home** | `ceph-block` (RWO) | 마스터 노드가 죽어서 다른 노드에 떠도 기존 설정/빌드 기록이 유지되어야 함. |

---

### 4. 정리 및 제안

**[최종 구성 제안서]**

1.  **OS 설치:** MDADM으로 2개 디스크의 앞부분(300GB)만 잘라서 **RAID 1** 구성. (안정성 확보)
2.  **데이터 영역:** 남은 공간은 LVM Volume Group (`vg_data`)으로 설정.
3.  **CSI 설치:** **OpenEBS LVM LocalPV**를 설치하여 `vg_data`를 쿠버네티스 `StorageClass`로 연결.
4.  **보조 스토리지:** Airflow DAG 공유나 Jenkins 등을 위해 **Rook-Ceph**를 일부 노드(또는 전용 노드)에 작게 구성하거나, 이미 있는 외부 NFS가 있다면 그것을 활용하여 복잡도를 줄임.

이 방식이 **"성능(Spark)"**과 **"안정성(OS)"** 두 마리 토끼를 모두 잡는, 그리고 191대 규모에서 스토리지 비용을 0원으로 만드는 가장 효율적인 방법입니다.

이 구성으로 **Phase 1: 아키텍처 설계**에 반영해도 될까요? 승인하시면 이를 바탕으로 Kubespray 변수 설정값까지 포함된 상세 계획을 짜겠습니다.

>>>

마지막으로 대안 (Cluster API - CAPI): 최근 엔터프라이즈 환경에서는 **Cluster API (with Metal3 or Tinkerbell)**를 사용하여 쿠버네티스를 '선언적'으로 관리하는 추세입니다. 이는 업데이트 실패 시 롤백이 더 용이하고, 노드 프로비저닝부터 생명주기 관리가 통합됩니다. Metal3나 Tinkerbell에 대해 좀 더 자세히 알려줘. 

>>>

Kubespray가 '익숙한 방식(Ansible로 접속해서 설치)'이라면, **Cluster API (CAPI) + Metal3/Tinkerbell**은 '클라우드 방식(API로 서버를 찍어내기)'입니다.

이 기술들은 **"물리 서버(Baremetal)를 마치 AWS EC2 인스턴스 다루듯이 관리하게 해주는 도구"**입니다. 191대 규모에서 운영 효율성을 획기적으로 높일 수 있는 기술들이지만, 러닝 커브가 확실히 존재합니다.

두 기술의 핵심과 차이점, 그리고 현재 상황에서의 적합성을 정리해 드릴게요.

---

### 1. 기본 개념: 왜 필요한가? (Immutable Infrastructure)

**Kubespray (Mutable)**
* 업데이트 시: 기존 서버에 접속해서 패키지를 지우고, 새 버전을 깔고, 설정을 바꿉니다.
* 문제점: 업데이트 도중 에러가 나면? 설정이 꼬여서 "어정쩡한 상태"가 된 노드가 발생합니다. 롤백하려면 사람이 들어가서 고쳐야 합니다.

**CAPI + Metal3/Tinkerbell (Immutable)**
* 업데이트 시: **"기존 노드를 버리고(Wipe), 새 OS 이미지가 구워진 노드로 다시 띄웁니다."**
* 장점: 항상 깨끗한 상태(Clean State)에서 시작합니다. 실패하면 그냥 이전 이미지로 다시 부팅하면 됩니다. 이것이 **완벽한 롤백**을 보장하는 핵심입니다.

---

### 2. Metal3 (Metal Kubed) 상세 설명



Metal3는 **Red Hat(OpenShift)**이 주도하는 프로젝트로, 사실상 베어메탈 프로비저닝의 '표준'에 가깝습니다.

* **핵심 엔진 (Ironic):** OpenStack에서 수년간 검증된 `Ironic`이라는 강력한 베어메탈 관리 도구를 사용합니다.
* **동작 방식:**
    1.  사용자가 "노드 1개 더 줘 (K8s v1.30 이미지로)"라고 YAML을 작성합니다.
    2.  Metal3 컨트롤러가 `BMC (IPMI/Redfish)`를 통해 꺼져 있는 물리 서버의 전원을 켭니다.
    3.  PXE 부팅을 통해 서버 디스크를 밀고, 지정된 OS 이미지를 씁니다.
    4.  설치가 끝나면 쿠버네티스 클러스터에 Join 시킵니다.
* **장점:** 매우 안정적이고 기능이 방대합니다. 서버 하드웨어 제어(Firmware update, RAID config 등) 기능이 강력합니다.
* **단점:** 무겁습니다. Ironic 자체가 복잡해서 구축 난이도가 높습니다.

### 3. Tinkerbell 상세 설명



[Image of Tinkerbell workflow architecture]


Tinkerbell은 **Equinix Metal(구 Packet)**이라는 베어메탈 클라우드 회사에서 만든 도구입니다. Metal3보다 좀 더 현대적이고 유연합니다.

* **핵심 엔진 (Workflow):** 모든 작업을 '워크플로우'로 정의합니다.
* **동작 방식:**
    1.  서버가 부팅되면 `Hook`이라는 램디스크 OS(InMemory OS)가 뜹니다.
    2.  이 `Hook`은 Tinkerbell 서버에게 "나 뭐 하면 돼?"라고 묻습니다.
    3.  Tinkerbell은 미리 정의된 **Docker 컨테이너**들을 순서대로 실행시킵니다. (예: 1. 디스크 포맷 컨테이너 실행 -> 2. OS 설치 컨테이너 실행 -> 3. 네트워크 설정 컨테이너 실행)
* **장점:** **유연성**이 최고입니다. OS 설치 과정을 도커 컨테이너로 맘대로 짤 수 있습니다. 구조가 Metal3보다 가볍습니다.
* **단점:** Metal3에 비해 커뮤니티 규모가 작고, 아직 엔터프라이즈 레퍼런스가 적은 편입니다.

---

### 4. 비교 및 요약

| 구분 | **Metal3** | **Tinkerbell** |
| :--- | :--- | :--- |
| **기반 기술** | OpenStack Ironic | Docker Container Workflow |
| **철학** | "모든 하드웨어를 완벽하게 제어한다" | "프로비저닝 과정을 내 맘대로 짠다" |
| **난이도** | 상 (구성 요소가 많고 복잡함) | 중상 (워크플로우 작성 필요) |
| **성숙도** | 매우 높음 (OpenShift의 기반 기술) | 성장 중 (CNCF Sandbox) |
| **K8s 통합** | Cluster API Provider Metal3 (CAPM3) | Cluster API Provider Tinkerbell (CAPT) |

---

### 5. 191대 클러스터 구축을 위한 제언 (의사결정 가이드)

현재 상황(네트워크/스토리지도 고민해야 하고, 일정 압박과 회사 분위기)을 고려했을 때의 솔직한 조언입니다.

#### Q. 지금 CAPI(Metal3/Tinkerbell)로 선회해야 할까요?
**아니요, 이번 프로젝트에서는 `Kubespray`를 유지하는 것을 추천합니다.**

**이유:**
1.  **복잡도의 폭발:** 191대의 하드웨어 제어를 위해 `Management Cluster`를 따로 구축해야 하고, PXE/TFTP/DHCP 환경을 완벽하게 구성해야 합니다. 네트워크 지식이 부족하다고 하셨는데, CAPI 베어메탈 세팅은 **네트워크 지옥**이 될 수 있습니다.
2.  **시간 부족:** Kubespray는 `Inventory` 파일만 잘 짜면 바로 돌릴 수 있지만, CAPI는 OS 이미지를 빌드하는 파이프라인부터 만들어야 합니다.
3.  **회사 표준:** 다른 팀이 이미 Kubespray를 쓰고 있다면, 장애 발생 시 도움을 받거나 지식을 공유하기 좋습니다. 혼자 튀는 기술을 썼다가 장애가 나면 모든 책임이 쏠립니다.

#### 대안: Kubespray의 단점(롤백/업데이트)을 보완하는 전략
Kubespray를 쓰시되, 운영 프로세스로 극복합시다.

1.  **철저한 백업:** Etcd 스냅샷 백업을 매일 수행하고, S3(MinIO)로 자동 전송하는 스크립트를 짭니다. (Velero 활용)
2.  **소규모 배치 업데이트:** 191대를 한 번에 업데이트하지 말고, **Worker Node 5대씩** 끊어서 업데이트하는 `ansible --limit` 전략을 수립합니다. 실패하더라도 5대만 영향받습니다.
3.  **노드 재설치 자동화:** 업데이트가 꼬여서 노드가 망가졌을 때, **"고치려 하지 말고 OS 다시 깔고 Kubespray 재실행"** 하는 절차(PXE 부팅 서버 정도만 간단히 구축)를 마련합니다.

---

### [Next Step]
이제 방향성이 명확해졌습니다.

1.  **설치 도구:** **Kubespray** (단, 안정성을 위한 백업/배치 전략 포함)
2.  **네트워크:** **L3 Switch + BGP + Cilium** (MetalLB 제거)
3.  **스토리지:** **Local PV (NVMe) + Rook-Ceph (보조)**

이 결정 사항들을 바탕으로 **[Phase 1: 아키텍처 상세 설계 및 Kubespray 튜닝 가이드]**를 작성해 드리겠습니다.
혹시 시작하기 전에 **"OS 버전(Ubuntu/Rocky 등)"**이나 **"컨테이너 런타임(Containerd/CRI-O)"** 선호도가 있으신가요? 특별히 없으면 가장 안정적인 조합(Rocky Linux 9 + Containerd)으로 설계하겠습니다.

>>>

OS는 델 서버 구입시에 Redhat 10 버전과 같이 들어와

>>>

RHEL(Red Hat Enterprise Linux) 10을 사용하신다면 OS 안정성과 보안 측면에서 엔터프라이즈 환경에 가장 적합한 선택을 하셨습니다. 다만, RHEL 10은 최신 버전이므로 Kubespray 최신 릴리즈와의 호환성을 확인하는 과정이 설치 전 필요할 수 있습니다.

지금까지 논의된 모든 결정 사항(네트워크, 스토리지, 도구 등)을 종합하여 \*\*[Phase 1: 클러스터 아키텍처 상세 설계 및 Kubespray 구성 전략]\*\*을 정리했습니다.

이 문서는 구축 실무의 **'설계도'** 역할을 합니다.

-----

# Phase 1. 온프레미스 대규모 쿠버네티스 구축 설계서

## 1\. 노드 구성 및 OS 레이아웃 (Node Layout)

191대 노드 전체에 동일하게 적용되는 구성입니다. RHEL 10의 특성과 고성능 I/O를 고려하여 설계했습니다.

### 1.1 디스크 파티셔닝 (Partitioning Strategy)

NVMe 3.84TB 2장을 활용하여 **OS 안정성**과 **데이터 성능**을 물리적으로 분리합니다.

| Disk ID | Partition | Size | Filesystem | Mount Point / Role | 상세 설명 |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **NVMe 0 & 1** | `p1` (RAID 1) | 300 GB | XFS | `/` (Root) | OS, Container Runtime, Kubelet 로그 저장용. 디스크 1장 장애 시에도 OS 생존. |
| **NVMe 0** | `p2` | \~3.5 TB | Raw | - | **Local PV (Volume Group 1)**. Spark Shuffle/Trino Spill 등 고속 I/O 전용. |
| **NVMe 1** | `p2` | \~3.5 TB | Raw | - | **Local PV (Volume Group 2)**. DB/Search/MsgQ 등 고속 I/O 전용. |

  * **RHEL 설정 포인트:** Swap은 비활성화(`swapoff -a`)하며, `/etc/fstab`에서도 주석 처리 필수.

### 1.2 네트워크 인터페이스 (Bonding)

Broadcom 10/25GbE 2-port를 사용하여 대역폭과 가용성을 확보합니다.

  * **Bonding Mode:** **Mode 4 (802.3ad LACP)**
      * 스위치 측 설정: LACP (Active-Active) 설정 필수.
      * Hash Policy: `layer3+4` (IP+Port 기반 해싱) → 트래픽 분산 효율 극대화.
  * **VLAN 구성:** 호스트 레벨에서는 단일 네트워크(Flat)를 사용하고, Pod/Service 네트워크는 Cilium 내부에서 관리하는 것을 권장.

-----

## 2\. 네트워크 아키텍처 (Network Architecture)

가장 중요한 성능 포인트입니다. 물리 스위치와 쿠버네티스가 BGP로 직접 대화합니다.

### 2.1 주요 구성

  * **CNI Plugin:** **Cilium** (eBPF 기반, 고성능)
  * **Routing Mode:** **Native Routing (Direct Routing)**
      * VXLAN/Geneve 터널링 **사용 안 함**. (Overhead Zero)
  * **Load Balancer:** **Cilium BGP Control Plane** (MetalLB 대체)
  * **External Access:** Ingress Controller(Nginx or Cilium Ingress)의 Service type을 `LoadBalancer`로 설정하여 외부 노출.

### 2.2 BGP 설정 값 (예시)

  * **AS Number:**
      * Spine/Leaf Switch: `65000` (예시)
      * Kubernetes Cluster: `65001`
  * **Peering:** 모든 노드(Worker 포함)가 ToR(Leaf) 스위치와 eBGP Peering.

-----

## 3\. 스토리지 전략 (Storage Strategy)

데이터 파이프라인(Spark)의 성능을 위해 **Local PV**를 주력으로 사용합니다.

### 3.1 StorageClass 정의

1.  **`sc-local-nvme` (Default):**
      * **Backend:** OpenEBS LVM LocalPV (또는 TopoLVM).
      * **특징:** 노드의 NVMe `p2` 파티션을 LVM으로 묶어 동적으로 할당. 속도 최상.
      * **용도:** Spark, Trino, Kafka, Opensearch, Postgres(CNPG).
2.  **`sc-ceph-block` (Optional):**
      * **Backend:** Rook-Ceph (RBD).
      * **구성:** 별도 스토리지 노드군(또는 일부 노드)에 구성.
      * **용도:** ArgoCD Config, Jenkins Home, Prometheus AlertRule 등 "지워지면 안 되고 용량이 작은" 데이터.

-----

## 4\. Kubespray 튜닝 및 설정 (Configuration)

191대 규모 설치 및 운영을 위한 `inventory/mycluster/group_vars` 핵심 설정값입니다. Kubespray 기본값으로는 설치가 너무 오래 걸리거나 실패할 수 있습니다.

### 4.1 `k8s-cluster.yml` (핵심 설정)

```yaml
# 네트워크 플러그인: Cilium
kube_network_plugin: cilium
cilium_kube_proxy_replacement: true # Kube-proxy 없이 eBPF로 처리 (성능 향상)
cilium_enable_bgp_control_plane: true # BGP 기능 활성화

# 컨테이너 런타임
container_manager: containerd

# 서비스 프록시 모드
kube_proxy_mode: ipvs # Cilium replacement 사용 시 무시되지만 설정 권장

# 노드 스케일링 고려
kube_api_qps: 100
kube_api_burst: 200
```

### 4.2 대규모 설치 속도 최적화 (`ansible.cfg` & CLI)

Kubespray는 기본적으로 안전하게 천천히 설치합니다. 191대를 위해 병렬 처리를 늘려야 합니다.

  * **Strategy:** `mitogen` (Ansible 속도 가속 플러그인) 사용 고려 (Kubespray 지원 여부 확인 필요) 또는 `pipelining = True`.
  * **Forks:** 기본 5 → **50 이상**으로 설정 (`ansible-playbook -f 50 ...`).
  * **Download Run Once:** `download_run_once: true`
      * 컨테이너 이미지를 배포 서버(Ansible실행기)가 한 번만 다운받고 노드들에게 전파 (외부 대역폭 절약).

-----

## 5\. 멀티테넌시 및 리소스 관리 계획

4개 팀이 서로 침범하지 않도록 논리적 격리를 수행합니다.

### 5.1 네임스페이스 및 RBAC

  * **구조:** `team-a`, `team-b`, `team-c`, `team-d`, `common-platform`
  * **접근 제어:** Keycloak + Kubernetes RBAC 연동. 각 팀원은 자신의 네임스페이스에만 `Edit` 권한, 타 팀 네임스페이스는 `View` 혹은 차단.

### 5.2 리소스 스케줄링 (YuniKorn)

  * **설치:** Helm Chart로 설치.
  * **Queue 구조:**
      * `root.platform` (Guaranteed): ArgoCD, Monitoring 등 (선점 불가).
      * `root.users.team-a`: Min/Max Resource 설정.
      * `root.users.team-b`: ...
  * **Gang Scheduling:** Spark Job 제출 시 "Executor 50개가 한 번에 안 뜨면 아예 대기"하도록 설정 (데드락 방지 및 자원 효율화).

-----

## 6\. [중요] RHEL 10 호환성 체크 포인트

RHEL 10은 매우 최신이므로 아래 사항을 설치 전 반드시 검증해야 합니다.

1.  **Python 버전:** Ansible/Kubespray는 타겟 노드의 Python 버전을 탑니다. RHEL 10의 기본 Python 버전이 Kubespray 요구사항과 맞는지 확인.
2.  **IPTables/NFTables:** RHEL 10부터 iptables가 완전히 deprecated 되고 nftables가 강제될 수 있습니다. Kubespray가 이를 처리하는지 확인 필요.
3.  **SELinux:** Kubespray가 자동으로 설정하지만, RHEL 메이저 버전 변경 시 정책 충돌이 있을 수 있습니다. (초기 설치 시엔 `Permissive` 모드 권장)

-----

### [Next Step] Phase 2: 상세 구축 계획 및 체크리스트 작성

위 아키텍처(설계도)에 동의하신다면, 이제 실제로 \*\*"어떤 순서로 명령어를 치고, 무엇을 확인해야 하는지"\*\*에 대한 \*\*[Phase 2: 구축 액션 플랜 및 체크리스트]\*\*를 작성하겠습니다.

여기에 포함될 내용은 다음과 같습니다:

1.  **Pre-requisite 체크리스트:** 네트워크 팀/서버 팀에게 요청할 구체적인 항목 (VLAN ID, IP 리스트, 스위치 설정값 등).
2.  **설치 순서 (Step-by-Step):** OS 세팅 → Kubespray 실행 → 애드온 설치 순서.
3.  **검증(Smoke Test) 시나리오:** "설치가 잘 되었다"를 판단하는 기준.

이대로 Phase 2 작성을 진행할까요? 아니면 수정하고 싶은 설계 부분이 있으신가요?

>>>

일단  Yunikorn 사용은 결정하지 못했으니까 계획에서 제거해주고나 추후에 추가하는 방향으로 설정해줘

>>>

알겠습니다. YuniKorn은 일단 제외하고, \*\*쿠버네티스 네이티브 기능(ResourceQuota, LimitRange)\*\*과 \*\*정책 엔진(Kyverno/OPA)\*\*을 활용하여 리소스 관리 및 제한 정책을 수립하는 것으로 계획을 수정하겠습니다.

이로써 아키텍처는 확정되었으므로, \*\*[Phase 2: 상세 구축 액션 플랜 및 체크리스트]\*\*를 작성합니다. 이 내용은 실제 구축 시 작업 순서서(Work Order)로 사용하시면 됩니다.

-----

# Phase 2. 구축 액션 플랜 및 체크리스트

## 1\. 사전 준비 단계 (Pre-requisites)

**목표:** 하드웨어 및 네트워크 팀으로부터 물리적 환경을 완벽하게 인도받기.

### 1.1 네트워크 팀 요청 사항 (Checklist)

  * [ ] **Switch 설정:** Top-of-Rack(Leaf) 스위치에 L3 라이선스 활성화 및 BGP 설정 완료 확인.
  * [ ] **LACP 구성:** 각 서버 포트 2개에 대해 LACP(802.3ad) Bonding 모드 설정 (Switch Side).
  * [ ] **MTU 설정:** Jumbo Frame 사용 여부 결정 (데이터 성능을 위해 9000 사용 권장, 스위치-서버 일치 필수).
  * [ ] **IP 할당:**
      * Node IP (191개 + 예비)
      * VIP용 IP 대역 (Ingress LoadBalancer용, BGP로 광고될 대역)
  * [ ] **ASN 정보:** 스위치 측 AS Number와 쿠버네티스 클러스터용 AS Number 확정.

### 1.2 서버 팀 요청 사항 (Checklist)

  * [ ] **BIOS/Firmware:** 모든 노드의 BIOS 및 NIC 펌웨어 최신 버전 통일.
  * [ ] **BIOS 설정:**
      * Hyper-Threading: Enabled
      * Power Profile: **Performance Mode** (매우 중요, Power Saving 금지)
      * NUMA: Enabled
  * [ ] **IPMI/iDRAC:** 원격 접속 가능한 IP 설정 및 계정 확보.

-----

## 2\. OS 및 디스크 구성 단계 (OS & Partitioning)

**목표:** RHEL 10 설치 및 스토리지 아키텍처 적용.

### 2.1 OS 설치 및 파티셔닝 (Kickstart 활용 권장)

  * **Action:** 191대 노드에 RHEL 10 설치.
  * **Disk Layout:**
      * `md0` (RAID 1, /dev/nvme0n1p1 + /dev/nvme1n1p1): 300GB, Mount `/` (XFS)
      * `nvme0n1p2`: 나머지 용량, **Unmounted** (Local PV용)
      * `nvme1n1p2`: 나머지 용량, **Unmounted** (Local PV용)
  * **필수 설정:**
      * [ ] Swap 비활성화 (`swapoff -a` & `/etc/fstab` 주석 처리).
      * [ ] Firewall: Firewalld 비활성화 또는 K8s 포트 허용.
      * [ ] SELinux: 초기 설치 시 `Permissive` 모드 설정 권장 (`/etc/selinux/config`).
      * [ ] Network: Bonding (mode 4) 설정 확인.

-----

## 3\. 쿠버네티스 클러스터 설치 (Kubespray)

**목표:** Kubespray를 통해 기본 클러스터 및 네트워크(Cilium) 배포.

### 3.1 Inventory 작성

  * **Control Plane:** 고사양 노드 3대 (Etcd 동시 배포 또는 분리).
  * **Worker Node:** 나머지 188대.

### 3.2 변수 설정 (`group_vars/k8s_cluster/k8s-cluster.yml`)

  * **Network:** `kube_network_plugin: cilium`
  * **Cilium:** `cilium_enable_bgp_control_plane: true`
  * **Proxy:** `kube_proxy_strict_arp: true`
  * **Container Runtime:** `container_manager: containerd`

### 3.3 설치 실행

  * **명령어:** `ansible-playbook -i inventory/mycluster/hosts.yaml --become --become-user=root cluster.yml`
  * **Tip:** 191대 한 번에 수행 시 타임아웃 가능성 있음. 배포 서버 성능을 높이거나 `-f 50` 등으로 병렬 처리 수 조정.

-----

## 4\. 시스템 애드온 및 스토리지 구성

**목표:** 스토리지 연결 및 모니터링/로깅 기반 마련.

### 4.1 스토리지 클래스 구축

1.  **Local PV (OpenEBS LVM or TopoLVM):**
      * [ ] 모든 노드에 에이전트 설치.
      * [ ] `nvme0n1p2`, `nvme1n1p2`를 Volume Group으로 등록.
      * [ ] StorageClass `local-nvme` 생성.
2.  **Distributed Storage (Rook-Ceph):** (옵션)
      * [ ] 선정된 스토리지 노드에 Rook-Ceph 클러스터 배포.
      * [ ] StorageClass `ceph-block`, `ceph-filesystem` 생성.

### 4.2 네트워크 BGP Peering

  * [ ] `CiliumBGPPeeringPolicy` 리소스 생성.
  * [ ] 물리 스위치와 BGP Session `Established` 상태 확인 (`cilium status` CLI 활용).

### 4.3 관측성 (Observability)

  * [ ] **Prometheus Stack:** Node Exporter, Kube-State-Metrics 설치. (데이터 저장소는 Local PV 권장)
  * [ ] **Opensearch + Fluentbit:** 로그 수집 파이프라인 구축.

-----

## 5\. 운영 정책 및 멀티테넌시 설정 (YuniKorn 제외 버전)

**목표:** 팀별 격리 및 리소스 제한 정책(LimitEnforcer) 적용.

### 5.1 네임스페이스 및 권한 (RBAC)

  * [ ] 4개 팀용 Namespace 생성 (`team-a`, `team-b`...).
  * [ ] Keycloak 연동을 통한 팀별 User Group 매핑.

### 5.2 리소스 쿼타 (ResourceQuota)

각 팀이 클러스터 자원을 무한정 쓰는 것을 막습니다. (YuniKorn 대체)

```yaml
apiVersion: v1
kind: ResourceQuota
metadata:
  name: team-a-quota
  namespace: team-a
spec:
  hard:
    requests.cpu: "1000"   # 팀 전체가 요청 가능한 CPU 총합
    requests.memory: 4Ti
    limits.cpu: "2000"
    limits.memory: 8Ti
```

### 5.3 리소스 정책 강제화 (LimitRange & Validating Webhook)

사용자 요구사항: **"Request/Limit 설정 없이는 파드 생성 금지"**

1.  **LimitRange (기본값 설정):** 사용자가 값을 안 적으면 자동으로 기본값을 넣어줍니다.
    ```yaml
    apiVersion: v1
    kind: LimitRange
    metadata:
      name: default-limits
      namespace: team-a
    spec:
      limits:
      - default:
          memory: 4Gi
          cpu: 2
        defaultRequest:
          memory: 2Gi
          cpu: 1
        type: Container
    ```
2.  **Kyverno (강제 차단 - 권장):** 값을 안 적으면 아예 에러를 뱉고 생성을 막으려면 Kyverno 같은 정책 엔진이 필요합니다.
      * [ ] Kyverno 설치.
      * [ ] `require-requests-limits` ClusterPolicy 적용.

-----

## 6\. 애플리케이션 플랫폼 설치

**목표:** 공통/개별 플랫폼 설치 및 연동.

  * [ ] **Common:** ArgoCD, Jenkins (StorageClass: Ceph 권장).
  * [ ] **Data:** Spark Operator, Trino (StorageClass: Local NVMe 필수).
  * [ ] **Ingress:** 각 서비스별 Ingress 생성 및 외부 접속 테스트.

-----

## 7\. 최종 검증 체크리스트 (Final Verification)

이 체크리스트는 보고서의 **"구축 결과"** 섹션에 들어갈 핵심 증빙 자료입니다.

### 7.1 인프라 레벨

  * [ ] **Node Ready:** `kubectl get nodes` 시 191대 모두 `Ready` 상태인가?
  * [ ] **Network Bandwidth:** `iperf3` 테스트 시 노드 간 20Gbps 이상(Bonding 효율 고려) 나오는가?
  * [ ] **Disk I/O:** `fio` 테스트 시 Local PV에서 NVMe 스펙에 준하는 IOPS가 나오는가?

### 7.2 쿠버네티스 기능 레벨

  * [ ] **Pod Scheduling:** 1000개 이상의 Dummy Pod 배포 시 고르게 분산되는가?
  * [ ] **Auto Recovery:** 노드 1대를 강제 종료했을 때, 해당 노드의 파드가 다른 노드로 이동하는가?
  * [ ] **External Access:** 외부망에서 Ingress 도메인으로 접속이 되는가? (BGP 라우팅 확인)

### 7.3 워크로드 레벨 (성능 테스트)

  * [ ] **Spark Job:** 샘플 Spark Job (TeraSort 등) 실행 시 Shuffle 단계에서 `Local PV`를 정상적으로 사용하며 속도가 나오는가?
  * [ ] **Trino Query:** 대량 데이터 조회 시 Worker들이 CPU/Memory를 고르게 사용하는가?
  * [ ] **Resource Limit:** Limit을 설정하지 않은 Pod 생성 시도 시 **거부(Deny)** 되는가? (정책 적용 확인)

-----

### [Next Step] Phase 3: 보고서 작성 가이드

이제 **구축 계획**과 **체크리스트**가 완성되었습니다. 이 내용을 바탕으로 실제 구축을 진행하시게 됩니다.

마지막으로, \*\*"상부 보고용 보고서 작성"\*\*을 위해 **목차와 핵심 강조 포인트**를 정리해 드릴까요? 아니면 특정 단계(예: Kubespray 변수, Kyverno 정책 상세 등)에 대해 더 깊은 기술적 설명이 필요하신가요?

>>>

Kyverno 말고 OPA를 사용할께

>>>

좋습니다. OPA(Gatekeeper)는 CNCF 졸업(Graduated) 프로젝트로, 쿠버네티스 정책 관리의 \*\*사실상 표준(De Facto Standard)\*\*에 가깝습니다. Rego 언어를 사용해야 해서 진입 장벽이 조금 있지만, 훨씬 강력하고 범용적인 정책 관리가 가능합니다.

변경된 결정을 반영하여 \*\*[Phase 2 수정안: OPA Gatekeeper 정책 설정]\*\*과 마지막 단계인 \*\*[Phase 3: 구축 결과 보고서 작성 가이드]\*\*를 정리해 드리겠습니다.

-----

### [수정] Phase 2-5.3. 리소스 정책 강제화 (OPA Gatekeeper)

Kyverno 대신 OPA Gatekeeper를 사용하여 \*\*"Request/Limit이 설정되지 않은 파드는 생성 자체를 거부(Deny)"\*\*하는 정책을 구현합니다.

#### 1\. 설치 (Installation)

  * 공식 Helm Chart를 사용하여 설치합니다.
  * 191대 규모이므로 `replicas`를 3개 이상으로 설정하여 고가용성을 확보해야 합니다.
    ```bash
    helm repo add gatekeeper https://open-policy-agent.github.io/gatekeeper/charts
    helm install gatekeeper/gatekeeper --name-template=gatekeeper --namespace gatekeeper-system --create-namespace --set replicas=3
    ```

#### 2\. 정책 정의: `ConstraintTemplate` (Rego 로직)

먼저 "컨테이너에 cpu/memory limit이 있는지 검사하는 로직"을 정의합니다.

```yaml
apiVersion: templates.gatekeeper.sh/v1
kind: ConstraintTemplate
metadata:
  name: k8srequiredresources
spec:
  crd:
    spec:
      names:
        kind: K8sRequiredResources
  targets:
    - target: admission.k8s.gatekeeper.sh
      rego: |
        package k8srequiredresources

        violation[{"msg": msg}] {
          container := input.review.object.spec.containers[_]
          not container.resources.limits.cpu
          msg := sprintf("Container <%v> must have a cpu limit", [container.name])
        }

        violation[{"msg": msg}] {
          container := input.review.object.spec.containers[_]
          not container.resources.limits.memory
          msg := sprintf("Container <%v> must have a memory limit", [container.name])
        }
        # (필요 시 requests 체크 로직도 동일하게 추가)
```

#### 3\. 정책 적용: `Constraint` (범위 설정)

위에서 만든 로직을 "어디에 적용할지" 정의합니다. `kube-system` 등 관리용 네임스페이스는 예외 처리(`excludedNamespaces`)하는 것이 안전합니다.

```yaml
apiVersion: constraints.gatekeeper.sh/v1beta1
kind: K8sRequiredResources
metadata:
  name: must-have-limits
spec:
  match:
    kinds:
      - apiGroups: [""]
        kinds: ["Pod"]
    excludedNamespaces: ["kube-system", "gatekeeper-system", "monitoring"]
```

-----

### [Phase 3] 구축 결과 보고서 작성 가이드

지금까지의 긴 논의(하드웨어, 네트워크, 스토리지, OPA 등)를 상급자나 타 부서에 설득력 있게 전달하기 위한 **보고서 목차와 핵심 작성 포인트**입니다.

#### 1\. 보고서 개요

  * **제목:** 온프레미스 대규모 데이터 분석용 쿠버네티스 클러스터 구축 계획(안)
  * **목표:** 191대 고사양 노드 기반의 고성능 데이터 파이프라인 및 멀티테넌시 환경 구축
  * **핵심 키워드:** **고성능 I/O (NVMe Local PV)**, **네트워크 최적화 (Cilium BGP)**, **운영 효율화 (Kubespray, OPA)**

#### 2\. 상세 목차 및 강조 포인트

| 목차 | 세부 내용 | 보고서 강조 포인트 (설득 논리) |
| :--- | :--- | :--- |
| **1. 추진 배경 및 목적** | - 191대 노드 통합 및 자원 효율화<br>- 팀별 격리된 분석 환경 제공 | - 단순한 서버 모음이 아닌 **'사내 프라이빗 클라우드'** 구축임을 강조.<br>- 개별 서버 운영 대비 **관리 비용 절감** 효과 언급. |
| **2. 인프라 아키텍처** | **2.1 하드웨어 및 OS**<br>- RHEL 10 도입 및 BIOS 튜닝<br>- 디스크 레이아웃 (OS RAID1 + Data NVMe)<br>**2.2 네트워크**<br>- L3 Switch + Cilium BGP (Native Routing)<br>- MetalLB 제거를 통한 구조 단순화 | - **스토리지 비용 0원 달성:** 고가의 SAN/NAS 구매 없이 NVMe Local PV 활용으로 성능 극대화 및 비용 절감.<br>- **네트워크 병목 해소:** BGP 도입으로 오버헤드 없는 통신 속도 보장 (데이터 분석 필수 요건). |
| **3. 클러스터 구축 방안** | **3.1 배포 전략 (Kubespray)**<br>- 사내 표준 준수 및 Ansible 튜닝<br>- 업데이트 및 롤백 시나리오 (배치 업데이트)<br>**3.2 스토리지 전략**<br>- 고속 데이터: Local PV (Spark/Trino)<br>- 데이터 보존: Rook-Ceph (설정/로그) | - **리스크 관리:** 업데이트 실패 시 전체 장애를 막기 위한 **'소규모 배치 업데이트'** 전략 수립.<br>- **기술 표준 준수:** 타 팀과의 기술 스택 통일로 유지보수 용이성 확보. |
| **4. 운영 및 정책 관리** | **4.1 멀티테넌시**<br>- Namespace 격리 및 RBAC (Keycloak 연동)<br>**4.2 리소스 통제 (OPA)**<br>- Request/Limit 강제화 (무제한 사용 방지)<br>- ResourceQuota를 통한 팀별 할당량 제한 | - **YuniKorn 대안:** OPA와 ResourceQuota만으로도 충분히 강력한 통제 가능함(복잡도 최소화).<br>- **리소스 독점 방지:** 특정 팀이나 작업이 전체 클러스터를 마비시키는 상황 원천 차단. |
| **5. 검증 계획 (Checklist)** | **5.1 기능 검증**<br>- 노드/파드 정상 동작, Failover 테스트<br>**5.2 성능 검증**<br>- Network Bandwidth, Disk IOPS 측정<br>- Spark Job 실제 수행 속도 측정 | - 단순 설치가 끝이 아님을 강조.<br>- **데이터 기반 검증:** "빠르다"가 아닌 "초당 OO GB 처리 가능"으로 수치화하여 보고. |
| **6. 향후 계획** | - 모니터링 대시보드 고도화<br>- YuniKorn 도입 검토 (워크로드 증가 시) | - 단계적 고도화 계획 제시로 신뢰도 확보. |

#### 3\. 첨부 자료 (Appendix)

보고서 뒤에 붙임 문서로 넣어두면 "제대로 준비했구나"라는 인상을 줄 수 있는 자료들입니다.

1.  **체크리스트 상세본:** 앞서 논의한 [Phase 2]의 체크리스트 엑셀 표.
2.  **네트워크 토폴로지 다이어그램:** 스위치-서버 연결 및 BGP 흐름도.
3.  **디스크 파티션 설계도:** NVMe 분할 및 용도 설명 그림.

-----

### [프로젝트 성공을 위한 마지막 조언]

191대 규모는 작은 실수가 큰 복구 비용으로 이어질 수 있습니다.

  * **"Keep It Simple (단순하게 유지하라)"**: YuniKorn을 일단 뺀 것은 아주 잘한 결정입니다. 초기 구축은 안정성과 기본 기능(네트워크, 스토리지)에 집중하고, 복잡한 스케줄링은 운영하면서 필요할 때 넣어도 늦지 않습니다.
  * **"자동화보다는 문서화 먼저"**: 처음부터 모든 걸 자동화하려다 지칠 수 있습니다. Kubespray 설정을 잘해두고, 운영 절차(트러블슈팅, 노드 증설 등)를 문서화하는 것이 더 중요합니다.

이제 계획과 설계, 그리고 보고서 작성 가이드까지 모두 마련되었습니다. 이 내용들을 바탕으로 성공적인 클러스터 구축이 되기를 응원합니다\! 혹시 진행하시다가 막히는 부분이 생기면 언제든 다시 물어봐 주세요.

>>>

변경 사항 추가할께. nvme 3.84T 2개 말고 OS용 디스크가 이미 있어. 그리고 Isilon이라는 NAS로 Ceph 대신에 사용할꺼야. 이 경우 디스크 구성을 어떻게 가져가는게 최적일까?

>>>

OS용 디스크가 따로 있고, 고성능 엔터프라이즈 NAS인 Isilon을 사용할 수 있다면 구성이 훨씬 깔끔하고 강력해집니다.

특히 **Isilon**은 엄청난 처리량을 가진 스케일아웃 NAS이므로, 굳이 관리 포인트가 늘어나는 Ceph를 구축할 필요가 없어졌습니다. 또한 **NVMe 2장을 온전히 데이터 처리에만 집중**할 수 있어 Spark/Trino 성능이 더욱 극대화됩니다.

변경된 환경에 맞춘 **[최적 스토리지 구성 전략]**과 이를 반영한 **[최종 보고서(Rev. 6)]**입니다.

---

### 💡 변경된 구성의 핵심 포인트

1.  **NVMe의 역할 변경:** OS 영역을 할당할 필요가 없어졌으므로, **NVMe 2장(약 7.68TB)을 통째로 LVM으로 묶어 고성능 데이터 전용 풀**로 사용합니다. (Spark Shuffle 속도 2배 향상 기대)
2.  **Etcd 성능 보장 (중요):** 별도의 OS 디스크가 있다고 하셨는데, 만약 이 디스크가 SATA SSD거나 HDD라면 Etcd 성능에 병목이 올 수 있습니다. 따라서 **Control Plane 노드는 NVMe의 일부를 잘라서 Etcd 전용으로 사용**하는 것을 강력히 권장합니다.
3.  **Isilon 활용:** Ceph 대신 Isilon을 사용하여 **"지워지면 안 되는 데이터(로그, 설정, 메트릭)"**를 저장합니다. `nfs-subdir-external-provisioner`를 사용하면 쿠버네티스에서 PVC를 생성할 때마다 Isilon에 자동으로 폴더가 생깁니다.

---

### [수정] 스토리지 파티션 및 구성 전략

| 노드 역할 | 디스크 구분 | 구성 방식 | 용도 및 마운트 포인트 |
| :--- | :--- | :--- | :--- |
| **Control Plane** | **OS Disk** | Standard Partition | `/` (OS 부팅 및 로그) |
| | **NVMe #1** | **Partition 1 (100GB)** | `/var/lib/etcd` (**Etcd 성능 보장**) |
| | | Partition 2 (Rest) | (예비용 또는 모니터링 데이터) |
| | **NVMe #2** | Raw | (예비용) |
| **Worker / Infra** | **OS Disk** | Standard Partition | `/` (OS 부팅, Docker Image) |
| | **NVMe #1 & #2** | **LVM Striping (RAID 0)** | **Local PV Pool (`vg_fast_data`)**- Spark Shuffle, Trino Spill, Opensearch Data- 2장을 스트라이핑하여 **속도 2배** 확보 |
| **공통 (Network)** | **Isilon NAS** | **NFS Mount** | **Shared Storage (`sc-isilon`)**- Jenkins Home, Airflow DAGs, Prometheus Data |

---

>>>

네트워크 인터페이스를 LCAP로 bonding mode 4로 사용하는 것과 용도별로 분리해서 쓰는것의 차이는?

>>>

매우 중요하고 실무적인 질문입니다. 특히 191대 규모에 \*\*Spark/Trino(대용량 데이터 트래픽)\*\*와 \*\*Isilon(스토리지 트래픽)\*\*이 혼재된 상황이라 네트워크 구성이 성능에 미치는 영향이 큽니다.

결론부터 말씀드리면, **현재 환경(데이터 분석, 2-Port NIC)에서는 LACP Bonding (Mode 4)가 압도적으로 유리합니다.**

두 방식의 차이점과 왜 LACP를 추천하는지, 그리고 물리적 분리가 필요한 예외 상황은 무엇인지 명확히 비교해 드립니다.

-----

### 1\. 두 방식의 개념적 차이

#### A. LACP Bonding (Mode 4) - "차선 합치기"

  * **개념:** 25G 포트 2개를 묶어 \*\*논리적인 50G 파이프 하나(`bond0`)\*\*로 만듭니다.
  * **동작:** 모든 트래픽(K8s 통신, Isilon NFS, SSH 관리 등)이 이 50G 파이프를 공유합니다. 스위치와 서버가 서로 합의(Handshake)하여 트래픽을 두 포트에 분산(Hashing)시킵니다.

#### B. 용도별 물리적 분리 (Separation) - "전용 차로제"

  * **개념:** 포트 1(`eth0`)은 서비스용, 포트 2(`eth1`)는 스토리지용으로 용도를 딱 정해서 씁니다.
  * **동작:**
      * **Port 1 (25G):** Spark Shuffle, Pod 간 통신, 외부 접속 (Service Network).
      * **Port 2 (25G):** Isilon NAS 마운트, 백업, 관리망 (Storage Network).

-----

### 2\. 상세 비교 분석표

| 비교 항목 | **LACP Bonding (Mode 4)** (추천) | **용도별 물리적 분리** |
| :--- | :--- | :--- |
| **대역폭 활용 (Bandwidth)** | **유연함 (Bursting 가능)**<br>스토리지 안 쓸 땐 서비스가 50G 풀사용.<br>서비스 안 쓸 땐 스토리지가 50G 풀사용. | **경직됨 (최대 25G 제한)**<br>서비스가 30G 필요해도 포트 1은 25G에서 막힘.<br>옆 포트(포트 2)가 놀고 있어도 못 빌려 씀. |
| **고가용성 (HA)** | **완벽한 이중화**<br>케이블 하나 끊어지거나 스위치 포트 1개 고장 나도,<br>남은 1개로 **서비스+스토리지 모두 생존**. | **이중화 불가 (치명적)**<br>포트 1 끊어짐 = 서비스 중단.<br>포트 2 끊어짐 = Isilon 연결 끊김 (Pod 에러). |
| **복잡도** | **중간** (스위치 설정 필요)<br>ToR 스위치에 LACP 설정 필수. | **높음** (라우팅 관리 필요)<br>OS 레벨에서 라우팅 테이블 쪼개야 함.<br>K8s CNI에 어떤 인터페이스 쓸지 지정해야 함. |
| **트래픽 간섭** | **발생 가능 (SW로 제어)**<br>Isilon 트래픽 폭주 시 서비스 느려질 수 있음.<br>(QoS나 VLAN으로 제어). | **물리적 차단**<br>스토리지 트래픽이 아무리 많아도<br>서비스 대역폭은 25G 보장됨. |

-----

### 3\. 왜 이번 프로젝트에는 LACP가 정답인가?

#### 이유 1: "이중화(Redundancy)"가 생명입니다.

현재 NIC 포트가 **단 2개**뿐입니다.

  * **분리할 경우:** 포트 하나만 죽어도 해당 노드는 **'반쪽짜리 장애'** 상태가 됩니다.
      * Ex) 스토리지용 포트 케이블 불량 → 해당 노드의 Spark Executor들이 Isilon에 로그를 못 써서 작업 실패.
  * **LACP의 경우:** 포트 하나가 죽으면 대역폭만 50G → 25G로 줄어들 뿐, **서비스와 스토리지는 끊김 없이 계속 돌아갑니다.** 191대 규모에서는 케이블/GBIC 불량이 생각보다 잦습니다.

#### 이유 2: 데이터 분석 워크로드의 "불규칙성"

Spark/Trino 작업은 항상 일정하게 네트워크를 쓰지 않습니다.

  * **Shuffle 단계:** Pod 간 통신(Service Network)이 폭발적으로 증가합니다. 이때 스토리지 쪽은 상대적으로 조용할 수 있습니다. LACP라면 50G를 셔플에 몰아줄 수 있습니다.
  * **Load/Save 단계:** Isilon(Storage Network) 트래픽이 폭발합니다. 이때 LACP라면 50G를 스토리지에 몰아줄 수 있습니다.
  * **물리적 분리 시:** 셔플할 때 25G에서 막히고, 저장할 때 25G에서 막힙니다. **전체 작업 시간이 늘어납니다.**

#### 이유 3: Isilon은 "네트워크 스토리지(NAS)"입니다.

Isilon은 iSCSI나 FC(Fibre Channel)처럼 전용 케이블이 필요한 게 아니라, \*\*일반 IP 네트워크(NFS)\*\*를 탑니다. 굳이 물리적으로 길을 나눌 필요 없이 \*\*VLAN(논리적 분리)\*\*으로 나누는 것이 현대적인 구성입니다.

-----

### 4\. 물리적 분리를 고려해야 하는 유일한 예외 상황

만약 아래 상황이라면 분리를 고민해볼 수 있습니다. 하지만 현재 구성(Spark/Trino)에는 해당하지 않아 보입니다.

1.  **보안상 물리적 망 분리가 법적으로 강제될 때:** (예: 공공/금융망에서 서비스망과 관리망을 물리적으로 섞지 마라)
2.  **포트가 4개 이상일 때:** 2개는 LACP로 서비스(Bond0), 나머지 2개는 LACP로 스토리지(Bond1) 구성. (이건 Best Practice지만 포트가 2개뿐이라 불가능)

-----

### 5\. 추천 구성 및 설정 가이드 (체크리스트용)

**결론: LACP Mode 4 (802.3ad) 사용**

다만, LACP를 쓰면서도 **"트래픽 간섭"을 막고 "관리 효율성"을 높이기 위해 VLAN을 사용**해야 합니다.

1.  **물리 구성:** 2개 포트를 묶어 `bond0` 생성.
2.  **논리 구성 (VLAN):**
      * `bond0.10` (VLAN 10): **Service Network** (K8s Pod IP, Node IP)
      * `bond0.20` (VLAN 20): **Storage/Mgmt Network** (Isilon 연결용 IP, SSH 접속)
3.  **라우팅 설정:**
      * 기본 게이트웨이(Default GW)는 `bond0.10`(서비스망)으로 설정.
      * Isilon 대역으로 가는 트래픽만 `bond0.20`을 타도록 정적 라우팅(Static Route) 추가.

**[보고서 반영 멘트]**

> "네트워크 인터페이스는 \*\*LACP Mode 4(50Gbps)\*\*로 구성하여 단일 포트 장애 시에도 서비스 연속성을 보장하며, 데이터 분석 워크로드의 특성상 발생하는 불규칙한 트래픽(Burst Traffic)을 유연하게 수용할 수 있도록 설계함. 서비스와 스토리지 트래픽의 논리적 격리는 **VLAN Tagging**을 통해 구현함."

>>>

카오스 엔지니어링 테스트 방법

>>>

191대 규모의 온프레미스 클러스터에서 카오스 엔지니어링을 시작하는 것은 매우 훌륭한 전략입니다. 하드웨어가 많을수록 \*\*"장애는 예외가 아니라 일상"\*\*이기 때문입니다.

현재 구축하신 환경(Spark/Trino, NVMe, Isilon, 외부 MinIO)에 맞춰, **안전하게 시작하여 점진적으로 강도를 높이는 4단계 로드맵**을 제안해 드립니다.

-----

### 1\. 도구 선정: Chaos Mesh (추천)

온프레미스 쿠버네티스 환경에서 가장 추천하는 도구는 \*\*[Chaos Mesh](https://chaos-mesh.org/)\*\*입니다.

  * **선정 이유:**
    1.  **CNCF 졸업 프로젝트:** 안정성이 검증됨.
    2.  **강력한 Web UI:** YAML을 몰라도 대시보드에서 실험을 설계하고 관제 가능.
    3.  **JVM 지원:** Spark, Trino가 Java 기반인데, Chaos Mesh는 JVM 메서드 단위의 지연/예외 주입이 가능합니다.
    4.  **Network Chaos:** Cilium 환경에서도 Pod 간 네트워크 지연/손실 시뮬레이션이 탁월합니다.

-----

### 2\. 단계별 적용 로드맵

카오스 엔지니어링은 \*\*"운영 환경을 부수는 것"이 아니라 "시스템의 견고함을 검증하는 것"\*\*입니다. 반드시 `Development` 또는 `Staging` 네임스페이스에서 시작하세요.

#### Phase 1: Pod Chaos (가장 기초적이고 안전한 단계)

**목표:** Spark Executor나 Trino Worker가 죽었을 때, 작업이 재시도(Retry)되거나 전체 클러스터가 죽지 않는지 확인.

  * **실험 대상:** `Spark-Worker` Pod, `Trino-Worker` Pod.
  * **시나리오:** "Pod Kill" (무작위로 파드 1\~2개를 강제 종료).
  * **검증 포인트:**
      * Spark Operator가 즉시 새로운 Executor Pod를 띄우는가?
      * 실행 중이던 Spark Job이 실패하지 않고, 해당 Task만 다른 노드로 옮겨가서 성공하는가?
      * Trino Coordinator가 죽은 Worker를 제외하고 쿼리를 처리하는가?

#### Phase 2: Network Chaos (데이터 파이프라인 핵심)

**목표:** 외부 MinIO와의 연결이 불안정할 때 애플리케이션의 타임아웃 설정이 적절한지 확인.

  * **실험 대상:** Spark Driver/Executor, Trino Worker.
  * **시나리오:**
    1.  **Latency Injection:** 외부 MinIO IP 대역으로 가는 패킷에 **latency 50ms \~ 100ms** 추가.
    2.  **Packet Loss:** 패킷의 **1\~5% 유실** 시뮬레이션.
  * **검증 포인트:**
      * Spark Shuffle 시 네트워크 지연이 발생했을 때 전체 Job이 멈추지 않는가? (Netty 타임아웃 설정 검증)
      * 외부 데이터 소스(MinIO) 조회 시, 재시도(Retry) 로직이 동작하는가, 아니면 바로 에러를 뱉고 죽는가?

#### Phase 3: IO Chaos (NVMe 및 Isilon 검증)

**목표:** 디스크 I/O 병목 시뮬레이션.

  * **실험 대상:** Local PV를 사용하는 Pod.
  * **시나리오:**
      * **IO Delay:** 파일 읽기/쓰기에 인위적인 지연 추가.
      * **IO Error:** 특정 파일 접근 시 IO 에러 리턴.
  * **검증 포인트:**
      * Isilon(NFS) 응답이 느려졌을 때, Jenkins나 Airflow가 좀비 상태(Stuck)에 빠지지 않는가?
      * NVMe I/O 지연 시 Spark의 Spill 동작이 어떻게 반응하는가?

#### Phase 4: Node Chaos (운영자 레벨)

**목표:** 물리 노드 장애(전원 차단, 커널 패닉) 대응.

  * **실험 대상:** Worker Node 1대.
  * **시나리오:** `Node Drain` 또는 Chaos Mesh의 `KernelChaos` 활용.
  * **검증 포인트:**
      * Isilon 마운트가 끊긴 노드에서 실행되던 Pod들이 다른 노드로 안전하게 이사(Eviction) 가는가?
      * Cilium BGP가 죽은 노드를 라우팅 테이블에서 즉시 제외하는가?

-----

### 3\. 실전 예시 (YAML)

Chaos Mesh를 설치(`helm install chaos-mesh ...`)한 후, 아래와 같은 YAML을 적용하여 테스트를 시작합니다.

**예시 1: Spark Executor 2개를 5분마다 죽이기 (PodChaos)**

```yaml
apiVersion: chaos-mesh.org/v1alpha1
kind: PodChaos
metadata:
  name: spark-executor-kill
  namespace: team-a-spark
spec:
  action: pod-kill
  mode: fixed
  value: "2" # 한 번에 2개씩 죽임
  selector:
    labelSelectors:
      "spark-role": "executor" # 타겟 라벨
  scheduler:
    cron: "*/5 * * * *" # 5분마다 실행
```

**예시 2: MinIO 연결 지연 시뮬레이션 (NetworkChaos)**

```yaml
apiVersion: chaos-mesh.org/v1alpha1
kind: NetworkChaos
metadata:
  name: minio-latency
  namespace: team-a-trino
spec:
  action: delay
  mode: all
  selector:
    labelSelectors:
      "app": "trino-worker"
  delay:
    latency: "100ms"
    correlation: "100"
    jitter: "0ms"
  direction: to
  target:
    mode: ip
    value: "10.x.x.x/32" # 외부 MinIO IP
```

-----

### 4\. 카오스 엔지니어링 수행 원칙 (게임 데이)

시스템만 설치한다고 끝이 아닙니다. \*\*"Game Day(게임 데이)"\*\*라는 이벤트를 만들어 팀원들과 함께 관측해야 합니다.

1.  **가설 수립:** "Trino Worker 하나가 죽어도, 실행 중인 쿼리는 약간 느려질 뿐 실패하지 않을 것이다."
2.  **실험 실행:** Chaos Mesh로 Worker 하나를 Kill.
3.  **관측(Monitoring):** 그라파나 대시보드에서 쿼리 성공률, 레이턴시, 에러 로그 확인.
4.  **학습 및 개선:** "쿼리가 실패했다면 왜 실패했는가? Trino의 `fault-tolerant-execution` 설정이 꺼져 있었는가?" -\> **설정 수정 후 재테스트.**

### [요약] 시작 가이드

1.  **설치:** **Chaos Mesh**를 Helm으로 설치하십시오.
2.  **첫 실험:** 개발용 Namespace에서 **Spark Executor Pod 하나를 죽이는 것**부터 시작하십시오.
3.  **확장:** 외부 **MinIO 네트워크 지연** 테스트로 넘어가십시오. (가장 장애가 많이 나는 구간입니다)

이 과정을 통해 구축하신 191대 클러스터의 신뢰성을 증명할 수 있습니다.

>>>

