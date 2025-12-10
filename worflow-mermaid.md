flowchart TD
    A[1. Kubernetes 설치 /w Kubespray] --> B[2-1. Cilium 설치]
    A --> C[2-2. Isilon CSI 설치]
    A --> D[2-3. OPA 설치]
    A --> E[2-4. ArgoCD 셋업]
    
    B --> F[3. 2단계 그룹 체크리스트]
    C --> F
    D --> F
    E --> F
    
    F --> G[4-1. CNPG 설치<br/>(ArgoCD 활용 가능)]
    F --> H[4-2. Prometheus Stack 설치]
    F --> I[4-3. Kafka 설치]
    F --> J[4-4. SparkOperator 설치<br/>(Hive 의존)]
    F --> K[4-5. Hive Metastore 설치<br/>(CSI 스토리지 의존)]
    F --> L[4-6. User Portal 설치]
    
    G --> M[5. 4단계 그룹 체크리스트]
    H --> M
    I --> M
    J --> M
    K --> M
    L --> M
    
    M --> N[6-1. Airflow 설치<br/>(Kafka/Spark 의존)]
    M --> O[6-2. OpenSearch 설치]
    M --> P[6-3. Spark-cli 설치<br/>(SparkOperator 의존)]
    M --> Q[6-4. Spark-history-server 설치<br/>(SparkOperator 의존)]
    M --> R[6-5. Trino 설치<br/>(Hive/OpenSearch 의존 가능)]
    
    N --> S[7. 6단계 그룹 체크리스트]
    O --> S
    P --> S
    Q --> S
    R --> S
    
    S --> T[8. 클러스터 & 앱 설치 완료<br/>서비스 이전 후속 작업<br/>(데이터 마이그레이션, E2E 테스트 등)]
    
    style A fill:#e1f5fe
    style T fill:#c8e6c9
    classDef group2 fill:#f3e5f5
    classDef group4 fill:#fff3e0
    classDef group6 fill:#e8f5e8
    class B,C,D,E group2
    class G,H,I,J,K,L group4
    class N,O,P,Q,R group6