# Frontend Integration Guide for Mayar License Payment Flow

This guide explains how to integrate the new Mayar license payment flow into your frontend application.

## Overview

The new payment flow works as follows:

1. User selects a premium plan and clicks "Buy Plan"
2. Frontend calls the backend to generate a payment link
3. User is redirected to Mayar's payment page
4. After successful payment, Mayar redirects the user back to our activation page with a license code
5. Frontend verifies the license code with the backend
6. Backend creates a subscription and returns a JWT token
7. Frontend stores the token and redirects the user to the home page

## API Endpoints

### 1. Generate Payment Link

**Endpoint:** `GET /license/payment-link/{plan_id}`

**Headers:**
- `Authorization`: Bearer token

**Response:**
```json
{
  "payment_link": "https://canducation.myr.id/m/special-plan?email=user@example.com&productId=00ba4212-56ea-4b33-82c7-4eb0ee96d1c8&name=User%20Name&phone=628123456789&user_id=123"
}
```

### 2. Verify License

**Endpoint:** `POST /license/verify`

**Headers:**
- `Content-Type`: application/json

**Request Body:**
```json
{
  "license_code": "0C6A09CFA61BBB98",
  "product_id": "aedda011-50a6-4839-9109-3e413ad5b887",
  "email": "user@example.com",
  "name": "User Name",
  "phone": "628123456789"
}
```

**Response:**
```json
{
  "success": true,
  "message": "License activated successfully",
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "user": {
    "id": "123",
    "email": "user@example.com",
    "display_name": "User Name"
  },
  "subscription": {
    "id": 1,
    "plan_id": 2,
    "plan_name": "Gold Plan",
    "expired_at": "2023-12-31T23:59:59Z",
    "is_lifetime": false
  }
}
```

### 3. Get User Licenses

**Endpoint:** `GET /license/user-licenses`

**Headers:**
- `Authorization`: Bearer token

**Response:**
```json
[
  {
    "id": 1,
    "license_code": "0C6A09CFA61BBB98",
    "user_id": "123",
    "plan_id": 2,
    "plan_name": "Gold Plan",
    "status": "Active",
    "expired_at": "2023-12-31T23:59:59Z",
    "days_remaining": 30
  }
]
```

## Integration Steps

### 1. Premium Plan Selection Page

```javascript
// Function to get premium plans
async function getPremiumPlans() {
  const response = await fetch('/premium/plans', {
    headers: {
      'Authorization': `Bearer ${localStorage.getItem('token')}`
    }
  });
  return response.json();
}

// Function to generate payment link
async function generatePaymentLink(planId) {
  const response = await fetch(`/license/payment-link/${planId}`, {
    headers: {
      'Authorization': `Bearer ${localStorage.getItem('token')}`
    }
  });
  const data = await response.json();
  return data.payment_link;
}

// Handle buy button click
async function handleBuyClick(planId) {
  try {
    const paymentLink = await generatePaymentLink(planId);
    // Redirect to Mayar payment page
    window.location.href = paymentLink;
  } catch (error) {
    console.error('Error generating payment link:', error);
    showError('Failed to generate payment link. Please try again.');
  }
}
```

### 2. License Activation Page

Create a page at `/aktivasi-berlangganan` that will handle the redirect from Mayar after successful payment.

```javascript
// Function to extract query parameters from URL
function getQueryParams() {
  const params = new URLSearchParams(window.location.search);
  return {
    licenseCode: params.get('licenseCode'),
    email: params.get('email'),
    productId: params.get('productId'),
    name: params.get('name'),
    phone: params.get('phone')
  };
}

// Function to verify license
async function verifyLicense(licenseData) {
  const response = await fetch('/license/verify', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json'
    },
    body: JSON.stringify(licenseData)
  });
  return response.json();
}

// Handle activation
async function handleActivation() {
  const params = getQueryParams();
  
  // Validate required parameters
  if (!params.licenseCode || !params.productId || !params.email || !params.name) {
    showError('Missing required parameters. Please try again.');
    return;
  }
  
  try {
    const result = await verifyLicense({
      license_code: params.licenseCode,
      product_id: params.productId,
      email: params.email,
      name: params.name,
      phone: params.phone
    });
    
    if (result.success) {
      // Store token and user info
      localStorage.setItem('token', result.token);
      localStorage.setItem('user', JSON.stringify(result.user));
      
      // Show success message
      showSuccess('License activated successfully!');
      
      // Redirect to home page after a short delay
      setTimeout(() => {
        window.location.href = '/';
      }, 2000);
    } else {
      showError(result.error || 'Failed to activate license. Please try again.');
    }
  } catch (error) {
    console.error('Error verifying license:', error);
    showError('Failed to verify license. Please try again.');
  }
}

// Call handleActivation when the page loads
document.addEventListener('DOMContentLoaded', () => {
  // Populate form fields with query parameters
  const params = getQueryParams();
  document.getElementById('licenseCode').value = params.licenseCode || '';
  document.getElementById('email').value = params.email || '';
  document.getElementById('productId').value = params.productId || '';
  document.getElementById('name').value = params.name || '';
  document.getElementById('phone').value = params.phone || '';
  
  // Add event listener to activation button
  document.getElementById('activateButton').addEventListener('click', handleActivation);
});
```

