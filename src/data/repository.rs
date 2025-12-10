use std::sync::Arc;

use crate::data::libsql_database::{Database, DatabaseError};
use super::model::{
    Agent, AgentWithProvider, CreateAgentRequest, CreateProviderRequest, Provider, ProviderModel,
    ProviderWithModels, UpdateAgentRequest, UpdateProviderRequest, Chat, ChatMessagePair, ProviderType,
};

#[derive(Clone)]
pub struct ChatRepository {
    pub db: Arc<Database>,
}

impl ChatRepository {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub async fn get_all_chats(&self, user_id: i64) -> Result<Vec<Chat>, DatabaseError> {
        let result = self.db.query(
            "SELECT id, user_id, name FROM chats WHERE user_id = ? ORDER BY created_at DESC",
            vec![serde_json::Value::Number(user_id.into())],
        ).await?;

        let mut chats = Vec::new();
        for row in result.rows {
            chats.push(Chat::from_json_row(&row)?);
        }

        Ok(chats)
    }

    pub async fn delete_chat(&self, chat_id: i64) -> Result<u64, DatabaseError> {
        let result = self.db.execute(
            "DELETE FROM chats WHERE id = ?",
            vec![serde_json::Value::Number(chat_id.into())],
        ).await?;

        Ok(result.rows_affected)
    }

    pub async fn retrieve_chat(&self, chat_id: i64) -> Result<Vec<ChatMessagePair>, DatabaseError> {
        let result = self.db.query(
            "SELECT * FROM v_chat_messages WHERE chat_id = ?",
            vec![serde_json::Value::Number(chat_id.into())],
        ).await?;

        let mut message_pairs = Vec::new();
        for row in result.rows {
            message_pairs.push(ChatMessagePair::from_json_row(&row)?);
        }

        Ok(message_pairs)
    }

    pub async fn create_chat(&self, user_id: i64, name: &str, model: &str) -> Result<i64, DatabaseError> {
        let result = self.db.execute(
            "INSERT INTO chats (user_id, name, model) VALUES (?, ?, ?)",
            vec![
                serde_json::Value::Number(user_id.into()),
                serde_json::Value::String(name.to_string()),
                serde_json::Value::String(model.to_string()),
            ],
        ).await?;

        // Get the last inserted row id
        let result = self.db.query(
            "SELECT last_insert_rowid() as id",
            vec![],
        ).await?;

        if let Some(row) = result.rows.first() {
            Ok(row["id"].as_i64().unwrap_or(0))
        } else {
            Err(DatabaseError("Failed to get inserted row id".to_string()))
        }
    }

    pub async fn add_ai_message_to_pair(&self, pair_id: i64, message: &str) -> Result<i64, DatabaseError> {
        // Insert message
        let result = self.db.execute(
            "INSERT INTO messages (message) VALUES (?)",
            vec![serde_json::Value::String(message.to_string())],
        ).await?;

        // Get message id
        let result = self.db.query(
            "SELECT last_insert_rowid() as id",
            vec![],
        ).await?;

        if let Some(row) = result.rows.first() {
            let message_id = row["id"].as_i64().unwrap_or(0);

            // Update message pair
            self.db.execute(
                "UPDATE message_pairs SET ai_message_id = ? WHERE id = ?",
                vec![
                    serde_json::Value::Number(message_id.into()),
                    serde_json::Value::Number(pair_id.into()),
                ],
            ).await?;

            Ok(message_id)
        } else {
            Err(DatabaseError("Failed to get inserted message id".to_string()))
        }
    }

