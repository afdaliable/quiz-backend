# Quiz Platform Admin API Documentation

## Overview
This document provides comprehensive documentation for the Quiz Platform Admin API endpoints. All admin endpoints require authentication with an admin role and are prefixed with `/admin`.

## Table of Contents
- [Authentication](#authentication)
- [Admin Users Management](#admin-users-management)
- [Admin Categories Management](#admin-categories-management)
- [Admin Questions Management](#admin-questions-management)
- [Admin Packages Management](#admin-packages-management)
- [Admin Analytics](#admin-analytics)
- [Error Responses](#error-responses)

## Authentication

### Admin Login
**POST** `/auth/login`

Login with admin credentials to get JWT token.

**Request Body:**
```json
{
  "email": "admin@example.com",
  "password": "admin123"
}
```

**Response:**
```json
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "user": {
    "id": "admin123",
    "email": "admin@example.com",
    "role": "admin"
  }
}
```

### Authentication Headers
All admin endpoints require the following header:
```
Authorization: Bearer <your_jwt_token>
```

## Admin Users Management

### Get All Users
**GET** `/admin/users`

Retrieve a paginated list of all users with optional filtering.

**Query Parameters:**
- `page` (optional): Page number (default: 1)
- `limit` (optional): Items per page (default: 20, max: 100)
- `search` (optional): Search by email or display name
- `role` (optional): Filter by user role (user, admin, superadmin)
- `status` (optional): Filter by user status (active, inactive, suspended)

**Response:**
```json
{
  "users": [
    {
      "id": "user123",
      "email": "user@example.com",
      "display_name": "John Doe",
      "picture_url": null,
      "phone_number": null,
      "role": "user",
      "status": "active",
      "last_login": "2024-01-01T10:00:00Z",
      "subscription_status": "active",
      "subscription_end_date": "2024-12-31T23:59:59Z",
      "created_at": "2024-01-01T00:00:00Z",
      "updated_at": "2024-01-01T00:00:00Z"
    }
  ],
  "pagination": {
    "page": 1,
    "limit": 20,
    "total": 100,
    "total_pages": 5
  }
}
```

### Get User Details
**GET** `/admin/users/{user_id}`

Get detailed information about a specific user.

**Response:**
```json
{
  "id": "user123",
  "email": "user@example.com",
  "display_name": "John Doe",
  "picture_url": null,
  "phone_number": null,
  "role": "user",
  "status": "active",
  "last_login": "2024-01-01T10:00:00Z",
  "created_at": "2024-01-01T00:00:00Z",
  "updated_at": "2024-01-01T00:00:00Z"
}
```

### Update User
**PUT** `/admin/users/{user_id}`

Update user information (role, status, etc.).

**Request Body:**
```json
{
  "role": "admin",
  "status": "active"
}
```

**Response:**
```json
{
  "message": "User updated successfully",
  "user": {
    "id": "user123",
    "email": "user@example.com",
    "role": "admin",
    "status": "active"
  }
}
```

### Delete User
**DELETE** `/admin/users/{user_id}`

Soft delete a user (sets deleted_at timestamp).

**Response:**
```json
{
  "message": "User deleted successfully"
}
```

### Get User Statistics
**GET** `/admin/users/stats`

Get aggregated user statistics.

**Response:**
```json
{
  "total_users": 1250,
  "active_users": 1180,
  "new_users_today": 15,
  "new_users_this_week": 87,
  "new_users_this_month": 342,
  "users_by_role": [
    {"role": "user", "count": 1200},
    {"role": "admin", "count": 48},
    {"role": "superadmin", "count": 2}
  ],
  "users_by_status": [
    {"status": "active", "count": 1180},
    {"status": "inactive", "count": 65},
    {"status": "suspended", "count": 5}
  ]
}
```

### Get User Subscriptions
**GET** `/admin/users/{user_id}/subscriptions`

Get subscription history for a specific user.

**Response:**
```json
[
  {
    "id": 1,
    "user_id": "user123",
    "plan_id": 1,
    "plan_name": "Premium Monthly",
    "price": 19.99,
    "duration_days": 30,
    "status": "active",
    "start_date": "2024-01-01T00:00:00Z",
    "end_date": "2024-01-31T23:59:59Z",
    "created_at": "2024-01-01T00:00:00Z",
    "updated_at": "2024-01-01T00:00:00Z"
  }
]
```

## Admin Categories Management

### Get All Categories
**GET** `/admin/categories`

Retrieve all quiz categories with statistics.

**Query Parameters:**
- `page` (optional): Page number (default: 1)
- `limit` (optional): Items per page (default: 20)

**Response:**
```json
{
  "categories": [
    {
      "id": 1,
      "nama_kategori": "Mathematics",
      "deskripsi": "Mathematical questions and problems",
      "packages_count": 15,
      "questions_count": 450,
      "created_at": "2024-01-01T00:00:00Z",
      "updated_at": "2024-01-01T00:00:00Z"
    }
  ],
  "pagination": {
    "page": 1,
    "limit": 20,
    "total": 10,
    "total_pages": 1
  }
}
```

### Get Category Details
**GET** `/admin/categories/{category_id}`

Get detailed information about a specific category.

**Response:**
```json
{
  "id": 1,
  "nama_kategori": "Mathematics",
  "deskripsi": "Mathematical questions and problems",
  "packages_count": 15,
  "questions_count": 450,
  "created_at": "2024-01-01T00:00:00Z",
  "updated_at": "2024-01-01T00:00:00Z"
}
```

### Create Category
**POST** `/admin/categories`

Create a new quiz category.

**Request Body:**
```json
{
  "nama_kategori": "Science",
  "deskripsi": "Science-related questions"
}
```

**Response:**
```json
{
  "message": "Category created successfully",
  "category": {
    "id": 2,
    "nama_kategori": "Science",
    "deskripsi": "Science-related questions",
    "created_at": "2024-01-01T00:00:00Z"
  }
}
```

### Update Category
**PUT** `/admin/categories/{category_id}`

Update an existing category.

**Request Body:**
```json
{
  "nama_kategori": "Advanced Mathematics",
  "deskripsi": "Advanced mathematical concepts and problems"
}
```

### Delete Category
**DELETE** `/admin/categories/{category_id}`

Delete a category (only if no packages are associated).

## Admin Questions Management

### Get All Questions
**GET** `/admin/questions`

Retrieve paginated list of questions with filtering options.

**Query Parameters:**
- `page` (optional): Page number
- `limit` (optional): Items per page
- `search` (optional): Search in question text
- `module` (optional): Filter by module
- `subject` (optional): Filter by subject/lesson
- `tag` (optional): Filter by tag

**Response:**
```json
{
  "questions": [
    {
      "id": 1,
      "soal": "What is 2 + 2?",
      "opt1": "3",
      "opt2": "4",
      "opt3": "5",
      "opt4": "6",
      "opt5": "7",
      "correct_answer": "4",
      "solution": "Simple addition: 2 + 2 = 4",
      "sumberfile": null,
      "modul": "Basic Math",
      "pelajaran": "Addition",
      "tag": "arithmetic",
      "created_at": "2024-01-01T00:00:00Z",
      "updated_at": "2024-01-01T00:00:00Z"
    }
  ],
  "pagination": {
    "page": 1,
    "limit": 20,
    "total": 1500,
    "total_pages": 75
  }
}
```

### Get Question Details
**GET** `/admin/questions/{question_id}`

Get detailed information about a specific question.

### Create Question
**POST** `/admin/questions`

Create a new question.

**Request Body:**
```json
{
  "soal": "What is the capital of Indonesia?",
  "opt1": "Jakarta",
  "opt2": "Bandung",
  "opt3": "Surabaya",
  "opt4": "Medan",
  "opt5": "Yogyakarta",
  "correct_answer": "Jakarta",
  "solution": "Jakarta is the capital and largest city of Indonesia.",
  "sumberfile": "geography_questions.pdf",
  "modul": "Geography",
  "pelajaran": "Indonesian Geography",
  "tag": "capitals"
}
```

### Update Question
**PUT** `/admin/questions/{question_id}`

Update an existing question.

### Delete Question
**DELETE** `/admin/questions/{question_id}`

Delete a question.

### Bulk Import Questions
**POST** `/admin/questions/bulk-import`

Import multiple questions at once.

**Request Body:**
```json
{
  "questions": [
    {
      "soal": "Question 1?",
      "opt1": "A",
      "opt2": "B",
      "opt3": "C",
      "opt4": "D",
      "opt5": "E",
      "correct_answer": "A",
      "solution": "Solution 1"
    },
    {
      "soal": "Question 2?",
      "opt1": "A",
      "opt2": "B",
      "opt3": "C",
      "opt4": "D",
      "opt5": "E",
      "correct_answer": "B",
      "solution": "Solution 2"
    }
  ]
}
```

**Response:**
```json
{
  "message": "Bulk import completed",
  "imported": 2,
  "failed": 0,
  "results": [
    {"id": 101, "status": "success"},
    {"id": 102, "status": "success"}
  ]
}
```

## Admin Packages Management

### Get All Packages
**GET** `/admin/packages`

Retrieve all quiz packages with statistics.

**Query Parameters:**
- `page` (optional): Page number
- `limit` (optional): Items per page
- `search` (optional): Search by package name
- `category_id` (optional): Filter by category
- `is_premium` (optional): Filter by premium status (true/false)

**Response:**
```json
{
  "packages": [
    {
      "id": 1,
      "nama_paket_soal": "Basic Math Quiz",
      "kategori_id": 1,
      "kategori_name": "Mathematics",
      "is_premium": false,
      "questions_count": 25,
      "created_at": "2024-01-01T00:00:00Z",
      "updated_at": "2024-01-01T00:00:00Z"
    }
  ],
  "pagination": {
    "page": 1,
    "limit": 20,
    "total": 50,
    "total_pages": 3
  }
}
```

### Get Package Details
**GET** `/admin/packages/{package_id}`

Get detailed information about a specific package.

### Create Package
**POST** `/admin/packages`

Create a new quiz package.

**Request Body:**
```json
{
  "nama_paket_soal": "Advanced Physics Quiz",
  "kategori_id": 2,
  "is_premium": true
}
```

### Update Package
**PUT** `/admin/packages/{package_id}`

Update an existing package.

### Delete Package
**DELETE** `/admin/packages/{package_id}`

Delete a package.

### Get Package Questions
**GET** `/admin/packages/{package_id}/questions`

Get all questions in a specific package.

### Add Questions to Package
**POST** `/admin/packages/{package_id}/questions`

Add questions to a package.

**Request Body:**
```json
{
  "question_ids": [1, 2, 3, 4, 5]
}
```

### Remove Questions from Package
**DELETE** `/admin/packages/{package_id}/questions/remove`

Remove questions from a package.

**Request Body:**
```json
{
  "question_ids": [1, 2, 3]
}
```

## Admin Analytics

### Dashboard Statistics
**GET** `/admin/analytics/dashboard`

Get high-level dashboard statistics.

**Response:**
```json
{
  "total_users": 1250,
  "active_users": 1180,
  "total_questions": 1500,
  "total_packages": 50,
  "total_categories": 10,
  "active_sessions": 25,
  "premium_subscriptions": 180,
  "revenue_this_month": 3500.00
}
```

### User Analytics
**GET** `/admin/analytics/users`

Get detailed user analytics and growth data.

**Response:**
```json
{
  "total_users": 1250,
  "new_users_today": 15,
  "new_users_this_week": 87,
  "new_users_this_month": 342,
  "active_users_today": 124,
  "users_by_provider": [
    {"provider": "email", "count": 800},
    {"provider": "google", "count": 350},
    {"provider": "facebook", "count": 100}
  ],
  "users_growth_chart": [
    {"date": "2024-01-01", "count": 5},
    {"date": "2024-01-02", "count": 8},
    {"date": "2024-01-03", "count": 12}
  ]
}
```

### Quiz Session Analytics
**GET** `/admin/analytics/quiz-sessions`

Get analytics about quiz sessions and completion rates.

**Response:**
```json
{
  "total_sessions": 5420,
  "completed_sessions": 4250,
  "active_sessions": 25,
  "average_completion_rate": 78.4,
  "sessions_today": 45,
  "sessions_this_week": 320,
  "sessions_this_month": 1250,
  "popular_packages": [
    {
      "package_name": "Basic Math Quiz",
      "category_name": "Mathematics",
      "session_count": 245
    }
  ],
  "completion_rate_by_package": [
    {
      "package_name": "Basic Math Quiz",
      "total_sessions": 245,
      "completed_sessions": 201,
      "completion_rate": 82.0
    }
  ]
}
```

### Revenue Analytics
**GET** `/admin/analytics/revenue`

Get revenue and subscription analytics.

**Response:**
```json
{
  "total_revenue": 45600.00,
  "revenue_this_month": 3500.00,
  "revenue_last_month": 3200.00,
  "growth_percentage": 9.375,
  "active_subscriptions": 180,
  "expired_subscriptions": 45,
  "revenue_by_plan": [
    {
      "plan_name": "Premium Monthly",
      "active_subscriptions": 120,
      "total_revenue": 2400.00
    }
  ],
  "monthly_revenue_chart": [
    {"month": "2024-01", "revenue": 3500.00},
    {"month": "2024-02", "revenue": 3800.00}
  ]
}
```

## Error Responses

All endpoints may return the following error responses:

### 401 Unauthorized
```json
{
  "error": "unauthenticated",
  "message": "Authentication required for admin access.",
  "status_code": 401
}
```

### 403 Forbidden
```json
{
  "error": "insufficient_permissions",
  "message": "You don't have admin permissions to access this resource.",
  "status_code": 403
}
```

### 404 Not Found
```json
{
  "error": "not_found",
  "message": "Resource not found",
  "status_code": 404
}
```

### 422 Validation Error
```json
{
  "error": "validation_failed",
  "message": "Validation failed",
  "details": {
    "field_name": ["Field is required"]
  },
  "status_code": 422
}
```

### 500 Internal Server Error
```json
{
  "error": "internal_server_error",
  "message": "An unexpected error occurred",
  "status_code": 500
}
```

## Rate Limiting
Admin endpoints may be rate limited. If you exceed the rate limit, you'll receive:

```json
{
  "error": "rate_limit_exceeded",
  "message": "Too many requests. Please try again later.",
  "status_code": 429
}
```

## Notes
1. All timestamps are in UTC ISO 8601 format
2. Pagination is 1-indexed (first page is 1, not 0)
3. Boolean values are represented as `true`/`false`
4. Numeric IDs are integers
5. All admin endpoints require the `Authorization: Bearer <token>` header
6. User must have `admin` or `superadmin` role to access admin endpoints