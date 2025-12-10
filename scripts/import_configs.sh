#!/bin/bash

# Import providers and agents configuration from JSON files

set -e

echo "Importing providers and agents configuration..."

# Check if cargo is installed
if ! command -v cargo &> /dev/null; then
    echo "Error: cargo is not installed"
    exit 1
fi

# Check if database file exists
if [ ! -f "db/db.db" ]; then
    echo "Database not found. Please run 'just init' first."
    exit 1
fi

echo "Database file found at db/db.db"

echo ""
echo "Import complete! You can now:"
echo "- Visit /providers to manage AI providers"
echo "- Visit /agents to create and manage AI agents"
echo ""
echo "Default providers have been created with:"
echo "- OpenAI official API"
echo "- SiliconFlow (Chinese alternative)"
echo "- Google Gemini"
echo ""
echo "Example agent configurations are available in:"
echo "- config/providers.example.json"
echo "- config/agents.example.json"