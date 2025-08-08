# Quiz Platform Backend - Product Requirements Document (PRD)

## Executive Summary

The Quiz Platform Backend is a high-performance Rust-based API service built with Actix-Web framework that provides comprehensive quiz management, premium subscription features, payment processing, and license management capabilities. The system supports multi-tier premium access, OAuth authentication, and integrates with external payment gateways to deliver a scalable quiz platform solution.

## Product Overview

### Vision
To provide a robust, secure, and scalable backend infrastructure that enables online quiz platforms to deliver premium educational content with sophisticated access control, payment processing, and user management capabilities.

### Mission
Empower educational platforms with enterprise-grade backend services that handle complex business logic including premium subscriptions, payment processing, license verification, and multi-tenant quiz management while maintaining high performance and security standards.

### Target Users
- **Educational Technology Companies**: Organizations building quiz and assessment platforms
- **Content Creators**: Premium educational content providers
- **Platform Administrators**: Quiz platform managers and administrators
- **End Users**: Students and professionals taking quizzes through client applications

## Product Goals & Success Metrics

### Primary Goals
1. **High Performance**: Handle 10,000+ concurrent users with sub-100ms response times
2. **Premium Access Control**: Sophisticated subscription and license management
3. **Payment Integration**: Seamless payment processing with multiple gateways
4. **Security**: Enterprise-grade authentication and authorization
5. **Scalability**: Support for millions of quiz questions and user sessions

### Key Performance Indicators (KPIs)
- **API Response Time**: Average response time < 100ms for 95% of requests
- **System Uptime**: 99.9% availability
- **Payment Success Rate**: >98% successful payment processing
- **User Session Management**: Support for 100K+ active sessions
- **Database Performance**: Query execution time < 50ms for 99% of operations

### Success Metrics
- **Revenue Growth**: 25% increase in premium subscription conversions
- **User Engagement**: 40% increase in quiz completion rates
- **System Reliability**: Zero critical security vulnerabilities
- **Developer Experience**: <2 hours for new feature deployment

## Core Features & Functionality

### 1. Quiz Management System

#### Quiz Structure
- **Hierarchical Organization**: Categories → Packages → Questions
- **Dynamic Content Delivery**: Real-time question serving with metadata
- **Premium Access Control**: Per-quiz and per-package access restrictions
- **Content Versioning**: Support for quiz updates and modifications

#### Question Management
```rust
// Question model with comprehensive metadata support
struct Soal {
    id: i32,
    soal: String,           // Question text
    opt1-opt5: String,      // Multiple choice options
    correct_answer: i32,    // Correct answer index
    solution: Option<String>, // Detailed explanation
    // Metadata fields for advanced features
    difficulty: Option<String>,
    topic: Option<String>,
    estimated_time: Option<i32>,
}
```

#### API Endpoints
- **GET /soal/{id}**: Retrieve individual questions with access control
- **GET /paket-soal-response/{category}/{package}**: Package-based question delivery
- **POST /soal**: Admin question creation with validation
- **PUT /set-quiz-premium/{id}**: Dynamic premium status management

### 2. Authentication & Authorization System

#### Multi-layered Authentication
- **Supabase Integration**: Primary authentication provider
- **Google OAuth 2.0**: Social login with state management
- **Custom Session Management**: Database-backed session tokens
- **JWT Token Support**: Stateless authentication for API access

#### Session Management
```rust
// Comprehensive session tracking
struct SessionDao {
    id: String,
    user_id: String,
    token: String,
    expires_at: DateTime<Utc>,
    ip_address: Option<String>,
    user_agent: Option<String>,
}
```

#### Authorization Features
- **Role-based Access Control**: Admin, premium, and regular user roles
- **Route-level Protection**: Granular endpoint access control
- **Session Validation**: Real-time session verification
- **Token Refresh**: Automatic token renewal mechanism

### 3. Premium Subscription System

#### Subscription Plans
- **Flexible Plan Structure**: Multiple duration and feature combinations
- **Hierarchical Access**: Plan-based access control with inheritance
- **Lifetime Subscriptions**: Permanent access options
- **Dynamic Pricing**: Admin-configurable pricing models

