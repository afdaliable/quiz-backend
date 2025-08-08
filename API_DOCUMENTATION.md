# API Documentation Summary

## Quiz Platform REST API

### Table of Contents
1. [Overview](#overview)
2. [Base Configuration](#base-configuration)
3. [Authentication](#authentication)
4. [Core API Endpoints](#core-api-endpoints)
5. [Data Models](#data-models)
6. [Error Handling](#error-handling)
7. [Integration Guidelines](#integration-guidelines)

---

## Overview

The Quiz Platform API provides a comprehensive REST interface for managing quizzes, users, categories, premium subscriptions, and payment processing. Built with Rust Actix-Web framework, it offers high-performance, type-safe endpoints with automatic OpenAPI documentation.

### API Characteristics
- **Protocol**: HTTP/HTTPS REST API
- **Data Format**: JSON
- **Authentication**: JWT Bearer tokens + Session-based
- **Documentation**: Auto-generated OpenAPI 3.0 spec
- **CORS**: Configurable cross-origin support
- **Rate Limiting**: Connection-based throttling

---

## Base Configuration

### API Base URLs
- **Development**: `http://localhost:8787/api`
- **Production**: `https://your-domain.com/api`
- **Swagger Documentation**: `/swagger-ui/`
- **OpenAPI Spec**: `/api-docs/openapi.json`

### HTTP Headers
```http
Content-Type: application/json
Accept: application/json
Authorization: Bearer <jwt_token>
```

### CORS Configuration
```rust
// Allowed origins, methods, and headers
.allow_any_origin()
.allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
.allowed_headers(vec![
    header::AUTHORIZATION,
    header::CONTENT_TYPE,
    header::ACCEPT,
    header::ORIGIN
])
.supports_credentials()
```

---

## Authentication

### Authentication Flow
1. **Google OAuth**: Users authenticate via Google OAuth 2.0
2. **Token Exchange**: Authorization code exchanged for JWT tokens
3. **Session Creation**: Server creates database-backed session
4. **API Access**: JWT token used for subsequent API calls

### Auth Endpoints

#### POST `/auth/google/callback`
Exchange Google OAuth authorization code for access tokens.

**Request Body:**
```json
{
  "code": "google_authorization_code"
}
```

**Response:**
```json
{
  "access_token": "jwt_token_here",
  "token_type": "Bearer",
  "expires_in": 3600,
  "refresh_token": "refresh_token_here",
  "user": {
    "id": "uuid",
    "email": "user@example.com",
    "display_name": "User Name",
    "picture": "https://profile-picture-url"
  }
}
```

#### POST `/auth/validate-session`
Validate current user session.

**Request Body:**
```json
{
  "token": "refresh_token_here"
}
```

**Response:**
```json
{
  "valid": true,
  "message": "Session is valid",
  "user_id": "uuid"
}
```

#### GET `/auth/sessions/{user_id}`
Get all active sessions for a user.

**Response:**
```json
[
  {
    "id": "session_id",
    "userId": "user_uuid",
    "createdAt": "2025-08-08T10:00:00Z",
    "lastActive": "2025-08-08T11:00:00Z",
    "userAgent": "browser_info",
    "ipAddress": "192.168.1.1",
    "isCurrentSession": true
  }
]
```

#### POST `/auth/logout`
Invalidate user session and logout.

**Request Body:**
```json
{
  "token": "refresh_token_here"
}
```

---

## Core API Endpoints

### Quiz Management

#### GET `/soal/{id}`
Get a specific quiz question by ID.

**Parameters:**
- `id` (path): Question identifier

**Response:**
```json
{
  "id": 1,
  "soal": "What is the capital of France?",
  "opt1": "London",
  "opt2": "Berlin", 
  "opt3": "Paris",
  "opt4": "Madrid",
  "opt5": "Rome",
  "correct_answer": "opt3",
  "solution": "Paris is the capital of France",
  "sumberfile": "https://source-file-url",
  "modul": "Geography",
  "pelajaran": "World Capitals"
}
```

#### GET `/paket-soal`
Get all quiz packages.

**Response:**
```json
[
  {
    "id": 1,
    "nama_paket_soal": "UTBK 2024 Package",
    "kategori_id": 1,
    "is_premium": false,
    "kategori_nama": "UTBK",
    "jumlah_soal": 50
  }
]
```

#### GET `/paket-soal/category/{kategori_id}`
Get quiz packages by category.

**Parameters:**
- `kategori_id` (path): Category identifier

#### POST `/soal/create`
Create a new quiz question.

**Request Body:**
```json
{
  "soal": "Question text",
  "opt1": "Option 1",
  "opt2": "Option 2",
  "opt3": "Option 3", 
  "opt4": "Option 4",
  "opt5": "Option 5",
  "correct_answer": "opt3",
  "solution": "Solution explanation",
  "sumberfile": "https://source-url",
  "modul": "Module name",
  "pelajaran": "Subject name",
  "tag": "comma,separated,tags"
}
```

### Category Management

#### GET `/kategori`
Get all quiz categories.

**Response:**
```json
[
  {
    "id": 1,
    "nama_kategori": "UTBK"
  },
  {
    "id": 2, 
    "nama_kategori": "CPNS"
  }
]
```

### Premium Subscription Management

#### GET `/premium/plans`
Get all available premium plans.

**Response:**
```json
[
  {
    "id": 1,
    "name": "Silver Plan",
    "description": "Basic premium features",
    "price": 99000.0,
    "duration_days": 30,
    "is_lifetime": false,
    "features": ["Access to premium quizzes", "Priority support"],
    "mayar_product_id": "product_id",
    "mayar_link_payment": "https://payment-link"
  }
]
```

#### GET `/premium/subscriptions/active`
Get user's active subscription.

**Headers Required:** `Authorization: Bearer <token>`

**Response:**
```json
{
  "id": 1,
  "user_id": "user_uuid",
  "plan_id": 1,
  "start_date": "2025-08-01T00:00:00Z",
  "end_date": "2025-08-31T23:59:59Z",
  "status": "active",
  "plan_name": "Silver Plan",
  "plan_features": ["feature1", "feature2"]
}
```

#### GET `/premium/check-quiz-access/{paket_soal_id}`
Check if user has access to a premium quiz.

**Parameters:**
- `paket_soal_id` (path): Quiz package identifier

**Headers Required:** `Authorization: Bearer <token>`

**Response:**
```json
{
  "has_access": true,
  "message": "User has active premium subscription",
  "subscription_end_date": "2025-08-31T23:59:59Z"
}
```

### Payment Management

#### POST `/payment/create`
Create a new payment transaction.

**Request Body:**
```json
{
  "plan_id": 1,
  "amount": 99000.0
}
```

**Response:**
```json
{
  "transaction_id": "txn_uuid",
  "payment_link": "https://payment-gateway-link",
  "amount": 99000.0,
  "status": "pending"
}
```

### License Management

#### POST `/license/activate`
Activate a license code.

**Request Body:**
```json
{
  "license_code": "ABC123DEF456"
}
```

**Response:**
```json
{
  "success": true,
  "message": "License activated successfully",
  "subscription": {
    "plan_id": 1,
    "end_date": "2025-09-01T00:00:00Z"
  }
}
```

#### GET `/license/validate/{license_code}`
Validate a license code with Mayar.

**Parameters:**
- `license_code` (path): License code to validate

### User Management

#### GET `/user/profile`
Get current user profile.

**Headers Required:** `Authorization: Bearer <token>`

**Response:**
```json
{
  "id": "user_uuid",
  "email": "user@example.com", 
  "display_name": "User Name",
  "picture_url": "https://profile-picture",
  "phone_number": "+1234567890",
  "created_at": "2025-08-01T00:00:00Z"
}
```

#### PUT `/user/profile`
Update user profile.

**Request Body:**
```json
{
  "display_name": "New Display Name",
  "phone_number": "+1234567890"
}
```

---

## Data Models

### Core Entities

#### User
```rust
pub struct User {
    pub id: String,              // UUID
    pub email: String,
    pub display_name: String,
    pub provider: String,        // 'local' or 'google'
    pub picture_url: Option<String>,
    pub phone_number: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>
}
```

#### Quiz Question (Soal)
```rust
pub struct Soal {
    pub id: i32,
    pub soal: String,            // Question text
    pub opt1: Option<String>,    // Option 1
    pub opt2: Option<String>,    // Option 2  
    pub opt3: Option<String>,    // Option 3
    pub opt4: Option<String>,    // Option 4
    pub opt5: Option<String>,    // Option 5
    pub correct_answer: Option<String>, // 'opt1', 'opt2', etc.
    pub solution: Option<String>, // Solution explanation
    pub sumberfile: Option<String>, // Source file URL
    pub modul: Option<String>,   // Module name
    pub pelajaran: Option<String>, // Subject name
    pub tag: Option<String>      // Comma-separated tags
}
```

#### Quiz Package (Paket Soal)
```rust
pub struct PaketSoal {
    pub id: i32,
    pub nama_paket_soal: String,
    pub kategori_id: Option<i32>,
    pub is_premium: bool
}
```

#### Premium Plan
```rust
pub struct PremiumPlan {
    pub id: i32,
    pub name: String,
    pub description: String,
    pub price: f64,
    pub duration_days: i32,      // 0 for lifetime
    pub is_lifetime: bool,
    pub features: String,        // JSON array
    pub mayar_product_id: Option<String>,
    pub mayar_link_payment: Option<String>
}
```

#### User Subscription
```rust
pub struct UserSubscription {
    pub id: i32,
    pub user_id: String,         // UUID
    pub plan_id: i32,
    pub start_date: DateTime<Utc>,
    pub end_date: Option<DateTime<Utc>>, // NULL for lifetime
    pub status: String           // 'active', 'expired', 'cancelled'
}
```

#### License Code
```rust
pub struct LicenseCode {
    pub id: i32,
    pub license_code: String,
    pub user_id: Option<String>, // UUID
    pub plan_id: i32,
    pub status: String,          // 'active', 'expired', 'cancelled'
    pub transaction_id: Option<String>,
    pub product_id: String,
    pub customer_id: Option<String>,
    pub expired_at: Option<DateTime<Utc>>,
    pub license_data: Option<String> // Full JSON from Mayar
}
```

---

## Error Handling

### Standard Error Response
```json
{
  "error": "Error message description",
  "code": "ERROR_CODE",
  "details": {
    "field": "additional error details"
  }
}
```

### HTTP Status Codes
- `200` - Success
- `201` - Created
- `400` - Bad Request (validation errors)
- `401` - Unauthorized (missing or invalid token)
- `403` - Forbidden (insufficient permissions)
- `404` - Not Found
- `409` - Conflict (duplicate data)
- `429` - Too Many Requests (rate limited)
- `500` - Internal Server Error

### Common Error Scenarios

#### Authentication Errors
```json
{
  "error": "Invalid or expired token",
  "code": "AUTH_TOKEN_INVALID"
}
```

#### Validation Errors
```json
{
  "error": "Validation failed",
  "code": "VALIDATION_ERROR",
  "details": {
    "email": "Invalid email format",
    "display_name": "Display name is required"
  }
}
```

#### Premium Access Errors
```json
{
  "error": "Premium subscription required",
  "code": "PREMIUM_REQUIRED",
  "details": {
    "paket_soal_id": 3,
    "required_plan": "Silver Plan"
  }
}
```

---

## Integration Guidelines

### Frontend Integration Example
```typescript
// Angular service integration
export class QuizApiService {
  private baseUrl = environment.apiUrl;
  
  getQuizPackages(): Observable<PaketSoal[]> {
    return this.http.get<PaketSoal[]>(`${this.baseUrl}/paket-soal`);
  }
  
  checkPremiumAccess(paketId: number): Observable<QuizAccessResponse> {
    return this.http.get<QuizAccessResponse>(
      `${this.baseUrl}/premium/check-quiz-access/${paketId}`
    );
  }
}
```

### Authentication Headers
```typescript
// HTTP Interceptor for automatic token attachment
intercept(req: HttpRequest<any>, next: HttpHandler): Observable<HttpEvent<any>> {
  const token = this.authService.getToken();
  if (token) {
    req = req.clone({
      setHeaders: {
        Authorization: `Bearer ${token}`
      }
    });
  }
  return next.handle(req);
}
```

### Rate Limiting Best Practices
- Implement exponential backoff for retry logic
- Cache frequently accessed data (categories, plans)
- Use WebSocket connections for real-time features
- Batch API calls where possible

### Security Considerations
- Always validate JWT tokens server-side
- Implement CSRF protection for state-changing operations  
- Use HTTPS in production environments
- Sanitize all user inputs before database operations
- Log security-relevant events for monitoring

---

This API documentation provides comprehensive coverage of all available endpoints and integration patterns for the Quiz Platform.