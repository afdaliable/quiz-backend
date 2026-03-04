use utoipa::OpenApi;
use crate::model::{
    Soal, LoginRequest,
    AuthResponse, SupabaseUser, PaketSoalResponse, ListPaketSoal, KategoriSoal, ListPaketSoalLengkap
};
use crate::model::paket_soal_items::{
    PaketSoalItem, PaketSoalItemRequest, PaketSoalItemWithDetails,
    MappingRequest, MappingResponse, AvailableSoal
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
        crate::controller::soal_controller::check_quiz_access,
        crate::controller::auth_controller::admin_login,
        crate::controller::auth_controller::google_callback,
        crate::controller::kategori_controller::get_semua_kategori,
        // Package Questions Mapping endpoints
        crate::controller::admin_paket_soal_items_controller::get_package_questions,
        crate::controller::admin_paket_soal_items_controller::get_available_questions,
        crate::controller::admin_paket_soal_items_controller::map_questions_to_package,
        crate::controller::admin_paket_soal_items_controller::unmap_questions_from_package,
        crate::controller::admin_paket_soal_items_controller::delete_mapping,
    ),
    components(
        schemas(
            Soal,
            LoginRequest, AuthResponse, SupabaseUser,
            PaketSoalResponse, ListPaketSoal, KategoriSoal, ListPaketSoalLengkap,
            // User phone number schemas
            CheckPhoneNumberRequest, CheckPhoneNumberResponse, 
            UpdatePhoneNumberRequest, UpdatePhoneNumberResponse,
            PhoneNumberCheckResponse,
            // Package Questions Mapping schemas
            PaketSoalItem, PaketSoalItemRequest, PaketSoalItemWithDetails,
            MappingRequest, MappingResponse, AvailableSoal
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
        (name = "user", description = "User profile management endpoints"),
        (name = "Package Questions Mapping", description = "Package and questions mapping management endpoints")
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