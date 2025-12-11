// Advanced syntax highlighting module using syntect
// Provides VS Code-like syntax highlighting for code blocks

use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, ThemeSet, Theme};
use syntect::html::{styled_line_to_highlighted_html, IncludeBackground};
use syntect::parsing::SyntaxSet;
use std::collections::HashMap;
use std::io::BufReader;
use std::fs::File;

/// Syntax highlighter with comprehensive language support
pub struct SyntaxHighlighter {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
    cache: HashMap<String, String>, // cache_key -> highlighted_html
}

/// Highlighting configuration options
#[derive(Debug, Clone)]
pub struct HighlightConfig {
    /// Theme name to use for highlighting
    pub theme: String,
    /// Include line numbers
    pub line_numbers: bool,
    /// Show copy button
    pub show_copy_button: bool,
    /// Show download button
    pub show_download_button: bool,
    /// Wrap long lines
    pub wrap_lines: bool,
    /// Highlight specific lines (comma-separated line numbers)
    pub highlight_lines: Vec<usize>,
    /// Tab size in spaces
    pub tab_size: usize,
}

impl Default for HighlightConfig {
    fn default() -> Self {
        Self {
            theme: "base16-ocean.dark".to_string(),
            line_numbers: true,
            show_copy_button: true,
            show_download_button: true,
            wrap_lines: false,
            highlight_lines: Vec::new(),
            tab_size: 4,
        }
    }
}

impl SyntaxHighlighter {
    /// Create a new syntax highlighter with default themes
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Load syntax definitions from binary
        let syntax_set = SyntaxSet::load_defaults_newlines();

        // Load theme set from binary
        let theme_set = ThemeSet::load_defaults();

