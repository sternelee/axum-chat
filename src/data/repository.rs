use std::sync::Arc;
use uuid::Uuid;

use super::model::{
    Agent, AgentWithProvider, Chat, ChatMessagePair, CreateAgentRequest, CreateProviderRequest,
    Provider, ProviderModel, ProviderModelInfo, ProviderType, ProviderWithModels,
    UpdateAgentRequest, UpdateProviderRequest, OpenAIModel, OpenAIModelResponse, OpenRouterPricing,
    ModelPricing,
};
use crate::data::libsql_database::{Database, DatabaseError};

#[derive(Clone)]
pub struct ChatRepository {
    pub db: Arc<Database>,
}

impl ChatRepository {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub async fn get_all_chats(&self, user_id: i64) -> Result<Vec<Chat>, DatabaseError> {
        let result = self
            .db
            .query(
                "SELECT id, user_id, name FROM chats WHERE user_id = ? ORDER BY created_at DESC",
                vec![serde_json::Value::Number(user_id.into())],
            )
            .await?;

        let mut chats = Vec::new();
        for row in result.rows {
            chats.push(Chat::from_json_row(&row)?);
        }

        Ok(chats)
    }

    pub async fn delete_chat(&self, chat_id: i64) -> Result<u64, DatabaseError> {
        let result = self
            .db
            .execute(
                "DELETE FROM chats WHERE id = ?",
                vec![serde_json::Value::Number(chat_id.into())],
            )
            .await?;

        Ok(result.rows_affected)
    }

    pub async fn retrieve_chat(&self, chat_id: i64) -> Result<Vec<ChatMessagePair>, DatabaseError> {
        let result = self
            .db
            .query(
                "SELECT * FROM v_chat_messages WHERE chat_id = ?",
                vec![serde_json::Value::Number(chat_id.into())],
            )
            .await?;

        let mut message_pairs = Vec::new();
        for row in result.rows {
            message_pairs.push(ChatMessagePair::from_json_row(&row)?);
        }

        Ok(message_pairs)
    }

    pub async fn create_chat(
        &self,
        user_id: i64,
        name: &str,
        model: &str,
    ) -> Result<i64, DatabaseError> {
        let result = self
            .db
            .execute(
                "INSERT INTO chats (user_id, name, model) VALUES (?, ?, ?)",
                vec![
                    serde_json::Value::Number(user_id.into()),
                    serde_json::Value::String(name.to_string()),
                    serde_json::Value::String(model.to_string()),
                ],
            )
            .await?;

        // Get the last inserted row id
        let result = self
            .db
            .query("SELECT last_insert_rowid() as id", vec![])
            .await?;

        if let Some(row) = result.rows.first() {
            Ok(row["id"].as_i64().unwrap_or(0))
        } else {
            Err(DatabaseError("Failed to get inserted row id".to_string()))
        }
    }

    pub async fn add_ai_message_to_pair(
        &self,
        pair_id: i64,
        message: &str,
    ) -> Result<i64, DatabaseError> {
        // Insert message
        let result = self
            .db
            .execute(
                "INSERT INTO messages (message) VALUES (?)",
                vec![serde_json::Value::String(message.to_string())],
            )
            .await?;

        // Get message id
        let result = self
            .db
            .query("SELECT last_insert_rowid() as id", vec![])
            .await?;

        if let Some(row) = result.rows.first() {
            let message_id = row["id"].as_i64().unwrap_or(0);

            // Update message pair
            self.db
                .execute(
                    "UPDATE message_pairs SET ai_message_id = ? WHERE id = ?",
                    vec![
                        serde_json::Value::Number(message_id.into()),
                        serde_json::Value::Number(pair_id.into()),
                    ],
                )
                .await?;

            Ok(message_id)
        } else {
            Err(DatabaseError(
                "Failed to get inserted message id".to_string(),
            ))
        }
    }

