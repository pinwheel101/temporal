Flink on Kubernetes 개발환경 가이드
목차
	∙	1. 아키텍처 개요
	∙	2. 환경 구성 정보
	∙	3. CI/CD 파이프라인
	∙	4. Flink 애플리케이션 배포
	∙	5. 개발 워크플로우
	∙	6. 모니터링 및 로깅
	∙	7. 트러블슈팅
	∙	8. 보안 및 권한
	∙	9. 부록

1. 아키텍처 개요
1.1 시스템 구성도

┌─────────────────────────────────────────────────────────────┐
│                        Developer                             │
│                            ↓                                 │
│                    Git Repository                            │
└─────────────────────────────────────────────────────────────┘
                             ↓
┌─────────────────────────────────────────────────────────────┐
│                        Jenkins                               │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐            │
│  │  Checkout  │→ │   Build    │→ │   Docker   │            │
│  │    Code    │  │   & Test   │  │   Image    │            │
│  └────────────┘  └────────────┘  └────────────┘            │
│                                        ↓                     │
│                                  Image Registry              │
│                                        ↓                     │
│                                 Update GitOps Repo           │
└─────────────────────────────────────────────────────────────┘
                             ↓
┌─────────────────────────────────────────────────────────────┐
│                        ArgoCD                                │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐            │
│  │    Sync    │→ │   Deploy   │→ │  Monitor   │            │
│  │    Repo    │  │     CR     │  │   Status   │            │
│  └────────────┘  └────────────┘  └────────────┘            │
└─────────────────────────────────────────────────────────────┘
                             ↓
┌─────────────────────────────────────────────────────────────┐
│                 Kubernetes Cluster                           │
│  ┌──────────────────────────────────────────────┐            │
│  │           Flink Kubernetes Operator          │            │
│  └──────────────────────────────────────────────┘            │
│                       ↓                                      │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐            │
│  │   Flink    │  │   Flink    │  │   Flink    │            │
│  │ JobManager │  │TaskManager │  │TaskManager │            │
│  └────────────┘  └────────────┘  └────────────┘            │
└─────────────────────────────────────────────────────────────┘


1.2 컴포넌트 역할



|컴포넌트                  |역할          |책임 범위                            |
|----------------------|------------|---------------------------------|
|**Git Repository**    |소스 코드 버전 관리 |애플리케이션 코드, Flink Job 설정          |
|**Jenkins**           |CI 파이프라인    |빌드, 테스트, 이미지 생성, GitOps 레포 업데이트  |
|**Container Registry**|이미지 저장소     |Docker 이미지 저장 및 버전 관리            |
|**ArgoCD**            |CD 파이프라인    |GitOps 기반 배포 자동화, 동기화            |
|**Flink K8s Operator**|Flink 리소스 관리|FlinkDeployment CR 처리, 클러스터 생성/관리|
|**Kubernetes**        |컨테이너 오케스트레이션|리소스 스케줄링, 네트워킹, 스토리지             |

1.3 배포 플로우

1. 개발자가 코드 커밋 & 푸시
   ↓
2. Jenkins 웹훅 트리거
   ↓
3. Jenkins Pipeline 실행
   - 코드 체크아웃
   - Maven/Gradle 빌드
   - 단위 테스트 실행
   - Docker 이미지 빌드 & 푸시
   - GitOps 레포의 이미지 태그 업데이트
   ↓
4. ArgoCD가 GitOps 레포 변경 감지
   ↓
5. ArgoCD가 FlinkDeployment CR 업데이트
   ↓
6. Flink Operator가 CR 변경 감지
   ↓
7. Flink 클러스터 생성/업데이트
   - JobManager Pod 생성
   - TaskManager Pod 생성
   - Job 제출 및 실행


2. 환경 구성 정보
2.1 Kubernetes 클러스터
클러스터 사양

Kubernetes Version: v1.28+
Node Count: 3+ (고가용성 구성)
Node Resources:
  - CPU: 16 cores per node
  - Memory: 64GB per node
  - Storage: 500GB SSD


네임스페이스 구조

# Flink Operator 네임스페이스
kubectl create namespace flink-operator-system

# 개발 환경
kubectl create namespace flink-dev

# 스테이징 환경
kubectl create namespace flink-staging

# 프로덕션 환경
kubectl create namespace flink-prod

# CI/CD 도구
kubectl create namespace cicd


2.2 Flink Kubernetes Operator 설치
공식 문서: https://nightlies.apache.org/flink/flink-kubernetes-operator-docs-stable/
Helm을 이용한 설치

# Helm 레포지토리 추가
helm repo add flink-operator-repo https://downloads.apache.org/flink/flink-kubernetes-operator-1.8.0/
helm repo update

# Operator 설치
helm install flink-kubernetes-operator flink-operator-repo/flink-kubernetes-operator \
  --namespace flink-operator-system \
  --create-namespace \
  --set webhook.create=true \
  --set metrics.port=9999

# 설치 확인
kubectl get pods -n flink-operator-system


values.yaml 커스터마이징

# flink-operator-values.yaml
image:
  repository: apache/flink-kubernetes-operator
  tag: 1.8.0

replicas: 2

resources:
  limits:
    cpu: 2000m
    memory: 2Gi
  requests:
    cpu: 500m
    memory: 512Mi

watchNamespaces:
  - flink-dev
  - flink-staging
  - flink-prod

defaultConfiguration:
  flink-conf.yaml: |+
    kubernetes.operator.reconcile.interval: 30s
    kubernetes.operator.observer.progress-check.interval: 5s
    kubernetes.operator.job.upgrade.last-state-fallback.enabled: true


2.3 Jenkins 설정
필수 플러그인

- Git Plugin
- Pipeline Plugin
- Docker Pipeline Plugin
- Kubernetes Plugin
- Credentials Binding Plugin
- Blue Ocean (선택사항)


Jenkins 설치 (Helm)

helm repo add jenkins https://charts.jenkins.io
helm repo update

helm install jenkins jenkins/jenkins \
  --namespace cicd \
  --create-namespace \
  --set controller.serviceType=LoadBalancer \
  --set controller.installPlugins[0]=kubernetes:latest \
  --set controller.installPlugins[1]=git:latest \
  --set controller.installPlugins[2]=workflow-aggregator:latest \
  --set controller.installPlugins[3]=docker-workflow:latest


Jenkins Credentials 설정

1. Manage Jenkins → Credentials → System → Global credentials
2. 추가 필요한 Credential:
   - git-credentials: GitHub/GitLab 접근용
   - docker-registry: Container Registry 푸시용
   - k8s-service-account: Kubernetes API 접근용
   - argocd-token: ArgoCD API 접근용


