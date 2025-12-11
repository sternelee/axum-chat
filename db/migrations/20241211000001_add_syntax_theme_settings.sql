-- Add syntax highlighting theme preferences to settings table
ALTER TABLE settings ADD COLUMN syntax_theme TEXT NOT NULL DEFAULT 'base16-ocean.dark';

-- Add UI preferences for markdown rendering
ALTER TABLE settings ADD COLUMN code_line_numbers BOOLEAN NOT NULL DEFAULT 1;
ALTER TABLE settings ADD COLUMN code_wrap_lines BOOLEAN NOT NULL DEFAULT 0;
ALTER TABLE settings ADD COLUMN enhanced_markdown BOOLEAN NOT NULL DEFAULT 1;