# Frontend Integration Guide: Phone Number Verification for Payments

This guide explains how to integrate the phone number verification flow into your frontend application before proceeding with payments.

## Overview

When a user attempts to make a payment, the system will first check if they have a phone number registered in their profile. If not, they will be prompted to provide one before proceeding with the payment.

## API Endpoints

The backend provides the following endpoints for this flow:

1. **Check Phone Number Before Payment**
   - `GET /payment/check-phone/{user_id}`
   - Checks if a user has a phone number registered
   - Returns whether the user can proceed with payment

2. **Check User Phone Number**
   - `POST /user/check-phone`
   - Checks if a user has a phone number in their profile
   - Request body: `{ "user_id": "user-id-here" }`

3. **Update User Phone Number**
   - `POST /user/update-phone`
   - Updates a user's phone number in their profile
   - Request body: `{ "user_id": "user-id-here", "phone_number": "1234567890" }`

4. **Create Payment**
   - `POST /payment/create`
   - Creates a payment transaction
   - Request body: `{ "user_id": "user-id-here", "plan_id": 1, "redirect_url": "https://your-app.com/payment-callback" }`

## Implementation Steps

### 1. When User Clicks "Buy Plan" Button

```javascript
async function handleBuyPlan(planId) {
  try {
    // Get the user ID from your authentication state
    const userId = getCurrentUserId();
    
    // Check if the user has a phone number
    const response = await fetch(`/payment/check-phone/${userId}`, {
      method: 'GET',
      headers: {
        'Authorization': `Bearer ${getAuthToken()}`,
        'Content-Type': 'application/json'
      }
    });
    
    const data = await response.json();
    
    if (data.can_proceed) {
      // User has a phone number, proceed with payment
      createPayment(userId, planId);
    } else {
      // User doesn't have a phone number, show phone number input form
      showPhoneNumberForm(userId, planId);
    }
  } catch (error) {
    console.error('Error checking phone number:', error);
    showErrorMessage('Failed to check phone number. Please try again.');
  }
}
```

### 2. Show Phone Number Input Form

```javascript
function showPhoneNumberForm(userId, planId) {
  // Display a modal or form for the user to enter their phone number
  // Example using a simple modal:
  const phoneNumberModal = document.getElementById('phone-number-modal');
  const phoneNumberInput = document.getElementById('phone-number-input');
  const submitButton = document.getElementById('submit-phone-number');
  
  // Show the modal
  phoneNumberModal.style.display = 'block';
  
  // Handle form submission
  submitButton.onclick = async () => {
    const phoneNumber = phoneNumberInput.value.trim();
    
    // Validate phone number (basic validation)
    if (phoneNumber.length < 10 || phoneNumber.length > 15) {
      showErrorMessage('Please enter a valid phone number (10-15 digits)');
      return;
    }
    
    // Update the user's phone number
    await updatePhoneNumber(userId, phoneNumber);
    
    // Hide the modal
    phoneNumberModal.style.display = 'none';
    
    // Proceed with payment
    createPayment(userId, planId);
  };
}
```

### 3. Update Phone Number

```javascript
async function updatePhoneNumber(userId, phoneNumber) {
  try {
    const response = await fetch('/user/update-phone', {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${getAuthToken()}`,
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({
        user_id: userId,
        phone_number: phoneNumber
      })
    });
    
    const data = await response.json();
    
    if (!data.success) {
      throw new Error(data.message || 'Failed to update phone number');
    }
    
    // Show success message
    showSuccessMessage('Phone number updated successfully');
    
    return data;
  } catch (error) {
    console.error('Error updating phone number:', error);
    showErrorMessage('Failed to update phone number. Please try again.');
    throw error;
  }
}
```

### 4. Create Payment

```javascript
async function createPayment(userId, planId) {
  try {
    const response = await fetch('/payment/create', {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${getAuthToken()}`,
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({
        user_id: userId,
        plan_id: planId,
        redirect_url: `${window.location.origin}/payment-callback`
      })
    });
    
    const data = await response.json();
    
    if (data.payment_link) {
      // Redirect to the payment page
      window.location.href = data.payment_link;
    } else {
      throw new Error('No payment link received');
    }
  } catch (error) {
    console.error('Error creating payment:', error);
    showErrorMessage('Failed to create payment. Please try again.');
  }
}
```

## HTML Example

Here's a simple HTML example for the phone number input modal:

```html
<!-- Phone Number Modal -->
<div id="phone-number-modal" class="modal">
  <div class="modal-content">
    <h2>Phone Number Required</h2>
    <p>Please enter your phone number to proceed with the payment.</p>
    
    <div class="form-group">
      <label for="phone-number-input">Phone Number:</label>
      <input type="tel" id="phone-number-input" placeholder="e.g., 081234567890" />
      <small>Please enter a valid phone number (10-15 digits)</small>
    </div>
    
    <div class="button-group">
      <button id="submit-phone-number" class="primary-button">Submit</button>
      <button id="cancel-phone-number" class="secondary-button">Cancel</button>
    </div>
  </div>
</div>
```

## CSS Example

```css
.modal {
  display: none;
  position: fixed;
  z-index: 1000;
  left: 0;
  top: 0;
  width: 100%;
  height: 100%;
  background-color: rgba(0, 0, 0, 0.5);
}

.modal-content {
  background-color: white;
  margin: 15% auto;
  padding: 20px;
  border-radius: 8px;
  width: 80%;
  max-width: 500px;
  box-shadow: 0 4px 8px rgba(0, 0, 0, 0.1);
}

.form-group {
  margin-bottom: 20px;
}

.form-group label {
  display: block;
  margin-bottom: 5px;
  font-weight: bold;
}

.form-group input {
  width: 100%;
  padding: 10px;
  border: 1px solid #ccc;
  border-radius: 4px;
  font-size: 16px;
}

.form-group small {
  display: block;
  margin-top: 5px;
  color: #666;
}

.button-group {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
}

.primary-button {
  background-color: #4CAF50;
  color: white;
  border: none;
  padding: 10px 20px;
  border-radius: 4px;
  cursor: pointer;
}

.secondary-button {
  background-color: #f1f1f1;
  color: #333;
  border: 1px solid #ccc;
  padding: 10px 20px;
  border-radius: 4px;
  cursor: pointer;
}
```

## Error Handling

Make sure to implement proper error handling in your frontend application:

1. Handle network errors when making API requests
2. Display appropriate error messages to the user
3. Provide a way for users to retry failed operations
4. Validate phone number input before submitting

## Testing

Test the following scenarios:

1. User with a phone number already registered
2. User without a phone number
3. Invalid phone number input
4. Network errors during API calls
5. Successful payment flow after phone number update

## Conclusion

By implementing this flow, you ensure that all users have a phone number registered before proceeding with payments, which is required by the payment gateway. This improves the user experience by preventing payment failures due to missing information. 