2.4 ArgoCD 설정
공식 문서: https://argo-cd.readthedocs.io/
ArgoCD 설치

# ArgoCD 설치
kubectl create namespace argocd
kubectl apply -n argocd -f https://raw.githubusercontent.com/argoproj/argo-cd/stable/manifests/install.yaml

# ArgoCD CLI 설치
curl -sSL -o argocd-linux-amd64 https://github.com/argoproj/argo-cd/releases/latest/download/argocd-linux-amd64
sudo install -m 555 argocd-linux-amd64 /usr/local/bin/argocd

# Admin 비밀번호 확인
kubectl -n argocd get secret argocd-initial-admin-secret -o jsonpath="{.data.password}" | base64 -d

# 포트포워딩으로 접근
kubectl port-forward svc/argocd-server -n argocd 8080:443


ArgoCD 프로젝트 생성

# argocd-project.yaml
apiVersion: argoproj.io/v1alpha1
kind: AppProject
metadata:
  name: flink-applications
  namespace: argocd
spec:
  description: Flink Streaming Applications
  sourceRepos:
    - 'https://github.com/your-org/flink-gitops.git'
  destinations:
    - namespace: 'flink-*'
      server: https://kubernetes.default.svc
  clusterResourceWhitelist:
    - group: ''
      kind: Namespace
    - group: 'flink.apache.org'
      kind: FlinkDeployment


ArgoCD Application 생성

# argocd-app-dev.yaml
apiVersion: argoproj.io/v1alpha1
kind: Application
metadata:
  name: flink-wordcount-dev
  namespace: argocd
spec:
  project: flink-applications
  source:
    repoURL: https://github.com/your-org/flink-gitops.git
    targetRevision: main
    path: apps/wordcount/dev
  destination:
    server: https://kubernetes.default.svc
    namespace: flink-dev
  syncPolicy:
    automated:
      prune: true
      selfHeal: true
      allowEmpty: false
    syncOptions:
      - CreateNamespace=true
    retry:
      limit: 5
      backoff:
        duration: 5s
        factor: 2
        maxDuration: 3m


3. CI/CD 파이프라인
3.1 Git 브랜치 전략

main (production)
  ↑
staging
  ↑
develop
  ↑
