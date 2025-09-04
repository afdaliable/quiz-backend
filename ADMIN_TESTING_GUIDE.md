# Quiz Platform Admin API Testing Guide

## Overview
This guide provides step-by-step instructions for testing the Quiz Platform Admin API using Postman. Follow these steps to set up and test all admin functionality.

## Table of Contents
- [Setup](#setup)
- [Authentication Setup](#authentication-setup)
- [Database Preparation](#database-preparation)
- [Testing Scenarios](#testing-scenarios)
- [Troubleshooting](#troubleshooting)

## Setup

### 1. Import Postman Files
1. Open Postman
2. Import the collection: `Quiz_Platform_Admin_API.postman_collection.json`
3. Import the environment: `Quiz_Platform_Admin.postman_environment.json`
4. Select the "Quiz Platform Admin Environment" in Postman

### 2. Start the Backend Server
```bash
cd /media/kpb8300d025/nvme5001/devarea/MONOREPO_QUIZ/quiz-backend-prod
cargo run --release
```

The server should start on `http://localhost:8080`

### 3. Verify Environment Variables
In Postman, check that these environment variables are set:
- `base_url`: `http://localhost:8080`
- `admin_email`: Your admin user email
- `admin_password`: Your admin user password

## Authentication Setup

### Step 1: Create Admin User (Database Setup Required)
Before testing, you need to create an admin user in the database. Run this SQL:

```sql
-- Create admin user
INSERT INTO dbquizapp.users (id, email, display_name, role, status, created_at)
VALUES ('admin123', 'admin@example.com', 'Admin User', 'admin', 'active', NOW());

-- Or update existing user to admin
UPDATE dbquizapp.users 
SET role = 'admin', status = 'active' 
WHERE email = 'your_admin_email@example.com';
```

### Step 2: Login and Get JWT Token
1. Open the "Authentication" folder in Postman
2. Select "Admin Login" request
3. Update the request body with your admin credentials:
   ```json
   {
     "email": "admin@example.com", 
     "password": "your_admin_password"
   }
   ```
4. Send the request
5. Copy the `token` from the response
6. Update the `admin_token` environment variable with this token

**Important**: The JWT token expires after a certain time. If you get 401 errors, repeat the login process.

## Database Preparation

### Required Database Schema Updates
Make sure your database has the required admin fields. Run these SQL statements:

```sql
-- Add admin fields to users table
ALTER TABLE dbquizapp.users 
ADD COLUMN IF NOT EXISTS role ENUM('user', 'admin', 'superadmin') DEFAULT 'user';

ALTER TABLE dbquizapp.users 
ADD COLUMN IF NOT EXISTS status ENUM('active', 'inactive', 'suspended') DEFAULT 'active';

ALTER TABLE dbquizapp.users 
ADD COLUMN IF NOT EXISTS last_login TIMESTAMP NULL;

-- Create some sample data for testing
INSERT INTO dbquizapp.kategori_soal (nama_kategori, deskripsi) VALUES 
('Mathematics', 'Mathematical questions and problems'),
('Science', 'Science-related questions'),
('History', 'Historical facts and events');

INSERT INTO dbquizapp.soal (soal, opt1, opt2, opt3, opt4, opt5, correct_answer, solution, modul, pelajaran, tag) VALUES
('What is 2 + 2?', '3', '4', '5', '6', '7', '4', 'Simple addition: 2 + 2 = 4', 'Basic Math', 'Addition', 'arithmetic'),
('What is the capital of Indonesia?', 'Jakarta', 'Bandung', 'Surabaya', 'Medan', 'Yogyakarta', 'Jakarta', 'Jakarta is the capital of Indonesia', 'Geography', 'Indonesian Geography', 'capitals');
```

## Testing Scenarios

### Scenario 1: Admin User Management

#### Test 1.1: Get All Users
1. Select "Admin Users" → "Get All Users"
2. Send the request
3. **Expected**: 200 OK with paginated user list
4. **Verify**: Response contains users array and pagination info

#### Test 1.2: Search Users
1. Select "Admin Users" → "Get All Users"
2. Add query parameter: `search=admin`
3. Send the request
4. **Expected**: Filtered results matching search term

#### Test 1.3: Get User Details
1. Copy a user ID from the previous response
2. Update the `user_id` environment variable
3. Select "Admin Users" → "Get User Details"
4. Send the request
5. **Expected**: 200 OK with detailed user information

#### Test 1.4: Update User Role
1. Select "Admin Users" → "Update User"
2. Modify request body:
   ```json
   {
     "role": "admin",
     "status": "active"
   }
   ```
3. Send the request
4. **Expected**: 200 OK with success message

#### Test 1.5: Get User Statistics
1. Select "Admin Users" → "Get User Statistics"
2. Send the request
3. **Expected**: 200 OK with aggregated user statistics

### Scenario 2: Category Management

#### Test 2.1: Get All Categories
1. Select "Admin Categories" → "Get All Categories"
2. Send the request
3. **Expected**: 200 OK with categories list

#### Test 2.2: Create New Category
1. Select "Admin Categories" → "Create Category"
2. Modify request body:
   ```json
   {
     "nama_kategori": "Programming",
     "deskripsi": "Programming and coding questions"
   }
   ```
3. Send the request
4. **Expected**: 201 Created with new category details

#### Test 2.3: Update Category
1. Copy category ID from previous response
2. Update `category_id` environment variable
3. Select "Admin Categories" → "Update Category"
4. Send the request
5. **Expected**: 200 OK with success message

### Scenario 3: Question Management

#### Test 3.1: Get All Questions
1. Select "Admin Questions" → "Get All Questions"
2. Send the request
3. **Expected**: 200 OK with questions list

#### Test 3.2: Create New Question
1. Select "Admin Questions" → "Create Question"
2. Send the request with the provided sample data
3. **Expected**: 201 Created with new question details

#### Test 3.3: Bulk Import Questions
1. Select "Admin Questions" → "Bulk Import Questions"
2. Send the request with multiple questions
3. **Expected**: 200 OK with import results

#### Test 3.4: Search Questions
1. Select "Admin Questions" → "Get All Questions"
2. Add query parameters: `search=capital&module=Geography`
3. Send the request
4. **Expected**: Filtered results matching criteria

### Scenario 4: Package Management

#### Test 4.1: Get All Packages
1. Select "Admin Packages" → "Get All Packages"
2. Send the request
3. **Expected**: 200 OK with packages list

#### Test 4.2: Create New Package
1. Select "Admin Packages" → "Create Package"
2. Send the request with sample data
3. **Expected**: 201 Created with new package details

#### Test 4.3: Add Questions to Package
1. Get a package ID and question IDs
2. Update environment variables
3. Select "Admin Packages" → "Add Questions to Package"
4. Send the request
5. **Expected**: 200 OK with success message

### Scenario 5: Analytics Dashboard

#### Test 5.1: Dashboard Statistics
1. Select "Admin Analytics" → "Dashboard Statistics"
2. Send the request
3. **Expected**: 200 OK with dashboard metrics

#### Test 5.2: User Analytics
1. Select "Admin Analytics" → "User Analytics"
2. Send the request
3. **Expected**: 200 OK with user analytics and growth data

#### Test 5.3: Revenue Analytics
1. Select "Admin Analytics" → "Revenue Analytics"
2. Send the request
3. **Expected**: 200 OK with revenue data and trends

## Advanced Testing Scenarios

### Authorization Testing

#### Test: Access Without Token
1. Remove the Authorization header from any admin request
2. Send the request
3. **Expected**: 401 Unauthorized

#### Test: Access With Invalid Token
1. Set Authorization header to: `Bearer invalid_token`
2. Send any admin request
3. **Expected**: 401 Unauthorized

#### Test: Access With Non-Admin User
1. Login with a regular user account
2. Use the user's token for admin requests
3. **Expected**: 403 Forbidden

### Error Handling Testing

#### Test: Invalid Data Validation
1. Send a create/update request with invalid data
2. **Expected**: 422 Validation Error with details

#### Test: Resource Not Found
1. Use a non-existent ID in any GET request
2. **Expected**: 404 Not Found

### Performance Testing

#### Test: Large Data Sets
1. Create many questions using bulk import
2. Test pagination with large data sets
3. Verify response times are acceptable

#### Test: Concurrent Requests
1. Send multiple requests simultaneously
2. Verify all requests complete successfully
3. Check for race conditions

## Troubleshooting

### Common Issues

#### 1. 401 Unauthorized Errors
**Cause**: JWT token expired or invalid
**Solution**: 
- Re-run the login request
- Copy new token to `admin_token` environment variable

#### 2. 403 Forbidden Errors
**Cause**: User doesn't have admin role
**Solution**:
- Check user role in database
- Update user role to 'admin' or 'superadmin'

#### 3. 500 Internal Server Error
**Cause**: Database connection issues or missing data
**Solution**:
- Check server logs
- Verify database connection
- Ensure required database schema exists

#### 4. Connection Refused
**Cause**: Backend server not running
**Solution**:
- Start the Rust backend server: `cargo run --release`
- Verify server is listening on port 8080

#### 5. Database Errors
**Cause**: Missing tables or columns
**Solution**:
- Run database migrations
- Add required admin columns to users table

### Debug Tips

1. **Check Server Logs**: Monitor the terminal where you started the server
2. **Verify Environment**: Ensure all environment variables are set correctly
3. **Test Basic Endpoints First**: Start with simple GET requests before testing complex operations
4. **Use Postman Console**: Check the Postman console for detailed request/response information
5. **Database Verification**: Use a database client to verify data changes

### Test Data Reset
To reset test data between test runs:

```sql
-- Reset users to default state
UPDATE dbquizapp.users SET role = 'user', status = 'active' WHERE role != 'admin';

-- Clear test categories (be careful with foreign key constraints)
DELETE FROM dbquizapp.kategori_soal WHERE nama_kategori IN ('Programming', 'Test Category');

-- Clear test questions
DELETE FROM dbquizapp.soal WHERE modul = 'Test Module';
```

## Automation Scripts

### Postman Test Scripts
Add these test scripts to your Postman requests for automated validation:

#### For Login Request:
```javascript
pm.test("Status code is 200", function () {
    pm.response.to.have.status(200);
});

pm.test("Token is present", function () {
    const responseJson = pm.response.json();
    pm.expect(responseJson.token).to.be.a('string');
    pm.environment.set("admin_token", responseJson.token);
});
```

#### For Get Requests:
```javascript
pm.test("Status code is 200", function () {
    pm.response.to.have.status(200);
});

pm.test("Response has data", function () {
    const responseJson = pm.response.json();
    pm.expect(responseJson).to.be.an('object');
});
```

#### For Create/Update Requests:
```javascript
pm.test("Status code is 200 or 201", function () {
    pm.expect(pm.response.code).to.be.oneOf([200, 201]);
});

pm.test("Success message present", function () {
    const responseJson = pm.response.json();
    pm.expect(responseJson.message).to.be.a('string');
});
```

This comprehensive testing guide should help you thoroughly test all admin functionality. Start with the basic scenarios and progressively test more complex features.