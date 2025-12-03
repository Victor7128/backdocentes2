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
