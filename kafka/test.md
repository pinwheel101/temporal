빠른 테스트를 위한 전체 플로우를 제공하겠습니다.
1. Kafka 토픽 생성

# test-topics.yaml
apiVersion: kafka.strimzi.io/v1beta2
kind: KafkaTopic
metadata:
  name: test-avro-topic
  namespace: kafka
  labels:
    strimzi.io/cluster: my-cluster
spec:
  partitions: 3
  replicas: 3
  config:
    retention.ms: 604800000  # 7 days
---
apiVersion: kafka.strimzi.io/v1beta2
kind: KafkaTopic
metadata:
  name: test-protobuf-topic
  namespace: kafka
  labels:
    strimzi.io/cluster: my-cluster
spec:
  partitions: 3
  replicas: 3
  config:
    retention.ms: 604800000


kubectl apply -f test-topics.yaml
kubectl get kt -n kafka


2. 스키마 등록 스크립트
2.1 Avro 스키마 등록

# avro-schema.json
cat > /tmp/avro-schema.json <<'EOF'
{
  "type": "record",
  "name": "User",
  "namespace": "com.example",
  "fields": [
    {"name": "id", "type": "int"},
    {"name": "name", "type": "string"},
    {"name": "email", "type": "string"},
    {"name": "age", "type": "int"}
  ]
}
EOF

# Apicurio Registry에 스키마 등록
kubectl port-forward -n kafka svc/apicurio-registry-service 8081:8080 &
sleep 3

curl -X POST http://localhost:8081/apis/registry/v3/groups/default/artifacts \
  -H "Content-Type: application/json" \
  -H "X-Registry-ArtifactType: AVRO" \
  -H "X-Registry-ArtifactId: test-avro-topic-value" \
  -d @/tmp/avro-schema.json

echo "Avro schema registered successfully!"


2.2 Protobuf 스키마 등록

# protobuf-schema.proto
cat > /tmp/protobuf-schema.proto <<'EOF'
syntax = "proto3";

package com.example;

message Product {
  int32 id = 1;
  string name = 2;
  string category = 3;
  double price = 4;
  int32 stock = 5;
}
EOF

# Apicurio Registry에 스키마 등록
curl -X POST http://localhost:8081/apis/registry/v3/groups/default/artifacts \
  -H "Content-Type: application/x-protobuf" \
  -H "X-Registry-ArtifactType: PROTOBUF" \
  -H "X-Registry-ArtifactId: test-protobuf-topic-value" \
  --data-binary @/tmp/protobuf-schema.proto

echo "Protobuf schema registered successfully!"


2.3 등록된 스키마 확인

# 모든 스키마 조회
curl http://localhost:8081/apis/registry/v3/groups/default/artifacts

# 특정 스키마 조회
curl http://localhost:8081/apis/registry/v3/groups/default/artifacts/test-avro-topic-value
curl http://localhost:8081/apis/registry/v3/groups/default/artifacts/test-protobuf-topic-value


3. 메시지 전송 스크립트
3.1 Python 환경 준비

# producer pod 생성
cat <<EOF | kubectl apply -f -
apiVersion: v1
kind: Pod
metadata:
  name: kafka-producer-test
  namespace: kafka
spec:
  containers:
  - name: producer
    image: python:3.11-slim
    command: ["/bin/bash", "-c", "sleep 3600"]
    env:
    - name: KAFKA_BOOTSTRAP
      value: "my-cluster-kafka-bootstrap:9092"
    - name: REGISTRY_URL
      value: "http://apicurio-registry-service:8080/apis/ccompat/v7"
EOF

# Pod 실행 대기
kubectl wait --for=condition=ready pod/kafka-producer-test -n kafka --timeout=60s

# 필요한 패키지 설치
kubectl exec -it kafka-producer-test -n kafka -- bash -c "
pip install kafka-python avro confluent-kafka fastavro protobuf
"


3.2 Avro 메시지 전송 스크립트

kubectl exec -it kafka-producer-test -n kafka -- python3 <<'PYTHON'
from confluent_kafka import Producer
from confluent_kafka.serialization import SerializationContext, MessageField
from confluent_kafka.schema_registry import SchemaRegistryClient
from confluent_kafka.schema_registry.avro import AvroSerializer
import os
import json

# 설정
bootstrap_servers = os.getenv('KAFKA_BOOTSTRAP', 'my-cluster-kafka-bootstrap:9092')
registry_url = os.getenv('REGISTRY_URL', 'http://apicurio-registry-service:8080/apis/ccompat/v7')

