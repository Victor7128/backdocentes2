use serde::{Deserialize, Serialize};

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
    pub value: String, // "AD","A","B","C"
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
    pub value: String, // "AD", "A", "B", "C"
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

//nuevos modelos paraa el endpoint de dashboard
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

// ============================================
// MODELOS DE AUTENTICACIÓN
// ============================================
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: i32,
    pub firebase_uid: String,
    pub email: String,
    pub role: String,   // DOCENTE, APODERADO, ALUMNO, ADMIN
    pub status: String, // ACTIVE, INACTIVE, SUSPENDED, PENDING
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
    pub role: String,
    pub status: String,
    pub profile_data: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub details: Option<String>,
}
