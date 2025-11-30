노드 스펙: 48코어 (24C×2), 768GB RAM, SAS SSD 800G×2, NVMe 3.84T×2, 10/25GbE 2포트x3 
규모: 190대 (컨트롤 플레인 포함)
OS/도구: RHEL 10, Kubespray, Cilium (BGP 가능)
용도: 프로덕션 빅데이터 파이프라인, 4개 팀 멀티테넌시
스토리지: 외부 MinIO/Iceberg (Object Storage), Dell Isilon NAS 가용
---
Airflow, Spark, Trino, Flink, Kafka 운용. 추후 StarRocks 검토
모니터링: Promethus + Grafana
로깅: OpenSearch
Ingress: Cilium