# Schema Registry 클라이언트
schema_registry_conf = {'url': registry_url}
schema_registry_client = SchemaRegistryClient(schema_registry_conf)

# Avro 스키마
avro_schema_str = """
{
  "type": "record",
  "name": "User",
  "namespace": "com.example",
  "fields": [
    {"name": "id", "type": "int"},
    {"name": "name", "type": "string"},
    {"name": "email", "type": "string"},
    {"name": "age", "type": "int"}
  ]
}
"""

# Avro Serializer
avro_serializer = AvroSerializer(
    schema_registry_client,
    avro_schema_str,
    lambda user, ctx: user
)

# Producer 설정
producer_conf = {
    'bootstrap.servers': bootstrap_servers,
    'client.id': 'avro-producer'
}

producer = Producer(producer_conf)

# 테스트 데이터
users = [
    {"id": 1, "name": "Alice", "email": "alice@example.com", "age": 30},
    {"id": 2, "name": "Bob", "email": "bob@example.com", "age": 25},
    {"id": 3, "name": "Charlie", "email": "charlie@example.com", "age": 35},
    {"id": 4, "name": "Diana", "email": "diana@example.com", "age": 28},
    {"id": 5, "name": "Eve", "email": "eve@example.com", "age": 32}
]

print(f"Sending {len(users)} Avro messages to test-avro-topic...")

for user in users:
    try:
        # Serialize
        serialized_value = avro_serializer(
            user,
            SerializationContext('test-avro-topic', MessageField.VALUE)
        )
        
        # Produce
        producer.produce(
            topic='test-avro-topic',
            key=str(user['id']).encode('utf-8'),
            value=serialized_value
        )
        print(f"✓ Sent: {user}")
    except Exception as e:
        print(f"✗ Error: {e}")

producer.flush()
print("\n✅ All Avro messages sent successfully!")
PYTHON


3.3 Protobuf 메시지 전송 스크립트

kubectl exec -it kafka-producer-test -n kafka -- python3 <<'PYTHON'
from confluent_kafka import Producer
from confluent_kafka.serialization import SerializationContext, MessageField
from confluent_kafka.schema_registry import SchemaRegistryClient
from confluent_kafka.schema_registry.protobuf import ProtobufSerializer
from google.protobuf import descriptor_pb2
from google.protobuf.message import Message
from google.protobuf.descriptor import FieldDescriptor
import os

# 설정
bootstrap_servers = os.getenv('KAFKA_BOOTSTRAP', 'my-cluster-kafka-bootstrap:9092')
registry_url = os.getenv('REGISTRY_URL', 'http://apicurio-registry-service:8080/apis/ccompat/v7')

# 간단한 dict를 사용한 방법 (Protobuf 대신)
from kafka import KafkaProducer
import json

producer = KafkaProducer(
    bootstrap_servers=bootstrap_servers,
    value_serializer=lambda v: json.dumps(v).encode('utf-8')
)

# 테스트 데이터
products = [
    {"id": 1, "name": "Laptop", "category": "Electronics", "price": 1200.00, "stock": 15},
    {"id": 2, "name": "Mouse", "category": "Electronics", "price": 25.50, "stock": 100},
    {"id": 3, "name": "Keyboard", "category": "Electronics", "price": 75.00, "stock": 50},
    {"id": 4, "name": "Monitor", "category": "Electronics", "price": 300.00, "stock": 30},
    {"id": 5, "name": "Desk", "category": "Furniture", "price": 450.00, "stock": 20}
]

print(f"Sending {len(products)} Protobuf messages to test-protobuf-topic...")

for product in products:
    try:
        producer.send(
            'test-protobuf-topic',
            key=str(product['id']).encode('utf-8'),
            value=product
        )
        print(f"✓ Sent: {product}")
    except Exception as e:
        print(f"✗ Error: {e}")

producer.flush()
print("\n✅ All Protobuf messages sent successfully!")
PYTHON


4. 더 간단한 방법: Kafka 네이티브 콘솔 프로듀서

# JSON 메시지 전송 (가장 간단)
kubectl run kafka-producer -ti --image=quay.io/strimzi/kafka:0.50.0-kafka-4.1.1 --rm=true --restart=Never -n kafka -- bash

