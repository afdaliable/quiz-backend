use utoipa::OpenApi;
use crate::model::{
    Soal, CreateSoalRequest, SignUpRequest, LoginRequest, 
    AuthResponse, SupabaseUser, PaketSoalResponse, ListPaketSoal, KategoriSoal
};

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::controller::soal_controller::get_soal,
        crate::controller::soal_controller::get_paket_soal_response,
        crate::controller::soal_controller::get_paket_soal_by_category,
        crate::controller::soal_controller::get_list_paket_soal,
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
            PaketSoalResponse, ListPaketSoal, KategoriSoal
        )
    ),
    tags(
        (name = "soal", description = "Soal management endpoints"),
        (name = "auth", description = "Authentication endpoints using Supabase")
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