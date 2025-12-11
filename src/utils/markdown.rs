// Enhanced Markdown rendering system inspired by Streamdown
// Features: syntax highlighting, copy buttons, enhanced styling with TailwindCSS + DaisyUI

use html_escape;
use regex::Regex;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use ::markdown::to_html;
use crate::utils::syntax::{SyntaxHighlighter, HighlightConfig};

/// Enhanced Markdown renderer with Streamdown-inspired features
pub struct EnhancedMarkdownRenderer {
    language_map: HashMap<String, String>,
    theme_colors: ThemeColors,
    syntax_highlighter: Arc<Mutex<SyntaxHighlighter>>,
}

/// Color theme for syntax highlighting
#[derive(Debug, Clone)]
pub struct ThemeColors {
    pub bg_primary: String,
    pub bg_secondary: String,
    pub text_primary: String,
    pub text_secondary: String,
    pub accent: String,
    pub border: String,
}

/// Enhanced markdown rendering options
#[derive(Debug, Clone)]
pub struct MarkdownOptions {
    pub syntax_theme: String,
    pub line_numbers: bool,
    pub wrap_lines: bool,
    pub copy_button: bool,
    pub download_button: bool,
    pub highlight_lines: Vec<usize>,
}

impl Default for MarkdownOptions {
    fn default() -> Self {
        Self {
            syntax_theme: "Material".to_string(),
            line_numbers: true,
            wrap_lines: false,
            copy_button: true,
            download_button: true,
            highlight_lines: Vec::new(),
        }
    }
}

impl Default for ThemeColors {
    fn default() -> Self {
        Self {
            bg_primary: "base-100".to_string(),
            bg_secondary: "base-200".to_string(),
            text_primary: "base-content".to_string(),
            text_secondary: "base-content/70".to_string(),
            accent: "primary".to_string(),
            border: "base-300".to_string(),
        }
    }
}

impl EnhancedMarkdownRenderer {
    /// Create a new enhanced markdown renderer
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let mut renderer = Self {
            language_map: HashMap::new(),
            theme_colors: ThemeColors::default(),
            syntax_highlighter: Arc::new(Mutex::new(SyntaxHighlighter::new()?)),
        };