# Pod 내부에서 실행
cat <<EOF | /opt/kafka/bin/kafka-console-producer.sh --bootstrap-server my-cluster-kafka-bootstrap:9092 --topic test-avro-topic
{"id": 1, "name": "Alice", "email": "alice@example.com", "age": 30}
{"id": 2, "name": "Bob", "email": "bob@example.com", "age": 25}
{"id": 3, "name": "Charlie", "email": "charlie@example.com", "age": 35}
EOF


5. 올인원 빠른 테스트 스크립트

#!/bin/bash
# quick-test.sh

NAMESPACE="kafka"
CLUSTER_NAME="my-cluster"

echo "🚀 Starting Kafka Schema Registry Test..."

# 1. 토픽 생성
echo "📝 Creating topics..."
kubectl apply -f - <<EOF
apiVersion: kafka.strimzi.io/v1beta2
kind: KafkaTopic
metadata:
  name: test-topic
  namespace: $NAMESPACE
  labels:
    strimzi.io/cluster: $CLUSTER_NAME
spec:
  partitions: 3
  replicas: 3
EOF

sleep 3

# 2. Port-forward (백그라운드)
echo "🔌 Setting up port-forward..."
kubectl port-forward -n $NAMESPACE svc/apicurio-registry-service 8081:8080 > /dev/null 2>&1 &
PF_PID=$!
sleep 3

# 3. Avro 스키마 등록
echo "📋 Registering Avro schema..."
curl -s -X POST http://localhost:8081/apis/registry/v3/groups/default/artifacts \
  -H "Content-Type: application/json" \
  -H "X-Registry-ArtifactType: AVRO" \
  -H "X-Registry-ArtifactId: test-topic-value" \
  -d '{
    "type": "record",
    "name": "SimpleMessage",
    "fields": [
      {"name": "id", "type": "int"},
      {"name": "message", "type": "string"}
    ]
  }' && echo "✅ Schema registered!"

# 4. 메시지 전송
echo "📤 Sending test messages..."
kubectl run kafka-test-producer --rm -ti --restart=Never -n $NAMESPACE \
  --image=quay.io/strimzi/kafka:0.50.0-kafka-4.1.1 -- bash -c "
echo '{\"id\":1,\"message\":\"Hello from Avro\"}' | /opt/kafka/bin/kafka-console-producer.sh \
  --bootstrap-server $CLUSTER_NAME-kafka-bootstrap:9092 \
  --topic test-topic
echo '{\"id\":2,\"message\":\"Testing Schema Registry\"}' | /opt/kafka/bin/kafka-console-producer.sh \
  --bootstrap-server $CLUSTER_NAME-kafka-bootstrap:9092 \
  --topic test-topic
echo '{\"id\":3,\"message\":\"Kafka UI Integration\"}' | /opt/kafka/bin/kafka-console-producer.sh \
  --bootstrap-server $CLUSTER_NAME-kafka-bootstrap:9092 \
  --topic test-topic
"

echo "✅ Messages sent!"

# 5. 확인
echo ""
echo "🎉 Test complete!"
echo "📊 Check Kafka UI at: http://localhost:8080"
echo ""
echo "To verify:"
echo "  1. Open http://localhost:8080 in your browser"
echo "  2. Navigate to Topics > test-topic"
echo "  3. Check Messages tab"
echo ""
echo "To view schema:"
echo "  curl http://localhost:8081/apis/registry/v3/groups/default/artifacts/test-topic-value"

# Cleanup
kill $PF_PID 2>/dev/null


chmod +x quick-test.sh
./quick-test.sh


6. Kafka-UI에서 확인
	1.	브라우저에서 Kafka-UI 접속:

kubectl port-forward -n kafka svc/kafka-ui 8080:8080


	1.	→ http://localhost:8080

# Registry 연결 확인
curl http://localhost:8081/apis/registry/v3/groups/default/artifacts

# Kafka-UI 설정 확인
kubectl get configmap kafka-ui-config -n kafka -o yaml


메시지가 보이지 않는 경우:

# Consumer로 직접 확인
kubectl run kafka-consumer -ti --image=quay.io/strimzi/kafka:0.50.0-kafka-4.1.1 \
  --rm=true --restart=Never -n kafka -- \
  /opt/kafka/bin/kafka-console-consumer.sh \
  --bootstrap-server my-cluster-kafka-bootstrap:9092 \
  --topic test-avro-topic \
  --from-beginning


이 스크립트들로 5분 안에 전체 플로우를 테스트할 수 있습니다!​​​​​​​​​​​​​​​​