    pub async fn add_message_block(
        &self,
        chat_id: i64,
        human_message: &str,
    ) -> Result<i64, DatabaseError> {
        // Create message block
        let result = self
            .db
            .execute(
                "INSERT INTO message_blocks (chat_id) VALUES (?)",
                vec![serde_json::Value::Number(chat_id.into())],
            )
            .await?;

        // Get message block id
        let result = self
            .db
            .query("SELECT last_insert_rowid() as id", vec![])
            .await?;

        if let Some(row) = result.rows.first() {
            let message_block_id = row["id"].as_i64().unwrap_or(0);

            // Insert message
            let result = self
                .db
                .execute(
                    "INSERT INTO messages (message) VALUES (?)",
                    vec![serde_json::Value::String(human_message.to_string())],
                )
                .await?;

            let result = self
                .db
                .query("SELECT last_insert_rowid() as id", vec![])
                .await?;

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

                let result = self
                    .db
                    .query("SELECT last_insert_rowid() as id", vec![])
                    .await?;

                if let Some(row) = result.rows.first() {
                    let message_pair_id = row["id"].as_i64().unwrap_or(0);

                    // Update message block with selected pair
                    self.db
                        .execute(
                            "UPDATE message_blocks SET selected_pair_id = ? WHERE id = ?",
                            vec![
                                serde_json::Value::Number(message_pair_id.into()),
                                serde_json::Value::Number(message_block_id.into()),
                            ],
                        )
                        .await?;

                    Ok(message_pair_id)
                } else {
                    Err(DatabaseError(
                        "Failed to get inserted message pair id".to_string(),
                    ))
                }
            } else {
                Err(DatabaseError(
                    "Failed to get inserted message id".to_string(),
                ))
            }
        } else {
            Err(DatabaseError(
                "Failed to get inserted message block id".to_string(),
            ))
        }
    }

    // Provider CRUD operations
    pub async fn get_all_providers(&self) -> Result<Vec<Provider>, DatabaseError> {
        let result = self
            .db
            .query(
                r#"
            SELECT
                id,
                uuid,
                name,
                provider_type,
                base_url,
                chat_endpoint,
                embed_endpoint,
                image_endpoint,
                models_endpoint,
                api_key_encrypted,
                support_chat,
                support_embed,
                support_image,
                support_streaming,
                support_tools,
                support_images,
                local_agent_config,
                is_active,
                COALESCE(is_legacy_id, TRUE) as is_legacy_id,
                datetime(created_at) as created_at,
                datetime(updated_at) as updated_at
            FROM providers
            ORDER BY created_at DESC
            "#,
                vec![],
            )
            .await?;

        let mut providers = Vec::new();
        for row in result.rows {
            providers.push(Provider::from_json_row(&row)?);
        }

        Ok(providers)
    }

    pub async fn get_provider_by_id(&self, id: i64) -> Result<Option<Provider>, DatabaseError> {
        let result = self
            .db
            .query(
                r#"
            SELECT
                id,
                uuid,
                name,
                provider_type,
                base_url,
                chat_endpoint,
                embed_endpoint,
                image_endpoint,
                models_endpoint,
                api_key_encrypted,
                support_chat,
                support_embed,
                support_image,
                support_streaming,
                support_tools,
                support_images,
                local_agent_config,
                is_active,
                COALESCE(is_legacy_id, TRUE) as is_legacy_id,
                datetime(created_at) as created_at,
                datetime(updated_at) as updated_at
            FROM providers
            WHERE id = ?
            "#,
                vec![serde_json::Value::Number(id.into())],
            )
            .await?;

        if let Some(row) = result.rows.first() {
            Ok(Some(Provider::from_json_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub async fn get_provider_by_name(
        &self,
        name: &str,
    ) -> Result<Option<Provider>, DatabaseError> {
        let result = self
            .db
            .query(
                r#"
            SELECT
                id,
                uuid,
                name,
                provider_type,
                base_url,
                chat_endpoint,
                embed_endpoint,
                image_endpoint,
                models_endpoint,
                api_key_encrypted,
                support_chat,
                support_embed,
                support_image,
                support_streaming,
                support_tools,
                support_images,
                local_agent_config,
                is_active,
                COALESCE(is_legacy_id, TRUE) as is_legacy_id,
                datetime(created_at) as created_at,
                datetime(updated_at) as updated_at
            FROM providers
            WHERE name = ?
            "#,
                vec![serde_json::Value::String(name.to_string())],
            )
            .await?;

        if let Some(row) = result.rows.first() {
            Ok(Some(Provider::from_json_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub async fn create_provider(
        &self,
        request: CreateProviderRequest,
    ) -> Result<i64, DatabaseError> {
        // Get default endpoints and capabilities from provider type
        let default_endpoints = request.provider_type.default_endpoints();

        let chat_endpoint = request.chat_endpoint.or(default_endpoints.chat);
        let embed_endpoint = request.embed_endpoint.or(default_endpoints.embed);
        let image_endpoint = request.image_endpoint.or(default_endpoints.image);
        let models_endpoint = request.models_endpoint.or(default_endpoints.models);

        // Use provided capabilities or defaults based on provider type
        let support_chat = request.support_chat.unwrap_or(chat_endpoint.is_some());
        let support_embed = request.support_embed.unwrap_or(embed_endpoint.is_some());
        let support_image = request.support_image.unwrap_or(image_endpoint.is_some());
        let support_streaming = request.support_streaming.unwrap_or(true);
        let support_tools = request.support_tools.unwrap_or(true);
        let support_images = request.support_images.unwrap_or(matches!(
            request.provider_type,
            crate::data::model::ProviderType::OpenAI
                | crate::data::model::ProviderType::OpenRouter
                | crate::data::model::ProviderType::AzureOpenAI
                | crate::data::model::ProviderType::Gemini
        ));

        // Generate UUID for the new provider
        let provider_uuid = Uuid::new_v4();

        let result = self
            .db
            .execute(
                r#"
            INSERT INTO providers (
                uuid, name, provider_type, base_url, chat_endpoint, embed_endpoint, image_endpoint,
                models_endpoint, api_key_encrypted, support_chat, support_embed, support_image,
                support_streaming, support_tools, support_images, local_agent_config, is_active, is_legacy_id
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
                vec![
                    serde_json::Value::String(provider_uuid.to_string()),
                    serde_json::Value::String(request.name),
                    serde_json::Value::String(request.provider_type.to_string()),
                    serde_json::Value::String(request.base_url),
                    serde_json::Value::String(chat_endpoint.unwrap_or_default()),
                    serde_json::Value::String(embed_endpoint.unwrap_or_default()),
                    serde_json::Value::String(image_endpoint.unwrap_or_default()),
                    serde_json::Value::String(models_endpoint.unwrap_or_default()),
                    serde_json::Value::String(request.api_key),
                    serde_json::Value::Bool(support_chat),
                    serde_json::Value::Bool(support_embed),
                    serde_json::Value::Bool(support_image),
                    serde_json::Value::Bool(support_streaming),
                    serde_json::Value::Bool(support_tools),
                    serde_json::Value::Bool(support_images),
                    serde_json::Value::String(String::new()), // local_agent_config = empty string for new records
                    serde_json::Value::Bool(true), // is_active = true for new records
                    serde_json::Value::Bool(false), // is_legacy_id = false for new records
                ],
            )
            .await?;

        let result = self
            .db
            .query("SELECT last_insert_rowid() as id", vec![])
            .await?;

        if let Some(row) = result.rows.first() {
            Ok(row["id"].as_i64().unwrap_or(0))
        } else {
            Err(DatabaseError(
                "Failed to get inserted provider_id".to_string(),
            ))
        }
    }

    pub async fn update_provider(
        &self,
        id: i64,
        request: UpdateProviderRequest,
    ) -> Result<u64, DatabaseError> {
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
        if let Some(chat_endpoint) = &request.chat_endpoint {
            updates.push("chat_endpoint = ?");
            params.push(serde_json::Value::String(chat_endpoint.clone()));
        }
        if let Some(embed_endpoint) = &request.embed_endpoint {
            updates.push("embed_endpoint = ?");
            params.push(serde_json::Value::String(embed_endpoint.clone()));
        }
        if let Some(image_endpoint) = &request.image_endpoint {
            updates.push("image_endpoint = ?");
            params.push(serde_json::Value::String(image_endpoint.clone()));
        }
        if let Some(models_endpoint) = &request.models_endpoint {
            updates.push("models_endpoint = ?");
            params.push(serde_json::Value::String(models_endpoint.clone()));
        }
        if let Some(api_key) = &request.api_key {
            updates.push("api_key_encrypted = ?");
            params.push(serde_json::Value::String(api_key.clone()));
        }
        if let Some(support_chat) = request.support_chat {
            updates.push("support_chat = ?");
            params.push(serde_json::Value::Bool(support_chat));
        }
        if let Some(support_embed) = request.support_embed {
            updates.push("support_embed = ?");
            params.push(serde_json::Value::Bool(support_embed));
        }
        if let Some(support_image) = request.support_image {
            updates.push("support_image = ?");
            params.push(serde_json::Value::Bool(support_image));
        }
        if let Some(support_streaming) = request.support_streaming {
            updates.push("support_streaming = ?");
            params.push(serde_json::Value::Bool(support_streaming));
        }
        if let Some(support_tools) = request.support_tools {
            updates.push("support_tools = ?");
            params.push(serde_json::Value::Bool(support_tools));
        }
        if let Some(support_images) = request.support_images {
            updates.push("support_images = ?");
            params.push(serde_json::Value::Bool(support_images));
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
        // First check if there are any agents using this provider
        let dependent_agents = self.get_agents_by_provider(id).await?;

        if !dependent_agents.is_empty() {
            let agent_names: Vec<String> = dependent_agents.iter()
                .map(|agent| format!("{} (ID: {})", agent.name, agent.id))
                .collect();

            return Err(DatabaseError(format!(
                "Cannot delete provider: {} agents depend on this provider. Dependent agents: {}",
                dependent_agents.len(),
                agent_names.join(", ")
            )));
        }

        // No dependent agents, proceed with deletion
        let result = self
            .db
            .execute(
                "DELETE FROM providers WHERE id = ?",
                vec![serde_json::Value::Number(id.into())],
            )
            .await?;

        Ok(result.rows_affected)
    }

    /// Get all agents that depend on a specific provider
    pub async fn get_agents_by_provider(&self, provider_id: i64) -> Result<Vec<Agent>, DatabaseError> {
        let result = self
            .db
            .query(
                r#"
            SELECT
                id, COALESCE(uuid, NULL) as uuid, user_id, COALESCE(user_uuid, NULL) as user_uuid, name,
                description, provider_id, COALESCE(provider_uuid, NULL) as provider_uuid, model_name,
                stream, chat, embed, image, tool,
                COALESCE(tools, '[]') as tools, COALESCE(allow_tools, '[]') as allow_tools, system_prompt,
                COALESCE(top_p, 1.0) as top_p, COALESCE(max_context, 4096) as max_context, file,
                COALESCE(file_types, '[]') as file_types, COALESCE(temperature, 0.7) as temperature,
                COALESCE(max_tokens, 2048) as max_tokens, COALESCE(presence_penalty, 0.0) as presence_penalty,
                COALESCE(frequency_penalty, 0.0) as frequency_penalty, COALESCE(icon, '🤖') as icon,
                COALESCE(category, 'general') as category, public, COALESCE(is_legacy_id, TRUE) as is_legacy_id, is_active,
                datetime(created_at) as created_at,
                datetime(updated_at) as updated_at
            FROM agents
            WHERE provider_id = ?
            ORDER BY name
            "#,
                vec![serde_json::Value::Number(provider_id.into())],
            )
            .await?;

        let mut agents = Vec::new();
        for row in result.rows {
            agents.push(Agent::from_json_row(&row)?);
        }

        Ok(agents)
    }

    // Provider Model operations
    pub async fn get_models_by_provider(
        &self,
        provider_id: i64,
    ) -> Result<Vec<ProviderModel>, DatabaseError> {
        let result = self
            .db
            .query(
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
            )
            .await?;

        let mut models = Vec::new();
        for row in result.rows {
            models.push(ProviderModel::from_json_row(&row)?);
        }

        Ok(models)
    }

    pub async fn get_provider_with_models(
        &self,
        id: i64,
    ) -> Result<Option<ProviderWithModels>, DatabaseError> {
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
                id, COALESCE(uuid, NULL) as uuid, user_id, COALESCE(user_uuid, NULL) as user_uuid, name,
                description, provider_id, COALESCE(provider_uuid, NULL) as provider_uuid, model_name,
                stream, chat, embed, image, tool,
                COALESCE(tools, '[]') as tools, COALESCE(allow_tools, '[]') as allow_tools, system_prompt,
                COALESCE(top_p, 1.0) as top_p, COALESCE(max_context, 4096) as max_context, file,
                COALESCE(file_types, '[]') as file_types, COALESCE(temperature, 0.7) as temperature,
                COALESCE(max_tokens, 2048) as max_tokens, COALESCE(presence_penalty, 0.0) as presence_penalty,
                COALESCE(frequency_penalty, 0.0) as frequency_penalty, COALESCE(icon, '🤖') as icon,
                COALESCE(category, 'general') as category, public, COALESCE(is_legacy_id, TRUE) as is_legacy_id, is_active,
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
                id, COALESCE(uuid, NULL) as uuid, user_id, COALESCE(user_uuid, NULL) as user_uuid, name,
                description, provider_id, COALESCE(provider_uuid, NULL) as provider_uuid, model_name,
                stream, chat, embed, image, tool,
                COALESCE(tools, '[]') as tools, COALESCE(allow_tools, '[]') as allow_tools, system_prompt,
                COALESCE(top_p, 1.0) as top_p, COALESCE(max_context, 4096) as max_context, file,
                COALESCE(file_types, '[]') as file_types, COALESCE(temperature, 0.7) as temperature,
                COALESCE(max_tokens, 2048) as max_tokens, COALESCE(presence_penalty, 0.0) as presence_penalty,
                COALESCE(frequency_penalty, 0.0) as frequency_penalty, COALESCE(icon, '🤖') as icon,
                COALESCE(category, 'general') as category, public, COALESCE(is_legacy_id, TRUE) as is_legacy_id, is_active,
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
                id, COALESCE(uuid, NULL) as uuid, user_id, COALESCE(user_uuid, NULL) as user_uuid, name,
                description, provider_id, COALESCE(provider_uuid, NULL) as provider_uuid, model_name,
                stream, chat, embed, image, tool,
                COALESCE(tools, '[]') as tools, COALESCE(allow_tools, '[]') as allow_tools, system_prompt,
                COALESCE(top_p, 1.0) as top_p, COALESCE(max_context, 4096) as max_context, file,
                COALESCE(file_types, '[]') as file_types, COALESCE(temperature, 0.7) as temperature,
                COALESCE(max_tokens, 2048) as max_tokens, COALESCE(presence_penalty, 0.0) as presence_penalty,
                COALESCE(frequency_penalty, 0.0) as frequency_penalty, COALESCE(icon, '🤖') as icon,
                COALESCE(category, 'general') as category, public, COALESCE(is_legacy_id, TRUE) as is_legacy_id, is_active,
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

    pub async fn get_agent_with_provider(
        &self,
        id: i64,
    ) -> Result<Option<AgentWithProvider>, DatabaseError> {
        let agent = self.get_agent_by_id(id).await?;

        if let Some(agent) = agent {
            let provider = self.get_provider_by_id(agent.provider_id).await?;

            if let Some(provider) = provider {
                let tools: Vec<String> = serde_json::from_str(&agent.tools).unwrap_or_default();
                let allow_tools: Vec<String> =
                    serde_json::from_str(&agent.allow_tools).unwrap_or_default();
                let file_types: Vec<String> =
                    serde_json::from_str(&agent.file_types).unwrap_or_default();

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

    pub async fn create_agent(
        &self,
        user_id: i64,
        request: CreateAgentRequest,
    ) -> Result<i64, DatabaseError> {
        let tools_json =
            serde_json::to_string(&request.tools.unwrap_or_default()).unwrap_or_default();
        let allow_tools_json =
            serde_json::to_string(&request.allow_tools.unwrap_or_default()).unwrap_or_default();
        let file_types_json =
            serde_json::to_string(&request.file_types.unwrap_or_default()).unwrap_or_default();

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

        // Generate UUID for the new agent
        let agent_uuid = Uuid::new_v4();

        let result = self.db.execute(
            r#"
            INSERT INTO agents (uuid, user_id, name, description, provider_id, model_name, stream, chat, embed, image, tool,
                               tools, allow_tools, system_prompt, top_p, max_context, file, file_types, temperature, max_tokens,
                               presence_penalty, frequency_penalty, icon, category, public, is_legacy_id, is_active)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            vec![
                serde_json::Value::String(agent_uuid.to_string()),
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
                serde_json::Value::Bool(false), // is_legacy_id = false for new records
                serde_json::Value::Bool(true), // is_active defaults to true
            ],
        ).await?;

        let result = self
            .db
            .query("SELECT last_insert_rowid() as id", vec![])
            .await?;

        if let Some(row) = result.rows.first() {
            Ok(row["id"].as_i64().unwrap_or(0))
        } else {
            Err(DatabaseError("Failed to get inserted agent_id".to_string()))
        }
    }

    pub async fn update_agent(
        &self,
        id: i64,
        request: UpdateAgentRequest,
    ) -> Result<u64, DatabaseError> {
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
            params.push(serde_json::Value::String(
                serde_json::to_string(tools).unwrap_or_default(),
            ));
        }
        if let Some(allow_tools) = &request.allow_tools {
            updates.push("allow_tools = ?");
            params.push(serde_json::Value::String(
                serde_json::to_string(allow_tools).unwrap_or_default(),
            ));
        }
        if let Some(system_prompt) = &request.system_prompt {
            updates.push("system_prompt = ?");
            params.push(serde_json::Value::String(system_prompt.clone()));
        }
        if let Some(top_p) = request.top_p {
            updates.push("top_p = ?");
            params.push(serde_json::Value::Number(
                serde_json::Number::from_f64(top_p).unwrap_or(serde_json::Number::from(0)),
            ));
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
            params.push(serde_json::Value::String(
                serde_json::to_string(file_types).unwrap_or_default(),
            ));
        }
        if let Some(temperature) = request.temperature {
            updates.push("temperature = ?");
            params.push(serde_json::Value::Number(
                serde_json::Number::from_f64(temperature).unwrap_or(serde_json::Number::from(0)),
            ));
        }
        if let Some(max_tokens) = request.max_tokens {
            updates.push("max_tokens = ?");
            params.push(serde_json::Value::Number(max_tokens.into()));
        }
        if let Some(presence_penalty) = request.presence_penalty {
            updates.push("presence_penalty = ?");
            params.push(serde_json::Value::Number(
                serde_json::Number::from_f64(presence_penalty)
                    .unwrap_or(serde_json::Number::from(0)),
            ));
        }
        if let Some(frequency_penalty) = request.frequency_penalty {
            updates.push("frequency_penalty = ?");
            params.push(serde_json::Value::Number(
                serde_json::Number::from_f64(frequency_penalty)
                    .unwrap_or(serde_json::Number::from(0)),
            ));
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
        let result = self
            .db
            .execute(
                "DELETE FROM agents WHERE id = ?",
                vec![serde_json::Value::Number(id.into())],
            )
            .await?;

        Ok(result.rows_affected)
    }

    // Fetch models from provider API endpoints
    pub async fn fetch_models_from_provider(
        &self,
        provider: &Provider,
    ) -> Result<Vec<ProviderModelInfo>, DatabaseError> {
        use crate::data::model::{
            AnthropicModelResponse, GeminiModelResponse, ModelPricing, OpenAIModelResponse,
            ProviderModelInfo,
        };

        let models_endpoint = provider
            .models_endpoint
            .clone()
            .or_else(|| {
                // Use default models endpoint based on provider type
                let default_endpoints = provider.provider_type.default_endpoints();
                default_endpoints.models
            })
            .unwrap_or_else(|| "/models".to_string());

        let url = format!("{}{}", provider.base_url, models_endpoint);

        // Decrypt the API key
        let api_key = if provider.api_key_encrypted.starts_with("${") {
            // Environment variable reference - return empty for now
            eprintln!("Provider {} has environment variable API key reference, skipping model fetch", provider.name);
            return Ok(Vec::new());
        } else {
            provider.api_key_encrypted.clone()
        };

        eprintln!("=== MODEL FETCH DEBUG ===");
        eprintln!("Provider Name: {}", provider.name);
        eprintln!("Provider ID: {}", provider.id);
        eprintln!("Provider Type: {:?}", provider.provider_type);
        eprintln!("Base URL: {}", provider.base_url);
        eprintln!("Models Endpoint: {}", models_endpoint);
        eprintln!("Final URL: {}", url);
        eprintln!("API Key Length: {}", api_key.len());
        eprintln!("API Key Preview: {}...", &api_key[..api_key.len().min(10)]);
        eprintln!("=========================");

        // Create HTTP client
        let client = reqwest::Client::new();

        let response = match client
            .get(&url)
            .bearer_auth(&api_key)
            .header("Content-Type", "application/json")
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                eprintln!(
                    "Error fetching models from provider {}: {:?}",
                    provider.name, e
                );
                return Ok(Vec::new());
            }
        };

        let status = response.status();
        eprintln!("Provider {} response status: {}", provider.name, status);

        if !status.is_success() {
            eprintln!("Provider {} returned status: {}", provider.name, status);
            return Ok(Vec::new());
        }

        let text = response.text().await.unwrap_or_default();
        eprintln!("Provider {} response length: {} characters", provider.name, text.len());
        eprintln!("Provider {} response preview: {}", provider.name, &text[..text.len().min(200)]);

        // Parse response based on provider type
        let models = match provider.provider_type {
            ProviderType::OpenAI
            | ProviderType::OpenRouter
            | ProviderType::DeepSeek
            | ProviderType::AzureOpenAI => {
                // Try parsing as OpenAI-compatible format
                match serde_json::from_str::<OpenAIModelResponse>(&text) {
                    Ok(response) => {
                        eprintln!("Successfully parsed OpenAI-compatible response with {} models", response.data.len());
                        response
                            .data
                            .into_iter()
                            .map(|model| {
                                // Extract pricing information from OpenRouter pricing
                                let (input_price, output_price) = if let Some(pricing) = &model.pricing {
                                    (
                                        pricing.prompt.as_ref().and_then(|p| p.parse::<f64>().ok()),
                                        pricing.completion.as_ref().and_then(|p| p.parse::<f64>().ok())
                                    )
                                } else {
                                    (None, None)
                                };

                                ProviderModelInfo {
                                    id: model.id.clone(),
                                    name: model.id.clone(),
                                    display_name: model.name.clone().unwrap_or_else(|| model.id.clone()),
                                    context_length: model.context_length,
                                    max_tokens: None,
                                    support_chat: true,
                                    support_streaming: true,
                                    support_images: matches!(
                                        provider.provider_type,
                                        ProviderType::OpenAI | ProviderType::AzureOpenAI | ProviderType::OpenRouter
                                    ),
                                    support_tools: matches!(
                                        provider.provider_type,
                                        ProviderType::OpenAI
                                            | ProviderType::OpenRouter
                                            | ProviderType::DeepSeek
                                    ),
                                    pricing: Some(ModelPricing {
                                        input_price,
                                        output_price,
                                        currency: Some("USD".to_string()),
                                    }),
                                }
                            })
                            .collect()
                    }
                    Err(e) => {
                        eprintln!("Failed to parse OpenAI-compatible response: {}", e);
                        eprintln!("Raw response (first 500 chars): {}", &text[..text.len().min(500)]);

                        // Try parsing as raw JSON array as fallback
                        match serde_json::from_str::<serde_json::Value>(&text) {
                            Ok(value) => {
                                if let Some(data_array) = value.get("data").and_then(|v| v.as_array()) {
                                    eprintln!("Fallback parsing: found {} models in data array", data_array.len());
                                    data_array
                                        .iter()
                                        .filter_map(|model| {
                                            let id = model.get("id")?.as_str()?;
                                            let name = model.get("name").and_then(|v| v.as_str()).unwrap_or(id);
                                            Some(ProviderModelInfo {
                                                id: id.to_string(),
                                                name: id.to_string(),
                                                display_name: name.to_string(),
                                                context_length: None,
                                                max_tokens: None,
                                                support_chat: true,
                                                support_streaming: true,
                                                support_images: matches!(
                                                    provider.provider_type,
                                                    ProviderType::OpenAI | ProviderType::AzureOpenAI | ProviderType::OpenRouter
                                                ),
                                                support_tools: matches!(
                                                    provider.provider_type,
                                                    ProviderType::OpenAI
                                                        | ProviderType::OpenRouter
                                                        | ProviderType::DeepSeek
                                                ),
                                                pricing: Some(ModelPricing {
                                                    input_price: None,
                                                    output_price: None,
                                                    currency: None,
                                                }),
                                            })
                                        })
                                        .collect()
                                } else {
                                    eprintln!("Fallback parsing: no 'data' array found in response");
                                    Vec::new()
                                }
                            }
                            Err(e) => {
                                eprintln!("Fallback parsing also failed: {}", e);
                                Vec::new()
                            }
                        }
                    }
                }
            }
            ProviderType::Anthropic => {
                // Try parsing as Anthropic format
                match serde_json::from_str::<AnthropicModelResponse>(&text) {
                    Ok(response) => response
                        .data
                        .into_iter()
                        .map(|model| ProviderModelInfo {
                            id: model.id.clone(),
                            name: model.id.clone(),
                            display_name: model.display_name.clone(),
                            context_length: None,
                            max_tokens: None,
                            support_chat: true,
                            support_streaming: true,
                            support_images: false,
                            support_tools: true,
                            pricing: None,
                        })
                        .collect(),
                    Err(_) => Vec::new(),
                }
            }
            ProviderType::Gemini => {
                // Try parsing as Gemini format
                match serde_json::from_str::<GeminiModelResponse>(&text) {
                    Ok(response) => response
                        .models
                        .into_iter()
                        .map(|model| ProviderModelInfo {
                            id: model.name.clone(),
                            name: model.name.clone(),
                            display_name: model.display_name.clone(),
                            context_length: model.input_token_limit,
                            max_tokens: model.output_token_limit,
                            support_chat: model
                                .supported_generation_methods
                                .iter()
                                .any(|m| m == "generateContent"),
                            support_streaming: false,
                            support_images: false,
                            support_tools: false,
                            pricing: None,
                        })
                        .collect(),
                    Err(_) => Vec::new(),
                }
            }
            _ => {
                // Generic fallback - try to parse as JSON array of models
                match serde_json::from_str::<serde_json::Value>(&text) {
                    Ok(value) => {
                        if let Some(array) = value.as_array() {
                            array
                                .iter()
                                .filter_map(|v| v.as_str())
                                .map(|name| ProviderModelInfo {
                                    id: name.to_string(),
                                    name: name.to_string(),
                                    display_name: name.to_string(),
                                    context_length: None,
                                    max_tokens: None,
                                    support_chat: true,
                                    support_streaming: true,
                                    support_images: false,
                                    support_tools: false,
                                    pricing: None,
                                })
                                .collect()
                        } else {
                            Vec::new()
                        }
                    }
                    Err(_) => Vec::new(),
                }
            }
        };

        eprintln!("Successfully fetched {} models", models.len());
        if models.len() > 0 {
            eprintln!("First 3 models:");
            for (i, model) in models.iter().take(3).enumerate() {
                eprintln!("  {}: {} ({})", i + 1, model.display_name, model.id);
            }
        }

        Ok(models)
    }
}

