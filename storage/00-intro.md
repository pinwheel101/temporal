아래는 쿠버네티스 온프레미스 환경에서 로컬 스토리지(Local NVMe / SAS SSD 등)의 성능을 체계적으로 검증하는 방법을 아키텍처·운영 관점 중심으로 정리한 것입니다.
(코드·명령 예시는 요청 전까지 포함하지 않습니다.)

⸻

1. 테스트의 목적과 범위 정의 (가장 중요)

로컬 스토리지 성능 테스트는 단순 IOPS 측정이 아니라, 쿠버네티스 워크로드 관점에서의 실제 병목을 확인하는 것이 목적입니다. 따라서 아래 항목을 명확히 구분해야 합니다.

1.1 검증 대상 성능 지표
	•	IOPS (random read/write)
	•	Throughput (sequential read/write, MB/s)
	•	Latency 분포
	•	p50 / p95 / p99 (평균은 의미 없음)
	•	Tail latency (동시 부하 시)
	•	CPU 사용률 대비 IO 효율
	•	노드 간 편차 (동일 스펙 노드라도 중요)

1.2 테스트 관점 구분

관점	의미
Bare Metal	디스크의 물리적 최대 성능
Container	컨테이너 런타임 오버헤드
PVC / CSI	스토리지 추상화 오버헤드
Multi-Pod	실제 운영 시 경합
장애 상황	노드/파드 재시작 영향


⸻

2. 테스트 레벨별 접근 전략

2.1 레벨 0 – 노드 단독(Bare Metal) 기준선

목적: 디스크 자체 성능 상한선 확보
의미: 이후 모든 테스트는 이 수치를 넘을 수 없음
	•	NVMe / SAS SSD 제조사 스펙 대비 실제 성능
	•	NUMA 영향 여부
	•	RAID / JBOD 구성 차이

👉 이 단계 결과가 없으면 쿠버네티스 테스트 해석이 왜곡됨

⸻

2.2 레벨 1 – 단일 파드 + 로컬 스토리지

목적:
컨테이너 런타임(containerd) + cgroup + filesystem 오버헤드 측정
	•	emptyDir (medium: "")
	•	hostPath
	•	Local PV (CSI 미사용)

검증 포인트
	•	Bare Metal 대비 성능 손실률
	•	파일시스템(ext4 vs xfs)
	•	fsync / direct IO 차이

⸻

2.3 레벨 2 – Local Persistent Volume (CSI 포함)

목적:
운영에서 실제 사용할 Local PV + CSI Driver 오버헤드 측정
	•	Local Static PV
	•	VolumeBindingMode: WaitForFirstConsumer
	•	Pod scheduling과 IO locality 검증

중요 포인트
	•	스케줄러가 실제로 같은 노드에 파드를 고정하는지
	•	재스케줄 시 데이터 접근 불가 리스크 인지

⸻

2.4 레벨 3 – 동시성 테스트 (현실적 시나리오)

목적:
실제 워크로드에서 발생하는 IO 경합 상황 재현

테스트 시나리오 예
	•	노드 1대에 파드 1 / 5 / 10 / 20개
	•	각 파드가 동일 디스크 접근
	•	Read-heavy / Write-heavy 혼합

관찰 지표
	•	p99 latency 폭증 시점
	•	CPU softirq / iowait 증가
	•	스토리지 큐 depth 포화 여부

👉 이 단계가 가장 중요하며, 단순 벤치마크보다 운영 판단에 결정적

⸻

3. 쿠버네티스 관점 핵심 체크 포인트

3.1 스케줄링과 스토리지 정합성
	•	PodAffinity / NodeAffinity 설정 여부
	•	StatefulSet vs Deployment 차이
	•	재시작 시 동일 노드 보장 가능성

3.2 리소스 격리 영향
	•	CPU pinning 여부
	•	io.max, blkio cgroup 제한 여부
	•	BestEffort vs Guaranteed QoS

3.3 파일시스템과 마운트 옵션
	•	noatime / nodiratime
	•	barrier 설정
	•	discard(TRIM) 영향

⸻

4. 장애·비정상 상황 테스트 (반드시 포함)

4.1 노드 장애
	•	IO 중 노드 전원 차단
	•	재기동 후 데이터 정합성

4.2 파드 재시작
	•	동일 PVC 재연결 시간
	•	filesystem replay 시간

4.3 네트워크 영향 (의외로 중요)
	•	Cilium BPF CPU 사용 증가 시 IO latency 영향
	•	Prometheus scrape 부하 시 영향

⸻

5. 결과 해석 시 흔한 함정

함정	설명
평균 latency만 보는 것	p99가 실제 장애를 만든다
단일 파드 결과로 결론	운영에서는 항상 다중 파드
벤치마크 도구 성능 믿기	설정에 따라 결과가 완전히 달라짐
노드별 편차 무시	온프레미스에서 매우 흔함


⸻

6. 데이터 파이프라인 워크로드 관점 요약

(사용자 환경 기준)

워크로드	중요 지표
Apache Flink (State)	random write latency, fsync
Kafka (Log)	sequential write throughput
Trino / Iceberg	random read p95
Spark Shuffle	mixed IO + concurrency


⸻

7. 다음 단계 제안

다음 중 어떤 단계로 진행할지 선택해주시면, 그 단계만 상세화하겠습니다.
	1.	테스트 도구 선정 비교 (fio, vdbench, ycsb, etc.)
	2.	Local PV 아키텍처 베스트 프랙티스
	3.	190노드 규모 기준 테스트 계획서
	4.	Flink State / Iceberg 워크로드 맞춤 시나리오
	5.	결과를 의사결정으로 연결하는 방법 (노드풀·디스크 구성)

원하시는 방향을 지정해 주십시오.