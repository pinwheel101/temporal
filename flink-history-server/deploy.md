# flink-deployment.yaml
apiVersion: flink.apache.org/v1beta1
kind: FlinkDeployment
metadata:
  name: my-flink-job
  namespace: flink
spec:
  image: flink:2.0
  flinkVersion: v2_0
  flinkConfiguration:
    # Task Slots
    taskmanager.numberOfTaskSlots: "2"
    
    # History Server 연동 - 완료된 job 정보 저장 위치
    jobmanager.archive.fs.dir: "s3://my-bucket/flink/completed-jobs"
    
    # State Backend (Flink 2.0: forst 옵션 추가)
    state.backend.type: "rocksdb"
    state.backend.incremental: "true"
    
    # Checkpoint/Savepoint
    execution.checkpointing.dir: "s3://my-bucket/flink/checkpoints"
    execution.checkpointing.savepoint-dir: "s3://my-bucket/flink/savepoints"
    execution.checkpointing.interval: "60000"
    
    # S3 설정
    s3.access.key: "${AWS_ACCESS_KEY_ID}"
    s3.secret.key: "${AWS_SECRET_ACCESS_KEY}"
    s3.endpoint: "s3.amazonaws.com"
    s3.path.style.access: "false"
    
    # HA 설정 (선택사항)
    high-availability.type: "kubernetes"
    high-availability.storageDir: "s3://my-bucket/flink/ha"
    
  serviceAccount: flink
  jobManager:
    resource:
      memory: "2048m"
      cpu: 1
  taskManager:
    resource:
      memory: "2048m"
      cpu: 1
  job:
    jarURI: local:///opt/flink/examples/streaming/StateMachineExample.jar
    parallelism: 2
    upgradeMode: savepoint  # History Server 연동 시 savepoint 권장
    state: running