### 3. HTML for License Activation Page

```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Activate Subscription</title>
  <link rel="stylesheet" href="styles.css">
</head>
<body>
  <div class="container">
    <h1>Activate Your Subscription</h1>
    <p>Please review your information and click the Activate button to complete your subscription.</p>
    
    <div class="form-container">
      <div class="form-group">
        <label for="licenseCode">License Code</label>
        <input type="text" id="licenseCode" readonly>
      </div>
      
      <div class="form-group">
        <label for="email">Email</label>
        <input type="email" id="email" readonly>
      </div>
      
      <div class="form-group">
        <label for="name">Name</label>
        <input type="text" id="name" readonly>
      </div>
      
      <div class="form-group">
        <label for="phone">Phone Number</label>
        <input type="text" id="phone">
      </div>
      
      <div class="form-group hidden">
        <label for="productId">Product ID</label>
        <input type="text" id="productId" readonly>
      </div>
      
      <div class="form-actions">
        <button id="activateButton" class="btn-primary">Activate Subscription</button>
      </div>
    </div>
    
    <div id="message" class="message hidden"></div>
  </div>
  
  <script src="activation.js"></script>
</body>
</html>
```

### 4. CSS for License Activation Page

```css
.container {
  max-width: 600px;
  margin: 0 auto;
  padding: 20px;
}

.form-container {
  background-color: #f9f9f9;
  border-radius: 8px;
  padding: 20px;
  margin-top: 20px;
}

.form-group {
  margin-bottom: 15px;
}

.form-group label {
  display: block;
  margin-bottom: 5px;
  font-weight: bold;
}

.form-group input {
  width: 100%;
  padding: 10px;
  border: 1px solid #ddd;
  border-radius: 4px;
  font-size: 16px;
}

.form-group input[readonly] {
  background-color: #f0f0f0;
}

.form-actions {
  margin-top: 20px;
}

.btn-primary {
  background-color: #4CAF50;
  color: white;
  border: none;
  padding: 12px 20px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 16px;
}

.btn-primary:hover {
  background-color: #45a049;
}

.message {
  margin-top: 20px;
  padding: 10px;
  border-radius: 4px;
}

.message.success {
  background-color: #d4edda;
  color: #155724;
  border: 1px solid #c3e6cb;
}

.message.error {
  background-color: #f8d7da;
  color: #721c24;
  border: 1px solid #f5c6cb;
}

.hidden {
  display: none;
}
```

### 5. Helper Functions

```javascript
// Show success message
function showSuccess(message) {
  const messageElement = document.getElementById('message');
  messageElement.textContent = message;
  messageElement.classList.remove('hidden', 'error');
  messageElement.classList.add('success');
}

// Show error message
function showError(message) {
  const messageElement = document.getElementById('message');
  messageElement.textContent = message;
  messageElement.classList.remove('hidden', 'success');
  messageElement.classList.add('error');
}
```

## Important Notes

1. **No Phone Number Check Required**: Unlike the previous implementation, there's no need to check for a phone number before redirecting to the payment page. The phone number will be collected by Mayar during the payment process.

2. **Automatic User Creation**: If a user doesn't exist in the system, the backend will automatically create a new user account based on the information provided in the license verification request.

3. **JWT Token**: After successful license verification, the backend returns a JWT token that should be stored and used for subsequent API calls.

4. **Error Handling**: Make sure to implement proper error handling for all API calls and display user-friendly error messages.

5. **Responsive Design**: Ensure that the activation page is responsive and works well on both desktop and mobile devices.

## Testing

1. **Test Payment Flow**: Test the complete payment flow by selecting a plan, making a payment, and activating the license.

2. **Test Error Handling**: Test error scenarios such as invalid license codes, missing parameters, and server errors.

3. **Test User Creation**: Test the automatic user creation by using an email that doesn't exist in the system.

4. **Test Subscription Creation**: Verify that a subscription is created after successful license activation.

5. **Test Token Usage**: Verify that the JWT token returned after license activation can be used for subsequent API calls.

## Conclusion

This guide provides a comprehensive overview of how to integrate the new Mayar license payment flow into your frontend application. By following these steps, you can ensure a seamless payment experience for your users. 