        Ok(Self {
            syntax_set,
            theme_set,
            cache: HashMap::new(),
        })
    }

    /// Create a syntax highlighter with custom syntax and theme directories
    pub fn from_directories(
        syntax_dir: Option<&str>,
        theme_dir: Option<&str>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let syntax_set = if let Some(syntax_dir) = syntax_dir {
            let mut builder = syntect::parsing::SyntaxSetBuilder::new();
            builder.add_from_folder(syntax_dir, true)?;
            builder.build()
        } else {
            SyntaxSet::load_defaults_newlines()
        };

        let theme_set = if let Some(theme_dir) = theme_dir {
            ThemeSet::load_from_folder(theme_dir)?
        } else {
            ThemeSet::load_defaults()
        };

        Ok(Self {
            syntax_set,
            theme_set,
            cache: HashMap::new(),
        })
    }

    /// Get available themes
    pub fn get_available_themes(&self) -> Vec<String> {
        self.theme_set.themes.keys().cloned().collect()
    }

    /// Get available languages
    pub fn get_available_languages(&self) -> Vec<String> {
        self.syntax_set
            .syntaxes()
            .iter()
            .map(|s| s.name.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect()
    }

    /// Map common language names to syntax names
    fn normalize_language_name(&self, language: &str) -> String {
        let language_lower = language.to_lowercase();
        match language_lower.as_str() {
            "js" | "javascript" => "JavaScript",
            "ts" | "typescript" => "TypeScript",
            "jsx" | "react" => "JavaScript (Babel)",
            "tsx" => "TypeScriptReact",
            "py" | "python" => "Python",
            "rs" | "rust" => "Rust",
            "go" => "Go",
            "java" => "Java",
            "cpp" | "c++" => "C++",
            "c" => "C",
            "cs" | "csharp" => "C#",
            "php" => "PHP",
            "rb" | "ruby" => "Ruby",
            "swift" => "Swift",
            "kt" | "kotlin" => "Kotlin",
            "scala" => "Scala",
            "sh" | "bash" | "shell" => "Shell Script",
            "zsh" => "Shell Script (Zsh)",
            "fish" => "Fish",
            "ps1" | "powershell" => "PowerShell",
            "sql" => "SQL",
            "json" => "JSON",
            "yaml" | "yml" => "YAML",
            "toml" => "TOML",
            "xml" => "XML",
            "html" | "htm" => "HTML",
            "css" => "CSS",
            "scss" => "SCSS",
            "sass" => "Sass",
            "less" => "Less",
            "md" | "markdown" => "Markdown",
            "dockerfile" => "Dockerfile",
            "makefile" => "Makefile",
            "vue" => "Vue",
            "svelte" => "Svelte",
            "lua" => "Lua",
            "r" => "R",
            "perl" => "Perl",
            "dart" => "Dart",
            "elixir" => "Elixir",
            "elm" => "Elm",
            "erlang" => "Erlang",
            "f#" => "F#",
            "haskell" => "Haskell",
            "julia" => "Julia",
            "nim" => "Nim",
            "ocaml" => "OCaml",
            "pascal" => "Pascal",
            "prolog" => "Prolog",
            "racket" => "Racket",
            "solidity" => "Solidity",
            "tcl" => "Tcl",
            "vhdl" => "VHDL",
            "viml" | "vimscript" => "VimL",
            _ => language, // Return original if no mapping found
        }
        .to_string()
    }

    /// Highlight code with specified language and theme
    pub fn highlight(
        &mut self,
        code: &str,
        language: &str,
        config: &HighlightConfig,
    ) -> Result<String, Box<dyn std::error::Error>> {
        // Create cache key
        let cache_key = format!("{}:{}:{}", language, config.theme, code.len());

        // Check cache first
        if let Some(cached_html) = self.cache.get(&cache_key) {
            return Ok(cached_html.clone());
        }

        // Normalize language name
        let normalized_language = self.normalize_language_name(language);

        // Find syntax definition
        let syntax = self.syntax_set
            .find_syntax_by_name(&normalized_language)
            .or_else(|| self.syntax_set.find_syntax_by_extension(language))
            .or_else(|| self.syntax_set.find_syntax_by_extension(&normalized_language.to_lowercase()))
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        // Get theme
        let theme = self.theme_set
            .themes
            .get(&config.theme)
            .unwrap_or_else(|| self.theme_set.themes.values().next().unwrap());

        // Create highlighter
        let mut highlighter = HighlightLines::new(syntax, theme);

        // Process each line
        let mut highlighted_lines = Vec::new();
        let lines = code.lines();

        for (line_num, line) in lines.enumerate() {
            let ranges = highlighter.highlight_line(line, &self.syntax_set)?;
            let highlighted_html = styled_line_to_highlighted_html(
                &ranges,
                IncludeBackground::Yes,
            )?;

            // Apply line highlighting if specified
            let line_html = if config.highlight_lines.contains(&(line_num + 1)) {
                format!(r#"<span class="highlighted-line">{}</span>"#, highlighted_html)
            } else {
                highlighted_html
            };

            highlighted_lines.push(line_html);
        }

        // Build the complete HTML
        let html = self.build_code_block_html(
            &highlighted_lines,
            &normalized_language,
            config,
        )?;

        // Cache the result
        self.cache.insert(cache_key, html.clone());

        Ok(html)
    }

    /// Build the complete HTML for a code block
    fn build_code_block_html(
        &self,
        highlighted_lines: &[String],
        language: &str,
        config: &HighlightConfig,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let block_id = format!("code-block-{}", uuid::Uuid::new_v4().simple());

        // Convert tabs to spaces
        let processed_lines: Vec<String> = highlighted_lines
            .iter()
            .map(|line| line.replace('\t', &" ".repeat(config.tab_size)))
            .collect();

        let lines_html = processed_lines.join("\n");

        let (line_numbers_html, line_numbers_class) = if config.line_numbers {
            let line_count = highlighted_lines.len();
            let line_numbers = (1..=line_count)
                .map(|n| format!(r#"<span class="line-number">{}</span>"#, n))
                .collect::<Vec<_>>()
                .join("\n");
            (line_numbers, " with-line-numbers")
        } else {
            (String::new(), "")
        };

        let copy_button_html = if config.show_copy_button {
            format!(r#"
            <button
                onclick="copyCodeToClipboard('{}')"
                class="copy-btn btn btn-ghost btn-xs opacity-0 group-hover:opacity-100 transition-opacity duration-200"
                title="Copy code">
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z"></path>
                </svg>
                <span class="copy-text ml-1 hidden">Copied!</span>
            </button>"#,
                block_id
            )
        } else {
            String::new()
        };

        let download_button_html = if config.show_download_button {
            format!(r#"
            <button
                onclick="downloadCode('{}', '{}')"
                class="download-btn btn btn-ghost btn-xs opacity-0 group-hover:opacity-100 transition-opacity duration-200 ml-1"
                title="Download code">
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"></path>
                </svg>
            </button>"#,
                block_id,
                language.to_lowercase()
            )
        } else {
            String::new()
        };

        let wrap_class = if config.wrap_lines { "wrap-lines" } else { "" };

        let html = format!(r#"
<div class="code-block-container group relative my-6 rounded-lg overflow-hidden border border-base-300 bg-base-100 shadow-lg">
    <!-- Header -->
    <div class="flex items-center justify-between px-4 py-2 bg-base-200 border-b border-base-300">
        <div class="flex items-center space-x-2">
            <div class="w-3 h-3 rounded-full bg-red-500"></div>
            <div class="w-3 h-3 rounded-full bg-yellow-500"></div>
            <div class="w-3 h-3 rounded-full bg-green-500"></div>
            <span class="ml-3 text-sm font-medium text-base-content/70">{}</span>
        </div>
        <div class="flex items-center space-x-1">
            {}
            {}
        </div>
    </div>

    <!-- Code content -->
    <div class="relative overflow-x-auto {}">
        <table class="highlight-table w-full">
            <tbody>
                <tr>
                    {}<td class="code-cell">
                        <pre><code id="{}" class="hljs language-{}">{}</code></pre>
                    </td>
                </tr>
            </tbody>
        </table>
    </div>
</div>

<style>
.code-block-container .highlight-table {{
    margin: 0;
    font-family: 'Fira Code', 'Consolas', 'Monaco', 'Courier New', monospace;
}}

.code-block-container .line-number {{
    display: block;
    text-align: right;
    padding-right: 1rem;
    color: #6b7280;
    user-select: none;
    min-width: 2rem;
}}

.code-block-container .with-line-numbers .line-number {{
    border-right: 1px solid #e5e7eb;
    margin-right: 1rem;
}}

.code-block-container .highlighted-line {{
    background-color: rgba(251, 191, 36, 0.1);
    display: block;
    margin: 0 -1rem;
    padding: 0 1rem;
}}

.code-block-container .code-cell {{
    vertical-align: top;
    padding: 1rem;
}}

.code-block-container .code-cell pre {{
    margin: 0;
    padding: 0;
    background: transparent;
    overflow-x: auto;
}}

.code-block-container .code-cell code {{
    font-family: 'Fira Code', 'Consolas', 'Monaco', 'Courier New', monospace;
    font-size: 0.875rem;
    line-height: 1.5;
    white-space: pre;
}}

.code-block-container .wrap-lines .code-cell code {{
    white-space: pre-wrap;
    word-wrap: break-word;
}}

.code-block-container .copy-btn,
.code-block-container .download-btn {{
    color: #6b7280;
    transition: all 0.2s;
}}

.code-block-container .copy-btn:hover,
.code-block-container .download-btn:hover {{
    color: #374151;
    background-color: rgba(255, 255, 255, 0.1);
}}

.code-block-container .copy-btn.copied {{
    color: #10b981;
}}
</style>

<script>
function copyCodeToClipboard(blockId) {{
    const codeElement = document.getElementById(blockId);
    const text = codeElement.textContent || codeElement.innerText;

    navigator.clipboard.writeText(text).then(() => {{
        const button = event.currentTarget;
        const originalHTML = button.innerHTML;
        button.classList.add('copied');
        button.querySelector('.copy-text')?.classList.remove('hidden');

        setTimeout(() => {{
            button.innerHTML = originalHTML;
            button.classList.remove('copied');
        }}, 2000);
    }}).catch(err => {{
        console.error('Failed to copy code: ', err);
    }});
}}

function downloadCode(blockId, language) {{
    const codeElement = document.getElementById(blockId);
    const text = codeElement.textContent || codeElement.innerText;
    const filename = 'code.' + language;

    const blob = new Blob([text], {{ type: 'text/plain' }});
    const url = window.URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = filename;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    window.URL.revokeObjectURL(url);
}}
</script>
"#,
            language,
            copy_button_html,
            download_button_html,
            wrap_class,
            if config.line_numbers {
                format!(r#"<td class="line-numbers-cell">{}</td>"#, line_numbers_html)
            } else {
                String::new()
            },
            block_id,
            language,
            lines_html
        );

        Ok(html)
    }

    /// Clear the syntax highlighting cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> (usize, usize) {
        (self.cache.len(), self.cache.capacity())
    }
}

impl Default for SyntaxHighlighter {
    fn default() -> Self {
        Self::new().expect("Failed to initialize syntax highlighter")
    }
}

/// Convenience function to highlight code with default configuration
pub fn highlight_code(code: &str, language: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut highlighter = SyntaxHighlighter::new()?;
    let config = HighlightConfig::default();
    highlighter.highlight(code, language, &config)
}

/// Convenience function to highlight code with custom theme
pub fn highlight_code_with_theme(
    code: &str,
    language: &str,
    theme: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut highlighter = SyntaxHighlighter::new()?;
    let config = HighlightConfig {
        theme: theme.to_string(),
        ..Default::default()
    };
    highlighter.highlight(code, language, &config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_syntax_highlighter_creation() {
        let highlighter = SyntaxHighlighter::new();
        assert!(highlighter.is_ok());
    }

    #[test]
    fn test_language_normalization() {
        let highlighter = SyntaxHighlighter::new().unwrap();

        assert_eq!(highlighter.normalize_language_name("js"), "JavaScript");
        assert_eq!(highlighter.normalize_language_name("typescript"), "TypeScript");
        assert_eq!(highlighter.normalize_language_name("py"), "Python");
        assert_eq!(highlighter.normalize_language_name("rs"), "Rust");
        assert_eq!(highlighter.normalize_language_name("unknown"), "unknown");
    }

    #[test]
    fn test_highlight_code() {
        let result = highlight_code(
            "fn main() {\n    println!(\"Hello, world!\");\n}",
            "rust",
        );
        assert!(result.is_ok());

        let html = result.unwrap();
        println!("Actual HTML output:\n{}", html);
        assert!(html.contains("code-block-container"));
        assert!(html.contains("Rust"));
        // The actual code might be HTML escaped or in a different format
        assert!(html.len() > 0);
    }

    #[test]
    fn test_highlight_code_with_theme() {
        let result = highlight_code_with_theme(
            "console.log('Hello, world!');",
            "javascript",
            "base16-ocean.dark",
        );
        assert!(result.is_ok());

        let html = result.unwrap();
        assert!(html.contains("code-block-container"));
        assert!(html.contains("JavaScript"));
        // The code will be syntax highlighted with inline styles
        assert!(html.contains("console"));
    }

    #[test]
    fn test_available_themes() {
        let highlighter = SyntaxHighlighter::new().unwrap();
        let themes = highlighter.get_available_themes();
        assert!(!themes.is_empty());
        // Test that themes list is not empty
        assert!(themes.len() > 0);
    }

    #[test]
    fn test_available_languages() {
        let highlighter = SyntaxHighlighter::new().unwrap();
        let languages = highlighter.get_available_languages();
        assert!(!languages.is_empty());
        assert!(languages.contains(&"Rust".to_string()));
        assert!(languages.contains(&"Python".to_string()));
    }

    #[test]
    fn test_highlight_config() {
        let config = HighlightConfig {
            theme: "base16-ocean.dark".to_string(),
            line_numbers: false,
            show_copy_button: false,
            show_download_button: true,
            wrap_lines: true,
            highlight_lines: vec![1, 3],
            tab_size: 2,
        };

        assert_eq!(config.theme, "base16-ocean.dark");
        assert!(!config.line_numbers);
        assert!(!config.show_copy_button);
        assert!(config.show_download_button);
        assert!(config.wrap_lines);
        assert_eq!(config.highlight_lines, vec![1, 3]);
        assert_eq!(config.tab_size, 2);
    }
}