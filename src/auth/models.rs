use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "user_role", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UserRole {
    Docente,
    Apoderado,
    Alumno,
    Admin,
}

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

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: i32,
    pub firebase_uid: String,
    pub email: String,
    pub role: UserRole,
    pub status: UserStatus,
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
    pub role: String,
    pub status: String,
    pub profile_data: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub details: Option<String>,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct Section {
    pub id: i32,
    pub grade_id: i32,
    pub letter: String,
}

#[derive(Serialize, sqlx::FromRow)]
#[allow(dead_code)]
pub struct StudentGradeItem {
    pub full_name: String,
    pub section_letter: String,
    pub grade_number: i32,
    pub bimester_name: String,
    pub session_title: String,
    pub competency_name: String,
    pub ability_name: String,
    pub criterion_name: String,
    pub value: String,
    pub observation: Option<String>,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Serialize)]
#[allow(dead_code)]
pub struct CriterionItem {
    pub name: String,
    pub value: String,
    pub observation: Option<String>,
    pub updated_at: String,
}

#[derive(Serialize)]
#[allow(dead_code)]
pub struct AbilityItem {
    pub name: String,
    pub criteria: Vec<CriterionItem>,
}

#[derive(Serialize)]
#[allow(dead_code)]
pub struct CompetencyItem {
    pub name: String,
    pub abilities: Vec<AbilityItem>,
}

#[derive(Serialize)]
#[allow(dead_code)]
pub struct SessionGrades {
    pub session_title: String,
    pub bimester_name: String,
    pub section_letter: String,
    pub grade_number: i32,
    pub competencies: Vec<CompetencyItem>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct FlatGradeRow {
    pub session_title: String,
    pub bimester_name: String,
    pub section_letter: String,
    pub grade_number: i32,
    pub competency_name: String,
    pub ability_name: String,
    pub criterion_name: String,
    pub value: String,
    pub observation: Option<String>,
    pub updated_at: String,
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

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct CreateGuardianRelationshipIn {
    pub guardian_user_id: i32,
    pub student_user_id: i32,
    pub relationship_type: String,
    pub is_primary: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ReniecRequest {
    pub dni: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ReniecData {
    pub numero: Option<String>,
    pub nombre_completo: Option<String>,
    pub nombres: Option<String>,
    pub apellido_paterno: Option<String>,
    pub apellido_materno: Option<String>,

    #[serde(rename = "codVerifica")]
    pub codigo_verificacion: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ReniecResponse {
    pub success: bool,
    pub data: Option<ReniecData>,
    pub message: Option<String>,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct LinkingInfo {
    pub student_id: i32,
    pub student_name: String,
    pub linked_by: String,
    pub success: bool,
}
