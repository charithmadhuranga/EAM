//! Current-state persistence helpers (kept separate for clarity).

/// Return the `EntityValue` tag used by its serde representation.
///
/// `EntityValue` serializes as `{"type": <tag>, "value": ...}`; this returns
/// just the tag, e.g. `"number"`, for the `value_type` column.
#[must_use]
pub fn value_tag(value: &eam_domain::EntityValue) -> &'static str {
    match value {
        eam_domain::EntityValue::Number(_) => "number",
        eam_domain::EntityValue::Bool(_) => "bool",
        eam_domain::EntityValue::Text(_) => "text",
        eam_domain::EntityValue::Json(_) => "json",
        eam_domain::EntityValue::Unknown => "unknown",
    }
}
