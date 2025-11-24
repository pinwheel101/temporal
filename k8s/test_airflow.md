# **쿠버네티스 Airflow 구축 검증 및 테스트 계획서**

작성일: 2025년 00월 00일  
대상 클러스터: 온프레미스 K8s (191 Node)  
테스트 목적: Airflow의 기본 기능, 쿠버네티스 통합(K8sExecutor/KubernetesPodOperator), Spark 연동, 스토리지 및 로깅 기능의 정상 동작 검증

## **1\. 인프라 및 배포 상태 점검 (Infrastructure Check)**

가장 먼저 Airflow 컴포넌트들이 쿠버네티스 상에서 정상적으로 기동되었는지 확인합니다.

| ID | 테스트 항목 | 점검 방법 (CLI/UI) | 예상 결과 (Expected Result) | 확인 |
| :---- | :---- | :---- | :---- | :---- |
| **INF-01** | **Pod 상태 점검** | kubectl get pods \-n \<airflow-namespace\> | Webserver, Scheduler, Triggerer, Worker(또는 Redis/Postgres) 등 모든 Pod가 Running 상태여야 함. CrashLoopBackOff나 Error가 없어야 함. | \[ \] |
| **INF-02** | **Service 연결 점검** | kubectl get svc \-n \<airflow-namespace\> | Webserver, Flower(있을 경우) 등의 Service ClusterIP가 할당되어 있어야 함. | \[ \] |
| **INF-03** | **Ingress 접속 점검** | 브라우저에서 Airflow 도메인 접속 (예: airflow.internal.com) | 502/504 에러 없이 Airflow 로그인 화면이 즉시 로딩되어야 함. | \[ \] |
| **INF-04** | **DB 연결 점검** | Scheduler/Webserver 로그 확인 (kubectl logs ...) | "Connected to metadata database" 메시지가 확인되고 DB 연결 에러가 없어야 함. | \[ \] |
| **INF-05** | **PVC 마운트 점검** | kubectl describe pod \<scheduler-pod\> | Isilon NAS가 DAGs 폴더와 Logs 폴더에 정상적으로 RWX(ReadWriteMany) 모드로 마운트되어 있어야 함. | \[ \] |

## **2\. 기본 기능 및 스케줄링 테스트 (Core Functionality)**

Airflow의 핵심인 스케줄러와 웹서버가 유기적으로 동작하는지 확인합니다.

| ID | 테스트 항목 | 테스트 시나리오 | 예상 결과 | 확인 |
| :---- | :---- | :---- | :---- | :---- |
| **COR-01** | **DAG 파싱 테스트** | 간단한 example\_bash\_operator DAG 활성화 (Unpause) | UI 대시보드에 에러 경고창("Broken DAG") 없이 DAG가 활성화되어야 함. | \[ \] |
| **COR-02** | **수동 트리거 테스트** | UI 우측 상단 'Trigger DAG' 버튼 클릭 | DAG Run이 생성되고, Task 상태가 Queued \-\> Running \-\> Success로 순차 변경되어야 함. | \[ \] |
| **COR-03** | **XCom 동작 확인** | Task A에서 값을 Return하고 Task B에서 Pull 하는 DAG 실행 | Task B 로그에서 Task A가 넘겨준 값이 정상적으로 출력되어야 함 (DB 통신 검증). | \[ \] |
| **COR-04** | **Task Log 확인** | 실행 완료된 Task 클릭 \-\> 'Logs' 탭 확인 | UI에서 Task의 실행 로그가 실시간(또는 완료 후)으로 보여야 함. (Remote Logging 설정 검증) | \[ \] |
| **COR-05** | **스케줄링 간격** | schedule\_interval='\*/5 \* \* \* \*' 설정 후 대기 | 5분마다 자동으로 DAG가 트리거되어야 함. | \[ \] |

## **3\. 쿠버네티스 통합 테스트 (K8s Integration)**

**가장 중요한 단계입니다.** Airflow가 K8s 클러스터의 자원을 사용하여 Pod를 생성하고 제어할 수 있는지 검증합니다.

### **K8s-01: KubernetesPodOperator 테스트**

Airflow 워커가 아닌, **별도의 독립된 Pod**를 생성하여 작업을 수행하는지 확인합니다.

* **테스트 DAG 코드 (예시):**

from airflow import DAG  
from airflow.providers.cncf.kubernetes.operators.kubernetes\_pod import KubernetesPodOperator  
from datetime import datetime