feature/* (개발 브랜치)


배포 트리거 조건
	∙	feature/* → develop 머지: 자동 빌드만 수행
	∙	develop → staging 머지: staging 환경 자동 배포
	∙	staging → main 머지: production 환경 수동 승인 후 배포
3.2 Jenkins Pipeline 스크립트
Jenkinsfile 예제

pipeline {
    agent {
        kubernetes {
            yaml """
apiVersion: v1
kind: Pod
spec:
  containers:
  - name: maven
    image: maven:3.9-eclipse-temurin-11
    command: ['cat']
    tty: true
  - name: docker
    image: docker:24.0
    command: ['cat']
    tty: true
    volumeMounts:
    - name: docker-sock
      mountPath: /var/run/docker.sock
  volumes:
  - name: docker-sock
    hostPath:
      path: /var/run/docker.sock
"""
        }
    }
    
    environment {
        DOCKER_REGISTRY = 'your-registry.io'
        DOCKER_REPO = 'flink-apps'
        APP_NAME = 'wordcount'
        GIT_COMMIT_SHORT = sh(returnStdout: true, script: 'git rev-parse --short HEAD').trim()
        IMAGE_TAG = "${env.BRANCH_NAME}-${GIT_COMMIT_SHORT}-${env.BUILD_NUMBER}"
        GITOPS_REPO = 'git@github.com:your-org/flink-gitops.git'
    }
    
    stages {
        stage('Checkout') {
            steps {
                checkout scm
                script {
                    env.GIT_COMMIT_MSG = sh(returnStdout: true, script: 'git log -1 --pretty=%B').trim()
                }
            }
        }
        
        stage('Build & Test') {
            steps {
                container('maven') {
                    sh '''
                        mvn clean package -DskipTests
                        mvn test
                        mvn verify
                    '''
                }
            }
            post {
                always {
                    junit '**/target/surefire-reports/*.xml'
                }
            }
        }
        
        stage('Build Docker Image') {
            steps {
                container('docker') {
                    script {
                        withCredentials([usernamePassword(
                            credentialsId: 'docker-registry',
                            usernameVariable: 'DOCKER_USER',
                            passwordVariable: 'DOCKER_PASS'
                        )]) {
                            sh """
                                docker login -u \${DOCKER_USER} -p \${DOCKER_PASS} ${DOCKER_REGISTRY}
                                docker build -t ${DOCKER_REGISTRY}/${DOCKER_REPO}/${APP_NAME}:${IMAGE_TAG} .
                                docker push ${DOCKER_REGISTRY}/${DOCKER_REPO}/${APP_NAME}:${IMAGE_TAG}
                                docker tag ${DOCKER_REGISTRY}/${DOCKER_REPO}/${APP_NAME}:${IMAGE_TAG} \
                                           ${DOCKER_REGISTRY}/${DOCKER_REPO}/${APP_NAME}:${env.BRANCH_NAME}-latest
                                docker push ${DOCKER_REGISTRY}/${DOCKER_REPO}/${APP_NAME}:${env.BRANCH_NAME}-latest
                            """
                        }
                    }
                }
            }
        }
        
        stage('Update GitOps Repo') {
            when {
                anyOf {
                    branch 'develop'
                    branch 'staging'
                    branch 'main'
                }
            }
            steps {
                script {
                    def environment = ''
                    switch(env.BRANCH_NAME) {
                        case 'develop':
                            environment = 'dev'
                            break
                        case 'staging':
                            environment = 'staging'
                            break
                        case 'main':
                            environment = 'prod'
                            break
                    }
                    
                    withCredentials([sshUserPrivateKey(
                        credentialsId: 'git-credentials',
                        keyFileVariable: 'SSH_KEY'
                    )]) {
                        sh """
                            # GitOps 레포지토리 클론
                            export GIT_SSH_COMMAND='ssh -i \${SSH_KEY} -o StrictHostKeyChecking=no'
                            git clone ${GITOPS_REPO} gitops-repo
                            cd gitops-repo
                            
                            # 이미지 태그 업데이트
                            sed -i 's|image: .*|image: ${DOCKER_REGISTRY}/${DOCKER_REPO}/${APP_NAME}:${IMAGE_TAG}|g' \
                                apps/${APP_NAME}/${environment}/flinkdeployment.yaml
                            
                            # 변경사항 커밋 & 푸시
                            git config user.email "jenkins@ci.com"
                            git config user.name "Jenkins CI"
                            git add .
                            git commit -m "Update ${APP_NAME} image to ${IMAGE_TAG} for ${environment}"
                            git push origin main
                        """
                    }
                }
            }
        }
        
        stage('Trigger ArgoCD Sync') {
            when {
                anyOf {
                    branch 'develop'
                    branch 'staging'
                }
            }
            steps {
                script {
                    def environment = env.BRANCH_NAME == 'develop' ? 'dev' : 'staging'
                    withCredentials([string(credentialsId: 'argocd-token', variable: 'ARGOCD_TOKEN')]) {
                        sh """
                            argocd app sync flink-${APP_NAME}-${environment} \
                                --auth-token \${ARGOCD_TOKEN} \
                                --server argocd-server.argocd.svc.cluster.local \
                                --grpc-web
                        """
                    }
                }
            }
        }
        
        stage('Manual Approval for Production') {
            when {
                branch 'main'
            }
            steps {
                input message: 'Deploy to Production?', ok: 'Deploy'
                script {
                    withCredentials([string(credentialsId: 'argocd-token', variable: 'ARGOCD_TOKEN')]) {
                        sh """
                            argocd app sync flink-${APP_NAME}-prod \
                                --auth-token \${ARGOCD_TOKEN} \
                                --server argocd-server.argocd.svc.cluster.local \
                                --grpc-web
                        """
                    }
                }
            }
        }
    }
    
    post {
        success {
            echo "Pipeline succeeded! Image: ${DOCKER_REGISTRY}/${DOCKER_REPO}/${APP_NAME}:${IMAGE_TAG}"
        }
        failure {
            echo "Pipeline failed!"
        }
    }
}


3.3 Dockerfile 예제

# Dockerfile
FROM flink:1.18.1-scala_2.12-java11

# 애플리케이션 JAR 복사
COPY target/flink-wordcount-1.0-SNAPSHOT.jar /opt/flink/usrlib/application.jar

# 추가 의존성 라이브러리 (필요한 경우)
COPY target/lib/*.jar /opt/flink/lib/

# 환경 변수 설정
ENV FLINK_PROPERTIES="jobmanager.rpc.address: localhost"

# 헬스체크
HEALTHCHECK --interval=30s --timeout=10s --retries=3 \
  CMD curl -f http://localhost:8081/overview || exit 1

USER flink


4. Flink 애플리케이션 배포
4.1 FlinkDeployment CR 기본 구조
공식 문서: https://nightlies.apache.org/flink/flink-kubernetes-operator-docs-stable/docs/custom-resource/reference/

apiVersion: flink.apache.org/v1beta1
kind: FlinkDeployment
metadata:
  name: wordcount-app
  namespace: flink-dev
spec:
  # 이미지 정보
  image: your-registry.io/flink-apps/wordcount:develop-abc123-42
  imagePullPolicy: Always
  
  # Flink 버전
  flinkVersion: v1_18
  
  # Flink 설정
  flinkConfiguration:
    taskmanager.numberOfTaskSlots: "2"
    state.backend: rocksdb
    state.checkpoints.dir: s3://flink-checkpoints/
    state.savepoints.dir: s3://flink-savepoints/
    execution.checkpointing.interval: 60s
    execution.checkpointing.min-pause: 30s
    high-availability: kubernetes
    high-availability.storageDir: s3://flink-ha/
    restart-strategy: failure-rate
    restart-strategy.failure-rate.max-failures-per-interval: 3
    restart-strategy.failure-rate.failure-rate-interval: 5 min
    restart-strategy.failure-rate.delay: 10 s
  
  # ServiceAccount
  serviceAccount: flink-service-account
  
  # JobManager 설정
  jobManager:
    resource:
      memory: 2048m
      cpu: 1.0
    replicas: 1
    podTemplate:
      spec:
        containers:
          - name: flink-main-container
            env:
              - name: AWS_REGION
                value: us-west-2
            volumeMounts:
              - name: flink-logs
                mountPath: /opt/flink/log
        volumes:
          - name: flink-logs
            emptyDir: {}
  
  # TaskManager 설정
  taskManager:
    resource:
      memory: 4096m
      cpu: 2.0
    replicas: 2
    podTemplate:
      spec:
        containers:
          - name: flink-main-container
            env:
              - name: AWS_REGION
                value: us-west-2
  
  # Job 설정
  job:
    jarURI: local:///opt/flink/usrlib/application.jar
    entryClass: com.example.flink.WordCount
    args:
      - "--input"
      - "kafka://input-topic"
      - "--output"
      - "kafka://output-topic"
    parallelism: 4
    upgradeMode: savepoint
    state: running
    savepointTriggerNonce: 0


4.2 주요 파라미터 설명



|파라미터                              |설명                                                         |권장값            |
|----------------------------------|-----------------------------------------------------------|---------------|
|`flinkVersion`                    |Flink 버전 (v1_15, v1_16, v1_17, v1_18)                      |v1_18          |
|`taskmanager.numberOfTaskSlots`   |TaskManager당 슬롯 수                                          |CPU 코어 수와 동일   |
|`execution.checkpointing.interval`|체크포인트 간격                                                   |60s ~ 300s     |
|`job.parallelism`                 |Job 병렬도                                                    |전체 슬롯 수의 50-80%|
|`job.upgradeMode`                 |업그레이드 모드 (stateless, savepoint, last-state)                |savepoint      |
|`restart-strategy`                |재시작 전략 (none, fixed-delay, failure-rate, exponential-delay)|failure-rate   |

4.3 Job vs Session 클러스터
Application Mode (권장)

# Application Mode - Job별 전용 클러스터
apiVersion: flink.apache.org/v1beta1
kind: FlinkDeployment
metadata:
  name: wordcount-app
spec:
  mode: native
  job:
    jarURI: local:///opt/flink/usrlib/application.jar
    parallelism: 4
    upgradeMode: savepoint


Session Mode

# Session Mode - 공유 클러스터
apiVersion: flink.apache.org/v1beta1
kind: FlinkDeployment
metadata:
  name: flink-session-cluster
spec:
  mode: native
  # job 섹션 없음
  
---
# SessionJob - 별도 제출
apiVersion: flink.apache.org/v1beta1
kind: FlinkSessionJob
metadata:
  name: wordcount-session-job
spec:
  deploymentName: flink-session-cluster
  job:
    jarURI: https://repo.maven.apache.org/maven2/.../flink-wordcount.jar
    parallelism: 4


선택 가이드
	∙	Application Mode: 프로덕션 환경, 장기 실행 Job, 리소스 격리 필요
	∙	Session Mode: 개발/테스트, 짧은 Job 여러 개, 리소스 공유
4.4 리소스 할당 전략
메모리 계산 공식

Total Memory = Flink Memory + JVM Metaspace + JVM Overhead

JobManager Total Memory = 1GB (기본) + 애플리케이션 복잡도
TaskManager Total Memory = Task Memory + Network Memory + Managed Memory

예시:
- TaskManager Total: 4GB
- Task Heap: 1.5GB (37.5%)
- Network Memory: 512MB (12.5%)
- Managed Memory: 1.6GB (40%)
- JVM Overhead: 400MB (10%)


수평 확장 vs 수직 확장

# 수평 확장 (더 많은 TaskManager)
taskManager:
  resource:
    memory: 2048m
    cpu: 1.0
  replicas: 8

# 수직 확장 (더 큰 TaskManager)
taskManager:
  resource:
    memory: 8192m
    cpu: 4.0
  replicas: 2


권장사항: 수평 확장 선호 (장애 격리, 탄력성)
4.5 Checkpointing & Savepoint 설정

flinkConfiguration:
  # 체크포인트 설정
  execution.checkpointing.mode: EXACTLY_ONCE
  execution.checkpointing.interval: 60s
  execution.checkpointing.min-pause: 30s
  execution.checkpointing.timeout: 10min
  execution.checkpointing.max-concurrent-checkpoints: 1
  state.backend: rocksdb
  state.backend.incremental: true
  state.checkpoints.dir: s3://flink-checkpoints/
  state.checkpoints.num-retained: 3
  
  # Savepoint 설정
  state.savepoints.dir: s3://flink-savepoints/
  execution.savepoint.ignore-unclaimed-state: false


Savepoint 수동 트리거

# Savepoint 생성
kubectl patch flinkdeployment wordcount-app \
  --type merge \
  -p '{"spec":{"job":{"savepointTriggerNonce": 123456}}}'

# Savepoint에서 복원
kubectl patch flinkdeployment wordcount-app \
  --type merge \
  -p '{"spec":{"job":{"initialSavepointPath": "s3://flink-savepoints/savepoint-abc123"}}}'


5. 개발 워크플로우
5.1 로컬 개발 환경
사전 준비

# Java 11 설치
sudo apt install openjdk-11-jdk

# Maven 설치
sudo apt install maven

# Flink 다운로드
wget https://archive.apache.org/dist/flink/flink-1.18.1/flink-1.18.1-bin-scala_2.12.tgz
tar -xzf flink-1.18.1-bin-scala_2.12.tgz
export FLINK_HOME=$(pwd)/flink-1.18.1
export PATH=$FLINK_HOME/bin:$PATH


프로젝트 구조

flink-wordcount/
├── pom.xml
├── src/
│   ├── main/
│   │   ├── java/
│   │   │   └── com/example/flink/
│   │   │       └── WordCount.java
│   │   └── resources/
│   │       ├── log4j2.properties
│   │       └── application.conf
│   └── test/
│       └── java/
│           └── com/example/flink/
│               └── WordCountTest.java
├── Dockerfile
└── README.md


pom.xml 예제

<project>
    <modelVersion>4.0.0</modelVersion>
    <groupId>com.example</groupId>
    <artifactId>flink-wordcount</artifactId>
    <version>1.0-SNAPSHOT</version>
    
    <properties>
        <flink.version>1.18.1</flink.version>
        <scala.binary.version>2.12</scala.binary.version>
        <java.version>11</java.version>
    </properties>
    
    <dependencies>
        <dependency>
            <groupId>org.apache.flink</groupId>
            <artifactId>flink-streaming-java</artifactId>
            <version>${flink.version}</version>
            <scope>provided</scope>
        </dependency>
        
        <dependency>
            <groupId>org.apache.flink</groupId>
            <artifactId>flink-clients</artifactId>
            <version>${flink.version}</version>
            <scope>provided</scope>
        </dependency>
        
        <!-- Kafka Connector -->
        <dependency>
            <groupId>org.apache.flink</groupId>
            <artifactId>flink-connector-kafka</artifactId>
            <version>3.0.2-1.18</version>
        </dependency>
        
        <!-- 테스트 -->
        <dependency>
            <groupId>org.apache.flink</groupId>
            <artifactId>flink-test-utils</artifactId>
            <version>${flink.version}</version>
            <scope>test</scope>
        </dependency>
    </dependencies>
    
    <build>
        <plugins>
            <plugin>
                <groupId>org.apache.maven.plugins</groupId>
                <artifactId>maven-shade-plugin</artifactId>
                <version>3.5.0</version>
                <executions>
                    <execution>
                        <phase>package</phase>
                        <goals>
                            <goal>shade</goal>
                        </goals>
                        <configuration>
                            <transformers>
                                <transformer implementation="org.apache.maven.plugins.shade.resource.ManifestResourceTransformer">
                                    <mainClass>com.example.flink.WordCount</mainClass>
                                </transformer>
                            </transformers>
                        </configuration>
                    </execution>
                </executions>
            </plugin>
        </plugins>
    </build>
</project>


5.2 로컬 테스트
단위 테스트

// WordCountTest.java
import org.apache.flink.streaming.api.environment.StreamExecutionEnvironment;
import org.apache.flink.test.util.AbstractTestBase;
import org.junit.Test;

public class WordCountTest extends AbstractTestBase {
    
    @Test
    public void testWordCount() throws Exception {
        StreamExecutionEnvironment env = 
            StreamExecutionEnvironment.getExecutionEnvironment();
        env.setParallelism(1);
        
        // 테스트 로직
        // ...
        
        env.execute();
    }
}


로컬 Flink 클러스터 실행

# Flink 로컬 클러스터 시작
$FLINK_HOME/bin/start-cluster.sh

# Job 제출
$FLINK_HOME/bin/flink run \
  -c com.example.flink.WordCount \
  target/flink-wordcount-1.0-SNAPSHOT.jar \
  --input kafka://localhost:9092/input \
  --output kafka://localhost:9092/output

# Web UI 접속
http://localhost:8081

# 클러스터 중지
$FLINK_HOME/bin/stop-cluster.sh


5.3 개발 → 배포 전체 프로세스

# 1. 기능 브랜치 생성
git checkout -b feature/add-windowing

# 2. 코드 작성 및 로컬 테스트
mvn clean test

# 3. 커밋 및 푸시
git add .
git commit -m "Add tumbling window aggregation"
git push origin feature/add-windowing

# 4. Pull Request 생성 → develop 브랜치로
# GitHub/GitLab에서 PR 생성 및 코드 리뷰

# 5. develop 브랜치에 머지
# → Jenkins 자동 빌드 트리거
# → Docker 이미지 빌드 & 푸시
# → GitOps 레포 업데이트
# → ArgoCD 자동 배포 (flink-dev 네임스페이스)

# 6. 개발 환경 검증
kubectl get flinkdeployment -n flink-dev
kubectl logs -n flink-dev wordcount-app-<pod-id> -f

# 7. staging 브랜치로 머지
git checkout staging
git merge develop
git push origin staging
# → 자동으로 staging 환경 배포

# 8. 스테이징 환경 검증 후 프로덕션 배포
git checkout main
git merge staging
git push origin main
# → Jenkins에서 수동 승인 대기
# → 승인 후 프로덕션 배포


5.4 빠른 반복 개발 (Skaffold 활용)

# skaffold.yaml
apiVersion: skaffold/v4beta6
kind: Config
build:
  artifacts:
    - image: flink-wordcount
      jib:
        project: com.example:flink-wordcount
deploy:
  kubectl:
    manifests:
      - k8s/flinkdeployment-dev.yaml
portForward:
  - resourceType: service
    resourceName: wordcount-app-rest
    namespace: flink-dev
    port: 8081
    localPort: 8081


# 개발 모드 실행 (코드 변경 시 자동 재배포)
skaffold dev

# 한 번만 배포
skaffold run


6. 모니터링 및 로깅
6.1 Flink Web UI 접근
Port-forward를 통한 접근

# JobManager REST 서비스 포트포워딩
kubectl port-forward -n flink-dev svc/wordcount-app-rest 8081:8081

# 브라우저에서 접속
http://localhost:8081


Ingress를 통한 접근

# flink-ingress.yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: flink-wordcount-ingress
  namespace: flink-dev
  annotations:
    nginx.ingress.kubernetes.io/rewrite-target: /
spec:
  ingressClassName: nginx
  rules:
    - host: wordcount-dev.flink.example.com
      http:
        paths:
          - path: /
            pathType: Prefix
            backend:
              service:
                name: wordcount-app-rest
                port:
                  number: 8081


6.2 로그 수집 및 조회
Fluentd/Fluent Bit를 통한 중앙 집중 로깅

# fluentd-daemonset.yaml
apiVersion: apps/v1
kind: DaemonSet
metadata:
  name: fluentd
  namespace: kube-system
spec:
  selector:
    matchLabels:
      app: fluentd
  template:
    metadata:
      labels:
        app: fluentd
    spec:
      containers:
        - name: fluentd
          image: fluent/fluentd-kubernetes-daemonset:v1-debian-elasticsearch
          env:
            - name: FLUENT_ELASTICSEARCH_HOST
              value: "elasticsearch.logging.svc.cluster.local"
            - name: FLUENT_ELASTICSEARCH_PORT
              value: "9200"
          volumeMounts:
            - name: varlog
              mountPath: /var/log
            - name: varlibdockercontainers
              mountPath: /var/lib/docker/containers
              readOnly: true
      volumes:
        - name: varlog
          hostPath:
            path: /var/log
        - name: varlibdockercontainers
          hostPath:
            path: /var/lib/docker/containers


로그 조회 명령어

# 특정 Pod 로그 확인
kubectl logs -n flink-dev wordcount-app-taskmanager-1-1 -f

# 모든 TaskManager 로그 확인
kubectl logs -n flink-dev -l component=taskmanager,app=wordcount-app -f --max-log-requests=10

# 로그 파일로 저장
kubectl logs -n flink-dev wordcount-app-jobmanager-0 > jobmanager.log

# 이전 컨테이너 로그 (재시작된 경우)
kubectl logs -n flink-dev wordcount-app-taskmanager-1-1 --previous


6.3 메트릭 모니터링
Prometheus & Grafana 설정

# prometheus-scrape-config.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: prometheus-config
  namespace: monitoring
data:
  prometheus.yml: |
    global:
      scrape_interval: 15s
    
    scrape_configs:
      - job_name: 'flink'
        kubernetes_sd_configs:
          - role: pod
            namespaces:
              names:
                - flink-dev
                - flink-staging
                - flink-prod
        relabel_configs:
          - source_labels: [__meta_kubernetes_pod_label_component]
            action: keep
            regex: (jobmanager|taskmanager)
          - source_labels: [__meta_kubernetes_pod_annotation_prometheus_io_port]
            action: replace
            target_label: __address__
            regex: ([^:]+)(?::\d+)?
            replacement: $1:9999


FlinkDeployment에 메트릭 활성화

spec:
  flinkConfiguration:
    metrics.reporter.prom.factory.class: org.apache.flink.metrics.prometheus.PrometheusReporterFactory
    metrics.reporter.prom.port: 9999
  
  jobManager:
    podTemplate:
      spec:
        containers:
          - name: flink-main-container
            ports:
              - name: metrics
                containerPort: 9999
                protocol: TCP
  
  taskManager:
    podTemplate:
      spec:
        containers:
          - name: flink-main-container
            ports:
              - name: metrics
                containerPort: 9999
                protocol: TCP


주요 메트릭 목록



|메트릭                                                                   |설명          |알람 기준               |
|----------------------------------------------------------------------|------------|--------------------|
|`flink_taskmanager_Status_JVM_Memory_Heap_Used`                       |Heap 메모리 사용량|> 80%               |
|`flink_jobmanager_job_uptime`                                         |Job 실행 시간   |재시작 감지              |
|`flink_taskmanager_job_task_numRecordsInPerSecond`                    |입력 레코드 수    |< threshold (처리량 저하)|
|`flink_taskmanager_job_task_numRecordsOutPerSecond`                   |출력 레코드 수    |lag 증가              |
|`flink_jobmanager_job_numberOfFailedCheckpoints`                      |실패한 체크포인트 수 |> 3 in 10min        |
|`flink_taskmanager_Status_JVM_GarbageCollector_G1_Old_Generation_Time`|GC 시간       |> 5초                |

Grafana 대시보드

# Flink 공식 Grafana 대시보드 임포트
# Dashboard ID: 14644 (Apache Flink Dashboard)


6.4 알람 설정
Prometheus AlertManager 규칙

# flink-alerts.yaml
groups:
  - name: flink_alerts
    interval: 30s
    rules:
      - alert: FlinkJobDown
        expr: up{job="flink"} == 0
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "Flink job {{ $labels.job_name }} is down"
          description: "Flink job has been down for more than 5 minutes"
      
      - alert: FlinkHighCheckpointFailureRate
        expr: rate(flink_jobmanager_job_numberOfFailedCheckpoints[10m]) > 0.5
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "High checkpoint failure rate for {{ $labels.job_name }}"
          description: "Checkpoint failure rate is {{ $value }} per minute"
      
      - alert: FlinkHighBackpressure
        expr: flink_taskmanager_job_task_buffers_outPoolUsage > 0.9
        for: 15m
        labels:
          severity: warning
        annotations:
          summary: "High backpressure detected in {{ $labels.job_name }}"
          description: "Output buffer pool usage is {{ $value }}"
      
      - alert: FlinkHighMemoryUsage
        expr: (flink_taskmanager_Status_JVM_Memory_Heap_Used / flink_taskmanager_Status_JVM_Memory_Heap_Max) > 0.85
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "High heap memory usage in {{ $labels.job_name }}"
          description: "Heap memory usage is {{ $value | humanizePercentage }}"


7. 트러블슈팅
7.1 자주 발생하는 문제
문제 1: Pod가 Pending 상태
증상

kubectl get pods -n flink-dev
NAME                              READY   STATUS    RESTARTS   AGE
wordcount-app-taskmanager-1-1     0/1     Pending   0          5m


원인 및 해결

# 원인 확인
kubectl describe pod wordcount-app-taskmanager-1-1 -n flink-dev

# 일반적인 원인:
# 1. 리소스 부족
#    → 노드 리소스 확인: kubectl top nodes
#    → FlinkDeployment의 resource 요청량 줄이기

# 2. PVC Pending
#    → PVC 상태 확인: kubectl get pvc -n flink-dev
#    → StorageClass 확인

# 3. Node Affinity/Taint 문제
#    → 노드 레이블 확인: kubectl get nodes --show-labels


문제 2: Job이 계속 재시작
증상

kubectl get flinkdeployment -n flink-dev
NAME            JOB STATUS   LIFECYCLE STATE
wordcount-app   FAILED       DEPLOYED


원인 및 해결

# 로그 확인
kubectl logs -n flink-dev wordcount-app-jobmanager-0 | grep -i error

# 일반적인 원인:
# 1. OutOfMemoryError
#    → JobManager/TaskManager 메모리 증가
#    → JVM 힙 설정 조정

# 2. 체크포인트 실패
#    → 체크포인트 타임아웃 증가
#    → State Backend 설정 확인 (S3 접근 권한 등)

# 3. 네트워크 문제
#    → Kafka/외부 시스템 연결 확인
#    → NetworkPolicy 확인


문제 3: 업그레이드 시 State 손실
증상

Job을 업그레이드했는데 이전 상태가 복원되지 않음


해결

# 1. Savepoint 생성 확인
kubectl get flinkdeployment wordcount-app -n flink-dev -o jsonpath='{.status.jobStatus.savepointInfo.lastSavepoint}'

# 2. upgradeMode 확인
spec:
  job:
    upgradeMode: savepoint  # 반드시 savepoint로 설정
    allowNonRestoredState: false

# 3. State 스키마 호환성
# - Flink State Processor API 사용하여 state 마이그레이션
# - 상태 직렬화 설정 확인


문제 4: 체크포인트 실패
증상

Checkpoint timeout 또는 실패 빈번


해결

flinkConfiguration:
  # 체크포인트 타임아웃 증가
  execution.checkpointing.timeout: 15min
  
  # 동시 체크포인트 제한
  execution.checkpointing.max-concurrent-checkpoints: 1
  
  # 최소 간격 설정
  execution.checkpointing.min-pause: 60s
  
  # Unaligned checkpoint 활성화 (Backpressure 시 유용)
  execution.checkpointing.unaligned.enabled: true
  
  # RocksDB 튜닝
  state.backend.rocksdb.thread.num: 4
  state.backend.rocksdb.checkpoint.transfer.thread.num: 4


7.2 롤백 절차
방법 1: ArgoCD를 통한 롤백

# 이전 버전 확인
argocd app history flink-wordcount-dev

# 특정 리비전으로 롤백
argocd app rollback flink-wordcount-dev <REVISION>


방법 2: GitOps 레포지토리 롤백

cd flink-gitops
git log apps/wordcount/dev/flinkdeployment.yaml

# 이전 커밋으로 되돌리기
git revert <commit-hash>
git push origin main


방법 3: 수동 롤백 (긴급 상황)

# 1. 현재 Deployment 삭제
kubectl delete flinkdeployment wordcount-app -n flink-dev

# 2. 이전 버전 YAML 적용
kubectl apply -f flinkdeployment-v1.0.yaml -n flink-dev

# 3. 특정 Savepoint에서 복원
kubectl patch flinkdeployment wordcount-app -n flink-dev \
  --type merge \
  -p '{"spec":{"job":{"initialSavepointPath":"s3://flink-savepoints/savepoint-abc123"}}}'


7.3 디버깅 팁
Pod 내부 접속

# JobManager 컨테이너 접속
kubectl exec -it -n flink-dev wordcount-app-jobmanager-0 -- /bin/bash

# 로그 파일 직접 확인
ls -lh /opt/flink/log/
tail -f /opt/flink/log/flink-*-standalonesession-*.log


이벤트 확인

# FlinkDeployment 이벤트
kubectl describe flinkdeployment wordcount-app -n flink-dev

# Pod 이벤트
kubectl get events -n flink-dev --sort-by='.lastTimestamp'


Flink REST API 직접 호출

# Job 목록 조회
kubectl port-forward -n flink-dev svc/wordcount-app-rest 8081:8081
curl http://localhost:8081/jobs

# Job 상세 정보
curl http://localhost:8081/jobs/<job-id>

# Savepoint 트리거
curl -X POST http://localhost:8081/jobs/<job-id>/savepoints \
  -H "Content-Type: application/json" \
  -d '{"target-directory": "s3://flink-savepoints/", "cancel-job": false}'


네트워크 디버깅

# Pod에서 외부 연결 테스트
kubectl run -it --rm debug --image=nicolaka/netshoot -n flink-dev -- /bin/bash

# Kafka 연결 테스트
nc -zv kafka-broker.kafka.svc.cluster.local 9092

# DNS 조회
nslookup kafka-broker.kafka.svc.cluster.local


8. 보안 및 권한
8.1 RBAC 설정
ServiceAccount 생성

# flink-rbac.yaml
apiVersion: v1
kind: ServiceAccount
metadata:
  name: flink-service-account
  namespace: flink-dev

---
apiVersion: rbac.authorization.k8s.io/v1
kind: Role
metadata:
  name: flink-role
  namespace: flink-dev
rules:
  - apiGroups: [""]
    resources: ["pods", "services", "configmaps"]
    verbs: ["get", "list", "watch", "create", "update", "patch", "delete"]
  - apiGroups: ["apps"]
    resources: ["deployments"]
    verbs: ["get", "list", "watch"]

---
apiVersion: rbac.authorization.k8s.io/v1
kind: RoleBinding
metadata:
  name: flink-role-binding
  namespace: flink-dev
subjects:
  - kind: ServiceAccount
    name: flink-service-account
    namespace: flink-dev
roleRef:
  kind: Role
  name: flink-role
  apiGroup: rbac.authorization.k8s.io


Operator용 ClusterRole

# operator-rbac.yaml
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRole
metadata:
  name: flink-operator-role
rules:
  - apiGroups: ["flink.apache.org"]
    resources: ["flinkdeployments", "flinkdeployments/status", "flinksessionjobs"]
    verbs: ["*"]
  - apiGroups: [""]
    resources: ["pods", "services", "configmaps", "events"]
    verbs: ["*"]
  - apiGroups: ["apps"]
    resources: ["deployments", "replicasets"]
    verbs: ["*"]


8.2 Secret 관리
Kubernetes Secret 사용

# kafka-credentials-secret.yaml
apiVersion: v1
kind: Secret
metadata:
  name: kafka-credentials
  namespace: flink-dev
type: Opaque
stringData:
  username: flink-user
  password: supersecret
  ca.crt: |
    -----BEGIN CERTIFICATE-----
    MIIDXTCCAkWgAwIBAgIJAKZ...
    -----END CERTIFICATE-----


FlinkDeployment에서 Secret 사용

spec:
  jobManager:
    podTemplate:
      spec:
        containers:
          - name: flink-main-container
            env:
              - name: KAFKA_USERNAME
                valueFrom:
                  secretKeyRef:
                    name: kafka-credentials
                    key: username
              - name: KAFKA_PASSWORD
                valueFrom:
                  secretKeyRef:
                    name: kafka-credentials
                    key: password
            volumeMounts:
              - name: kafka-certs
                mountPath: /etc/flink/certs
                readOnly: true
        volumes:
          - name: kafka-certs
            secret:
              secretName: kafka-credentials
              items:
                - key: ca.crt
                  path: ca.crt


External Secrets Operator (권장)

# external-secret.yaml
apiVersion: external-secrets.io/v1beta1
kind: ExternalSecret
metadata:
  name: flink-kafka-credentials
  namespace: flink-dev
spec:
  refreshInterval: 1h
  secretStoreRef:
    name: aws-secrets-manager
    kind: SecretStore
  target:
    name: kafka-credentials
    creationPolicy: Owner
  data:
    - secretKey: username
      remoteRef:
        key: prod/flink/kafka
        property: username
    - secretKey: password
      remoteRef:
        key: prod/flink/kafka
        property: password


8.3 네트워크 정책
기본 격리 정책

# network-policy.yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: flink-network-policy
  namespace: flink-dev
spec:
  podSelector:
    matchLabels:
      app: wordcount-app
  policyTypes:
    - Ingress
    - Egress
  
  ingress:
    # JobManager REST API 접근 허용
    - from:
        - namespaceSelector:
            matchLabels:
              name: ingress-nginx
      ports:
        - protocol: TCP
          port: 8081
    
    # TaskManager 간 통신 허용
    - from:
        - podSelector:
            matchLabels:
              app: wordcount-app
      ports:
        - protocol: TCP
          port: 6121
        - protocol: TCP
          port: 6122
  
  egress:
    # DNS 허용
    - to:
        - namespaceSelector: {}
          podSelector:
            matchLabels:
              k8s-app: kube-dns
      ports:
        - protocol: UDP
          port: 53
    
    # Kafka 접근 허용
    - to:
        - namespaceSelector:
            matchLabels:
              name: kafka
      ports:
        - protocol: TCP
          port: 9092
    
    # S3/Object Storage 허용
    - to:
        - namespaceSelector: {}
      ports:
        - protocol: TCP
          port: 443


8.4 Pod Security Standards

# pod-security-policy.yaml
apiVersion: v1
kind: Namespace
metadata:
  name: flink-dev
  labels:
    pod-security.kubernetes.io/enforce: restricted
    pod-security.kubernetes.io/audit: restricted
    pod-security.kubernetes.io/warn: restricted

---
# FlinkDeployment에서 보안 컨텍스트 설정
spec:
  jobManager:
    podTemplate:
      spec:
        securityContext:
          runAsNonRoot: true
          runAsUser: 9999
          fsGroup: 9999
          seccompProfile:
            type: RuntimeDefault
        containers:
          - name: flink-main-container
            securityContext:
              allowPrivilegeEscalation: false
              readOnlyRootFilesystem: true
              capabilities:
                drop:
                  - ALL
            volumeMounts:
              - name: tmp
                mountPath: /tmp
              - name: flink-logs
                mountPath: /opt/flink/log
        volumes:
          - name: tmp
            emptyDir: {}
          - name: flink-logs
            emptyDir: {}


9. 부록
9.1 명령어 치트시트
Kubernetes 기본 명령어

# FlinkDeployment 조회
kubectl get flinkdeployment -n flink-dev
kubectl get flink -n flink-dev  # 축약형

# 상세 정보
kubectl describe flinkdeployment wordcount-app -n flink-dev

# YAML 출력
kubectl get flinkdeployment wordcount-app -n flink-dev -o yaml

# Job 상태 확인
kubectl get flinkdeployment wordcount-app -n flink-dev -o jsonpath='{.status.jobStatus.state}'

# Pod 목록
kubectl get pods -n flink-dev -l app=wordcount-app

# 로그 확인
kubectl logs -n flink-dev wordcount-app-jobmanager-0 -f
kubectl logs -n flink-dev -l component=taskmanager -f --max-log-requests=10

# 실시간 리소스 사용량
kubectl top pods -n flink-dev

# Pod 내부 접속
kubectl exec -it -n flink-dev wordcount-app-jobmanager-0 -- /bin/bash

# 포트 포워딩
kubectl port-forward -n flink-dev svc/wordcount-app-rest 8081:8081

# ConfigMap 조회
kubectl get configmap -n flink-dev

# Secret 조회 (base64 디코딩)
kubectl get secret kafka-credentials -n flink-dev -o jsonpath='{.data.password}' | base64 -d


Flink Operator 명령어

# Savepoint 트리거
kubectl patch flinkdeployment wordcount-app -n flink-dev \
  --type merge \
  -p '{"spec":{"job":{"savepointTriggerNonce": '$(date +%s)'}}}' 

# Job 중지
kubectl patch flinkdeployment wordcount-app -n flink-dev \
  --type merge \
  -p '{"spec":{"job":{"state": "suspended"}}}'

# Job 재시작
kubectl patch flinkdeployment wordcount-app -n flink-dev \
  --type merge \
  -p '{"spec":{"job":{"state": "running"}}}'

# Upgrade mode 변경
kubectl patch flinkdeployment wordcount-app -n flink-dev \
  --type merge \
  -p '{"spec":{"job":{"upgradeMode": "last-state"}}}'

# Parallelism 변경
kubectl patch flinkdeployment wordcount-app -n flink-dev \
  --type merge \
  -p '{"spec":{"job":{"parallelism": 8}}}'


ArgoCD 명령어

# 로그인
argocd login argocd-server.argocd.svc.cluster.local

# 애플리케이션 목록
argocd app list

# 애플리케이션 상태
argocd app get flink-wordcount-dev

# 수동 동기화
argocd app sync flink-wordcount-dev

# 히스토리 조회
argocd app history flink-wordcount-dev

# 롤백
argocd app rollback flink-wordcount-dev <REVISION>

# Diff 확인
argocd app diff flink-wordcount-dev

# 로그 확인
argocd app logs flink-wordcount-dev


Flink CLI 명령어

# Flink 클러스터에 연결
export FLINK_REST_URL=http://localhost:8081

# Job 목록
flink list

# Job 제출
flink run -d \
  -c com.example.flink.WordCount \
  /path/to/flink-wordcount.jar \
  --input kafka://input \
  --output kafka://output

# Job 취소
flink cancel <job-id>

# Savepoint 생성
flink savepoint <job-id> s3://flink-savepoints/

# Savepoint로 복원
flink run -s s3://flink-savepoints/savepoint-abc123 \
  -c com.example.flink.WordCount \
  /path/to/flink-wordcount.jar

# Job 정보
flink info <job-id>


9.2 참고 자료
공식 문서
	∙	Apache Flink Documentation
	∙	Flink Kubernetes Operator
	∙	Flink API Reference
	∙	ArgoCD Documentation
	∙	Jenkins Pipeline Syntax
Kubernetes Resources
	∙	Kubernetes Documentation
	∙	Helm Charts
	∙	Kubernetes Operators
Best Practices
	∙	Flink Production Readiness Checklist
	∙	Flink Performance Tuning
	∙	GitOps Principles
Community
	∙	Flink Mailing Lists
	∙	Flink Slack Channel
	∙	Stack Overflow - Flink Tag
9.3 FAQ
Q: Application Mode와 Session Mode 중 어느 것을 선택해야 하나요?
A: 대부분의 프로덕션 환경에서는 Application Mode를 권장합니다. 각 Job이 전용 클러스터를 가지므로 리소스 격리가 보장되고, Job 간 영향을 받지 않습니다. Session Mode는 개발/테스트 환경이나 짧은 임시 Job에 적합합니다.
Q: Savepoint와 Checkpoint의 차이는 무엇인가요?
A: Checkpoint는 자동으로 주기적으로 생성되는 경량 스냅샷으로, 장애 복구용입니다. Savepoint는 수동으로 트리거하는 완전한 스냅샷으로, Job 업그레이드나 클러스터 마이그레이션 시 사용됩니다.
Q: TaskManager 수를 어떻게 결정하나요?
A: TaskManager 수 = Job Parallelism / TaskManager당 슬롯 수로 계산합니다. 예를 들어, parallelism=16이고 슬롯=2라면 TaskManager는 8개가 필요합니다. 고가용성을 위해 여유분을 두는 것이 좋습니다.
Q: State Backend는 어떤 것을 선택해야 하나요?
A:
	∙	HashMapStateBackend: 작은 상태 (GB 이하), 빠른 성능
	∙	EmbeddedRocksDBStateBackend: 큰 상태 (수십 GB~TB), 증분 체크포인트 지원 (권장)
Q: Job 업그레이드 시 다운타임을 최소화하려면?
A:
	1.	upgradeMode: savepoint 설정
	2.	Blue-Green 배포: 새 버전을 별도 namespace에 배포 후 트래픽 전환
	3.	Canary 배포: 일부 트래픽만 새 버전으로 라우팅
Q: 체크포인트가 자주 실패합니다. 어떻게 해결하나요?
A:
	1.	타임아웃 증가: execution.checkpointing.timeout
	2.	Unaligned checkpoint 활성화: backpressure 상황에서 유용
	3.	State 크기 최적화: TTL 설정, 불필요한 상태 제거
	4.	RocksDB 튜닝: 스레드 수, 블록 캐시 크기 조정
Q: 어떻게 Job의 처리량을 높일 수 있나요?
A:
	1.	Parallelism 증가
	2.	TaskManager 리소스 증가 (CPU, 메모리)
	3.	Kafka partition 수 증가 (입력 소스)
	4.	네트워크 버퍼 크기 조정: taskmanager.network.memory.fraction
	5.	불필요한 shuffle 최소화: keyBy 연산 최적화
Q: Production 환경에서 권장하는 HA 설정은?
A:

flinkConfiguration:
  high-availability: kubernetes
  high-availability.storageDir: s3://flink-ha/
  high-availability.cluster-id: /flink-cluster-wordcount
  restart-strategy: failure-rate
  restart-strategy.failure-rate.max-failures-per-interval: 5
  restart-strategy.failure-rate.failure-rate-interval: 10 min

jobManager:
  replicas: 2  # HA JobManager


Q: 로그를 ELK 스택으로 전송하려면?
A: Fluentd/Fluent Bit DaemonSet을 사용하거나, Flink의 Log4j 설정에서 직접 Elasticsearch Appender를 사용할 수 있습니다. 위 “6.2 로그 수집 및 조회” 섹션 참고.
Q: CI/CD 파이프라인에서 통합 테스트는 어떻게 수행하나요?
A:
	1.	Testcontainers를 사용한 Kafka/DB 모킹
	2.	MiniCluster를 사용한 Flink Job 테스트
	3.	별도 테스트 네임스페이스에 임시 배포 후 검증
	4.	스테이징 환경에서 실제 데이터로 smoke test

문서 변경 이력



|버전 |날짜        |작성자  |변경 내용   |
|---|----------|-----|--------|
|1.0|2025-01-23|Donam|초기 문서 작성|

연락처 및 지원
	∙	개발팀: dev-team@example.com
	∙	운영팀: ops-team@example.com
	∙	Slack 채널: #flink-support
문의사항이나 개선 제안이 있으시면 언제든지 연락 주세요!​​​​​​​​​​​​​​​​