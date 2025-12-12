-- Add models_endpoint column to providers table
-- This migration adds an API endpoint field to fetch available models from providers

ALTER TABLE providers ADD COLUMN models_endpoint TEXT;

-- Update existing providers with default models endpoints
UPDATE providers SET
    models_endpoint = CASE provider_type
        WHEN 'openai' THEN '/models'
        WHEN 'openrouter' THEN '/models'
        WHEN 'deepseek' THEN '/models'
        WHEN 'azure' THEN '/models?api-version=2024-02-15-preview'
        WHEN 'anthropic' THEN '/messages'
        WHEN 'cohere' THEN '/models'
        WHEN 'groq' THEN '/models'
        WHEN 'mistral' THEN '/models'
        WHEN 'gemini' THEN '/models'
        WHEN 'huggingface' THEN '/models'
        WHEN 'xai' THEN '/models'
        ELSE NULL
    END
WHERE models_endpoint IS NULL;

-- Create index for models_endpoint
CREATE INDEX idx_providers_models_endpoint ON providers(models_endpoint) WHERE models_endpoint IS NOT NULL;