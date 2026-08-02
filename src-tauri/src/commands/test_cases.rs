use tauri::State;
use uuid::Uuid;

use crate::database::{Database, TestCase};
use crate::licensing::feature_gate;

#[tauri::command]
pub fn save_test_case(
    id: Option<String>,
    name: String,
    description: String,
    category: String,
    tags: String,
    content: String,
    expected_message_type: String,
    expected_validation_result: String,
    db: State<'_, Database>,
) -> Result<TestCase, String> {
    // Community cap: block only NEW saves beyond the limit. Existing cases
    // (even ones saved over the cap during a trial) stay editable/runnable.
    if let Some(max) = feature_gate::test_case_limit() {
        let existing = db.get_test_cases(None)?;
        let is_update = id
            .as_deref()
            .map(|i| existing.iter().any(|tc| tc.id == i))
            .unwrap_or(false);
        if !is_update && existing.len() >= max {
            feature_gate::require("test_cases_unlimited")?;
        }
    }

    let tc = TestCase {
        id: id.unwrap_or_else(|| Uuid::new_v4().to_string()),
        name, description, category, tags, content,
        expected_message_type, expected_validation_result,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };
    db.save_test_case(&tc)?;
    Ok(tc)
}

#[tauri::command]
pub fn get_test_cases(
    category: Option<String>,
    db: State<'_, Database>,
) -> Result<Vec<TestCase>, String> {
    db.get_test_cases(category.as_deref())
}

#[tauri::command]
pub fn delete_test_case(id: String, db: State<'_, Database>) -> Result<(), String> {
    db.delete_test_case(&id)
}