    pub async fn add_message_block(&self, chat_id: i64, human_message: &str) -> Result<i64, DatabaseError> {
        // Create message block
        let result = self.db.execute(
            "INSERT INTO message_blocks (chat_id) VALUES (?)",
            vec![serde_json::Value::Number(chat_id.into())],
        ).await?;

        // Get message block id
        let result = self.db.query(
            "SELECT last_insert_rowid() as id",
            vec![],
        ).await?;

        if let Some(row) = result.rows.first() {
            let message_block_id = row["id"].as_i64().unwrap_or(0);

            // Insert message
            let result = self.db.execute(
                "INSERT INTO messages (message) VALUES (?)",
                vec![serde_json::Value::String(human_message.to_string())],
            ).await?;

            let result = self.db.query(
                "SELECT last_insert_rowid() as id",
                vec![],
            ).await?;

            if let Some(row) = result.rows.first() {
                let message_id = row["id"].as_i64().unwrap_or(0);

                // Create message pair
                let result = self.db.execute(
                    "INSERT INTO message_pairs (human_message_id, message_block_id) VALUES (?, ?)",
                    vec![
                        serde_json::Value::Number(message_id.into()),
                        serde_json::Value::Number(message_block_id.into()),
                    ],
                ).await?;

                let result = self.db.query(
                    "SELECT last_insert_rowid() as id",
                    vec![],
                ).await?;

                if let Some(row) = result.rows.first() {
                    let message_pair_id = row["id"].as_i64().unwrap_or(0);

                    // Update message block with selected pair
                    self.db.execute(
                        "UPDATE message_blocks SET selected_pair_id = ? WHERE id = ?",
                        vec![
                            serde_json::Value::Number(message_pair_id.into()),
                            serde_json::Value::Number(message_block_id.into()),
                        ],
                    ).await?;

                    Ok(message_pair_id)
                } else {
                    Err(DatabaseError("Failed to get inserted message pair id".to_string()))
                }
            } else {
                Err(DatabaseError("Failed to get inserted message id".to_string()))
            }
        } else {
            Err(DatabaseError("Failed to get inserted message block id".to_string()))
        }
    }

    // Provider CRUD operations
    pub async fn get_all_providers(&self) -> Result<Vec<Provider>, DatabaseError> {
        let result = self.db.query(
            r#"
            SELECT
                id,
                name,
                provider_type,
                base_url,
                api_key_encrypted,
                is_active,
                datetime(created_at) as created_at,
                datetime(updated_at) as updated_at
            FROM providers
            ORDER BY created_at DESC
            "#,
            vec![],
        ).await?;

        let mut providers = Vec::new();
        for row in result.rows {
            providers.push(Provider::from_json_row(&row)?);
        }

        Ok(providers)
    }

