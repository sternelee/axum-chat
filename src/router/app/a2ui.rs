use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

// Default prompt shown to A2UI renderers when the caller omits an intent.
// Translates to: "Help me plan a simple trip".
const DEFAULT_INTENT: &str = "帮我规划一个简单的行程";
// Default location used by the sample UI when the caller does not provide one.
// Translates to: "Beijing".
const DEFAULT_LOCATION: &str = "北京";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct A2uiComponent {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub props: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct A2uiResponse {
    pub schema_version: String,
    pub root: String,
    pub components: Vec<A2uiComponent>,
    pub data: Value,
}

#[derive(Debug, Deserialize)]
pub struct A2uiRequest {
    /// Intent or task description from the client. Defaults to empty when omitted.
    #[serde(default)]
    pub intent: String,
    /// Location or context parameter from the client. Defaults to empty when omitted.
    #[serde(default)]
    pub location: String,
}

pub async fn a2ui_example() -> Json<A2uiResponse> {
    Json(build_a2ui_response(String::new(), String::new()))
}

pub async fn generate_a2ui(Json(body): Json<A2uiRequest>) -> Json<A2uiResponse> {
    Json(build_a2ui_response(body.intent, body.location))
}

fn build_a2ui_response(intent: String, location: String) -> A2uiResponse {
    let intent = if intent.trim().is_empty() {
        DEFAULT_INTENT.to_string()
    } else {
        intent
    };

    let location = if location.trim().is_empty() {
        DEFAULT_LOCATION.to_string()
    } else {
        location
    };

    let data = json!({
        "form": {
            "destination": location,
            "notes": intent,
            "travel_date": ""
        }
    });

    let components = vec![
        A2uiComponent {
            id: "root".to_string(),
            kind: "column".to_string(),
            children: vec![
                "title".to_string(),
                "subtitle".to_string(),
                "form-card".to_string(),
            ],
            props: Some(json!({
                "gap": "medium",
                "padding": "medium"
            })),
        },
        A2uiComponent {
            id: "title".to_string(),
            kind: "text".to_string(),
            children: vec![],
            props: Some(json!({
                "value": "Rust A2UI 示例",
                "variant": "headline"
            })),
        },
        A2uiComponent {
            id: "subtitle".to_string(),
            kind: "text".to_string(),
            children: vec![],
            props: Some(json!({
                "value": "基于 A2UI 的声明式界面，由后端返回组件树。",
                "tone": "secondary"
            })),
        },
        A2uiComponent {
            id: "form-card".to_string(),
            kind: "card".to_string(),
            children: vec![
                "destination".to_string(),
                "notes".to_string(),
                "travel-date".to_string(),
                "submit".to_string(),
            ],
            props: Some(json!({
                "title": "行程偏好",
                "subtitle": "填入目的地和需求，前端 A2UI 渲染器会根据组件树展示表单。"
            })),
        },
        A2uiComponent {
            id: "destination".to_string(),
            kind: "text-field".to_string(),
            children: vec![],
            props: Some(json!({
                "label": "目的地",
                "placeholder": "例如：上海、杭州",
                "binding": "form.destination"
            })),
        },
        A2uiComponent {
            id: "notes".to_string(),
            kind: "text-area".to_string(),
            children: vec![],
            props: Some(json!({
                "label": "行程备注",
                "rows": 3,
                "binding": "form.notes"
            })),
        },
        A2uiComponent {
            id: "travel-date".to_string(),
            kind: "date-picker".to_string(),
            children: vec![],
            props: Some(json!({
                "label": "出行日期",
                "binding": "form.travel_date"
            })),
        },
        A2uiComponent {
            id: "submit".to_string(),
            kind: "button".to_string(),
            children: vec![],
            props: Some(json!({
                "label": "提交 A2UI 请求",
                "action": {
                    "type": "event",
                    "name": "submit-trip",
                    "payload_binding": "form"
                },
                "style": "primary"
            })),
        },
    ];

    A2uiResponse {
        schema_version: "0.8".to_string(),
        root: "root".to_string(),
        components,
        data,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_defaults_when_request_empty() {
        let response = build_a2ui_response(String::new(), String::new());

        assert_eq!(response.schema_version, "0.8");
        assert_eq!(response.root, "root");
        assert!(response
            .components
            .iter()
            .any(|component| component.id == "form-card"));

        let form = response.data.get("form").expect("form data exists");
        assert_eq!(form.get("destination").unwrap(), DEFAULT_LOCATION);
        assert_eq!(form.get("notes").unwrap(), DEFAULT_INTENT);
    }

    #[test]
    fn echoes_request_into_payload() {
        let response = build_a2ui_response("去上海品尝美食".into(), "上海".into());

        let form = response.data.get("form").expect("form data exists");
        assert_eq!(form.get("destination").unwrap(), "上海");
        assert_eq!(form.get("notes").unwrap(), "去上海品尝美食");

        let notes_component = response
            .components
            .iter()
            .find(|component| component.id == "notes")
            .expect("notes component exists");

        let binding = notes_component
            .props
            .as_ref()
            .and_then(|props| props.get("binding"))
            .expect("binding present");

        assert_eq!(binding, "form.notes");
    }
}