#### Premium Plans Configuration
```rust
struct PremiumPlan {
    id: i32,
    name: String,
    description: String,
    price: Decimal,
    duration_days: Option<i32>, // None for lifetime
    is_lifetime: bool,
    features: Json,             // Flexible feature definition
    mayar_product_id: String,   // Payment gateway integration
}
```

#### Access Control Logic
- **Quiz-level Restrictions**: Individual quiz premium requirements
- **Package-level Access**: Bulk access control for quiz packages
- **User Subscription Validation**: Real-time access verification
- **Graceful Degradation**: Free content access for non-subscribers

### 4. Payment Processing System

#### Mayar Gateway Integration
- **Payment Link Generation**: Dynamic payment URL creation
- **Webhook Processing**: Real-time payment status updates
- **Transaction Tracking**: Complete payment lifecycle management
- **Multi-currency Support**: Flexible pricing for different markets

#### Payment Flow
1. **Payment Request**: User initiates payment for premium plan
2. **Link Generation**: System creates secure payment URL
3. **Payment Processing**: User completes payment on Mayar gateway
4. **Webhook Reception**: System receives payment confirmation
5. **Subscription Activation**: Automatic premium access granting
6. **Notification**: User receives confirmation and access details

#### Transaction Management
```rust
struct PaymentTransaction {
    id: String,
    user_id: String,
    plan_id: i32,
    amount: Decimal,
    status: PaymentStatus, // pending, completed, failed, expired
    transaction_id: Option<String>,
    payment_method: Option<String>,
    webhook_data: Option<Json>,
}
```

### 5. License Management System

#### License-based Access
- **External License Verification**: Integration with Mayar SaaS API
- **Automatic User Onboarding**: License-based account creation
- **Plan Mapping**: Product ID to subscription plan conversion
- **Usage Tracking**: License activation and usage monitoring

#### License Verification Flow
1. **License Input**: User provides license code and personal information
2. **External Validation**: API call to Mayar for license verification
3. **User Management**: Automatic user account creation or retrieval
4. **Subscription Creation**: Premium plan activation based on license
5. **Access Granting**: Immediate quiz access provision
6. **Data Persistence**: Complete license information storage

### 6. User Management System

#### User Profile Management
- **Multi-provider Authentication**: Support for multiple auth providers
- **Profile Information**: Comprehensive user data management
- **Phone Number Integration**: Required for payment processing
- **Soft Delete Support**: User account deactivation without data loss

#### User Data Structure
```rust
struct User {
    id: String,
    email: String,
    display_name: Option<String>,
    provider: String,           // local, google, etc.
    picture_url: Option<String>,
    phone_number: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    deleted_at: Option<DateTime<Utc>>,
}
```

## Technical Architecture

### Technology Stack
- **Framework**: Actix-Web 4.3.1
- **Language**: Rust 2021 Edition
- **Database**: MySQL with SQLx for async operations
- **Authentication**: Hybrid Supabase + OAuth + JWT
- **Payment**: Mayar Gateway Integration
- **Documentation**: OpenAPI/Swagger with utoipa
- **Configuration**: JSON-based environment configuration

### Database Design
- **Relational Model**: Normalized schema with foreign key constraints
- **Indexing Strategy**: Optimized for query performance
- **JSON Fields**: Flexible metadata storage for extensibility
- **Migration Support**: Versioned schema evolution
- **Connection Pooling**: High-performance database access

### API Architecture
- **RESTful Design**: Standard HTTP methods and status codes
- **Async Processing**: Non-blocking I/O for high performance
- **Error Handling**: Structured error responses with detailed information
- **Request Validation**: Comprehensive input validation
- **Rate Limiting**: Protection against abuse and overload

### Security Features
- **SQL Injection Protection**: Parameterized queries throughout
- **CSRF Protection**: State parameter validation for OAuth flows
- **Session Security**: Secure token generation and validation
- **Input Sanitization**: Comprehensive data validation
- **Audit Logging**: Request and transaction logging

## API Specification

