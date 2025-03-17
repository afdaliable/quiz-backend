#!/bin/bash

# Test script for the license controller API
# This script tests the license controller endpoints

# Set the base URL
BASE_URL="http://localhost:8787"

# Set colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

# Function to print colored output
print_success() {
    echo -e "${GREEN}$1${NC}"
}

print_error() {
    echo -e "${RED}$1${NC}"
}

print_warning() {
    echo -e "${YELLOW}$1${NC}"
}

# Function to test an endpoint
test_endpoint() {
    local endpoint=$1
    local method=$2
    local data=$3
    local auth_token=$4
    local description=$5

    echo "Testing: $description"
    echo "Endpoint: $endpoint"
    echo "Method: $method"
    
    if [ -n "$data" ]; then
        echo "Data: $data"
    fi
    
    if [ -n "$auth_token" ]; then
        echo "Using auth token: ${auth_token:0:10}..."
        
        if [ "$method" == "GET" ]; then
            response=$(curl -s -X $method -H "Authorization: Bearer $auth_token" "$BASE_URL$endpoint")
        else
            response=$(curl -s -X $method -H "Content-Type: application/json" -H "Authorization: Bearer $auth_token" -d "$data" "$BASE_URL$endpoint")
        fi
    else
        if [ "$method" == "GET" ]; then
            response=$(curl -s -X $method "$BASE_URL$endpoint")
        else
            response=$(curl -s -X $method -H "Content-Type: application/json" -d "$data" "$BASE_URL$endpoint")
        fi
    fi
    
    echo "Response: $response"
    echo ""
    
    # Return the response for further processing
    echo "$response"
}

# Login to get a token
echo "Logging in to get a token..."
login_data='{"email":"test@example.com","password":"password123"}'
login_response=$(test_endpoint "/login" "POST" "$login_data" "" "Login to get a token")

# Extract the token from the login response
token=$(echo $login_response | grep -o '"token":"[^"]*' | cut -d'"' -f4)

if [ -z "$token" ]; then
    print_error "Failed to get token. Exiting."
    exit 1
fi

print_success "Successfully obtained token: ${token:0:10}..."

# Test 1: Generate payment link
echo "Test 1: Generate payment link"
payment_link_response=$(test_endpoint "/license/payment-link/2" "GET" "" "$token" "Generate payment link for premium plan")

# Check if the payment link was generated successfully
if [[ $payment_link_response == *"payment_link"* ]]; then
    print_success "Payment link generated successfully"
else
    print_error "Failed to generate payment link"
fi

# Extract the payment link from the response
payment_link=$(echo $payment_link_response | grep -o '"payment_link":"[^"]*' | cut -d'"' -f4)
echo "Payment link: $payment_link"
echo ""

# Test 2: Verify license
echo "Test 2: Verify license"
license_data='{
    "license_code": "TEST_LICENSE_123",
    "product_id": "test-product-id",
    "email": "test@example.com",
    "name": "Test User",
    "phone": "1234567890"
}'
verify_response=$(test_endpoint "/license/verify" "POST" "$license_data" "" "Verify license")

# Check if the license was verified successfully
if [[ $verify_response == *"success\":true"* ]]; then
    print_success "License verified successfully"
else
    print_error "Failed to verify license"
fi

# Extract the token from the verify response
license_token=$(echo $verify_response | grep -o '"token":"[^"]*' | cut -d'"' -f4)

if [ -n "$license_token" ]; then
    print_success "Successfully obtained license token: ${license_token:0:10}..."
    
    # Test 3: Get user licenses
    echo "Test 3: Get user licenses"
    licenses_response=$(test_endpoint "/license/user-licenses" "GET" "" "$license_token" "Get user licenses")
    
    # Check if the licenses were retrieved successfully
    if [[ $licenses_response == *"license_code"* ]]; then
        print_success "User licenses retrieved successfully"
    else
        print_error "Failed to retrieve user licenses"
    fi
else
    print_warning "No license token obtained, skipping user licenses test"
fi

echo "License API tests completed" 