        // Initialize language display names
        renderer.init_language_map();
        Ok(renderer)
    }

    /// Create a new enhanced markdown renderer with custom options
    pub fn with_options(options: MarkdownOptions) -> Result<Self, Box<dyn std::error::Error>> {
        let mut renderer = Self {
            language_map: HashMap::new(),
            theme_colors: ThemeColors::default(),
            syntax_highlighter: Arc::new(Mutex::new(SyntaxHighlighter::new()?)),
        };

        renderer.init_language_map();
        Ok(renderer)
    }

    /// Initialize language mapping for better display names
    fn init_language_map(&mut self) {
        self.language_map.insert("js".to_string(), "JavaScript".to_string());
        self.language_map.insert("ts".to_string(), "TypeScript".to_string());
        self.language_map.insert("jsx".to_string(), "React JSX".to_string());
        self.language_map.insert("tsx".to_string(), "React TSX".to_string());
        self.language_map.insert("py".to_string(), "Python".to_string());
        self.language_map.insert("rs".to_string(), "Rust".to_string());
        self.language_map.insert("go".to_string(), "Go".to_string());
        self.language_map.insert("java".to_string(), "Java".to_string());
        self.language_map.insert("cpp".to_string(), "C++".to_string());
        self.language_map.insert("c".to_string(), "C".to_string());
        self.language_map.insert("cs".to_string(), "C#".to_string());
        self.language_map.insert("php".to_string(), "PHP".to_string());
        self.language_map.insert("rb".to_string(), "Ruby".to_string());
        self.language_map.insert("swift".to_string(), "Swift".to_string());
        self.language_map.insert("kt".to_string(), "Kotlin".to_string());
        self.language_map.insert("scala".to_string(), "Scala".to_string());
        self.language_map.insert("sh".to_string(), "Shell".to_string());
        self.language_map.insert("bash".to_string(), "Bash".to_string());
        self.language_map.insert("zsh".to_string(), "Zsh".to_string());
        self.language_map.insert("fish".to_string(), "Fish".to_string());
        self.language_map.insert("ps1".to_string(), "PowerShell".to_string());
        self.language_map.insert("sql".to_string(), "SQL".to_string());
        self.language_map.insert("json".to_string(), "JSON".to_string());
        self.language_map.insert("yaml".to_string(), "YAML".to_string());
        self.language_map.insert("toml".to_string(), "TOML".to_string());
        self.language_map.insert("xml".to_string(), "XML".to_string());
        self.language_map.insert("html".to_string(), "HTML".to_string());
        self.language_map.insert("css".to_string(), "CSS".to_string());
        self.language_map.insert("scss".to_string(), "SCSS".to_string());
        self.language_map.insert("sass".to_string(), "Sass".to_string());
        self.language_map.insert("less".to_string(), "Less".to_string());
        self.language_map.insert("md".to_string(), "Markdown".to_string());
        self.language_map.insert("dockerfile".to_string(), "Dockerfile".to_string());
        self.language_map.insert("makefile".to_string(), "Makefile".to_string());
        self.language_map.insert("vue".to_string(), "Vue".to_string());
        self.language_map.insert("svelte".to_string(), "Svelte".to_string());
    }

    /// Convert markdown to enhanced HTML with Streamdown-inspired styling
    pub fn render(&self, markdown: &str) -> String {
        // First convert markdown to HTML
        let html = self.markdown_to_html(markdown);

        // Then enhance with advanced styling
        self.enhance_html(&html)
    }

    /// Convert markdown to enhanced HTML with custom options
    pub fn render_with_options(&self, markdown: &str, options: &MarkdownOptions) -> String {
        // First convert markdown to HTML
        let html = self.markdown_to_html(markdown);

        // Then enhance with custom options
        self.enhance_html_with_options(&html, options)
    }

    /// Basic markdown to HTML conversion
    fn markdown_to_html(&self, markdown: &str) -> String {
        to_html(markdown)
    }

    /// Enhance HTML with advanced styling inspired by Streamdown
    fn enhance_html(&self, html: &str) -> String {
        let mut enhanced = html.to_string();

        // Process code blocks first (most complex transformation)
        enhanced = self.enhance_code_blocks(&enhanced);

        // Process inline code
        enhanced = self.enhance_inline_code(&enhanced);

        // Enhance headings with anchor links
        enhanced = self.enhance_headings(&enhanced);

        // Enhance tables
        enhanced = self.enhance_tables(&enhanced);

        // Enhance lists
        enhanced = self.enhance_lists(&enhanced);

        // Enhance blockquotes
        enhanced = self.enhance_blockquotes(&enhanced);

        // Enhance links and buttons
        enhanced = self.enhance_links(&enhanced);

        // Enhance task lists
        enhanced = self.enhance_task_lists(&enhanced);

        // Enhance paragraphs
        enhanced = self.enhance_paragraphs(&enhanced);

        // Add responsive containers
        self.wrap_with_responsive_container(enhanced)
    }

    /// Enhance HTML with custom options
    fn enhance_html_with_options(&self, html: &str, options: &MarkdownOptions) -> String {
        let mut enhanced = html.to_string();

        // Process code blocks with custom options
        enhanced = self.enhance_code_blocks_with_options(&enhanced, options);

        // Process inline code
        enhanced = self.enhance_inline_code(&enhanced);

        // Enhance headings with anchor links
        enhanced = self.enhance_headings(&enhanced);

        // Enhance tables
        enhanced = self.enhance_tables(&enhanced);

        // Enhance lists
        enhanced = self.enhance_lists(&enhanced);

        // Enhance blockquotes
        enhanced = self.enhance_blockquotes(&enhanced);

        // Enhance links and buttons
        enhanced = self.enhance_links(&enhanced);

        // Enhance task lists
        enhanced = self.enhance_task_lists(&enhanced);

        // Enhance paragraphs
        enhanced = self.enhance_paragraphs(&enhanced);

        // Add responsive containers
        self.wrap_with_responsive_container(enhanced)
    }

    /// Enhanced code block rendering with custom options
    fn enhance_code_blocks_with_options(&self, html: &str, options: &MarkdownOptions) -> String {
        let mut result = String::new();
        let mut pos = 0;
        let html_len = html.len();

        // Regex to match code blocks with language
        let code_block_regex = Regex::new(r#"<pre><code class="language-([^"]+)">([^<]+)</code></pre>"#).unwrap();

        // Process each code block
        while pos < html_len {
            if let Some(captures) = code_block_regex.captures(&html[pos..]) {
                let full_match = captures.get(0).unwrap();
                let language = captures.get(1).unwrap().as_str();
                let code_content = captures.get(2).unwrap().as_str();

                // Add content before the code block
                let before_start = pos + full_match.start();
                result.push_str(&html[pos..before_start]);

                // Create enhanced code block with custom options
                let enhanced_block = self.create_enhanced_code_block_with_options(language, code_content, options);
                result.push_str(&enhanced_block);

                pos = before_start + full_match.len();
            } else if let Some(start_pos) = html[pos..].find("<pre><code>") {
                let full_start = pos + start_pos;

                // Add content before the code block
                result.push_str(&html[pos..full_start]);

                // Handle plain code block without language
                if let Some(code_end) = html[full_start + "<pre><code>".len()..].find("</code></pre>") {
                    let code_start = full_start + "<pre><code>".len();
                    let code_end_full = code_start + code_end;
                    let code_content = &html[code_start..code_end_full];

                    let clean_code = html_escape::decode_html_entities(code_content);
                    let enhanced_block = self.create_enhanced_code_block_with_options("text", &clean_code, options);
                    result.push_str(&enhanced_block);

                    pos = code_end_full + "</code></pre>".len();
                    continue;
                } else {
                    pos = full_start + 1;
                }
            } else {
                // No more code blocks
                result.push_str(&html[pos..]);
                break;
            }
        }

        result
    }

    /// Create an enhanced code block with custom options
    fn create_enhanced_code_block_with_options(&self, language: &str, code_content: &str, options: &MarkdownOptions) -> String {
        let lang_upper = language.to_uppercase();
        let display_name = self.language_map.get(language).unwrap_or(&lang_upper);

        // Clean the code content (remove HTML entities)
        let clean_code = html_escape::decode_html_entities(code_content);

        // Use syntax highlighter with custom theme
        if let Ok(mut highlighter) = self.syntax_highlighter.lock() {
            let config = HighlightConfig {
                theme: options.syntax_theme.clone(),
                line_numbers: options.line_numbers,
                show_copy_button: options.copy_button,
                show_download_button: options.download_button,
                wrap_lines: options.wrap_lines,
                highlight_lines: options.highlight_lines.clone(),
                tab_size: 4,
            };

            match highlighter.highlight(&clean_code, language, &config) {
                Ok(highlighted_html) => {
                    // Apply DaisyUI styling with custom options
                    self.apply_daisyui_styling_with_options(&highlighted_html, display_name, options)
                }
                Err(_) => {
                    // Fallback to basic code block if syntax highlighting fails
                    self.create_basic_code_block_with_options(language, &clean_code, display_name, options)
                }
            }
        } else {
            // Fallback to basic code block if syntax highlighter is locked
            self.create_basic_code_block_with_options(language, &clean_code, display_name, options)
        }
    }

    /// Apply DaisyUI styling with custom options
    fn apply_daisyui_styling_with_options(&self, highlighted_html: &str, language_display: &str, options: &MarkdownOptions) -> String {
        let mut buttons_html = String::new();

        if options.copy_button {
            buttons_html.push_str(r#"
            <button class="btn btn-ghost btn-xs" title="Copy code">
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z"></path>
                </svg>
            </button>"#);
        }

        if options.download_button {
            buttons_html.push_str(r#"
            <button class="btn btn-ghost btn-xs ml-1" title="Download code">
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"></path>
                </svg>
            </button>"#);
        }

        format!(r#"
<div class="code-block-container group relative my-6 rounded-lg overflow-hidden border border-base-300 bg-base-100 shadow-lg">
    <!-- Header with language display -->
    <div class="flex items-center justify-between px-4 py-2 bg-base-200 border-b border-base-300">
        <div class="flex items-center space-x-2">
            <div class="w-3 h-3 rounded-full bg-red-500"></div>
            <div class="w-3 h-3 rounded-full bg-yellow-500"></div>
            <div class="w-3 h-3 rounded-full bg-green-500"></div>
            <span class="ml-3 text-sm font-medium text-base-content/70">{}</span>
        </div>
        <div class="flex items-center space-x-1 opacity-0 group-hover:opacity-100 transition-opacity duration-200">
            {}
        </div>
    </div>

    <!-- Syntax highlighted code content -->
    <div class="overflow-x-auto {}">
        {}
    </div>
</div>
"#,
            language_display,
            buttons_html,
            if options.wrap_lines { "wrap-lines" } else { "" },
            highlighted_html)
    }

    /// Create a basic code block with custom options
    fn create_basic_code_block_with_options(&self, language: &str, code_content: &str, display_name: &str, options: &MarkdownOptions) -> String {
        let escaped_code = code_content
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace("\"", "&quot;");

        // Generate a unique ID for this code block
        let block_id = format!("code-block-{}", uuid::Uuid::new_v4().simple());

        let mut buttons_html = String::new();

        if options.copy_button {
            buttons_html.push_str(&format!(r#"
            <button
                onclick="copyCodeToClipboard('{}')"
                class="opacity-0 group-hover:opacity-100 transition-opacity duration-200 btn btn-ghost btn-xs text-base-content/70 hover:text-base-content"
                title="Copy code">
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z"></path>
                </svg>
            </button>"#, block_id));
        }

        format!(r#"
<div class="code-block-container group relative my-6 rounded-lg overflow-hidden border border-base-300 bg-base-100 shadow-lg">
    <!-- Code block header -->
    <div class="flex items-center justify-between px-4 py-2 bg-base-200 border-b border-base-300">
        <div class="flex items-center space-x-2">
            <div class="w-3 h-3 rounded-full bg-red-500"></div>
            <div class="w-3 h-3 rounded-full bg-yellow-500"></div>
            <div class="w-3 h-3 rounded-full bg-green-500"></div>
            <span class="ml-3 text-sm font-medium text-base-content/70">{}</span>
        </div>
        <div class="flex items-center space-x-2">
            {}
        </div>
    </div>

    <!-- Code content -->
    <div class="overflow-x-auto {}">
        <pre class="p-4 m-0 text-sm leading-relaxed bg-base-100"><code id="{}" class="language-{} text-base-content">{}</code></pre>
    </div>
</div>

<script>
function copyCodeToClipboard(blockId) {{
    const codeElement = document.getElementById(blockId);
    const text = codeElement.textContent || codeElement.innerText;

    navigator.clipboard.writeText(text).then(() => {{
        // Visual feedback
        const button = event.currentTarget;
        const originalHTML = button.innerHTML;
        button.innerHTML = '<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path></svg>';
        button.classList.add('text-success');

        setTimeout(() => {{
            button.innerHTML = originalHTML;
            button.classList.remove('text-success');
        }}, 2000);
    }}).catch(err => {{
        console.error('Failed to copy code: ', err);
    }});
}}
</script>
"#,
            display_name,
            buttons_html,
            if options.wrap_lines { "wrap-lines" } else { "" },
            block_id,
            language,
            escaped_code
        )
    }

    /// Enhanced code block rendering with copy button and language display
    fn enhance_code_blocks(&self, html: &str) -> String {
        let mut result = String::new();
        let mut pos = 0;
        let html_len = html.len();

        // Regex to match code blocks with language
        let code_block_regex = Regex::new(r#"<pre><code class="language-([^"]+)">([^<]+)</code></pre>"#).unwrap();

        // Process each code block
        while pos < html_len {
            if let Some(captures) = code_block_regex.captures(&html[pos..]) {
                let full_match = captures.get(0).unwrap();
                let language = captures.get(1).unwrap().as_str();
                let code_content = captures.get(2).unwrap().as_str();

                // Add content before the code block
                let before_start = pos + full_match.start();
                result.push_str(&html[pos..before_start]);

                // Create enhanced code block
                let enhanced_block = self.create_enhanced_code_block(language, code_content);
                result.push_str(&enhanced_block);

                pos = before_start + full_match.len();
            } else if let Some(start_pos) = html[pos..].find("<pre><code>") {
                let full_start = pos + start_pos;

                // Add content before the code block
                result.push_str(&html[pos..full_start]);

                // Handle plain code block without language
                if let Some(code_end) = html[full_start + "<pre><code>".len()..].find("</code></pre>") {
                    let code_start = full_start + "<pre><code>".len();
                    let code_end_full = code_start + code_end;
                    let code_content = &html[code_start..code_end_full];

                    let clean_code = html_escape::decode_html_entities(code_content);
                    let enhanced_block = self.create_enhanced_code_block("text", &clean_code);
                    result.push_str(&enhanced_block);

                    pos = code_end_full + "</code></pre>".len();
                    continue;
                } else {
                    pos = full_start + 1;
                }
            } else {
                // No more code blocks
                result.push_str(&html[pos..]);
                break;
            }
        }

        result
    }

    /// Create an enhanced code block component with syntax highlighting
    fn create_enhanced_code_block(&self, language: &str, code_content: &str) -> String {
        let lang_upper = language.to_uppercase();
        let display_name = self.language_map.get(language).unwrap_or(&lang_upper);

        // Clean the code content (remove HTML entities)
        let clean_code = html_escape::decode_html_entities(code_content);

        // Use syntax highlighter if available, fallback to basic styling
        if let Ok(mut highlighter) = self.syntax_highlighter.lock() {
            let config = HighlightConfig {
                theme: "Material".to_string(),
                line_numbers: true,
                show_copy_button: true,
                show_download_button: true,
                wrap_lines: false,
                highlight_lines: Vec::new(),
                tab_size: 4,
            };

            match highlighter.highlight(&clean_code, language, &config) {
                Ok(highlighted_html) => {
                    // Apply DaisyUI styling to the syntax highlighted HTML
                    self.apply_daisyui_styling(&highlighted_html, display_name)
                }
                Err(_) => {
                    // Fallback to basic code block if syntax highlighting fails
                    self.create_basic_code_block(language, &clean_code, display_name)
                }
            }
        } else {
            // Fallback to basic code block if syntax highlighter is locked
            self.create_basic_code_block(language, &clean_code, display_name)
        }
    }

    /// Apply DaisyUI styling to syntax highlighted HTML
    fn apply_daisyui_styling(&self, highlighted_html: &str, language_display: &str) -> String {
        // Wrap the syntax highlighted HTML with DaisyUI classes
        format!(r#"
<div class="code-block-container group relative my-6 rounded-lg overflow-hidden border border-base-300 bg-base-100 shadow-lg">
    <!-- Header with language display -->
    <div class="flex items-center justify-between px-4 py-2 bg-base-200 border-b border-base-300">
        <div class="flex items-center space-x-2">
            <div class="w-3 h-3 rounded-full bg-red-500"></div>
            <div class="w-3 h-3 rounded-full bg-yellow-500"></div>
            <div class="w-3 h-3 rounded-full bg-green-500"></div>
            <span class="ml-3 text-sm font-medium text-base-content/70">{}</span>
        </div>
        <div class="flex items-center space-x-1 opacity-0 group-hover:opacity-100 transition-opacity duration-200">
            <button class="btn btn-ghost btn-xs" title="Copy code">
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z"></path>
                </svg>
            </button>
            <button class="btn btn-ghost btn-xs ml-1" title="Download code">
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"></path>
                </svg>
            </button>
        </div>
    </div>

    <!-- Syntax highlighted code content -->
    <div class="overflow-x-auto">
        {}
    </div>
</div>
"#, language_display, highlighted_html)
    }

    /// Create a basic code block without syntax highlighting (fallback)
    fn create_basic_code_block(&self, language: &str, code_content: &str, display_name: &str) -> String {
        let escaped_code = code_content
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace("\"", "&quot;");

        // Generate a unique ID for this code block
        let block_id = format!("code-block-{}", uuid::Uuid::new_v4().simple());

        format!(r#"
<div class="code-block-container group relative my-6 rounded-lg overflow-hidden border border-base-300 bg-base-100 shadow-lg">
    <!-- Code block header -->
    <div class="flex items-center justify-between px-4 py-2 bg-base-200 border-b border-base-300">
        <div class="flex items-center space-x-2">
            <div class="w-3 h-3 rounded-full bg-red-500"></div>
            <div class="w-3 h-3 rounded-full bg-yellow-500"></div>
            <div class="w-3 h-3 rounded-full bg-green-500"></div>
            <span class="ml-3 text-sm font-medium text-base-content/70">{}</span>
        </div>
        <div class="flex items-center space-x-2">
            <button
                onclick="copyCodeToClipboard('{}')"
                class="opacity-0 group-hover:opacity-100 transition-opacity duration-200 btn btn-ghost btn-xs text-base-content/70 hover:text-base-content"
                title="Copy code">
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z"></path>
                </svg>
            </button>
        </div>
    </div>

    <!-- Code content -->
    <div class="overflow-x-auto">
        <pre class="p-4 m-0 text-sm leading-relaxed bg-base-100"><code id="{}" class="language-{} text-base-content">{}</code></pre>
    </div>
</div>

<script>
function copyCodeToClipboard(blockId) {{
    const codeElement = document.getElementById(blockId);
    const text = codeElement.textContent || codeElement.innerText;

    navigator.clipboard.writeText(text).then(() => {{
        // Visual feedback
        const button = event.currentTarget;
        const originalHTML = button.innerHTML;
        button.innerHTML = '<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path></svg>';
        button.classList.add('text-success');

        setTimeout(() => {{
            button.innerHTML = originalHTML;
            button.classList.remove('text-success');
        }}, 2000);
    }}).catch(err => {{
        console.error('Failed to copy code: ', err);
    }});
}}
</script>
"#,
            display_name,
            block_id,
            block_id,
            language,
            escaped_code
        )
    }

    /// Enhance inline code with better styling
    fn enhance_inline_code(&self, html: &str) -> String {
        let re = Regex::new(r#"<code>([^<]+)</code>"#).unwrap();
        re.replace_all(html, r#"<code class="px-1.5 py-0.5 bg-$bg_secondary text-$accent font-mono text-sm rounded">$1</code>"#)
            .to_string()
            .replace("$bg_secondary", &self.theme_colors.bg_secondary)
            .replace("$accent", &self.theme_colors.accent)
    }

    /// Enhance headings with anchor links and better styling
    fn enhance_headings(&self, html: &str) -> String {
        let mut enhanced = html.to_string();

        // Add heading styles with anchor links
        enhanced = enhanced.replace("<h1>", "<h1 class=\"text-4xl md:text-5xl font-bold mb-6 mt-8 text-$text_primary border-b border-$border pb-4\">");
        enhanced = enhanced.replace("<h2>", "<h2 class=\"text-3xl md:text-4xl font-bold mb-5 mt-7 text-$text_primary\">");
        enhanced = enhanced.replace("<h3>", "<h3 class=\"text-2xl md:text-3xl font-bold mb-4 mt-6 text-$text_primary\">");
        enhanced = enhanced.replace("<h4>", "<h4 class=\"text-xl md:text-2xl font-bold mb-3 mt-5 text-$text_primary\">");
        enhanced = enhanced.replace("<h5>", "<h5 class=\"text-lg md:text-xl font-bold mb-2 mt-4 text-$text_primary\">");
        enhanced = enhanced.replace("<h6>", "<h6 class=\"text-base md:text-lg font-bold mb-2 mt-4 text-$text_secondary\">");

        // Replace color placeholders
        enhanced.replace("$text_primary", &self.theme_colors.text_primary)
            .replace("$text_secondary", &self.theme_colors.text_secondary)
            .replace("$border", &self.theme_colors.border)
    }

    /// Enhance tables with modern styling
    fn enhance_tables(&self, html: &str) -> String {
        html.replace("<table>", r#"<div class="overflow-x-auto my-6"><table class="min-w-full divide-y divide-$border table-zebra">"#)
            .replace("</table>", "</table></div>")
            .replace("<thead>", "<thead class=\"bg-$bg_secondary\">")
            .replace("$bg_secondary", &self.theme_colors.bg_secondary)
            .replace("$border", &self.theme_colors.border)
    }

    /// Enhance lists with better styling
    fn enhance_lists(&self, html: &str) -> String {
        let mut enhanced = html.to_string();

        // Only style non-task lists
        if enhanced.contains("<ul>") && !enhanced.contains("type=\"checkbox\"") {
            enhanced = enhanced.replace("<ul>", "<ul class=\"space-y-2 my-4\">");
        }

        enhanced = enhanced.replace("<ol>", "<ol class=\"list-decimal list-inside space-y-2 my-4\">");
        enhanced = enhanced.replace("<li>", "<li class=\"text-$text_primary leading-relaxed\">");

        enhanced.replace("$text_primary", &self.theme_colors.text_primary)
    }

    /// Enhance blockquotes with modern styling
    fn enhance_blockquotes(&self, html: &str) -> String {
        html.replace("<blockquote>", r#"<blockquote class="border-l-4 border-$accent bg-$bg_secondary pl-6 py-4 my-6 rounded-r-lg italic text-$text_secondary">"#)
            .replace("$accent", &self.theme_colors.accent)
            .replace("$bg_secondary", &self.theme_colors.bg_secondary)
            .replace("$text_secondary", &self.theme_colors.text_secondary)
    }

    /// Enhance links with button styling for external links
    fn enhance_links(&self, html: &str) -> String {
        html.replace("<a href=", r#"<a class="text-$accent hover:text-$accent/80 underline decoration-2 underline-offset-4 font-medium transition-colors duration-200" href="#)
            .replace("$accent", &self.theme_colors.accent)
    }

    /// Enhance task lists with custom checkbox styling
    fn enhance_task_lists(&self, html: &str) -> String {
        let mut enhanced = html.to_string();

        enhanced = enhanced.replace(
            r#"<input type="checkbox" disabled="" checked="" />"#,
            r#"<input type="checkbox" class="checkbox checkbox-primary checkbox-sm mr-2" checked disabled />"#,
        ).replace(
            r#"<input type="checkbox" disabled="" />"#,
            r#"<input type="checkbox" class="checkbox checkbox-primary checkbox-sm mr-2" disabled />"#,
        );

        // Style task list items
        if enhanced.contains("checkbox") {
            enhanced = enhanced.replace("<ul", "<ul class=\"space-y-2 my-4\"");
            enhanced = enhanced.replace("<li>", "<li class=\"flex items-start text-$text_primary\">");
        }

        enhanced.replace("$text_primary", &self.theme_colors.text_primary)
    }

    /// Enhance paragraphs with better spacing
    fn enhance_paragraphs(&self, html: &str) -> String {
        html.replace("<p>", "<p class=\"mb-4 leading-relaxed text-$text_primary\">")
            .replace("$text_primary", &self.theme_colors.text_primary)
    }

    /// Wrap content with responsive container
    fn wrap_with_responsive_container(&self, html: String) -> String {
        format!(r#"<div class="prose prose-lg max-w-none prose-headings:text-{text_primary} prose-p:text-{text_primary} prose-li:text-{text_primary} prose-blockquote:text-{text_secondary} prose-code:text-{accent} prose-pre:text-{text_primary}">{}</div>"#,
            html,
            text_primary = self.theme_colors.text_primary,
            text_secondary = self.theme_colors.text_secondary,
            accent = self.theme_colors.accent
        )
    }
}

impl Default for EnhancedMarkdownRenderer {
    fn default() -> Self {
        Self::new().expect("Failed to initialize EnhancedMarkdownRenderer")
    }
}

/// Convert markdown to enhanced HTML with user preferences
pub fn markdown_to_html_with_user_prefs(
    markdown: &str,
    use_enhanced: bool,
    syntax_theme: &str,
    line_numbers: bool,
    wrap_lines: bool,
) -> String {
    if !use_enhanced {
        // Fall back to basic markdown if enhanced is disabled
        let html = to_html(markdown);
        return crate::utils::add_daisyui_classes(&html);
    }

    // Create enhanced renderer with user preferences
    let renderer = EnhancedMarkdownRenderer::new()
        .unwrap_or_else(|_| EnhancedMarkdownRenderer::default());

    // Create options with user preferences
    let options = MarkdownOptions {
        syntax_theme: syntax_theme.to_string(),
        line_numbers,
        wrap_lines,
        copy_button: true,
        download_button: true,
        highlight_lines: Vec::new(),
    };

    // Render with custom options
    renderer.render_with_options(markdown, &options)
}

/// Convert markdown to enhanced HTML (convenience function)
pub fn markdown_to_enhanced_html(markdown: &str) -> String {
    let renderer = EnhancedMarkdownRenderer::new()
        .unwrap_or_else(|_| EnhancedMarkdownRenderer::default());
    renderer.render(markdown)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enhanced_markdown_renderer() {
        let renderer = EnhancedMarkdownRenderer::new()
            .unwrap_or_else(|_| EnhancedMarkdownRenderer::default());
        let markdown = r#"# Hello World

This is a **test** with `inline code` and a code block:

```rust
fn main() {
    println!("Hello, Rust!");
}
```

## Features

- [ ] Task 1
- [x] Task 2

> A blockquote with **important** information."#;

        let html = renderer.render(markdown);

        assert!(html.contains("Hello World"));
        assert!(html.contains("code-block-container"));
        assert!(html.contains("Rust"));
        assert!(html.contains("checkbox checkbox-primary"));
        assert!(html.contains("border-l-4"));
    }

    #[test]
    fn test_language_map() {
        let renderer = EnhancedMarkdownRenderer::new()
            .unwrap_or_else(|_| EnhancedMarkdownRenderer::default());
        assert_eq!(renderer.language_map.get("rs"), Some(&"Rust".to_string()));
        assert_eq!(renderer.language_map.get("js"), Some(&"JavaScript".to_string()));
        assert_eq!(renderer.language_map.get("unknown"), None);
    }

    #[test]
    fn test_convenience_function() {
        let markdown = "```rust\nlet x = 42;\n```";
        let html = markdown_to_enhanced_html(markdown);
        assert!(html.contains("Rust"));
        assert!(html.contains("let x = 42;"));
    }
}