### Authentication Endpoints
```
POST   /signup                    - User registration
POST   /auth/v1/token            - Email/password login
POST   /auth/google/callback     - Google OAuth callback
POST   /auth/logout              - Session termination
GET    /auth/sessions/{user_id}  - Session listing
POST   /auth/validate-session    - Token validation
```

### Quiz Management Endpoints
```
GET    /soal/{id}                        - Question retrieval
GET    /paket-soal-response/{cat}/{pkg}  - Package content
GET    /listpaketsoal                    - Package listing
GET    /kumpulan-soal                    - Question collection
POST   /soal                             - Question creation
PUT    /set-quiz-premium/{id}            - Premium status update
GET    /check-quiz-access/{id}           - Access verification
```

### Premium Feature Endpoints
```
GET    /premium/plans                    - Plan listing
GET    /premium/plans/{id}               - Plan details
POST   /premium/plans                    - Plan creation
GET    /premium/subscriptions            - User subscriptions
POST   /premium/subscriptions            - Subscription creation
GET    /premium/check-status             - Premium status check
GET    /premium/check-quiz-access/{id}   - Quiz access check
```

### Payment Processing Endpoints
```
GET    /payment/transactions             - Transaction history
POST   /payment/create                   - Payment initiation
POST   /payment/webhook                  - Payment webhook
GET    /payment/check/{id}               - Payment status
GET    /payment/check-phone/{user_id}    - Phone verification
```

### License Management Endpoints
```
GET    /license/payment-link/{plan_id}   - Payment link generation
GET    /license/user-licenses            - User license listing
POST   /license-public/verify            - License verification
POST   /license-public/verify-mock       - Mock verification (dev)
```

## Data Models

### Core Data Relationships
```
Users (1) → (*) Sessions
Users (1) → (*) UserSubscriptions → (*) PremiumPlans
Users (1) → (*) PaymentTransactions
Users (1) → (*) LicenseCodes
PremiumPlans (1) → (*) PremiumQuizAccess → (*) PaketSoal
KategoriSoal (1) → (*) PaketSoal (1) → (*) PaketSoalItems → (*) Soal
```

### Database Schema Highlights
- **Foreign Key Constraints**: Maintains referential integrity
- **Composite Indexes**: Optimized for common query patterns
- **JSON Columns**: Flexible metadata and configuration storage
- **Timestamp Tracking**: Audit trail for all critical operations
- **Enum Types**: Controlled vocabularies for status fields

## Configuration Management

### Environment Configuration
```json
{
  "database": {
    "url": "mysql://user:pass@host:port/database",
    "max_connections": 100,
    "timeout": 30
  },
  "auth": {
    "jwt_secret": "secure_secret_key",
    "supabase": {
      "url": "https://project.supabase.co",
      "anon_key": "public_anon_key"
    },
    "google_oauth": {
      "client_id": "oauth_client_id",
      "client_secret": "oauth_secret"
    }
  },
  "payment": {
    "mayar": {
      "api_url": "https://api.mayar.id",
      "api_key": "mayar_api_key"
    }
  },
  "server": {
    "host": "0.0.0.0",
    "port": 8080
  }
}
```

## Security & Compliance

### Security Measures
- **Data Encryption**: Sensitive data encryption at rest and in transit
- **Access Control**: Multi-layered authorization system
- **Session Management**: Secure session handling with expiration
- **Input Validation**: Comprehensive request payload validation
- **Audit Logging**: Complete request and transaction logging

### Compliance Considerations
- **Data Privacy**: User data protection and privacy controls
- **Payment Security**: PCI DSS compliant payment processing
- **Session Management**: Secure session handling practices
- **Data Retention**: Configurable data retention policies

## Performance & Scalability

### Performance Targets
- **Response Time**: <100ms for 95% of API requests
- **Throughput**: Support for 10,000+ concurrent connections
- **Database Performance**: <50ms query execution for 99% of operations
- **Memory Usage**: Efficient memory management with connection pooling

