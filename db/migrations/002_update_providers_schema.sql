-- Update providers table to support multiple service types per provider
-- This migration adds endpoint fields and capability flags to the providers table

-- Add new columns for service endpoints
ALTER TABLE providers ADD COLUMN chat_endpoint TEXT;
ALTER TABLE providers ADD COLUMN embed_endpoint TEXT;
ALTER TABLE providers ADD COLUMN image_endpoint TEXT;

-- Add capability flags
ALTER TABLE providers ADD COLUMN support_chat BOOLEAN DEFAULT TRUE;
ALTER TABLE providers ADD COLUMN support_embed BOOLEAN DEFAULT FALSE;
ALTER TABLE providers ADD COLUMN support_image BOOLEAN DEFAULT FALSE;
ALTER TABLE providers ADD COLUMN support_streaming BOOLEAN DEFAULT TRUE;
ALTER TABLE providers ADD COLUMN support_tools BOOLEAN DEFAULT TRUE;
ALTER TABLE providers ADD COLUMN support_images BOOLEAN DEFAULT FALSE;

-- Update existing providers with default endpoints and capabilities
UPDATE providers SET
    chat_endpoint = CASE provider_type
        WHEN 'openai' THEN '/chat/completions'
        WHEN 'openrouter' THEN '/chat/completions'
        WHEN 'deepseek' THEN '/chat/completions'
        WHEN 'azure' THEN '/chat/completions?api-version=2024-02-15-preview'
        WHEN 'anthropic' THEN '/messages'
        WHEN 'cohere' THEN '/chat'
        WHEN 'groq' THEN '/chat/completions'
        WHEN 'mistral' THEN '/chat/completions'
        WHEN 'gemini' THEN '/models/{model}:generateContent'
        WHEN 'huggingface' THEN '/models/{model}/v1/chat/completions'
        WHEN 'xai' THEN '/chat/completions'
        ELSE NULL
    END,
    embed_endpoint = CASE provider_type
        WHEN 'openai' THEN '/embeddings'
        WHEN 'openrouter' THEN '/embeddings'
        WHEN 'deepseek' THEN '/embeddings'
        WHEN 'azure' THEN '/embeddings?api-version=2024-02-15-preview'
        WHEN 'cohere' THEN '/embed'
        WHEN 'mistral' THEN '/embeddings'
        WHEN 'gemini' THEN '/models/{model}:embedContent'
        WHEN 'huggingface' THEN '/pipeline/feature-extraction'
        ELSE NULL
    END,
    image_endpoint = CASE provider_type
        WHEN 'openai' THEN '/images/generations'
        WHEN 'openrouter' THEN '/images/generations'
        WHEN 'azure' THEN '/images/generations?api-version=2024-02-15-preview'
        ELSE NULL
    END,
    support_chat = CASE provider_type
        WHEN 'dalle' OR 'midjourney' OR 'stability' OR 'openai-embed' OR 'cohere-embed' OR 'huggingface-embed' THEN FALSE
        ELSE TRUE
    END,
    support_embed = CASE provider_type
        WHEN 'openai-embed' OR 'cohere-embed' OR 'huggingface-embed' THEN TRUE
        WHEN 'openai' OR 'openrouter' OR 'deepseek' OR 'azure' OR 'cohere' OR 'mistral' OR 'gemini' OR 'huggingface' THEN TRUE
        ELSE FALSE
    END,
    support_image = CASE provider_type
        WHEN 'dalle' OR 'midjourney' OR 'stability' THEN TRUE
        WHEN 'openai' OR 'openrouter' OR 'azure' THEN TRUE
        ELSE FALSE
    END,
    support_streaming = CASE provider_type
        WHEN 'dalle' OR 'midjourney' OR 'stability' OR 'openai-embed' OR 'cohere-embed' OR 'huggingface-embed' THEN FALSE
        ELSE TRUE
    END,
    support_tools = CASE provider_type
        WHEN 'dalle' OR 'midjourney' OR 'stability' OR 'openai-embed' OR 'cohere-embed' OR 'huggingface-embed' OR 'xai' OR 'huggingface' THEN FALSE
        ELSE TRUE
    END,
    support_images = CASE provider_type
        WHEN 'deepseek' OR 'cohere' OR 'groq' OR 'mistral' OR 'huggingface' OR 'openai-embed' OR 'cohere-embed' OR 'huggingface-embed' THEN FALSE
        ELSE TRUE
    END;

-- Remove old service-specific providers and convert them to multi-service providers
-- Convert DALL-E 3 providers to OpenAI providers with image generation capability
UPDATE providers SET
    provider_type = 'openai',
    support_chat = TRUE,
    support_embed = TRUE,
    support_image = TRUE,
    support_tools = TRUE,
    support_streaming = TRUE
WHERE provider_type = 'dalle';

-- Convert Midjourney providers (this would need special handling as Midjourney has different API)
-- For now, we'll set them as inactive and require manual configuration
UPDATE providers SET
    is_active = FALSE,
    name = name || ' (Deprecated - please recreate as OpenAI provider with image generation)'
WHERE provider_type = 'midjourney';

-- Convert Stability AI providers
UPDATE providers SET
    provider_type = 'openrouter',
    support_chat = FALSE,
    support_embed = FALSE,
    support_image = TRUE,
    support_tools = FALSE,
    support_streaming = FALSE,
    name = name || ' (Stability AI via OpenRouter)'
WHERE provider_type = 'stability';

-- Convert embedding providers to their chat counterparts
UPDATE providers SET
    provider_type = 'openai',
    support_chat = TRUE,
    support_embed = TRUE,
    support_image = TRUE,
    support_tools = TRUE,
    support_streaming = TRUE,
    name = name || ' (Multi-service)'
WHERE provider_type = 'openai-embed';

UPDATE providers SET
    provider_type = 'cohere',
    support_chat = TRUE,
    support_embed = TRUE,
    support_image = FALSE,
    support_tools = TRUE,
    support_streaming = TRUE,
    name = name || ' (Multi-service)'
WHERE provider_type = 'cohere-embed';

UPDATE providers SET
    provider_type = 'huggingface',
    support_chat = TRUE,
    support_embed = TRUE,
    support_image = FALSE,
    support_tools = FALSE,
    support_streaming = TRUE,
    name = name || ' (Multi-service)'
WHERE provider_type = 'huggingface-embed';

-- Create indexes for performance
CREATE INDEX idx_providers_support_chat ON providers(support_chat);
CREATE INDEX idx_providers_support_embed ON providers(support_embed);
CREATE INDEX idx_providers_support_image ON providers(support_image);
CREATE INDEX idx_providers_is_active ON providers(is_active, provider_type);