use std::sync::Arc;

use sqlx::sqlite::SqlitePool;
use sqlx::{Sqlite, Transaction};

use super::model::{
    Agent, AgentWithProvider, CreateAgentRequest, CreateProviderRequest, Provider, ProviderModel,
    ProviderWithModels, UpdateAgentRequest, UpdateProviderRequest, Chat, ChatMessagePair, ProviderType,
};

#[derive(Clone)]
pub struct ChatRepository {
    pub pool: Arc<SqlitePool>,
}

impl ChatRepository {
    pub async fn get_all_chats(&self, user_id: i64) -> sqlx::Result<Vec<Chat>> {
        sqlx::query_as!(
            Chat,
            "SELECT id, user_id, name FROM chats WHERE user_id = ? ORDER BY created_at DESC",
            user_id
        )
        .fetch_all(&*self.pool)
        .await
    }

    pub async fn delete_chat(&self, chat_id: i64) -> sqlx::Result<u64> {
        let result = sqlx::query("DELETE FROM chats WHERE id = ?")
            .bind(chat_id)
            .execute(&*self.pool)
            .await?;
        Ok(result.rows_affected())
    }

    pub async fn retrieve_chat(&self, chat_id: i64) -> sqlx::Result<Vec<ChatMessagePair>> {
        sqlx::query_as!(
            ChatMessagePair,
            "SELECT * FROM v_chat_messages WHERE chat_id = ?",
            chat_id
        )
        .fetch_all(&*self.pool)
        .await
    }
    pub async fn create_chat(&self, user_id: i64, name: &str, model: &str) -> sqlx::Result<i64> {
        //create chat
        let chat = sqlx::query!(
            r#"
            INSERT INTO chats (user_id, name, model)
            VALUES (?, ?, ?) RETURNING id;
            "#,
            user_id,
            name,
            model
        )
        .fetch_one(&*self.pool)
        .await?;

        Ok(chat.id.unwrap())
    }
    pub async fn add_ai_message_to_pair(&self, pair_id: i64, message: &str) -> sqlx::Result<i64> {
        let mut tx: Transaction<Sqlite> = self.pool.begin().await?;

        let message = sqlx::query!(
            r#"
            INSERT INTO messages (message)
            VALUES (?) RETURNING id;
            "#,
            message
        )
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query!(
            r#"
            UPDATE message_pairs
            SET ai_message_id = ?
            WHERE id = ?;
            "#,
            message.id,
            pair_id
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(message.id)
    }

    pub async fn add_message_block(&self, chat_id: i64, human_message: &str) -> sqlx::Result<i64> {
        //create chat
        let mut tx: Transaction<Sqlite> = self.pool.begin().await?;

        let message_block = sqlx::query!(
            r#"
            INSERT INTO message_blocks (chat_id)
            VALUES (?) RETURNING id;
            "#,
            chat_id,
        )
        .fetch_one(&mut *tx)
        .await?;

        let message = sqlx::query!(
            r#"
            INSERT INTO messages (message)
            VALUES (?) RETURNING id;
            "#,
            human_message
        )
        .fetch_one(&mut *tx)
        .await?;

        let message_pair = sqlx::query!(
            r#"
            INSERT INTO message_pairs (human_message_id, message_block_id)
            VALUES (?, ?) RETURNING id;
            "#,
            message.id,
            message_block.id,
        )
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query!(
            r#"
            UPDATE message_blocks
            SET selected_pair_id = ?
            WHERE id = ?;
            "#,
            message_pair.id,
            message_block.id
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(message_pair.id.unwrap())
    }

    // Provider CRUD operations
    pub async fn get_all_providers(&self) -> sqlx::Result<Vec<Provider>> {
        sqlx::query_as!(
            Provider,
            r#"
            SELECT
                id as "id!",
                name as "name!",
                provider_type as "provider_type: ProviderType",
                base_url as "base_url!",
                api_key_encrypted as "api_key_encrypted!",
                is_active as "is_active!",
                datetime(created_at) as "created_at!: String",
                datetime(updated_at) as "updated_at!: String"
            FROM providers
            ORDER BY created_at DESC
            "#
        )
        .fetch_all(&*self.pool)
        .await
    }

    pub async fn get_provider_by_id(&self, id: i64) -> sqlx::Result<Option<Provider>> {
        sqlx::query_as!(
            Provider,
            r#"
            SELECT
                id as "id!",
                name as "name!",
                provider_type as "provider_type: ProviderType",
                base_url as "base_url!",
                api_key_encrypted as "api_key_encrypted!",
                is_active as "is_active!",
                datetime(created_at) as "created_at!: String",
                datetime(updated_at) as "updated_at!: String"
            FROM providers
            WHERE id = ?
            "#,
            id
        )
        .fetch_optional(&*self.pool)
        .await
    }

    pub async fn get_provider_by_name(&self, name: &str) -> sqlx::Result<Option<Provider>> {
        sqlx::query_as!(
            Provider,
            r#"
            SELECT
                id as "id!",
                name as "name!",
                provider_type as "provider_type: ProviderType",
                base_url as "base_url!",
                api_key_encrypted as "api_key_encrypted!",
                is_active as "is_active!",
                datetime(created_at) as "created_at!: String",
                datetime(updated_at) as "updated_at!: String"
            FROM providers
            WHERE name = ?
            "#,
            name
        )
        .fetch_optional(&*self.pool)
        .await
    }

    pub async fn create_provider(&self, request: CreateProviderRequest) -> sqlx::Result<i64> {
        let provider_type_str = match request.provider_type {
            ProviderType::OpenAI => "openai",
            ProviderType::Gemini => "gemini",
        };

        let result = sqlx::query!(
            r#"
            INSERT INTO providers (name, provider_type, base_url, api_key_encrypted)
            VALUES (?, ?, ?, ?)
            "#,
            request.name,
            provider_type_str,
            request.base_url,
            request.api_key
        )
        .execute(&*self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    pub async fn update_provider(&self, id: i64, request: UpdateProviderRequest) -> sqlx::Result<u64> {
        let mut query = String::from("UPDATE providers SET updated_at = CURRENT_TIMESTAMP");
        let mut params = Vec::new();

        if let Some(name) = &request.name {
            query.push_str(", name = ?");
            params.push(name.clone());
        }
        if let Some(base_url) = &request.base_url {
            query.push_str(", base_url = ?");
            params.push(base_url.clone());
        }
        if let Some(api_key) = &request.api_key {
            query.push_str(", api_key_encrypted = ?");
            params.push(api_key.clone());
        }
        if let Some(is_active) = request.is_active {
            query.push_str(", is_active = ?");
            params.push(is_active.to_string());
        }

        query.push_str(" WHERE id = ?");

        let mut query_builder = sqlx::query(&query);
        for param in params {
            query_builder = query_builder.bind(param);
        }
        query_builder = query_builder.bind(id);

        let result = query_builder.execute(&*self.pool).await?;
        Ok(result.rows_affected())
    }

    pub async fn delete_provider(&self, id: i64) -> sqlx::Result<u64> {
        let result = sqlx::query("DELETE FROM providers WHERE id = ?")
            .bind(id)
            .execute(&*self.pool)
            .await?;
        Ok(result.rows_affected())
    }

    // Provider Model operations
    pub async fn get_models_by_provider(&self, provider_id: i64) -> sqlx::Result<Vec<ProviderModel>> {
        sqlx::query_as!(
            ProviderModel,
            r#"
            SELECT
                id as "id!",
                provider_id as "provider_id!",
                name as "name!",
                display_name as "display_name!",
                context_length as "context_length!",
                input_price,
                output_price,
                capabilities as "capabilities!",
                is_active as "is_active!",
                datetime(created_at) as "created_at!: String"
            FROM provider_models
            WHERE provider_id = ? AND is_active = TRUE
            ORDER BY display_name
            "#,
            provider_id
        )
        .fetch_all(&*self.pool)
        .await
    }

    pub async fn get_provider_with_models(&self, id: i64) -> sqlx::Result<Option<ProviderWithModels>> {
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
    pub async fn get_all_agents(&self) -> sqlx::Result<Vec<Agent>> {
        sqlx::query_as!(
            Agent,
            r#"
            SELECT
                id, user_id, name, description, provider_id, model_name, stream, chat, embed, image, tool,
                COALESCE(tools, '[]') as tools, system_prompt,
                COALESCE(top_p, 1.0) as top_p, COALESCE(max_context, 4096) as max_context, file,
                COALESCE(file_types, '[]') as file_types, COALESCE(temperature, 0.7) as temperature,
                COALESCE(max_tokens, 2048) as max_tokens, COALESCE(presence_penalty, 0.0) as presence_penalty,
                COALESCE(frequency_penalty, 0.0) as frequency_penalty, COALESCE(icon, '🤖') as icon,
                COALESCE(category, 'general') as category, public, is_active,
                datetime(created_at) as "created_at!: String",
                datetime(updated_at) as "updated_at!: String"
            FROM agents
            ORDER BY created_at DESC
            "#
        )
        .fetch_all(&*self.pool)
        .await
    }

    pub async fn get_agents_by_user(&self, user_id: i64) -> sqlx::Result<Vec<Agent>> {
        sqlx::query_as!(
            Agent,
            r#"
            SELECT
                id, user_id, name, description, provider_id, model_name, stream, chat, embed, image, tool,
                COALESCE(tools, '[]') as tools, system_prompt,
                COALESCE(top_p, 1.0) as top_p, COALESCE(max_context, 4096) as max_context, file,
                COALESCE(file_types, '[]') as file_types, COALESCE(temperature, 0.7) as temperature,
                COALESCE(max_tokens, 2048) as max_tokens, COALESCE(presence_penalty, 0.0) as presence_penalty,
                COALESCE(frequency_penalty, 0.0) as frequency_penalty, COALESCE(icon, '🤖') as icon,
                COALESCE(category, 'general') as category, public, is_active,
                datetime(created_at) as "created_at!: String",
                datetime(updated_at) as "updated_at!: String"
            FROM agents
            WHERE user_id = ? OR public = TRUE
            ORDER BY created_at DESC
            "#,
            user_id
        )
        .fetch_all(&*self.pool)
        .await
    }

    pub async fn get_agent_by_id(&self, id: i64) -> sqlx::Result<Option<Agent>> {
        sqlx::query_as!(
            Agent,
            r#"
            SELECT
                id, user_id, name, description, provider_id, model_name, stream, chat, embed, image, tool,
                COALESCE(tools, '[]') as tools, system_prompt,
                COALESCE(top_p, 1.0) as top_p, COALESCE(max_context, 4096) as max_context, file,
                COALESCE(file_types, '[]') as file_types, COALESCE(temperature, 0.7) as temperature,
                COALESCE(max_tokens, 2048) as max_tokens, COALESCE(presence_penalty, 0.0) as presence_penalty,
                COALESCE(frequency_penalty, 0.0) as frequency_penalty, COALESCE(icon, '🤖') as icon,
                COALESCE(category, 'general') as category, public, is_active,
                datetime(created_at) as "created_at!: String",
                datetime(updated_at) as "updated_at!: String"
            FROM agents
            WHERE id = ?
            "#,
            id
        )
        .fetch_optional(&*self.pool)
        .await
    }

    pub async fn get_agent_with_provider(&self, id: i64) -> sqlx::Result<Option<AgentWithProvider>> {
        // Let me get the agent and provider separately to avoid complex SQLx type issues
        let agent = sqlx::query!(
            r#"
            SELECT
                id, user_id, name, description, provider_id, model_name, stream, chat, embed, image, tool,
                COALESCE(tools, '[]') as tools, system_prompt,
                COALESCE(top_p, 1.0) as top_p, COALESCE(max_context, 4096) as max_context, file,
                COALESCE(file_types, '[]') as file_types, COALESCE(temperature, 0.7) as temperature,
                COALESCE(max_tokens, 2048) as max_tokens, COALESCE(presence_penalty, 0.0) as presence_penalty,
                COALESCE(frequency_penalty, 0.0) as frequency_penalty, COALESCE(icon, '🤖') as icon,
                COALESCE(category, 'general') as category, public, is_active,
                datetime(created_at) as "created_at!: String",
                datetime(updated_at) as "updated_at!: String"
            FROM agents
            WHERE id = ?
            "#,
            id
        )
        .fetch_optional(&*self.pool)
        .await?;

        if let Some(agent_row) = agent {
            let provider = self.get_provider_by_id(agent_row.provider_id).await?;

            if let Some(provider) = provider {
                let tools: Vec<String> = serde_json::from_str(&agent_row.tools).unwrap_or_default();
                let file_types: Vec<String> = serde_json::from_str(&agent_row.file_types).unwrap_or_default();

                Ok(Some(AgentWithProvider {
                    id: agent_row.id,
                    user_id: agent_row.user_id,
                    name: agent_row.name,
                    description: agent_row.description,
                    provider,
                    model_name: agent_row.model_name,
                    stream: agent_row.stream,
                    chat: agent_row.chat,
                    embed: agent_row.embed,
                    image: agent_row.image,
                    tool: agent_row.tool,
                    tools,
                    system_prompt: agent_row.system_prompt,
                    top_p: agent_row.top_p,
                    max_context: agent_row.max_context,
                    file: agent_row.file,
                    file_types,
                    temperature: agent_row.temperature,
                    max_tokens: agent_row.max_tokens,
                    presence_penalty: agent_row.presence_penalty,
                    frequency_penalty: agent_row.frequency_penalty,
                    icon: agent_row.icon,
                    category: agent_row.category,
                    public: agent_row.public,
                    is_active: agent_row.is_active,
                    created_at: agent_row.created_at,
                    updated_at: agent_row.updated_at,
                }))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    pub async fn create_agent(&self, user_id: i64, request: CreateAgentRequest) -> sqlx::Result<i64> {
        let tools_json = serde_json::to_string(&request.tools.unwrap_or_default()).unwrap_or_default();
        let file_types_json = serde_json::to_string(&request.file_types.unwrap_or_default()).unwrap_or_default();

        // Create bindings for values that need longer lifetimes
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

        let result: sqlx::sqlite::SqliteQueryResult = sqlx::query!(
            r#"
            INSERT INTO agents (user_id, name, description, provider_id, model_name, stream, chat, embed, image, tool,
                               tools, system_prompt, top_p, max_context, file, file_types, temperature, max_tokens,
                               presence_penalty, frequency_penalty, icon, category, public, is_active)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            user_id,
            request.name,
            request.description,
            request.provider_id,
            request.model_name,
            stream_val,
            chat_val,
            embed_val,
            image_val,
            tool_val,
            tools_json,
            request.system_prompt,
            top_p_val,
            max_context_val,
            file_val,
            file_types_json,
            temperature_val,
            max_tokens_val,
            presence_penalty_val,
            frequency_penalty_val,
            icon_val,
            category_val,
            public_val,
            true  // is_active defaults to true
        )
        .execute(&*self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    pub async fn update_agent(&self, id: i64, request: UpdateAgentRequest) -> sqlx::Result<u64> {
        let mut query = String::from("UPDATE agents SET updated_at = CURRENT_TIMESTAMP");
        let mut params = Vec::new();

        if let Some(name) = &request.name {
            query.push_str(", name = ?");
            params.push(name.clone());
        }
        if let Some(description) = &request.description {
            query.push_str(", description = ?");
            params.push(description.clone());
        }
        if let Some(provider_id) = request.provider_id {
            query.push_str(", provider_id = ?");
            params.push(provider_id.to_string());
        }
        if let Some(model_name) = &request.model_name {
            query.push_str(", model_name = ?");
            params.push(model_name.clone());
        }
        if let Some(stream) = request.stream {
            query.push_str(", stream = ?");
            params.push(stream.to_string());
        }
        if let Some(chat) = request.chat {
            query.push_str(", chat = ?");
            params.push(chat.to_string());
        }
        if let Some(embed) = request.embed {
            query.push_str(", embed = ?");
            params.push(embed.to_string());
        }
        if let Some(image) = request.image {
            query.push_str(", image = ?");
            params.push(image.to_string());
        }
        if let Some(tool) = request.tool {
            query.push_str(", tool = ?");
            params.push(tool.to_string());
        }
        if let Some(tools) = &request.tools {
            query.push_str(", tools = ?");
            params.push(serde_json::to_string(tools).unwrap_or_default());
        }
        if let Some(system_prompt) = &request.system_prompt {
            query.push_str(", system_prompt = ?");
            params.push(system_prompt.clone());
        }
        if let Some(top_p) = request.top_p {
            query.push_str(", top_p = ?");
            params.push(top_p.to_string());
        }
        if let Some(max_context) = request.max_context {
            query.push_str(", max_context = ?");
            params.push(max_context.to_string());
        }
        if let Some(file) = request.file {
            query.push_str(", file = ?");
            params.push(file.to_string());
        }
        if let Some(file_types) = &request.file_types {
            query.push_str(", file_types = ?");
            params.push(serde_json::to_string(file_types).unwrap_or_default());
        }
        if let Some(temperature) = request.temperature {
            query.push_str(", temperature = ?");
            params.push(temperature.to_string());
        }
        if let Some(max_tokens) = request.max_tokens {
            query.push_str(", max_tokens = ?");
            params.push(max_tokens.to_string());
        }
        if let Some(presence_penalty) = request.presence_penalty {
            query.push_str(", presence_penalty = ?");
            params.push(presence_penalty.to_string());
        }
        if let Some(frequency_penalty) = request.frequency_penalty {
            query.push_str(", frequency_penalty = ?");
            params.push(frequency_penalty.to_string());
        }
        if let Some(icon) = &request.icon {
            query.push_str(", icon = ?");
            params.push(icon.clone());
        }
        if let Some(category) = &request.category {
            query.push_str(", category = ?");
            params.push(category.clone());
        }
        if let Some(public) = request.public {
            query.push_str(", public = ?");
            params.push(public.to_string());
        }
        if let Some(is_active) = request.is_active {
            query.push_str(", is_active = ?");
            params.push(is_active.to_string());
        }

        query.push_str(" WHERE id = ?");

        let mut query_builder = sqlx::query(&query);
        for param in params {
            query_builder = query_builder.bind(param);
        }
        query_builder = query_builder.bind(id);

        let result = query_builder.execute(&*self.pool).await?;
        Ok(result.rows_affected())
    }

    pub async fn delete_agent(&self, id: i64) -> sqlx::Result<u64> {
        let result = sqlx::query("DELETE FROM agents WHERE id = ?")
            .bind(id)
            .execute(&*self.pool)
            .await?;
        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use sqlx::migrate::Migrator;

    use super::*;

    async fn setup() -> (Arc<SqlitePool>, ChatRepository, i64) {
        let x = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:db.db".to_string());
        let pool = SqlitePool::connect(&x).await.unwrap();

        // let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        let pool = Arc::new(pool);

        let migrator = Migrator::new(Path::new(dotenv::var("MIGRATIONS_PATH").unwrap().as_str()))
            .await
            .unwrap();
        // Run the migrations.
        migrator.run(&*pool).await.unwrap();

        let user = sqlx::query!(
            "INSERT INTO users (email, password) VALUES (?, ?)",
            "test@test.com",
            "test"
        )
        .execute(&*pool)
        .await
        .unwrap();

        let repo = ChatRepository { pool: pool.clone() };

        (pool, repo, user.last_insert_rowid())
    }

    #[tokio::test]
    async fn test_create_chat() {
        let (pool, repo, user_id) = setup().await;
        let chat = repo.create_chat(user_id, "test", "gpt-4").await;
        assert!(chat.is_ok(), "Failed to create chat");
    }

    #[tokio::test]
    async fn test_add_message_block() {
        let (pool, repo, user_id) = setup().await;
        let chat = repo.create_chat(user_id, "test", "gpt-4").await;
        assert!(chat.is_ok(), "Failed to create chat");
        let chat_id = chat.unwrap();

        let message_block = repo.add_message_block(chat_id, "Test").await;
        assert!(message_block.is_ok(), "Failed to add message_block")
    }

    #[tokio::test]
    async fn test_json() {
        let (pool, repo, user_id) = setup().await;
        let chat = repo.create_chat(user_id, "test", "gpt-4").await;
        assert!(chat.is_ok(), "Failed to create chat");
        let chat_id = chat.unwrap();

        let message_block = repo.add_message_block(chat_id, "Test").await;
        assert!(message_block.is_ok(), "Failed to add message_block");

        let chat_message_pairs = repo.retrieve_chat(chat_id).await;
        print!("{:#?}", chat_message_pairs)
    }
}
