## 네트워크 아키텍처 확정

### 최종 확정 사항

|항목           |결정                     |
|-------------|-----------------------|
|CNI          |Cilium (Native Routing)|
|BGP          |✅ 사용                   |
|ECMP         |✅ 사용                   |
|API Server HA|kube-vip (BGP 모드)      |
|Ingress      |Cilium Ingress         |
|TLS 종료       |Cilium Ingress에서 처리    |
|내부 통신        |HTTP                   |
|Bond 구성      |물리적 분리, VLAN 없음        |

### IP 대역 (10.200.0.0/16)

|용도            |CIDR           |IP 수  |
|--------------|---------------|------|
|Node IP       |10.200.0.0/23  |512   |
|Infrastructure|10.200.2.0/24  |256   |
|예비            |10.200.3.0/24  |256   |
|Pod CIDR      |10.200.4.0/17  |32,768|
|Service CIDR  |10.200.128.0/20|4,096 |
|예비            |10.200.144.0/20|4,096 |

### Infrastructure 대역 상세

```
10.200.2.1              API Server VIP
10.200.2.2 ~ 2.254      LoadBalancer Pool (253개)
```

### 트래픽 흐름

```
[외부 사용자]
     │ HTTPS
     ▼
[Cilium Ingress] ── TLS 종료
     │ HTTP
     ▼
[Pod: Airflow, Grafana 등]
     │ HTTPS (사설 CA)
     ▼
[외부 Keycloak]
```

-----

## 다음 단계

1. **노드 역할 분리** - 190대 노드 배치 전략
1. **멀티테넌시 전략** - 4개 팀 격리 방식
1. **NetworkPolicy 전략** - 팀간 격리
1. **워크로드별 고려사항** - Spark, Kafka 등 배치
1. **관측성(Observability)** - Prometheus, Thanos 등

어느 것부터 진행할까요?​​​​​​​​​​​​​​​​