### Scalability Features
- **Horizontal Scaling**: Stateless design for easy horizontal scaling
- **Database Connection Pooling**: Efficient resource utilization
- **Async Processing**: Non-blocking I/O throughout the application
- **Caching Strategy**: Redis-ready architecture for caching layer

## Deployment & Infrastructure

### Deployment Architecture
- **Containerization**: Docker support with multi-stage builds
- **Load Balancing**: Support for multiple backend instances
- **Health Checks**: Built-in health monitoring endpoints
- **Graceful Shutdown**: Proper connection cleanup on termination

### Infrastructure Requirements
- **CPU**: Minimum 2 cores, recommended 4+ cores for production
- **Memory**: Minimum 4GB RAM, recommended 8GB+ for production
- **Storage**: SSD storage for database with regular backups
- **Network**: High-bandwidth internet connection for external APIs

## Monitoring & Observability

### Logging Strategy
- **Structured Logging**: JSON-formatted logs for easy parsing
- **Request Tracing**: Complete request lifecycle tracking
- **Error Tracking**: Detailed error logging with stack traces
- **Performance Metrics**: Response time and throughput monitoring

### Health Monitoring
- **Health Check Endpoints**: Built-in system health verification
- **Database Health**: Connection pool and query performance monitoring
- **External Service Health**: Payment gateway and auth provider monitoring
- **Resource Usage**: CPU, memory, and disk usage tracking

## Development & Maintenance

### Development Workflow
- **Code Quality**: Comprehensive testing with unit and integration tests
- **Documentation**: API documentation with OpenAPI/Swagger
- **Version Control**: Git-based version management
- **Continuous Integration**: Automated testing and deployment pipelines

### Maintenance Procedures
- **Database Migrations**: Versioned schema evolution
- **Backup Strategy**: Regular automated database backups
- **Security Updates**: Regular dependency and security updates
- **Performance Optimization**: Continuous performance monitoring and optimization

## Roadmap & Future Enhancements

### Phase 1 (Current): Core Platform
- ✅ Quiz management and delivery system
- ✅ Premium subscription management
- ✅ Payment processing integration
- ✅ License management system
- ✅ User authentication and authorization

### Phase 2 (Q2 2024): Advanced Features
- **Advanced Analytics**: User behavior and quiz performance analytics
- **Content Management**: Enhanced quiz creation and management tools
- **Multi-tenant Support**: Support for multiple quiz platform instances
- **Advanced Caching**: Redis integration for improved performance

### Phase 3 (Q3 2024): Enterprise Features
- **Role-based Administration**: Advanced user role management
- **Bulk Operations**: Batch quiz and user management
- **API Rate Limiting**: Advanced rate limiting and throttling
- **Audit Trail**: Comprehensive system audit logging

### Phase 4 (Q4 2024): Scaling & Optimization
- **Microservices Architecture**: Service decomposition for better scalability
- **Advanced Monitoring**: Comprehensive observability and alerting
- **Performance Optimization**: Database and query optimization
- **Mobile API Optimization**: Enhanced mobile application support

## Risk Assessment & Mitigation

### Technical Risks
- **Database Performance**: Regular optimization and scaling strategies
- **External Service Dependencies**: Fallback mechanisms for payment gateways
- **Security Vulnerabilities**: Regular security audits and updates
- **Scalability Bottlenecks**: Performance monitoring and optimization

### Business Risks
- **Payment Integration Changes**: Flexible payment gateway abstraction
- **Compliance Requirements**: Proactive compliance monitoring
- **Data Loss**: Comprehensive backup and disaster recovery procedures
- **Service Availability**: High availability architecture and monitoring

## Conclusion

The Quiz Platform Backend provides a comprehensive, secure, and scalable foundation for premium quiz applications. With its sophisticated subscription management, payment integration, and license management capabilities, it enables educational technology companies to build successful quiz platforms with advanced monetization features.

The system's modular architecture, comprehensive API coverage, and focus on security and performance make it an ideal choice for organizations looking to deploy enterprise-grade quiz platforms with premium subscription capabilities.

---

**Document Version**: 1.0  
**Last Updated**: August 8, 2025  
**Document Owner**: Backend Development Team  
**Review Cycle**: Quarterly