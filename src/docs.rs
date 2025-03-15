use utoipa::OpenApi;
use crate::model::{
    Soal, CreateSoalRequest, SignUpRequest, LoginRequest, 
    AuthResponse, SupabaseUser, PaketSoalResponse, ListPaketSoal, KategoriSoal,ListPaketSoalLengkap
};
// Premium models
use crate::model::premium_plan::{PremiumPlan, PremiumPlanResponse, CreatePremiumPlanRequest, UpdatePremiumPlanRequest};
use crate::model::user_subscription::{UserSubscription, UserSubscriptionResponse, CreateUserSubscriptionRequest, UpdateUserSubscriptionRequest};
use crate::model::premium_quiz_access::{PremiumQuizAccess, PremiumQuizAccessResponse, CreatePremiumQuizAccessRequest, UpdatePremiumQuizAccessRequest, QuizAccessCheckResponse};
use crate::model::payment_transaction::{PaymentTransaction, PaymentTransactionResponse, CreatePaymentRequest, PaymentStatus, MayarWebhookPayload};
use crate::model::users::{CheckPhoneNumberRequest, CheckPhoneNumberResponse, UpdatePhoneNumberRequest, UpdatePhoneNumberResponse};
use crate::controller::payment_controller::PhoneNumberCheckResponse;

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::controller::soal_controller::get_soal,
        crate::controller::soal_controller::get_paket_soal_response,
        crate::controller::soal_controller::get_paket_soal_by_category,
        crate::controller::soal_controller::get_list_paket_soal,
        crate::controller::soal_controller::get_list_paket_soal_lengkap,
        crate::controller::soal_controller::get_all_soal,
        crate::controller::soal_controller::create_soal,
        crate::controller::auth_controller::signup,
        crate::controller::auth_controller::login,
        crate::controller::kategori_controller::get_semua_kategori,
    ),
    components(
        schemas(
            Soal, CreateSoalRequest, 
            SignUpRequest, LoginRequest, AuthResponse, SupabaseUser,
            PaketSoalResponse, ListPaketSoal, KategoriSoal, ListPaketSoalLengkap,
            // User phone number schemas
            CheckPhoneNumberRequest, CheckPhoneNumberResponse, 
            UpdatePhoneNumberRequest, UpdatePhoneNumberResponse,
            PhoneNumberCheckResponse
            // Premium schemas are commented out because they don't implement ToSchema
            // PremiumPlan, PremiumPlanResponse, CreatePremiumPlanRequest, UpdatePremiumPlanRequest,
            // UserSubscription, UserSubscriptionResponse, CreateUserSubscriptionRequest, UpdateUserSubscriptionRequest,
            // PremiumQuizAccess, PremiumQuizAccessResponse, CreatePremiumQuizAccessRequest, UpdatePremiumQuizAccessRequest,
            // QuizAccessCheckResponse,
            // Payment schemas
            // PaymentTransaction, PaymentTransactionResponse, CreatePaymentRequest, PaymentStatus, MayarWebhookPayload
        )
    ),
    tags(
        (name = "soal", description = "Soal management endpoints"),
        (name = "auth", description = "Authentication endpoints using Supabase"),
        (name = "premium", description = "Premium subscription management endpoints"),
        (name = "payment", description = "Payment processing endpoints"),
        (name = "user", description = "User profile management endpoints")
    ),
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;

struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        use utoipa::openapi::security::{SecurityScheme, HttpBuilder, HttpAuthScheme};
        
        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            "bearer_auth",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .build()
            ),
        );
    }
}