with DAG('test\_k8s\_pod\_operator', start\_date=datetime(2023, 1, 1), schedule\_interval=None) as dag:  
    k8s\_task \= KubernetesPodOperator(  
        task\_id="dry\_run",  
        name="k8s-test-pod",  
        namespace="\<airflow-namespace\>",  
        image="busybox",  
        cmds=\["sh", "-c", "echo 'Hello K8s Cluster'; sleep 30;"\],  
        is\_delete\_operator\_pod=True, \# 테스트 후 파드 삭제 여부  
        get\_logs=True  
    )

* **점검 포인트:**  
  1. Task 실행 시 kubectl get pods로 k8s-test-pod-\*\*\* 파드가 생성되는지 확인.  
  2. 해당 파드가 Running 상태가 되는지 확인.  
  3. Airflow UI 로그 탭에 "Hello K8s Cluster"가 찍히는지 확인 (로그 수집 검증).

## **4\. 데이터 파이프라인 연동 테스트 (Spark & MinIO)**

실제 분석 업무 환경과 동일한 흐름을 검증합니다.

| ID | 테스트 항목 | 테스트 시나리오 | 예상 결과 | 확인 |
| :---- | :---- | :---- | :---- | :---- |
| **DAT-01** | **MinIO 연결 테스트** | S3Hook을 사용하여 타 클러스터 MinIO 버킷 목록 조회 | Airflow Connection 설정이 올바르고, 네트워크 방화벽이 열려있어 버킷 리스트가 출력되어야 함. | \[ \] |
| **DAT-02** | **SparkApp 배포 (CRD)** | Airflow에서 SparkKubernetesOperator 등을 사용해 SparkApplication YAML 배포 | K8s에 sparkapplication 리소스가 생성되고, Spark Driver/Executor 파드가 지정된 Namespace에 떴다가 성공적으로 종료되어야 함. | \[ \] |
| **DAT-03** | **Spark Log 연동** | Spark 작업 종료 후 Airflow UI 확인 | Spark Driver의 로그가 Airflow UI Task Log 탭에 통합되어 보여야 함 (설정 필요). | \[ \] |
| **DAT-04** | **Trino 쿼리 실행** | TrinoOperator를 사용하여 간단한 SELECT 1 쿼리 수행 | Trino 클러스터로 쿼리가 전송되고 결과가 반환되어 Task가 성공해야 함. | \[ \] |

## **5\. 장애 및 성능 테스트 (Failure & Performance)**

운영 환경에서의 안정성을 검증합니다.

| ID | 테스트 항목 | 테스트 시나리오 | 예상 결과 | 확인 |
| :---- | :---- | :---- | :---- | :---- |
| **PFM-01** | **Scheduler Failover** | kubectl delete pod \<scheduler-pod\> (강제 삭제) | K8s가 즉시 스케줄러를 다시 띄워야 하며, 잠시 후 스케줄링이 중단 없이 재개되어야 함. | \[ \] |
| **PFM-02** | **Worker 확장성** | sleep 60 Task를 50개 병렬로 실행 (concurrency 설정 확인) | 설정된 Parallelism 값만큼 Task가 동시에 Running 상태로 진입해야 함. | \[ \] |
| **PFM-03** | **Time-out 처리** | execution\_timeout을 10초로 설정하고 30초 걸리는 작업 실행 | 10초 후 Airflow가 해당 Task(또는 Pod)에 SIGTERM/SIGKILL을 보내고 State: Failed로 처리해야 함. | \[ \] |
| **PFM-04** | **Remote Logging 지속성** | Worker Pod를 강제 삭제 후 재생성 | 삭제 전 실행했던 Task의 로그가 UI에서 여전히 조회가 가능해야 함 (Isilon/MinIO 저장 여부 확인). | \[ \] |

## **6\. 테스트 결과 요약 (Summary)**

* **테스트 수행자:** \_\_\_\_\_\_\_\_\_\_\_\_\_\_\_\_\_\_\_  
* **수행 일시:** \_\_\_\_\_\_\_\_\_\_\_\_\_\_\_\_\_\_\_  
* **성공 항목:** \_\_\_\_ / \_\_\_\_  
* **실패 항목:** \_\_\_\_\_\_\_\_\_\_\_\_\_\_\_\_\_\_\_ (Jira 티켓 번호: \_\_\_\_\_)  
* **최종 판정:** \[ \] 운영 이관 가능 / \[ \] 보완 필요