# Requirements

## 하드웨어
- 노드 스펙: 48코어 (24C×2), 768GB RAM, SAS SSD 800G×2, NVMe 3.84T×2, 10/25GbE 2포트x3 
- 규모: 190대 (컨트롤 플레인 포함)
- OS/도구: RHEL 10, Kubespray, Cilium (BGP 가능)
- 용도: 프로덕션 빅데이터 파이프라인, 4개 팀 멀티테넌시
- 스토리지: 외부 MinIO/Iceberg (Object Storage), Dell Isilon NAS 가용

## 어플리케이션
- Ingress: Cilium
- 어플리케이션: Airflow, Spark, Trino, Flink, Kafka 운용. 추후 StarRocks 검토
- 모니터링: Promethus + Grafana
- 로깅: OpenSearch
- CI/CD: Bitbucket, Jenkins, ArgoCD (외부)

## 클러스터 예상 사용 패턴
- 4개의 서비스(팀)는 엄격하게 namespace로 분리
- 4개의 namespace는 ResourceQuota로 관리 되어야 함
- 4개의 팀이 공통으로 사용하는 서비스는 platform namespace에서 관리
- Airflow, Spark는 배치 워크로드에 사용되며 Dynamic Provisioning을 사용
- Trino는 HPA를 사용하지 않고 Static Provisioning으로 운용
- 위에 두가지를 제외한 모든 어플리케이션 사용 패턴은 일단 Static Provisioning과 엄격한 리소스 리밋을 사용