    pub async fn get_provider_by_id(&self, id: i64) -> Result<Option<Provider>, DatabaseError> {
        let result = self.db.query(
            r#"
            SELECT
                id,
                name,
                provider_type,
                base_url,
                api_key_encrypted,
                is_active,
                datetime(created_at) as created_at,
                datetime(updated_at) as updated_at
            FROM providers
            WHERE id = ?
            "#,
            vec![serde_json::Value::Number(id.into())],
        ).await?;

        if let Some(row) = result.rows.first() {
            Ok(Some(Provider::from_json_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub async fn get_provider_by_name(&self, name: &str) -> Result<Option<Provider>, DatabaseError> {
        let result = self.db.query(
            r#"
            SELECT
                id,
                name,
                provider_type,
                base_url,
                api_key_encrypted,
                is_active,
                datetime(created_at) as created_at,
                datetime(updated_at) as updated_at
            FROM providers
            WHERE name = ?
            "#,
            vec![serde_json::Value::String(name.to_string())],
        ).await?;

        if let Some(row) = result.rows.first() {
            Ok(Some(Provider::from_json_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub async fn create_provider(&self, request: CreateProviderRequest) -> Result<i64, DatabaseError> {
        let result = self.db.execute(
            "INSERT INTO providers (name, provider_type, base_url, api_key_encrypted) VALUES (?, ?, ?, ?)",
            vec![
                serde_json::Value::String(request.name),
                serde_json::Value::String(request.provider_type.to_string()),
                serde_json::Value::String(request.base_url),
                serde_json::Value::String(request.api_key),
            ],
        ).await?;

        let result = self.db.query(
            "SELECT last_insert_rowid() as id",
            vec![],
        ).await?;

        if let Some(row) = result.rows.first() {
            Ok(row["id"].as_i64().unwrap_or(0))
        } else {
            Err(DatabaseError("Failed to get inserted provider_id".to_string()))
        }
    }

    pub async fn update_provider(&self, id: i64, request: UpdateProviderRequest) -> Result<u64, DatabaseError> {
        let mut updates = Vec::new();
        let mut params = Vec::new();

        if let Some(name) = &request.name {
            updates.push("name = ?");
            params.push(serde_json::Value::String(name.clone()));
        }
        if let Some(base_url) = &request.base_url {
            updates.push("base_url = ?");
            params.push(serde_json::Value::String(base_url.clone()));
        }
        if let Some(api_key) = &request.api_key {
            updates.push("api_key_encrypted = ?");
            params.push(serde_json::Value::String(api_key.clone()));
        }
        if let Some(is_active) = request.is_active {
            updates.push("is_active = ?");
            params.push(serde_json::Value::Bool(is_active));
        }

        if updates.is_empty() {
            return Ok(0);
        }

        let query = format!(
            "UPDATE providers SET updated_at = CURRENT_TIMESTAMP, {} WHERE id = ?",
            updates.join(", ")
        );

        params.push(serde_json::Value::Number(id.into()));

        let result = self.db.execute(&query, params).await?;
        Ok(result.rows_affected)
    }

    pub async fn delete_provider(&self, id: i64) -> Result<u64, DatabaseError> {
        let result = self.db.execute(
            "DELETE FROM providers WHERE id = ?",
            vec![serde_json::Value::Number(id.into())],
        ).await?;

        Ok(result.rows_affected)
    }

    // Provider Model operations
    pub async fn get_models_by_provider(&self, provider_id: i64) -> Result<Vec<ProviderModel>, DatabaseError> {
        let result = self.db.query(
            r#"
            SELECT
                id,
                provider_id,
                name,
                display_name,
                context_length,
                input_price,
                output_price,
                capabilities,
                is_active,
                datetime(created_at) as created_at
            FROM provider_models
            WHERE provider_id = ? AND is_active = TRUE
            ORDER BY display_name
            "#,
            vec![serde_json::Value::Number(provider_id.into())],
        ).await?;

        let mut models = Vec::new();
        for row in result.rows {
            models.push(ProviderModel::from_json_row(&row)?);
        }

        Ok(models)
    }

    pub async fn get_provider_with_models(&self, id: i64) -> Result<Option<ProviderWithModels>, DatabaseError> {
        let provider = self.get_provider_by_id(id).await?;
        if let Some(provider) = provider {
            let models = self.get_models_by_provider(id).await?;
            Ok(Some(ProviderWithModels {
                id: provider.id,
                name: provider.name,
                provider_type: provider.provider_type,
                base_url: provider.base_url,
                is_active: provider.is_active,
                created_at: provider.created_at,
                updated_at: provider.updated_at,
                models,
            }))
        } else {
            Ok(None)
        }
    }

    // Agent CRUD operations
    pub async fn get_all_agents(&self) -> Result<Vec<Agent>, DatabaseError> {
        let result = self.db.query(
            r#"
            SELECT
                id, user_id, name, description, provider_id, model_name, stream, chat, embed, image, tool,
                COALESCE(tools, '[]') as tools, COALESCE(allow_tools, '[]') as allow_tools, system_prompt,
                COALESCE(top_p, 1.0) as top_p, COALESCE(max_context, 4096) as max_context, file,
                COALESCE(file_types, '[]') as file_types, COALESCE(temperature, 0.7) as temperature,
                COALESCE(max_tokens, 2048) as max_tokens, COALESCE(presence_penalty, 0.0) as presence_penalty,
                COALESCE(frequency_penalty, 0.0) as frequency_penalty, COALESCE(icon, '🤖') as icon,
                COALESCE(category, 'general') as category, public, is_active,
                datetime(created_at) as created_at,
                datetime(updated_at) as updated_at
            FROM agents
            ORDER BY created_at DESC
            "#,
            vec![],
        ).await?;

        let mut agents = Vec::new();
        for row in result.rows {
            agents.push(Agent::from_json_row(&row)?);
        }

        Ok(agents)
    }

    pub async fn get_agents_by_user(&self, user_id: i64) -> Result<Vec<Agent>, DatabaseError> {
        let result = self.db.query(
            r#"
            SELECT
                id, user_id, name, description, provider_id, model_name, stream, chat, embed, image, tool,
                COALESCE(tools, '[]') as tools, COALESCE(allow_tools, '[]') as allow_tools, system_prompt,
                COALESCE(top_p, 1.0) as top_p, COALESCE(max_context, 4096) as max_context, file,
                COALESCE(file_types, '[]') as file_types, COALESCE(temperature, 0.7) as temperature,
                COALESCE(max_tokens, 2048) as max_tokens, COALESCE(presence_penalty, 0.0) as presence_penalty,
                COALESCE(frequency_penalty, 0.0) as frequency_penalty, COALESCE(icon, '🤖') as icon,
                COALESCE(category, 'general') as category, public, is_active,
                datetime(created_at) as created_at,
                datetime(updated_at) as updated_at
            FROM agents
            WHERE user_id = ? OR public = TRUE
            ORDER BY created_at DESC
            "#,
            vec![serde_json::Value::Number(user_id.into())],
        ).await?;

        let mut agents = Vec::new();
        for row in result.rows {
            agents.push(Agent::from_json_row(&row)?);
        }

        Ok(agents)
    }

    pub async fn get_agent_by_id(&self, id: i64) -> Result<Option<Agent>, DatabaseError> {
        let result = self.db.query(
            r#"
            SELECT
                id, user_id, name, description, provider_id, model_name, stream, chat, embed, image, tool,
                COALESCE(tools, '[]') as tools, COALESCE(allow_tools, '[]') as allow_tools, system_prompt,
                COALESCE(top_p, 1.0) as top_p, COALESCE(max_context, 4096) as max_context, file,
                COALESCE(file_types, '[]') as file_types, COALESCE(temperature, 0.7) as temperature,
                COALESCE(max_tokens, 2048) as max_tokens, COALESCE(presence_penalty, 0.0) as presence_penalty,
                COALESCE(frequency_penalty, 0.0) as frequency_penalty, COALESCE(icon, '🤖') as icon,
                COALESCE(category, 'general') as category, public, is_active,
                datetime(created_at) as created_at,
                datetime(updated_at) as updated_at
            FROM agents
            WHERE id = ?
            "#,
            vec![serde_json::Value::Number(id.into())],
        ).await?;

        if let Some(row) = result.rows.first() {
            Ok(Some(Agent::from_json_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub async fn get_agent_with_provider(&self, id: i64) -> Result<Option<AgentWithProvider>, DatabaseError> {
        let agent = self.get_agent_by_id(id).await?;

        if let Some(agent) = agent {
            let provider = self.get_provider_by_id(agent.provider_id).await?;

            if let Some(provider) = provider {
                let tools: Vec<String> = serde_json::from_str(&agent.tools).unwrap_or_default();
                let allow_tools: Vec<String> = serde_json::from_str(&agent.allow_tools).unwrap_or_default();
                let file_types: Vec<String> = serde_json::from_str(&agent.file_types).unwrap_or_default();

                Ok(Some(AgentWithProvider {
                    id: agent.id,
                    user_id: agent.user_id,
                    name: agent.name,
                    description: agent.description,
                    provider,
                    model_name: agent.model_name,
                    stream: agent.stream,
                    chat: agent.chat,
                    embed: agent.embed,
                    image: agent.image,
                    tool: agent.tool,
                    tools,
                    allow_tools,
                    system_prompt: agent.system_prompt,
                    top_p: agent.top_p,
                    max_context: agent.max_context,
                    file: agent.file,
                    file_types,
                    temperature: agent.temperature,
                    max_tokens: agent.max_tokens,
                    presence_penalty: agent.presence_penalty,
                    frequency_penalty: agent.frequency_penalty,
                    icon: agent.icon,
                    category: agent.category,
                    public: agent.public,
                    is_active: agent.is_active,
                    created_at: agent.created_at,
                    updated_at: agent.updated_at,
                }))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    pub async fn create_agent(&self, user_id: i64, request: CreateAgentRequest) -> Result<i64, DatabaseError> {
        let tools_json = serde_json::to_string(&request.tools.unwrap_or_default()).unwrap_or_default();
        let allow_tools_json = serde_json::to_string(&request.allow_tools.unwrap_or_default()).unwrap_or_default();
        let file_types_json = serde_json::to_string(&request.file_types.unwrap_or_default()).unwrap_or_default();

        let stream_val = request.stream.unwrap_or(true);
        let chat_val = request.chat.unwrap_or(true);
        let embed_val = request.embed.unwrap_or(false);
        let image_val = request.image.unwrap_or(false);
        let tool_val = request.tool.unwrap_or(false);
        let top_p_val = request.top_p.unwrap_or(1.0);
        let max_context_val = request.max_context.unwrap_or(4096);
        let file_val = request.file.unwrap_or(false);
        let temperature_val = request.temperature.unwrap_or(0.7);
        let max_tokens_val = request.max_tokens.unwrap_or(2048);
        let presence_penalty_val = request.presence_penalty.unwrap_or(0.0);
        let frequency_penalty_val = request.frequency_penalty.unwrap_or(0.0);
        let icon_val = request.icon.unwrap_or("🤖".to_string());
        let category_val = request.category.unwrap_or("general".to_string());
        let public_val = request.public.unwrap_or(false);

        let result = self.db.execute(
            r#"
            INSERT INTO agents (user_id, name, description, provider_id, model_name, stream, chat, embed, image, tool,
                               tools, allow_tools, system_prompt, top_p, max_context, file, file_types, temperature, max_tokens,
                               presence_penalty, frequency_penalty, icon, category, public, is_active)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            vec![
                serde_json::Value::Number(user_id.into()),
                serde_json::Value::String(request.name),
                serde_json::Value::String(request.description.unwrap_or_default()),
                serde_json::Value::Number(request.provider_id.into()),
                serde_json::Value::String(request.model_name),
                serde_json::Value::Bool(stream_val),
                serde_json::Value::Bool(chat_val),
                serde_json::Value::Bool(embed_val),
                serde_json::Value::Bool(image_val),
                serde_json::Value::Bool(tool_val),
                serde_json::Value::String(tools_json),
                serde_json::Value::String(allow_tools_json),
                serde_json::Value::String(request.system_prompt.unwrap_or_default()),
                serde_json::Value::Number(serde_json::Number::from_f64(top_p_val).unwrap_or(serde_json::Number::from(0))),
                serde_json::Value::Number(max_context_val.into()),
                serde_json::Value::Bool(file_val),
                serde_json::Value::String(file_types_json),
                serde_json::Value::Number(serde_json::Number::from_f64(temperature_val).unwrap_or(serde_json::Number::from(0))),
                serde_json::Value::Number(max_tokens_val.into()),
                serde_json::Value::Number(serde_json::Number::from_f64(presence_penalty_val).unwrap_or(serde_json::Number::from(0))),
                serde_json::Value::Number(serde_json::Number::from_f64(frequency_penalty_val).unwrap_or(serde_json::Number::from(0))),
                serde_json::Value::String(icon_val),
                serde_json::Value::String(category_val),
                serde_json::Value::Bool(public_val),
                serde_json::Value::Bool(true), // is_active defaults to true
            ],
        ).await?;

        let result = self.db.query(
            "SELECT last_insert_rowid() as id",
            vec![],
        ).await?;

        if let Some(row) = result.rows.first() {
            Ok(row["id"].as_i64().unwrap_or(0))
        } else {
            Err(DatabaseError("Failed to get inserted agent_id".to_string()))
        }
    }

    pub async fn update_agent(&self, id: i64, request: UpdateAgentRequest) -> Result<u64, DatabaseError> {
        let mut updates = Vec::new();
        let mut params = Vec::new();

        if let Some(name) = &request.name {
            updates.push("name = ?");
            params.push(serde_json::Value::String(name.clone()));
        }
        if let Some(description) = &request.description {
            updates.push("description = ?");
            params.push(serde_json::Value::String(description.clone()));
        }
        if let Some(provider_id) = request.provider_id {
            updates.push("provider_id = ?");
            params.push(serde_json::Value::Number(provider_id.into()));
        }
        if let Some(model_name) = &request.model_name {
            updates.push("model_name = ?");
            params.push(serde_json::Value::String(model_name.clone()));
        }
        if let Some(stream) = request.stream {
            updates.push("stream = ?");
            params.push(serde_json::Value::Bool(stream));
        }
        if let Some(chat) = request.chat {
            updates.push("chat = ?");
            params.push(serde_json::Value::Bool(chat));
        }
        if let Some(embed) = request.embed {
            updates.push("embed = ?");
            params.push(serde_json::Value::Bool(embed));
        }
        if let Some(image) = request.image {
            updates.push("image = ?");
            params.push(serde_json::Value::Bool(image));
        }
        if let Some(tool) = request.tool {
            updates.push("tool = ?");
            params.push(serde_json::Value::Bool(tool));
        }
        if let Some(tools) = &request.tools {
            updates.push("tools = ?");
            params.push(serde_json::Value::String(serde_json::to_string(tools).unwrap_or_default()));
        }
        if let Some(allow_tools) = &request.allow_tools {
            updates.push("allow_tools = ?");
            params.push(serde_json::Value::String(serde_json::to_string(allow_tools).unwrap_or_default()));
        }
        if let Some(system_prompt) = &request.system_prompt {
            updates.push("system_prompt = ?");
            params.push(serde_json::Value::String(system_prompt.clone()));
        }
        if let Some(top_p) = request.top_p {
            updates.push("top_p = ?");
            params.push(serde_json::Value::Number(serde_json::Number::from_f64(top_p).unwrap_or(serde_json::Number::from(0))));
        }
        if let Some(max_context) = request.max_context {
            updates.push("max_context = ?");
            params.push(serde_json::Value::Number(max_context.into()));
        }
        if let Some(file) = request.file {
            updates.push("file = ?");
            params.push(serde_json::Value::Bool(file));
        }
        if let Some(file_types) = &request.file_types {
            updates.push("file_types = ?");
            params.push(serde_json::Value::String(serde_json::to_string(file_types).unwrap_or_default()));
        }
        if let Some(temperature) = request.temperature {
            updates.push("temperature = ?");
            params.push(serde_json::Value::Number(serde_json::Number::from_f64(temperature).unwrap_or(serde_json::Number::from(0))));
        }
        if let Some(max_tokens) = request.max_tokens {
            updates.push("max_tokens = ?");
            params.push(serde_json::Value::Number(max_tokens.into()));
        }
        if let Some(presence_penalty) = request.presence_penalty {
            updates.push("presence_penalty = ?");
            params.push(serde_json::Value::Number(serde_json::Number::from_f64(presence_penalty).unwrap_or(serde_json::Number::from(0))));
        }
        if let Some(frequency_penalty) = request.frequency_penalty {
            updates.push("frequency_penalty = ?");
            params.push(serde_json::Value::Number(serde_json::Number::from_f64(frequency_penalty).unwrap_or(serde_json::Number::from(0))));
        }
        if let Some(icon) = &request.icon {
            updates.push("icon = ?");
            params.push(serde_json::Value::String(icon.clone()));
        }
        if let Some(category) = &request.category {
            updates.push("category = ?");
            params.push(serde_json::Value::String(category.clone()));
        }
        if let Some(public) = request.public {
            updates.push("public = ?");
            params.push(serde_json::Value::Bool(public));
        }
        if let Some(is_active) = request.is_active {
            updates.push("is_active = ?");
            params.push(serde_json::Value::Bool(is_active));
        }

        if updates.is_empty() {
            return Ok(0);
        }

        let query = format!(
            "UPDATE agents SET updated_at = CURRENT_TIMESTAMP, {} WHERE id = ?",
            updates.join(", ")
        );

        params.push(serde_json::Value::Number(id.into()));

        let result = self.db.execute(&query, params).await?;
        Ok(result.rows_affected)
    }

    pub async fn delete_agent(&self, id: i64) -> Result<u64, DatabaseError> {
        let result = self.db.execute(
            "DELETE FROM agents WHERE id = ?",
            vec![serde_json::Value::Number(id.into())],
        ).await?;

        Ok(result.rows_affected)
    }
}