#!/bin/bash
# Demo: Evidence-based worker promotion system
# Shows how to register a worker, record performance, evaluate eligibility, and promote.

BASE_URL="http://localhost:8080"

echo "=== Crabjar Evidence-Based Worker Promotion Demo ==="
echo ""

# 1. Register a worker
echo "Step 1: Registering worker 'test-worker-1'..."
curl -s -X POST "${BASE_URL}/workers/register" \
  -H "Content-Type: application/json" \
  -d '{"name":"test-worker-1","capabilities":["shell"],"ssh_key":"ssh-ed25519 AAAA..."}' | jq .

# 2. Record task completions (simulate 50 successful tasks)
echo ""
echo "Step 2: Recording 50 successful task completions..."
for i in {1..50}; do
    curl -s -X POST "${BASE_URL}/promotion/metrics/test-worker-1/task" \
      -H "Content-Type: application/json" \
      -d '{"success":true}' > /dev/null
done
echo "Done."

# 3. Record decision outcomes (simulate 10 accurate decisions)
echo ""
echo "Step 3: Recording 10 accurate routing decisions..."
for i in {1..10}; do
    curl -s -X POST "${BASE_URL}/promotion/metrics/test-worker-1/decision" \
      -H "Content-Type: application/json" \
      -d '{"accurate":true}' > /dev/null
done
echo "Done."

# 4. Check worker metrics
echo ""
echo "Step 4: Checking worker competence metrics..."
curl -s "${BASE_URL}/promotion/metrics/test-worker-1" | jq .

# 5. Evaluate promotion eligibility
echo ""
echo "Step 5: Evaluating promotion eligibility to supervisor rank..."
curl -s "${BASE_URL}/promotion/evaluate/test-worker-1" | jq .

# 6. Execute promotion (if eligible)
echo ""
echo "Step 6: Executing promotion to supervisor rank..."
curl -s -X POST "${BASE_URL}/promotion/execute/test-worker-1" | jq .

# 7. Check promotion history
echo ""
echo "Step 7: Checking promotion history..."
curl -s "${BASE_URL}/promotion/history/test-worker-1" | jq .

echo ""
echo "=== Demo Complete ==="
