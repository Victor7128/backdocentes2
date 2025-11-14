use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// ============================================
// ENUMS
// ============================================

/// ENUM para roles de usuario (debe coincidir con PostgreSQL)
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "user_role", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UserRole {
    Docente,
    Apoderado,
    Alumno,
    Admin,
}

// Implementar Display para facilitar conversión a String
impl std::fmt::Display for UserRole {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            UserRole::Docente => write!(f, "DOCENTE"),
            UserRole::Apoderado => write!(f, "APODERADO"),
            UserRole::Alumno => write!(f, "ALUMNO"),
            UserRole::Admin => write!(f, "ADMIN"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "account_status", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UserStatus {
    Active,
    Inactive,
    Suspended,
    Pending,
}

impl std::fmt::Display for UserStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            UserStatus::Active => write!(f, "ACTIVE"),
            UserStatus::Inactive => write!(f, "INACTIVE"),
            UserStatus::Suspended => write!(f, "SUSPENDED"),
            UserStatus::Pending => write!(f, "PENDING"),
        }
    }
}

// ============================================
// MODELOS DE AUTENTICACIÓN
// ============================================

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: i32,
    pub firebase_uid: String,
    pub email: String,
    pub role: UserRole,     // ✅ Ahora usa el ENUM
    pub status: UserStatus, // ✅ También el status
    pub created_at: Option<chrono::NaiveDateTime>,
    pub updated_at: Option<chrono::NaiveDateTime>,
    pub last_login: Option<chrono::NaiveDateTime>,
    pub profile_photo_url: Option<String>,
    pub phone: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct TeacherProfile {
    pub user_id: i32,
    pub area_id: Option<i32>,
    pub full_name: String,
    pub specialization: Option<String>,
    pub hire_date: Option<chrono::NaiveDate>,
    pub employee_code: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct StudentProfile {
    pub user_id: i32,
    pub dni: String,
    pub full_name: String,
    pub date_of_birth: Option<chrono::NaiveDate>,
    pub gender: Option<String>,
    pub address: Option<String>,
    pub enrollment_code: Option<String>,
    pub enrollment_date: Option<chrono::NaiveDate>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct GuardianProfile {
    pub user_id: i32,
    pub full_name: String,
    pub dni: Option<String>,
    pub relationship_type: Option<String>,
    pub occupation: Option<String>,
    pub workplace: Option<String>,
    pub emergency_phone: Option<String>,
}

// ============================================
// DTOs para requests
// ============================================

#[derive(Debug, Deserialize)]
pub struct RegisterAlumnoRequest {
    pub dni: String,
    pub full_name: String,
    pub email: String,
    pub firebase_uid: String,
    #[allow(dead_code)]
    pub nombres: Option<String>,
    #[allow(dead_code)]
    pub apellido_paterno: Option<String>,
    #[allow(dead_code)]
    pub apellido_materno: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RegisterApoderadoRequest {
    pub dni: String,
    pub full_name: String,
    pub phone: String,
    pub relationship_type: String,
    pub email: String,
    pub firebase_uid: String,
    pub occupation: Option<String>,
    pub workplace: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RegisterDocenteRequest {
    pub dni: String,
    pub full_name: String,
    pub area_name: String,
    pub area_id: Option<i32>,
    pub email: String,
    pub firebase_uid: String,
    pub employee_code: Option<String>,
    pub specialization: Option<String>,
}

// ============================================
// Responses
// ============================================

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub message: String,
    pub data: Option<T>,
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: i32,
    pub email: String,
    pub role: String, // ✅ Aquí puede quedarse String para la respuesta JSON
    pub status: String,
    pub profile_data: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub details: Option<String>,
}

// ============================================
// OTROS MODELOS (sin cambios)
// ============================================

#[derive(Serialize, sqlx::FromRow)]
pub struct Bimester {
    pub id: i32,
    pub name: String,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct Grade {
    pub id: i32,
    pub bimester_id: i32,
    pub number: i32,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct Section {
    pub id: i32,
    pub grade_id: i32,
    pub letter: String,
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct NewName {
    pub full_name: String,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct Student {
    pub id: i32,
    pub section_id: i32,
    pub full_name: String,
    pub user_id: Option<i32>,
}

#[derive(Serialize)]
pub struct StudentGradeItem {
    pub full_name: String,
    pub section_letter: String,
    pub grade_number: i32,
    pub bimester_name: String,
    pub session_title: Option<String>,
    pub competency_name: Option<String>,
    pub value: String,
    pub observation: Option<String>,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Serialize)]
pub struct UnlinkedStudent {
    pub id: i32,
    pub full_name: String,
    pub section_id: i32,
    pub section_letter: String,
    pub grade_number: i32,
    pub bimester_name: String,
}

#[derive(Serialize)]
pub struct LinkedStudent {
    pub student_id: i32,
    pub full_name: String,
    pub section_letter: String,
    pub grade_number: i32,
    pub bimester_name: String,
    pub year: i32,
}

#[derive(Deserialize)]
pub struct SearchStudentQuery {
    pub name: String,
}

#[derive(Deserialize)]
pub struct LinkStudentIn {
    pub student_id: i32,
    pub user_id: i32,
}

#[derive(Deserialize)]
pub struct LinkByDniIn {
    pub student_id: i32,
    pub dni: String,
}

#[derive(Deserialize)]
pub struct UnlinkStudentIn {
    pub student_id: i32,
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct Session {
    pub id: i32,
    pub section_id: i32,
    pub number: i32,
    pub title: Option<String>,
    pub date: Option<chrono::NaiveDate>,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Competency {
    pub id: i32,
    pub session_id: i32,
    pub number: i32,
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct Product {
    pub id: i32,
    pub session_id: i32,
    pub number: i32,
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct Ability {
    pub id: i32,
    pub competency_id: i32,
    pub number: i32,
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct Criterion {
    pub id: i32,
    pub ability_id: i32,
    pub number: i32,
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Deserialize, sqlx::FromRow)]
pub struct EvalValueIn {
    pub session_id: i32,
    pub competency_id: i32,
    pub ability_id: i32,
    pub criterion_id: i32,
    pub product_id: i32,
    pub student_id: i32,
    pub value: String,
    pub observation: Option<String>,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct EvaluationItem {
    pub id: i32,
    pub session_id: i32,
    pub competency_id: i32,
    pub ability_id: i32,
    pub criterion_id: i32,
    pub product_id: i32,
    pub student_id: i32,
    pub value: String,
    pub updated_at: chrono::NaiveDateTime,
    pub observation: Option<String>,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct MatrixResponse {
    pub locked: bool,
    pub competency: serde_json::Value,
    pub abilities: Vec<serde_json::Value>,
    pub criteria: Vec<serde_json::Value>,
    pub products: Vec<serde_json::Value>,
    pub students: Vec<serde_json::Value>,
    pub values: Vec<serde_json::Value>,
}

#[derive(Serialize)]
#[allow(dead_code)]
pub struct BimesterWithGrades {
    pub id: i32,
    pub name: String,
    pub grades: Vec<GradeWithSections>,
}

#[derive(Serialize)]
#[allow(dead_code)]
pub struct GradeWithSections {
    pub id: i32,
    pub bimester_id: i32,
    pub number: i32,
    pub sections: Vec<Section>,
}
