#!/bin/bash

# EZNebula Backend Test Script

API_BASE="http://localhost:8080/api/v1"

echo "=== EZNebula Backend API Test ==="
echo ""

# 1. Health Check
echo "1. Testing health endpoint..."
curl -s "${API_BASE}/health" | jq .
echo ""

# 2. Create Network Group
echo "2. Creating network group 'test-network'..."
RESPONSE=$(curl -s -X POST "${API_BASE}/admin/groups?groupName=test-network&cidrBlock=10.168.0.0/16&description=Test+Network")
echo "$RESPONSE" | jq .

# Extract join token
JOIN_TOKEN=$(echo "$RESPONSE" | jq -r '.data.joinToken')
echo "Join Token: $JOIN_TOKEN"
echo ""

# 3. List Network Groups
echo "3. Listing all network groups..."
curl -s "${API_BASE}/admin/groups" | jq .
echo ""

# 4. Simulate Client Join (requires nebula-cert to generate keypair)
echo "4. To test client join, you need to:"
echo "   - Generate a keypair: nebula-cert keygen -out-key client.key -out-pub client.pub"
echo "   - POST to ${API_BASE}/join with:"
echo "     {"
echo "       \"groupName\": \"test-network\","
echo "       \"joinToken\": \"$JOIN_TOKEN\","
echo "       \"clientPublicKey\": \"<content of client.pub>\","
echo "       \"clientName\": \"test-client\""
echo "     }"
echo ""

echo "=== Test Complete ==="
