-- Recreate the view with extended fields
DROP VIEW IF EXISTS v_chat_messages;
CREATE VIEW v_chat_messages AS
SELECT
  message_pairs.id,
  message_block_id,
  message_blocks.chat_id AS chat_id,
  chats.model AS model,
  human_message.message AS human_message,
  ai_message.message AS ai_message,
  ai_message.thinking AS thinking,
  ai_message.tool_calls AS tool_calls,
  ai_message.images AS images,
  ai_message.reasoning AS reasoning,
  ai_message.usage_prompt_tokens AS usage_prompt_tokens,
  ai_message.usage_completion_tokens AS usage_completion_tokens,
  ai_message.usage_total_tokens AS usage_total_tokens,
  ai_message.sources AS sources,
  RANK() OVER (
    PARTITION BY message_block_id
    ORDER BY
      message_pairs.created_at ASC
  ) AS block_rank,
  COUNT(*) OVER (PARTITION BY message_block_id) AS block_size
FROM
  message_pairs
  JOIN messages human_message ON human_message.id = message_pairs.human_message_id
  LEFT JOIN messages ai_message ON ai_message.id = message_pairs.ai_message_id
  JOIN message_blocks ON message_blocks.id = message_pairs.message_block_id
  JOIN chats ON chats.id = message_blocks.chat_id;